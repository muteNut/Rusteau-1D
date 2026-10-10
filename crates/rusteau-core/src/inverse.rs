//! Inverse solution (parameter estimation) as in HYDRUS-1D: the Fit.in settings, the objective function with the
//! HYDRUS weighting of the measurement sets, the Marquardt-Levenberg optimization and the statistics of Fit.out.
//!
//! The 2009 Fortran source has no inverse code; the algorithm is reconstructed from the manual (chapter 9, Table 9.1,
//! Table 12.10) and from the Fit.out files of the HYDRUS 4.17 inverse examples:
//!
//! * the model is evaluated at the observation times exactly (they are fixed times of the time step control, like print
//!   times: the final runs of 4.17 have time levels at every observation time);
//! * objective `SSQ = Σ (W_i (FO_i - F_i))²` with `W_i = w_i v_j`; `v_j` of measurement set `j` is 1 (iWeight 0),
//!   `min_k(mean_k) / mean_j` of the absolute values (1, weighting by mean ratio) or `1 / (sqrt(n_j) σ_j)` with the
//!   population standard deviation of the absolute values (2, by standard deviation); retention points (type 5) are
//!   one set with the water contents (type 2), sets of one value or without scatter get 1;
//! * Marquardt-Levenberg with forward-difference derivatives, scaling to a unit diagonal, parameter bounds;
//!   convergence when no parameter changes by more than 1 % in an iteration; not in HYDRUS (on unless
//!   `InverseSettings::hydrus_stop`): a stop is confirmed by one more iteration with other derivative increments,
//!   which goes on only if it lowers the objective by more than 1 % (noisy models reach lower minima);
//! * R² of the weighted regression of fitted on observed values, ME = Σ W r / n, MAE = Σ |W r| / n,
//!   RMSE = sqrt(SSQ / (n - 1)), AIC = n ln(SSQ₀/n) + 2p and BIC = n ln(SSQ₀/n) + p ln n with SSQ₀ the objective at the
//!   start of the last iteration, standard errors from the inverse of the scaled normal matrix and s² = SSQ / (n - p),
//!   95 % limits with Student's t for n - p degrees of freedom.

use crate::error::HydrusError;
use crate::material::{fk, fq, par_of};
use crate::model::*;
use crate::output::{BalanceOut, Results, SubRegion};
use crate::sim::{Simulation, StepStatus};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// Record of a parameter in Fit.in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ParamGroup {
    /// Soil hydraulic parameters Par(1..10, M)
    Water,
    /// Solute transport parameters ChPar(1..20, M)
    Solute,
    /// Heat transport parameters TPar(1..9, M): Qn, Qo, Disper., b1, b2, b3, Cn, Co, Cw (HYDRUS 4.17, lTempF)
    Heat,
}

/// One parameter of Fit.in (fitted or kept constant).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FitParameter {
    pub group: ParamGroup,
    /// 1-based position in its record: Par(index, M), ChPar(index, M) or TPar(index, M)
    pub index: usize,
    /// 0-based material
    pub mat: usize,
    /// Initial estimate (the value used when not fitted)
    pub value: f64,
    pub fit: bool,
    /// Constraints; a value of 0 means no constraint (HYDRUS), an upper limit not above the lower one is ignored
    pub min: f64,
    pub max: f64,
}

impl FitParameter {
    /// Lower and upper constraint actually applied.
    pub fn bounds(&self) -> (Option<f64>, Option<f64>) {
        let lo = (self.min != 0.0).then_some(self.min);
        let hi = (self.max != 0.0 && lo.is_none_or(|l| self.max > l)).then_some(self.max);
        (lo, hi)
    }
}

/// One measured value of Fit.in: `HO(i) FO(i) iType(i) iPos(i) Weight(i)`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Observation {
    /// Time (types 0-4), pressure head (5, 6), depth (12-15) or dummy
    pub x: f64,
    /// Measured value
    pub value: f64,
    pub kind: i32,
    pub pos: i32,
    pub weight: f64,
}

impl Observation {
    /// Types evaluated at a time of the simulation.
    pub fn is_timed(&self) -> bool {
        (0..=4).contains(&self.kind)
    }
    /// Measurement set used for the weighting: retention points are weighted with the water contents.
    pub fn weight_set(&self) -> i32 {
        if self.kind == 5 {
            2
        } else {
            self.kind
        }
    }
    /// Short description of the observed quantity.
    pub fn describe(&self) -> String {
        match (self.kind, self.pos) {
            (0, 1) => "cumulative top flux".into(),
            (0, _) => "cumulative bottom flux".into(),
            (1, p) => format!("h at obs. node {p}"),
            (2, 0) => "mean water content".into(),
            (2, p) if p < 0 => format!("mean water content, sub-region {}", -p),
            (2, p) => format!("θ at obs. node {p}"),
            (3, 1) => "top flux".into(),
            (3, _) => "bottom flux".into(),
            (4, 0) => "solute mass in the profile".into(),
            (4, p) => format!("c (T when heat parameters are fitted) at obs. node {p}"),
            (5, m) => format!("θ(h), material {m}"),
            (6, m) => format!("K(h), material {m}"),
            (7, m) => format!("α, material {m}"),
            (8, m) => format!("n, material {m}"),
            (9, m) => format!("θr, material {m}"),
            (10, m) => format!("θs, material {m}"),
            (11, m) => format!("Ks, material {m}"),
            (12, p) => format!("h(x) at print time {p}"),
            (13, p) => format!("θ(x) at print time {p}"),
            (14, p) => format!("c(x) at print time {p}"),
            (15, p) => format!("sorbed c(x) at print time {p}"),
            (k, p) => format!("type {k}, position {p}"),
        }
    }
}

/// Content of Fit.in (Block J of the manual).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct InverseSettings {
    pub title1: String,
    pub title2: String,
    /// MIT
    pub max_iter: usize,
    /// iWeight: 0 no internal weighting, 1 weighting by mean ratio, 2 weighting by standard deviation
    pub weighting: i32,
    /// lWatF, lChemF, lTempF
    pub fit_water: bool,
    pub fit_solute: bool,
    pub fit_heat: bool,
    pub n_mat: usize,
    /// iModel and iHyst of the water parameter records
    pub model: i32,
    pub hysteresis: i32,
    /// Hysteresis constraints: iQSame (0: θsd > θsw, 1: θm = θs, 2: θm = θsw = θs), αw = 2α, Ksw = Ks, iKappa
    pub q_same: i32,
    pub alpha_w_2d: bool,
    pub ks_same: bool,
    pub kappa: i32,
    /// NS and iConcType (0 resident, 1 log resident, 2 flux, 3 total, 4 mobile + immobile concentration)
    pub n_solutes: usize,
    pub conc_type: i32,
    /// Number of values per water record (6 or 10)
    pub n_water_par: usize,
    pub params: Vec<FitParameter>,
    pub observations: Vec<Observation>,
    /// Stop as HYDRUS 4.17 does (not part of Fit.in). Otherwise a stop (converged or no further reduction) is
    /// confirmed by one more iteration with derivatives from other increments, see [`MarqOptions::confirm`].
    #[serde(default)]
    pub hydrus_stop: bool,
    /// Additional starting points (not part of Fit.in): the estimation is repeated from this many initial estimates
    /// spread over the parameter space, and the best result is kept, see [`optimize`]. 0 runs HYDRUS' single
    /// estimation.
    #[serde(default = "default_extra_starts")]
    pub extra_starts: usize,
}

/// Default number of additional starting points.
pub const DEFAULT_EXTRA_STARTS: usize = 8;

fn default_extra_starts() -> usize {
    DEFAULT_EXTRA_STARTS
}

impl Default for InverseSettings {
    fn default() -> Self {
        InverseSettings {
            title1: String::new(),
            title2: String::new(),
            max_iter: 50,
            weighting: 2,
            fit_water: true,
            fit_solute: false,
            fit_heat: false,
            n_mat: 1,
            model: 0,
            hysteresis: 0,
            q_same: 1,
            alpha_w_2d: false,
            ks_same: false,
            kappa: -1,
            n_solutes: 1,
            conc_type: 0,
            n_water_par: 6,
            params: vec![],
            observations: vec![],
            hydrus_stop: false,
            extra_starts: DEFAULT_EXTRA_STARTS,
        }
    }
}

pub const WATER_NAMES: [&str; 6] = ["WCR", "WCS", "ALPHA", "N", "CONDS", "L"];
pub const WATER_HYST_NAMES: [&str; 4] = ["WCM", "WCSW", "AlphaW", "CONDSW"];
pub const WATER_MVG_NAMES: [&str; 4] = ["WCM", "WCA", "WCK", "CONDK"];
/// Names of the heat parameters in Fit.out
pub const HEAT_NAMES: [&str; 9] = ["Qn", "Qo", "DisperH", "B1", "B2", "B3", "Cn", "Co", "Cw"];
/// Column labels of the heat records of Fit.in
pub const HEAT_LABELS: [&str; 9] = ["Qn", "Qo", "Disper.", "B1", "B2", "B3", "Cn", "Co", "Cw"];
pub const SOLUTE_NAMES: [&str; 20] = [
    "BULK.D.", "DISPER.", "FRAC", "THIMOB", "DIFW", "DIFG", "KD", "NU", "BETA", "HENRY", "SNKL1", "SNKS1", "SNKG1", "SNKL1`",
    "SNKS1`", "SNKG1`", "SNKL0", "SNKS0", "SNKG0", "ALPHA",
];
/// Column labels of the Fit.in records
pub const WATER_LABELS: [&str; 10] = ["thr", "ths", "Alfa", "n", "Ks", "l", "thm", "thsW", "AlfaW", "KsW"];
pub const WATER_MVG_LABELS: [&str; 4] = ["thm", "tha", "thk", "Kk"];
pub const SOLUTE_LABELS: [&str; 20] = [
    "Bulk.d.", "DisperL.", "Frac", "MobileWC", "DifW", "DifG", "Ks", "Nu", "Beta", "Henry", "SnkL1", "SnkS1", "SnkG1", "SnkL1'",
    "SnkS1'", "SnkG1'", "SnkL0", "SnkS0", "SnkG0", "Alfa",
];

impl InverseSettings {
    /// Parameter name as printed in Fit.out.
    pub fn param_name(&self, p: &FitParameter) -> String {
        match p.group {
            ParamGroup::Water => match p.index {
                1..=6 => WATER_NAMES[p.index - 1].into(),
                7..=10 if self.hysteresis > 0 && self.model != 1 => WATER_HYST_NAMES[p.index - 7].into(),
                7..=10 => WATER_MVG_NAMES[p.index - 7].into(),
                k => format!("PAR{k}"),
            },
            ParamGroup::Solute => SOLUTE_NAMES.get(p.index.wrapping_sub(1)).map(|s| s.to_string()).unwrap_or_else(|| format!("CHPAR{}", p.index)),
            ParamGroup::Heat => HEAT_NAMES.get(p.index.wrapping_sub(1)).map(|s| s.to_string()).unwrap_or_else(|| format!("TPAR{}", p.index)),
        }
    }

    /// Observations of type 4 are temperatures when heat parameters are fitted (HYDRUS 4.17 estimates either solute
    /// or heat transport parameters, and both use the "Time - Conc" data type).
    pub fn observes_temperature(&self) -> bool {
        self.fit_heat
    }

    /// Indices (into `params`) of the fitted parameters, in the order of Fit.in.
    pub fn fitted(&self) -> Vec<usize> {
        self.params.iter().enumerate().filter(|(_, p)| p.fit && self.group_active(p.group)).map(|(i, _)| i).collect()
    }

    pub fn group_active(&self, g: ParamGroup) -> bool {
        match g {
            ParamGroup::Water => self.fit_water,
            ParamGroup::Solute => self.fit_solute,
            ParamGroup::Heat => self.fit_heat,
        }
    }

    /// Fit.in from a project: all hydraulic (and, with solute or heat transport, transport) parameters of the materials
    /// with their current values, nothing fitted, no observations.
    pub fn from_project(prj: &Project) -> InverseSettings {
        let mut s = InverseSettings {
            title1: prj.title.clone(),
            title2: "Parameter Estimation".into(),
            n_mat: prj.water.materials.len(),
            model: match prj.water.model {
                SoilModel::ModifiedVG => 1,
                SoilModel::BrooksCorey => 2,
                _ => 0,
            },
            hysteresis: match prj.water.hysteresis {
                Hysteresis::Retention => 1,
                Hysteresis::RetentionAndConductivity => 2,
                _ => 0,
            },
            fit_solute: false,
            n_solutes: prj.solute.species.len().max(1),
            ..Default::default()
        };
        s.n_water_par = if s.hysteresis > 0 || s.model == 1 { 10 } else { 6 };
        s.kappa = prj.water.init_kappa;
        if s.hysteresis > 0 {
            s.q_same = prj.water.materials.first().map(|m| if (m.qm - m.qs).abs() < 1e-12 { 1 } else { 0 }).unwrap_or(1);
        }
        let mut tmp = s.clone();
        for m in 0..s.n_mat {
            for k in 1..=s.n_water_par {
                let p = FitParameter { group: ParamGroup::Water, index: k, mat: m, value: 0.0, fit: false, min: 0.0, max: 0.0 };
                let v = tmp.get_value(prj, &p);
                tmp.params.push(FitParameter { value: v, ..p });
            }
        }
        if prj.processes.solute && !prj.solute.species.is_empty() {
            for m in 0..s.n_mat {
                for k in 1..=20 {
                    let p = FitParameter { group: ParamGroup::Solute, index: k, mat: m, value: 0.0, fit: false, min: 0.0, max: 0.0 };
                    let v = tmp.get_value(prj, &p);
                    tmp.params.push(FitParameter { value: v, ..p });
                }
            }
        }
        if prj.processes.heat {
            for m in 0..s.n_mat {
                for k in 1..=9 {
                    let p = FitParameter { group: ParamGroup::Heat, index: k, mat: m, value: 0.0, fit: false, min: 0.0, max: 0.0 };
                    let v = tmp.get_value(prj, &p);
                    tmp.params.push(FitParameter { value: v, ..p });
                }
            }
        }
        s.params = tmp.params;
        s
    }

    /// Current value of a parameter in the project.
    pub fn get_value(&self, prj: &Project, p: &FitParameter) -> f64 {
        match p.group {
            ParamGroup::Water => {
                let Some(m) = prj.water.materials.get(p.mat) else { return p.value };
                match p.index {
                    1 => m.qr,
                    2 => m.qs,
                    3 => m.alpha,
                    4 => m.n,
                    5 => m.ks,
                    6 => m.l,
                    7..=10 if self.hysteresis > 0 && self.model != 1 => [m.qm, m.qs_w, m.alpha_w, m.ks_w][p.index - 7],
                    7..=10 => m.extra[p.index - 7],
                    _ => p.value,
                }
            }
            ParamGroup::Solute => {
                let sm = prj.solute.materials.get(p.mat);
                let sp = prj.solute.species.first();
                let pm = sp.and_then(|s| s.per_material.get(p.mat));
                match p.index {
                    1 => sm.map(|m| m.bulk_density),
                    2 => sm.map(|m| m.disp_l),
                    3 => sm.map(|m| m.frac),
                    4 => sm.map(|m| m.th_immobile),
                    5 => sp.map(|s| s.diff_w),
                    6 => sp.map(|s| s.diff_g),
                    7 => pm.map(|q| q.ks),
                    8 => pm.map(|q| q.nu),
                    9 => pm.map(|q| q.beta),
                    10 => pm.map(|q| q.henry),
                    11 => pm.map(|q| q.mu_w),
                    12 => pm.map(|q| q.mu_s),
                    13 => pm.map(|q| q.mu_g),
                    14 => pm.map(|q| q.gam_w),
                    15 => pm.map(|q| q.gam_s),
                    16 => pm.map(|q| q.gam_g),
                    17 => pm.map(|q| q.mu0_w),
                    18 => pm.map(|q| q.mu0_s),
                    19 => pm.map(|q| q.mu0_g),
                    20 => pm.map(|q| q.omega),
                    _ => None,
                }
                .unwrap_or(p.value)
            }
            ParamGroup::Heat => {
                let Some(m) = prj.heat.materials.get(p.mat) else { return p.value };
                match p.index {
                    1 => m.qn,
                    2 => m.qo,
                    3 => m.disp,
                    4 => m.b1,
                    5 => m.b2,
                    6 => m.b3,
                    7 => m.cn,
                    8 => m.co,
                    9 => m.cw,
                    _ => p.value,
                }
            }
        }
    }

    pub fn set_value(&self, prj: &mut Project, p: &FitParameter, v: f64) {
        match p.group {
            ParamGroup::Water => {
                let hyst = self.hysteresis > 0 && self.model != 1;
                let Some(m) = prj.water.materials.get_mut(p.mat) else { return };
                match p.index {
                    1 => m.qr = v,
                    2 => m.qs = v,
                    3 => m.alpha = v,
                    4 => m.n = v,
                    5 => m.ks = v,
                    6 => m.l = v,
                    7 if hyst => m.qm = v,
                    8 if hyst => m.qs_w = v,
                    9 if hyst => m.alpha_w = v,
                    10 if hyst => m.ks_w = v,
                    7..=10 => m.extra[p.index - 7] = v,
                    _ => {}
                }
            }
            ParamGroup::Solute => {
                if let Some(sm) = prj.solute.materials.get_mut(p.mat) {
                    match p.index {
                        1 => sm.bulk_density = v,
                        2 => sm.disp_l = v,
                        3 => sm.frac = v,
                        4 => sm.th_immobile = v,
                        _ => {}
                    }
                }
                let Some(sp) = prj.solute.species.first_mut() else { return };
                match p.index {
                    5 => sp.diff_w = v,
                    6 => sp.diff_g = v,
                    _ => {}
                }
                let Some(q) = sp.per_material.get_mut(p.mat) else { return };
                match p.index {
                    7 => q.ks = v,
                    8 => q.nu = v,
                    9 => q.beta = v,
                    10 => q.henry = v,
                    11 => q.mu_w = v,
                    12 => q.mu_s = v,
                    13 => q.mu_g = v,
                    14 => q.gam_w = v,
                    15 => q.gam_s = v,
                    16 => q.gam_g = v,
                    17 => q.mu0_w = v,
                    18 => q.mu0_s = v,
                    19 => q.mu0_g = v,
                    20 => q.omega = v,
                    _ => {}
                }
            }
            ParamGroup::Heat => {
                let Some(m) = prj.heat.materials.get_mut(p.mat) else { return };
                match p.index {
                    1 => m.qn = v,
                    2 => m.qo = v,
                    3 => m.disp = v,
                    4 => m.b1 = v,
                    5 => m.b2 = v,
                    6 => m.b3 = v,
                    7 => m.cn = v,
                    8 => m.co = v,
                    9 => m.cw = v,
                    _ => {}
                }
            }
        }
    }

    /// Writes the parameters into the project: every parameter of an active record takes its Fit.in value, the fitted
    /// ones the values of `fitted` (in the order of [`InverseSettings::fitted`]); then the hysteresis constraints.
    pub fn apply(&self, prj: &mut Project, fitted: &[f64]) {
        let idx = self.fitted();
        for (k, p) in self.params.iter().enumerate() {
            if !self.group_active(p.group) {
                continue;
            }
            let v = idx.iter().position(|&i| i == k).and_then(|j| fitted.get(j).copied()).unwrap_or(p.value);
            self.set_value(prj, p, v);
        }
        if self.fit_water && self.hysteresis > 0 && self.model != 1 {
            for m in prj.water.materials.iter_mut() {
                match self.q_same {
                    1 => m.qm = m.qs,
                    2 => {
                        m.qm = m.qs;
                        m.qs_w = m.qs;
                    }
                    _ => {}
                }
                if self.alpha_w_2d {
                    m.alpha_w = 2.0 * m.alpha;
                }
                if self.ks_same {
                    m.ks_w = m.ks;
                }
            }
        }
    }

    /// Initial estimates of the fitted parameters.
    pub fn initial(&self) -> Vec<f64> {
        self.fitted().iter().map(|&i| self.params[i].value).collect()
    }

    /// Effective weight `W_i = w_i v_j` of every observation (the "Weight" column of Fit.out).
    pub fn effective_weights(&self) -> Vec<f64> {
        let mut sets: Vec<i32> = self.observations.iter().map(|o| o.weight_set()).collect();
        sets.sort_unstable();
        sets.dedup();
        let stat = |set: i32| {
            let v: Vec<f64> = self.observations.iter().filter(|o| o.weight_set() == set).map(|o| o.value.abs()).collect();
            let n = v.len() as f64;
            let mean = v.iter().sum::<f64>() / n.max(1.0);
            let var = v.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / n.max(1.0);
            (v.len(), mean, var.sqrt())
        };
        let v_set: Vec<(i32, f64)> = match self.weighting {
            1 => {
                let means: Vec<(i32, f64)> = sets.iter().map(|&s| (s, stat(s).1)).collect();
                let min_mean = means.iter().map(|m| m.1).filter(|m| *m > 0.0).fold(f64::INFINITY, f64::min);
                means.into_iter().map(|(s, m)| (s, if m > 0.0 && min_mean.is_finite() { min_mean / m } else { 1.0 })).collect()
            }
            2 => sets
                .iter()
                .map(|&s| {
                    let (n, _, sd) = stat(s);
                    (s, if n > 1 && sd > 0.0 { 1.0 / ((n as f64).sqrt() * sd) } else { 1.0 })
                })
                .collect(),
            _ => sets.iter().map(|&s| (s, 1.0)).collect(),
        };
        self.observations
            .iter()
            .map(|o| o.weight * v_set.iter().find(|(s, _)| *s == o.weight_set()).map(|x| x.1).unwrap_or(1.0))
            .collect()
    }

    /// Checks of the settings against the project.
    pub fn validate(&self, prj: &Project) -> Vec<String> {
        let mut e = vec![];
        if self.fit_heat && self.fit_solute {
            e.push("Solute and heat transport parameters cannot be estimated together (HYDRUS takes the concentration data type for temperatures).".into());
        }
        if self.fit_heat && !prj.processes.heat {
            e.push("Heat parameters are fitted but heat transport is not simulated.".into());
        }
        let nf = self.fitted().len();
        if nf == 0 {
            e.push("No parameter is selected for fitting.".into());
        }
        if self.observations.len() <= nf {
            e.push(format!("{} observations are not enough for {} fitted parameters.", self.observations.len(), nf));
        }
        let n_obs_nodes = prj.profile.observation_nodes.len() as i32;
        for (i, o) in self.observations.iter().enumerate() {
            let bad = match o.kind {
                0 | 3 => !(1..=6).contains(&o.pos),
                1 => o.pos == 0 || o.pos.abs() > n_obs_nodes,
                2 => o.pos > 2 * n_obs_nodes,
                4 if self.observes_temperature() => o.pos < 1 || o.pos > n_obs_nodes,
                4 => o.pos.abs() > n_obs_nodes,
                5..=11 => o.pos < 1 || o.pos as usize > prj.water.materials.len(),
                12..=15 => o.pos < 1,
                _ => true,
            };
            if bad {
                e.push(format!("Observation {}: type {} with position {} is not valid.", i + 1, o.kind, o.pos));
            }
        }
        if self.fit_solute && !prj.processes.solute {
            e.push("Solute parameters are fitted but solute transport is not simulated.".into());
        }
        if self.observations.iter().any(|o| o.kind == 4) && !self.observes_temperature() && !prj.processes.solute {
            e.push("Concentrations are observed but solute transport is not simulated.".into());
        }
        if self.observes_temperature() && self.observations.iter().any(|o| o.kind == 14 || o.kind == 15) {
            e.push("Concentration profiles (types 14, 15) cannot be used when heat parameters are fitted.".into());
        }
        e
    }
}

/// Unit conversion of the settings (see [`crate::units::convert_units`]): the parameter values and constraints are
/// converted like the project values they stand for, the observations with their physical dimensions (`fl`, `ft`: new
/// units per old unit of length and time). The weights are kept.
pub(crate) fn convert_settings(s: &mut InverseSettings, before: &Project, convert: impl Fn(&mut Project), fl: f64, ft: f64) {
    let params = s.params.clone();
    for (k, p) in params.iter().enumerate() {
        let mut out = [p.value, p.min, p.max];
        for v in out.iter_mut() {
            if *v == 0.0 {
                continue;
            }
            let mut q = before.clone();
            q.inverse = None;
            s.set_value(&mut q, p, *v);
            convert(&mut q);
            *v = s.get_value(&q, p);
        }
        s.params[k].value = out[0];
        s.params[k].min = out[1];
        s.params[k].max = out[2];
    }
    let f = |a: i32, b: i32| fl.powi(a) * ft.powi(b);
    let log_conc = s.conc_type == 1;
    let temperature = s.observes_temperature();
    for o in s.observations.iter_mut() {
        let (x, v): ((i32, i32), (i32, i32)) = match o.kind {
            0 => ((0, 1), (1, 0)),
            1 => ((0, 1), (1, 0)),
            2 => ((0, 1), (0, 0)),
            3 => ((0, 1), (1, -1)),
            4 if temperature => ((0, 1), (0, 0)),
            4 if o.pos == 0 => ((0, 1), (-2, 0)),
            4 => ((0, 1), (-3, 0)),
            5 => ((1, 0), (0, 0)),
            6 => ((1, 0), (1, -1)),
            7 => ((0, 0), (-1, 0)),
            11 => ((0, 0), (1, -1)),
            12 => ((1, 0), (1, 0)),
            14 => ((1, 0), (-3, 0)),
            13 | 15 => ((1, 0), (0, 0)),
            _ => ((0, 0), (0, 0)),
        };
        o.x *= f(x.0, x.1);
        if o.kind == 4 && log_conc && !temperature {
            o.value += f(v.0, v.1).log10();
        } else {
            o.value *= f(v.0, v.1);
        }
    }
}

// --------------------------------------------------------------------------------------------- forward evaluation

/// Result of one forward run.
#[derive(Clone, Debug, Default)]
pub struct ForwardRun {
    /// Model value of every observation (order of `InverseSettings::observations`)
    pub values: Vec<f64>,
    /// Water mass balance error of the run in % (WatBalR of the last balance)
    pub mass_balance_error: f64,
    /// Full results (only when requested)
    pub results: Option<Results>,
    /// Time levels of the run
    pub levels: std::sync::Arc<Vec<f64>>,
    /// Iterations of the water flow solution (time steps without water flow)
    pub work: usize,
}

/// Model value of an observation that does not depend on the time (types 5-11).
fn static_value(sim: &Simulation, o: &Observation) -> f64 {
    let m = (o.pos.max(1) as usize - 1).min(sim.n_mat.saturating_sub(1));
    let p = sim.prj.water.materials.get(m).map(|mm| par_of(mm, sim.model, sim.x_conv)).unwrap_or([0.0; 11]);
    let mat = sim.prj.water.materials.get(m).cloned().unwrap_or_default();
    match o.kind {
        5 => fq(sim.model, -o.x.abs(), &p),
        6 => fk(sim.model, -o.x.abs(), &p),
        7 => mat.alpha,
        8 => mat.n,
        9 => mat.qr,
        10 => mat.qs,
        11 => mat.ks,
        _ => 0.0,
    }
}

/// Mean water content of the profile (`pos` = 0) or of sub-region `-pos`.
fn mean_water_content(sim: &Simulation, layer: Option<usize>) -> f64 {
    let (mut v, mut l) = (0.0, 0.0);
    for i in 0..sim.n - 1 {
        if layer.is_some_and(|ly| sim.lay[i] != ly) {
            continue;
        }
        let dx = sim.x[i + 1] - sim.x[i];
        let th = |k: usize| {
            let mut t = sim.th_new[k];
            if sim.i_dual_por > 0 {
                t += sim.th_new_im[k];
            }
            if sim.l_dual_perm {
                t = sim.w_fracture * sim.th_new[k] + (1.0 - sim.w_fracture) * sim.th_matrix_new[k];
            }
            t
        };
        v += dx * (th(i) + th(i + 1)) / 2.0;
        l += dx;
    }
    if l > 0.0 {
        v / l
    } else {
        0.0
    }
}

/// Total mass of solute `js` in the profile (ConcVol + immobile / kinetic phases of Balance.out).
fn solute_mass(sim: &mut Simulation, js: usize) -> f64 {
    if sim.sol.is_none() {
        return 0.0;
    }
    let n_lay = sim.lay.iter().copied().max().unwrap_or(1);
    let mut rec = BalanceOut { sub: vec![SubRegion::default(); n_lay], ..Default::default() };
    // with a print level > 0 the balance routine only reads the state
    sim.sub_reg_solute(1, &mut rec);
    rec.total.c_vol.get(js).copied().unwrap_or(0.0) + rec.total.c_vol_im.get(js).copied().unwrap_or(0.0)
}

/// Model value of a timed observation (types 0-4) in the current state.
fn timed_value(sim: &mut Simulation, o: &Observation, conc_type: i32, temperature: bool) -> f64 {
    let n = sim.n;
    let obs_node = |sim: &Simulation, pos: i32| -> Option<usize> {
        let k = pos.unsigned_abs() as usize;
        let nd = *sim.prj.profile.observation_nodes.get(k.checked_sub(1)?)?;
        (1..=n).contains(&nd).then(|| n - nd)
    };
    match o.kind {
        0 => match o.pos {
            1 | 3 | 4 => sim.cum_q[2],
            _ => sim.cum_q[4],
        },
        3 => match o.pos {
            1 | 3 | 4 => sim.v_top,
            _ => sim.v_bot,
        },
        1 => {
            // the record of ObsNod: after a step hNew already holds the extrapolated start of the next step
            let k = o.pos.unsigned_abs() as usize;
            let Some(p) = sim.res.obs.last().and_then(|r| r.points.get(k.wrapping_sub(1))) else { return 0.0 };
            match p.h_matrix {
                Some(hm) if o.pos > 0 => hm,
                _ => p.h,
            }
        }
        2 => {
            if o.pos == 0 {
                return mean_water_content(sim, None);
            }
            let n_obs = sim.prj.profile.observation_nodes.len() as i32;
            if o.pos < 0 && !sim.l_dual_perm {
                return mean_water_content(sim, Some((-o.pos) as usize));
            }
            let pos = if o.pos > n_obs { o.pos - n_obs } else { o.pos };
            let Some(i) = obs_node(sim, pos) else { return 0.0 };
            if sim.l_dual_perm {
                let w = sim.w_fracture;
                if o.pos < 0 {
                    w * sim.th_new[i]
                } else if o.pos > n_obs {
                    (1.0 - w) * sim.th_matrix_new[i]
                } else {
                    w * sim.th_new[i] + (1.0 - w) * sim.th_matrix_new[i]
                }
            } else {
                sim.th_new[i] + if sim.i_dual_por > 0 { sim.th_new_im[i] } else { 0.0 }
            }
        }
        4 if temperature => {
            // temperature at the observation node: the ObsNod record of the time level (the initial value before it)
            let k = o.pos.unsigned_abs() as usize;
            match sim.res.obs.last().and_then(|r| r.points.get(k.wrapping_sub(1))) {
                Some(p) => p.temp,
                None => obs_node(sim, o.pos).and_then(|i| sim.heat.as_ref().map(|h| h.temp_n[i])).unwrap_or(0.0),
            }
        }
        4 => {
            if o.pos == 0 {
                let m = solute_mass(sim, 0);
                return if conc_type == 1 { m.max(1e-30).log10() } else { m };
            }
            let js = if o.pos < 0 { 1 } else { 0 };
            let Some(i) = obs_node(sim, o.pos) else { return 0.0 };
            let c = match conc_type {
                2 => sim.flux_conc(js).get(i).copied().unwrap_or(0.0),
                3 => {
                    // total solute mass per volume at the node (linear sorption)
                    let m = sim.mat[i];
                    let c = sim.conc(js, i);
                    let ro = sim.prj.solute.materials.get(m).map(|x| x.bulk_density).unwrap_or(0.0);
                    let kd = sim.prj.solute.species.get(js).and_then(|s| s.per_material.get(m)).map(|x| x.ks).unwrap_or(0.0);
                    let fr = sim.prj.solute.materials.get(m).map(|x| x.frac).unwrap_or(1.0);
                    c * (sim.th_new[i] + fr * ro * kd) + ro * sim.sorb(js, i)
                }
                4 if sim.i_dual_por > 0 => {
                    let (thm, thi) = (sim.th_new[i], sim.th_new_im[i]);
                    (sim.conc(js, i) * thm + sim.sorb(js, i) * thi) / (thm + thi).max(1e-30)
                }
                _ if sim.l_dual_perm && o.pos > 0 => sim.conc_matrix(js, i),
                _ => sim.conc(js, i),
            };
            if conc_type == 1 {
                c.max(1e-30).log10()
            } else {
                c
            }
        }
        _ => 0.0,
    }
}

/// Value of a profile observation (types 12-15) at print time `pos` and depth `|x|`, from the profile outputs.
fn profile_value(res: &Results, o: &Observation) -> f64 {
    let Some(p) = res.profiles.get(o.pos.max(0) as usize) else { return 0.0 };
    let z = o.x.abs();
    let get = |k: usize| -> f64 {
        let nd = &p.nodes[k];
        match o.kind {
            12 => nd.h,
            13 => nd.theta,
            14 => nd.conc.first().copied().unwrap_or(0.0),
            _ => nd.sorb.first().copied().unwrap_or(0.0) + nd.sorb2.first().copied().unwrap_or(0.0),
        }
    };
    if p.nodes.is_empty() {
        return 0.0;
    }
    let mut nodes: Vec<usize> = (0..p.nodes.len()).collect();
    nodes.sort_by(|&a, &b| p.nodes[a].depth.total_cmp(&p.nodes[b].depth));
    for w in nodes.windows(2) {
        let (a, b) = (&p.nodes[w[0]], &p.nodes[w[1]]);
        if z >= a.depth && z <= b.depth {
            let f = if b.depth > a.depth { (z - a.depth) / (b.depth - a.depth) } else { 0.0 };
            return get(w[0]) + f * (get(w[1]) - get(w[0]));
        }
    }
    if z < p.nodes[nodes[0]].depth {
        get(nodes[0])
    } else {
        get(*nodes.last().unwrap())
    }
}

/// Runs the model with the given values of the fitted parameters and evaluates every observation. The observation
/// times are fixed times of the time step control; a time that is passed nevertheless (shorter than the minimum time
/// step) is interpolated linearly from the time levels around it.
pub fn forward(prj: &Project, s: &InverseSettings, fitted: &[f64], keep_results: bool, cancel: Option<&AtomicBool>) -> Result<ForwardRun, HydrusError> {
    forward_on(prj, s, fitted, keep_results, cancel, None, 0)
}

/// [`forward`] on prescribed time levels (`Simulation::fixed_levels`) and with at most `max_steps` iterations of the
/// water flow solution (time steps without water flow; 0: no limit; a run that needs more fails).
pub fn forward_on(
    prj: &Project,
    s: &InverseSettings,
    fitted: &[f64],
    keep_results: bool,
    cancel: Option<&AtomicBool>,
    levels: Option<std::sync::Arc<Vec<f64>>>,
    max_steps: usize,
) -> Result<ForwardRun, HydrusError> {
    let mut p = prj.clone();
    s.apply(&mut p, fitted);
    let mut sim = Simulation::new(p)?;
    sim.fixed_levels = levels;
    let mut run_levels = vec![];
    let temp_obs = s.observes_temperature();
    let nobs = s.observations.len();
    let mut values = vec![0.0; nobs];
    let (t0, t_end) = (sim.t_init, sim.t_max);
    // timed observations by time
    let mut order: Vec<usize> = (0..nobs).filter(|&i| s.observations[i].is_timed()).collect();
    order.sort_by(|&a, &b| s.observations[a].x.total_cmp(&s.observations[b].x));
    let mut targets: Vec<f64> = order.iter().map(|&i| s.observations[i].x).filter(|&t| t > t0 && t < t_end).collect();
    targets.dedup_by(|a, b| (*a - *b).abs() <= 1e-9 * a.abs().max(1.0));
    let mut tt = std::mem::take(&mut sim.target_times);
    tt.extend(targets);
    tt.sort_by(f64::total_cmp);
    tt.dedup_by(|a, b| (*a - *b).abs() <= 1e-9 * a.abs().max(1.0));
    sim.target_times = tt;
    for (i, o) in s.observations.iter().enumerate() {
        if !o.is_timed() && o.kind < 12 {
            values[i] = static_value(&sim, o);
        }
    }
    let mut k = 0;
    // observations at (or before) the initial time
    while k < order.len() && s.observations[order[k]].x <= t0 {
        values[order[k]] = timed_value(&mut sim, &s.observations[order[k]], s.conc_type, temp_obs);
        k += 1;
    }
    // value of the next pending time at the previous time level (for interpolation)
    let mut prev: Vec<(usize, f64)> = vec![];
    let mut t_prev = t0;
    let pending = |k: usize, sim: &mut Simulation| -> Vec<(usize, f64)> {
        let mut v = vec![];
        if k < order.len() {
            let t = s.observations[order[k]].x;
            let mut j = k;
            while j < order.len() && (s.observations[order[j]].x - t).abs() <= 1e-9 * t.abs().max(1.0) {
                v.push((order[j], timed_value(sim, &s.observations[order[j]], s.conc_type, temp_obs)));
                j += 1;
            }
        }
        v
    };
    if k < order.len() {
        prev = pending(k, &mut sim);
    }
    loop {
        if cancel.is_some_and(|c| c.load(Ordering::Relaxed)) {
            return Err(HydrusError::Numerical("cancelled".into()));
        }
        let st = sim.step();
        if st == StepStatus::Failed {
            let msg = sim.res.messages.last().cloned().unwrap_or_else(|| "the simulation failed".into());
            return Err(HydrusError::Numerical(msg));
        }
        if max_steps > 0 && sim.it_cum.max(run_levels.len()) >= max_steps {
            return Err(HydrusError::Numerical(format!("more than {max_steps} iterations")));
        }
        // after a step the state belongs to tOld (the step has already advanced t to the next time level)
        let (t, dt) = if st == StepStatus::Finished { (sim.t, sim.dt) } else { (sim.t_old, sim.dt_old) };
        run_levels.push(t);
        let tol = 0.001 * dt.max(sim.dt_min);
        while k < order.len() && s.observations[order[k]].x <= t + tol {
            let tk = s.observations[order[k]].x;
            let exact = (tk - t).abs() <= tol;
            let f = if exact || t <= t_prev { 1.0 } else { (tk - t_prev) / (t - t_prev) };
            while k < order.len() && (s.observations[order[k]].x - tk).abs() <= 1e-9 * tk.abs().max(1.0) {
                let i = order[k];
                let now = timed_value(&mut sim, &s.observations[i], s.conc_type, temp_obs);
                let old = prev.iter().find(|x| x.0 == i).map(|x| x.1).unwrap_or(now);
                values[i] = old + f * (now - old);
                k += 1;
            }
        }
        if k < order.len() {
            prev = pending(k, &mut sim);
        }
        t_prev = t;
        if st == StepStatus::Finished {
            break;
        }
        if !keep_results {
            // only the last records are needed
            let r = &mut sim.res;
            if r.obs.len() > 2 {
                r.obs.drain(..r.obs.len() - 1);
            }
            if r.tlevel.len() > 2 {
                r.tlevel.drain(..r.tlevel.len() - 1);
            }
            if r.alevel.len() > 2 {
                r.alevel.drain(..r.alevel.len() - 1);
            }
        }
    }
    // observations after the end of the simulation take the final state
    while k < order.len() {
        values[order[k]] = timed_value(&mut sim, &s.observations[order[k]], s.conc_type, temp_obs);
        k += 1;
    }
    for (i, o) in s.observations.iter().enumerate() {
        if o.kind >= 12 {
            values[i] = profile_value(&sim.res, o);
        }
    }
    let mass_balance_error = sim.res.balance.last().map(|b| b.wat_bal_r).unwrap_or(0.0);
    let work = sim.it_cum.max(run_levels.len());
    Ok(ForwardRun { values, mass_balance_error, results: keep_results.then(|| std::mem::take(&mut sim.res)), levels: std::sync::Arc::new(run_levels), work })
}

// --------------------------------------------------------------------------------------------- statistics

/// Student's t distribution: two-sided 95 % quantile for `dof` degrees of freedom.
pub fn t_975(dof: usize) -> f64 {
    if dof == 0 {
        return f64::NAN;
    }
    let nu = dof as f64;
    // P(T <= t) through the regularized incomplete beta function, inverted by bisection
    let cdf = |t: f64| 1.0 - 0.5 * inc_beta(nu / 2.0, 0.5, nu / (nu + t * t));
    let (mut lo, mut hi) = (0.0, 1000.0);
    for _ in 0..200 {
        let mid = 0.5 * (lo + hi);
        if cdf(mid) < 0.975 {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    0.5 * (lo + hi)
}

fn ln_gamma(x: f64) -> f64 {
    let c = [76.18009172947146, -86.50532032941677, 24.01409824083091, -1.231739572450155, 0.1208650973866179e-2, -0.5395239384953e-5];
    let mut y = x;
    let tmp = x + 5.5 - (x + 0.5) * (x + 5.5).ln();
    let mut ser = 1.000000000190015;
    for cj in c {
        y += 1.0;
        ser += cj / y;
    }
    -tmp + (2.5066282746310005 * ser / x).ln()
}

fn beta_cf(a: f64, b: f64, x: f64) -> f64 {
    let (qab, qap, qam) = (a + b, a + 1.0, a - 1.0);
    let mut c = 1.0;
    let mut d = 1.0 - qab * x / qap;
    if d.abs() < 1e-300 {
        d = 1e-300;
    }
    d = 1.0 / d;
    let mut h = d;
    for m in 1..300 {
        let m = m as f64;
        let m2 = 2.0 * m;
        let aa = m * (b - m) * x / ((qam + m2) * (a + m2));
        d = 1.0 + aa * d;
        if d.abs() < 1e-300 {
            d = 1e-300;
        }
        c = 1.0 + aa / c;
        if c.abs() < 1e-300 {
            c = 1e-300;
        }
        d = 1.0 / d;
        h *= d * c;
        let aa = -(a + m) * (qab + m) * x / ((a + m2) * (qap + m2));
        d = 1.0 + aa * d;
        if d.abs() < 1e-300 {
            d = 1e-300;
        }
        c = 1.0 + aa / c;
        if c.abs() < 1e-300 {
            c = 1e-300;
        }
        d = 1.0 / d;
        let del = d * c;
        h *= del;
        if (del - 1.0).abs() < 1e-15 {
            break;
        }
    }
    h
}

/// Regularized incomplete beta function I_x(a, b).
fn inc_beta(a: f64, b: f64, x: f64) -> f64 {
    if x <= 0.0 {
        return 0.0;
    }
    if x >= 1.0 {
        return 1.0;
    }
    let bt = (ln_gamma(a + b) - ln_gamma(a) - ln_gamma(b) + a * x.ln() + b * (1.0 - x).ln()).exp();
    if x < (a + 1.0) / (a + b + 2.0) {
        bt * beta_cf(a, b, x) / a
    } else {
        1.0 - bt * beta_cf(b, a, 1.0 - x) / b
    }
}

/// Solves `a x = b` (Gauss-Jordan with partial pivoting); `None` when singular.
pub fn solve(a: &[Vec<f64>], b: &[f64]) -> Option<Vec<f64>> {
    let n = b.len();
    let mut m: Vec<Vec<f64>> = a.iter().zip(b).map(|(r, &bi)| r.iter().copied().chain(std::iter::once(bi)).collect()).collect();
    for c in 0..n {
        let piv = (c..n).max_by(|&i, &j| m[i][c].abs().total_cmp(&m[j][c].abs()))?;
        if m[piv][c].abs() < 1e-300 || !m[piv][c].is_finite() {
            return None;
        }
        m.swap(c, piv);
        let d = m[c][c];
        for x in m[c].iter_mut() {
            *x /= d;
        }
        let pivot_row = m[c].clone();
        for (r, row) in m.iter_mut().enumerate() {
            let f = row[c];
            if r != c && f != 0.0 {
                for (x, p) in row[c..].iter_mut().zip(&pivot_row[c..]) {
                    *x -= f * p;
                }
            }
        }
    }
    Some(m.iter().map(|r| r[n]).collect())
}

/// Inverse of a symmetric matrix (`None` when singular).
pub fn invert(a: &[Vec<f64>]) -> Option<Vec<Vec<f64>>> {
    let n = a.len();
    let mut cols = vec![vec![0.0; n]; n];
    for j in 0..n {
        let mut e = vec![0.0; n];
        e[j] = 1.0;
        let x = solve(a, &e)?;
        for i in 0..n {
            cols[i][j] = x[i];
        }
    }
    Some(cols)
}

/// Goodness-of-fit measures of Fit.out.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct FitMeasures {
    pub r_squared: f64,
    pub me: f64,
    pub mae: f64,
    pub rmse: f64,
    pub aic: f64,
    pub bic: f64,
}

/// R², ME, MAE and RMSE of observed `y` and fitted `f` with weights `w` (the final run), AIC and BIC with the objective
/// `ssq_aic` at the start of the last iteration (as HYDRUS prints them).
pub fn fit_measures(y: &[f64], f: &[f64], w: &[f64], ssq_aic: f64, n_par: usize) -> FitMeasures {
    let n = y.len() as f64;
    let (mut sw, mut sy, mut sf, mut syf, mut syy, mut sff, mut swr, mut swar) = (0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
    let mut ssq = 0.0;
    for i in 0..y.len() {
        sw += w[i];
        sy += w[i] * y[i];
        sf += w[i] * f[i];
        syf += w[i] * y[i] * f[i];
        syy += w[i] * y[i] * y[i];
        sff += w[i] * f[i] * f[i];
        swr += w[i] * (y[i] - f[i]);
        swar += (w[i] * (y[i] - f[i])).abs();
        ssq += (w[i] * (y[i] - f[i])).powi(2);
    }
    let den = (syy - sy * sy / sw) * (sff - sf * sf / sw);
    let r2 = if den > 0.0 { (syf - sy * sf / sw).powi(2) / den } else { 0.0 };
    let p = n_par as f64;
    let ln = |s: f64| (s.max(1e-300) / n).ln();
    FitMeasures {
        r_squared: r2,
        me: swr / n,
        mae: swar / n,
        rmse: (ssq / (n - 1.0).max(1.0)).sqrt(),
        aic: n * ln(ssq_aic) + 2.0 * p,
        bic: n * ln(ssq_aic) + p * n.ln(),
    }
}

// --------------------------------------------------------------------------------------------- optimization

/// Statistics of one fitted parameter.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ParameterStat {
    pub name: String,
    pub value: f64,
    pub se: f64,
    pub lower_95: f64,
    pub upper_95: f64,
}

/// One row of the iteration table.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct IterationRecord {
    pub ssq: f64,
    pub params: Vec<f64>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum StopReason {
    /// No parameter changed by more than 1 %
    #[default]
    Converged,
    /// Maximum number of iterations (MIT) reached
    MaxIterations,
    /// "No further reduction in SSQ obtained, optimization stopped"
    NoReduction,
    Cancelled,
}

/// Result of the inverse solution (the content of Fit.out).
#[derive(Clone, Debug, Default)]
pub struct InverseResult {
    pub names: Vec<String>,
    pub iterations: Vec<IterationRecord>,
    pub stop: StopReason,
    pub params: Vec<ParameterStat>,
    pub correlation: Vec<Vec<f64>>,
    /// Effective weights, observed and fitted values (order of the observations)
    pub weights: Vec<f64>,
    pub fitted: Vec<f64>,
    /// Final value of the objective function
    pub ssq: f64,
    /// Contributions of the measurement sets (type, value)
    pub contributions: Vec<(i32, f64)>,
    pub measures: FitMeasures,
    pub mass_balance_error: f64,
    /// Results of the final run with the fitted parameters
    pub final_results: Option<Results>,
    /// Final project (fitted parameters written into the materials)
    pub project: Option<Project>,
    /// Number of forward runs
    pub runs: usize,
    pub elapsed_secs: f64,
    /// Every starting point (the first holds the initial estimates of the settings), with its outcome
    pub starts: Vec<StartOutcome>,
    /// Index into `starts` of the result returned
    pub best_start: usize,
    /// Number of stages on frozen time levels (`MarqOptions::frozen`; 1 without)
    pub stages: usize,
}

/// Outcome of the estimation from one starting point.
#[derive(Clone, Debug)]
pub struct StartOutcome {
    /// Initial estimates of the fitted parameters
    pub initial: Vec<f64>,
    /// Final values (empty when the estimation failed)
    pub fitted: Vec<f64>,
    /// Final objective function (infinite when the estimation failed)
    pub ssq: f64,
    pub stop: StopReason,
    pub iterations: usize,
    /// Error message when the estimation from this point failed
    pub error: Option<String>,
}

/// Progress information passed to the callback of [`optimize`].
#[derive(Clone, Debug)]
pub struct Progress {
    pub iteration: usize,
    pub ssq: f64,
    pub params: Vec<f64>,
    /// Number of forward runs so far (of this starting point)
    pub runs: usize,
    /// Starting point the message belongs to (0 = the initial estimates of the settings)
    pub start: usize,
    /// Set when the estimation from `start` has ended (`ssq` and `params` are then its final values)
    pub finished: bool,
}

/// Tunables of the Marquardt iteration (defaults reproduce HYDRUS).
#[derive(Clone, Debug)]
pub struct MarqOptions {
    /// Initial Marquardt parameter (divided by 10 at the start of every iteration)
    pub ga0: f64,
    /// Relative increment of the forward-difference derivatives
    pub deriv: f64,
    /// Convergence: maximum relative parameter change
    pub stop_cr: f64,
    /// Largest Marquardt parameter before giving up ("No further reduction")
    pub ga_max: f64,
    /// Smallest relative parameter change tried by halving the step before giving up
    pub min_change: f64,
    /// Run the Jacobian columns in parallel threads
    pub parallel: bool,
    /// Frozen time levels (not in HYDRUS; not used with `InverseSettings::hydrus_stop`): the runs of an iteration
    /// (derivatives and trial steps) use the time levels of the run at the start of the stage instead of the adaptive
    /// time step control, so the objective is a smooth function of the parameters within the stage. A stage ends when
    /// the iteration stops; the run at its end point (with the adaptive time steps) gives the objective of that point
    /// and the time levels of the next stage. The stages go on while they lower that objective by more than
    /// `stage_tol`, and the best point is kept.
    pub frozen: bool,
    /// Relative increment of the derivatives on frozen time levels
    pub frozen_deriv: f64,
    /// Smallest relative reduction of the objective by a stage for another one
    pub stage_tol: f64,
}

impl Default for MarqOptions {
    fn default() -> Self {
        MarqOptions { ga0: 0.02, deriv: 0.01, stop_cr: 0.01, ga_max: 1e10, min_change: 1e-6, parallel: true, frozen: true, frozen_deriv: 0.001, stage_tol: 1e-3 }
    }
}

impl MarqOptions {
    /// The HYDRUS 4.17 iteration (adaptive time steps in every run, one stage).
    pub fn hydrus() -> Self {
        MarqOptions { frozen: false, ..Default::default() }
    }
}

fn ssq_of(obs: &[Observation], w: &[f64], f: &[f64]) -> f64 {
    obs.iter().zip(w).zip(f).map(|((o, wi), fi)| (wi * (o.value - fi)).powi(2)).sum()
}

/// Model values for several parameter vectors (in parallel threads when `parallel`).
fn run_many(prj: &Project, s: &InverseSettings, sets: &[Vec<f64>], parallel: bool, cancel: Option<&AtomicBool>, levels: Option<&Arc<Vec<f64>>>, max_steps: usize) -> Vec<Result<Vec<f64>, HydrusError>> {
    if parallel && sets.len() > 1 {
        std::thread::scope(|sc| {
            let hs: Vec<_> = sets.iter().map(|b| sc.spawn(move || forward_on(prj, s, b, false, cancel, levels.cloned(), max_steps).map(|r| r.values))).collect();
            hs.into_iter().map(|h| h.join().unwrap_or_else(|_| Err(HydrusError::Numerical("forward run panicked".into())))).collect()
        })
    } else {
        sets.iter().map(|b| forward_on(prj, s, b, false, cancel, levels.cloned(), max_steps).map(|r| r.values)).collect()
    }
}

/// Clamps the fitted values into their constraints.
fn clamp_bounds(s: &InverseSettings, idx: &[usize], b: &mut [f64]) {
    for (j, &i) in idx.iter().enumerate() {
        let (lo, hi) = s.params[i].bounds();
        if let Some(l) = lo {
            b[j] = b[j].max(l);
        }
        if let Some(h) = hi {
            b[j] = b[j].min(h);
        }
    }
}

/// Jacobian `∂F_i/∂b_j` by forward differences with the relative increment `deriv`.
/// `deriv` < 0 gives backward differences.
#[allow(clippy::too_many_arguments)]
fn jacobian(prj: &Project, s: &InverseSettings, b: &[f64], f0: &[f64], deriv: f64, o: &MarqOptions, cancel: Option<&AtomicBool>, levels: Option<&Arc<Vec<f64>>>, max_steps: usize) -> Result<(Vec<Vec<f64>>, usize), HydrusError> {
    let np = b.len();
    let steps: Vec<f64> = b.iter().map(|&bj| if bj != 0.0 { deriv * bj } else { deriv * 1e-3 }).collect();
    let sets: Vec<Vec<f64>> = (0..np)
        .map(|j| {
            let mut bb = b.to_vec();
            bb[j] += steps[j];
            bb
        })
        .collect();
    let runs = run_many(prj, s, &sets, o.parallel, cancel, levels, max_steps);
    let mut extra = 0;
    let mut jac = vec![vec![0.0; np]; f0.len()];
    for (j, r) in runs.into_iter().enumerate() {
        if cancel.is_some_and(|c| c.load(Ordering::Relaxed)) {
            return Err(HydrusError::Numerical("cancelled".into()));
        }
        // a failed run: the derivative from the opposite increment, or none (the parameter stays this iteration)
        let (fj, h) = match r {
            Ok(fj) => (Some(fj), steps[j]),
            Err(_) => {
                let mut bb = b.to_vec();
                bb[j] -= steps[j];
                extra += 1;
                (forward_on(prj, s, &bb, false, cancel, levels.cloned(), max_steps).ok().map(|x| x.values), -steps[j])
            }
        };
        if let Some(fj) = fj {
            for i in 0..f0.len() {
                jac[i][j] = (fj[i] - f0[i]) / h;
            }
        }
    }
    // a parameter without any effect at a small increment (a threshold of the model, e.g. the linear isotherm of
    // HYDRUS for |beta - 1| <= 0.001): the derivative from tenfold larger increments, up to 10 %
    let mut factor = 10.0;
    while deriv.abs() * factor <= 0.1 + 1e-12 {
        let zero: Vec<usize> = (0..np).filter(|&j| jac.iter().all(|r| r[j] == 0.0)).collect();
        if zero.is_empty() {
            break;
        }
        let sets: Vec<Vec<f64>> = zero
            .iter()
            .map(|&j| {
                let mut bb = b.to_vec();
                bb[j] += factor * steps[j];
                bb
            })
            .collect();
        extra += sets.len();
        for (k, r) in run_many(prj, s, &sets, o.parallel, cancel, levels, max_steps).into_iter().enumerate() {
            if let Ok(fj) = r {
                let j = zero[k];
                for i in 0..f0.len() {
                    jac[i][j] = (fj[i] - f0[i]) / (factor * steps[j]);
                }
            }
        }
        factor *= 10.0;
    }
    Ok((jac, extra))
}

/// Outcome of one Marquardt iteration.
enum Trial {
    /// A step reduced the objective; the largest relative parameter change (before the constraints)
    Accepted(f64),
    NoReduction,
    Cancelled,
}

/// State of the Marquardt-Levenberg iteration.
struct Marq<'a> {
    prj: &'a Project,
    s: &'a InverseSettings,
    o: &'a MarqOptions,
    cancel: Option<&'a AtomicBool>,
    idx: Vec<usize>,
    w: Vec<f64>,
    b: Vec<f64>,
    f: Vec<f64>,
    /// frozen time levels of the stage (None: adaptive time steps)
    grid: Option<Arc<Vec<f64>>>,
    /// largest number of time steps of a run (0: no limit)
    max_steps: usize,
    ssq: f64,
    ga: f64,
    runs: usize,
}

impl Marq<'_> {
    fn cancelled(&self) -> bool {
        self.cancel.is_some_and(|c| c.load(Ordering::Relaxed))
    }

    /// Normal matrix `D = Jᵀ W² J` and gradient `g = Jᵀ W² r` at the current parameters.
    fn normal_equations(&mut self, deriv: f64) -> Result<(Vec<Vec<f64>>, Vec<f64>), HydrusError> {
        let np = self.b.len();
        let (jac, extra) = jacobian(self.prj, self.s, &self.b, &self.f, deriv, self.o, self.cancel, self.grid.as_ref(), self.max_steps)?;
        self.runs += np + extra;
        let mut d = vec![vec![0.0; np]; np];
        let mut g = vec![0.0; np];
        for (i, ob) in self.s.observations.iter().enumerate() {
            let wi2 = self.w[i] * self.w[i];
            let r = ob.value - self.f[i];
            for a in 0..np {
                g[a] += wi2 * jac[i][a] * r;
                for c in 0..np {
                    d[a][c] += wi2 * jac[i][a] * jac[i][c];
                }
            }
        }
        Ok((d, g))
    }

    /// One iteration: Marquardt steps of the system scaled to a unit diagonal, `(A + GA I) p = q`, parameter
    /// `b + step p / E`. A step that does not reduce the objective (or that changes the sign of a parameter) is halved
    /// when it points within 30° of the gradient, otherwise GA is increased tenfold.
    fn iterate(&mut self, d: &[Vec<f64>], g: &[f64]) -> Trial {
        let np = self.b.len();
        let e: Vec<f64> = (0..np).map(|a| d[a][a].sqrt().max(1e-30)).collect();
        let q: Vec<f64> = (0..np).map(|a| g[a] / e[a]).collect();
        loop {
            let mut a = vec![vec![0.0; np]; np];
            for r in 0..np {
                for c in 0..np {
                    a[r][c] = d[r][c] / (e[r] * e[c]);
                }
                a[r][r] = 1.0 + self.ga;
            }
            let Some(p) = solve(&a, &q) else {
                self.ga *= 10.0;
                if self.ga > self.o.ga_max {
                    return Trial::NoReduction;
                }
                continue;
            };
            let (pq, pp, qq) = (0..np).fold((0.0, 0.0, 0.0), |s, j| (s.0 + p[j] * q[j], s.1 + p[j] * p[j], s.2 + q[j] * q[j]));
            let angle = (pq / (pp * qq).sqrt().max(1e-300)).clamp(-1.0, 1.0).acos().to_degrees();
            let mut step = 1.0;
            loop {
                if self.cancelled() {
                    return Trial::Cancelled;
                }
                let mut tb: Vec<f64> = (0..np).map(|j| self.b[j] + step * p[j] / e[j]).collect();
                clamp_bounds(self.s, &self.idx, &mut tb);
                // a parameter may not change its sign
                if !(0..np).any(|j| self.b[j] * tb[j] < 0.0) {
                    self.runs += 1;
                    match forward_on(self.prj, self.s, &tb, false, self.cancel, self.grid.clone(), self.max_steps) {
                        Ok(r) => {
                            let st = ssq_of(&self.s.observations, &self.w, &r.values);
                            if st < self.ssq {
                                // convergence is judged on the step before the constraints are applied
                                let rel = (0..np).map(|j| (step * p[j] / e[j]).abs() / (1e-20 + self.b[j].abs())).fold(0.0, f64::max);
                                self.b = tb;
                                self.f = r.values;
                                self.ssq = st;
                                return Trial::Accepted(rel);
                            }
                        }
                        Err(_) if self.cancelled() => return Trial::Cancelled,
                        Err(_) => {}
                    }
                }
                if angle <= 30.0 {
                    step *= 0.5;
                    let rel = (0..np).map(|j| (tb[j] - self.b[j]).abs() / (1e-20 + self.b[j].abs())).fold(0.0, f64::max);
                    if rel < self.o.min_change {
                        return Trial::NoReduction;
                    }
                } else {
                    self.ga *= 10.0;
                    if self.ga > self.o.ga_max {
                        return Trial::NoReduction;
                    }
                    break;
                }
            }
        }
    }
}

/// Parameter estimation: Marquardt-Levenberg minimization of the weighted sum of squares, followed by a final run with
/// the fitted parameters (its results are returned) and the statistics of Fit.out.
fn optimize_single<F: FnMut(&Progress)>(prj: &Project, s: &InverseSettings, o: &MarqOptions, cancel: Option<&AtomicBool>, max_steps: usize, mut progress: F) -> Result<InverseResult, HydrusError> {
    let start = std::time::Instant::now();
    let problems = s.validate(prj);
    if !problems.is_empty() {
        return Err(HydrusError::Invalid(problems.join("\n")));
    }
    let idx = s.fitted();
    let np = idx.len();
    let names: Vec<String> = idx.iter().map(|&i| s.param_name(&s.params[i])).collect();
    let w = s.effective_weights();
    let mut b = s.initial();
    clamp_bounds(s, &idx, &mut b);
    let staged = o.frozen && !s.hydrus_stop;
    let deriv = if staged { o.frozen_deriv } else { o.deriv };
    let first = forward_on(prj, s, &b, false, cancel, None, max_steps)?;
    let ssq = ssq_of(&s.observations, &w, &first.values);
    let grid = staged.then(|| first.levels.clone());
    let mut m = Marq { prj, s, o, cancel, idx, w, b, f: first.values, grid, max_steps, ssq, ga: o.ga0, runs: 1 };
    // best point by the objective with adaptive time steps, and the point that objective was last computed at
    let mut best = (m.b.clone(), ssq);
    let mut checked = m.b.clone();
    let mut stages = 1;
    let mut iterations = vec![IterationRecord { ssq, params: m.b.clone() }];
    progress(&Progress { iteration: 0, ssq, params: m.b.clone(), runs: m.runs, start: 0, finished: false });

    let mut last_d: Option<Vec<Vec<f64>>> = None;
    let mut nit = 0;
    // objective at the start of the last iteration (AIC and BIC of Fit.out use it)
    let mut ssq_start;
    let stop = loop {
        ssq_start = m.ssq;
        if nit >= s.max_iter {
            break StopReason::MaxIterations;
        }
        if m.cancelled() {
            break StopReason::Cancelled;
        }
        nit += 1;
        m.ga *= 0.1;
        let (d, g) = match m.normal_equations(deriv) {
            Ok(x) => x,
            Err(_) if m.cancelled() => break StopReason::Cancelled,
            Err(e) => return Err(e),
        };
        let reason = match m.iterate(&d, &g) {
            Trial::Accepted(rel) => {
                // the statistics use the normal matrix of the last successful iteration
                last_d = Some(d);
                iterations.push(IterationRecord { ssq: m.ssq, params: m.b.clone() });
                progress(&Progress { iteration: nit, ssq: m.ssq, params: m.b.clone(), runs: m.runs, start: 0, finished: false });
                if rel >= o.stop_cr {
                    continue;
                }
                StopReason::Converged
            }
            Trial::NoReduction => StopReason::NoReduction,
            Trial::Cancelled => break StopReason::Cancelled,
        };
        if !staged || nit >= s.max_iter {
            break reason;
        }
        // end of a stage: the run with adaptive time steps at the end point gives its objective and the time levels
        // of the next stage
        m.runs += 1;
        checked = m.b.clone();
        let Ok(r) = forward_on(prj, s, &m.b, false, cancel, None, max_steps) else { break reason };
        let sf = ssq_of(&s.observations, &m.w, &r.values);
        let improved = sf < (1.0 - o.stage_tol) * best.1;
        if sf < best.1 {
            best = (m.b.clone(), sf);
        }
        if !improved {
            break reason;
        }
        stages += 1;
        m.grid = Some(r.levels.clone());
        m.f = r.values;
        m.ssq = sf;
        m.ga = o.ga0;
    };
    if staged && stop != StopReason::Cancelled && checked != m.b {
        m.runs += 1;
        if let Ok(r) = forward_on(prj, s, &m.b, false, cancel, None, max_steps) {
            let sf = ssq_of(&s.observations, &m.w, &r.values);
            if sf < best.1 {
                best = (m.b.clone(), sf);
            }
        }
    }
    if staged {
        m.b = best.0.clone();
    }
    let (b, w) = (m.b.clone(), m.w.clone());
    // final run with the fitted parameters (outputs, mass balance)
    let fin = forward(prj, s, &b, true, None)?;
    let fitted = fin.values.clone();
    let ssq = ssq_of(&s.observations, &w, &fitted);
    let obs = &s.observations;
    let nob = obs.len();
    let d = match last_d {
        Some(d) => d,
        None => m.normal_equations(deriv)?.0,
    };
    let dof = nob.saturating_sub(np);
    let s2 = if dof > 0 { ssq / dof as f64 } else { 0.0 };
    let t = t_975(dof);
    let cov = invert(&d);
    let mut params = vec![];
    let mut correlation = vec![vec![0.0; np]; np];
    for j in 0..np {
        let var = cov.as_ref().map(|c| c[j][j]).unwrap_or(f64::NAN);
        let se = (var.max(0.0) * s2).sqrt();
        params.push(ParameterStat { name: names[j].clone(), value: b[j], se, lower_95: b[j] - t * se, upper_95: b[j] + t * se });
        for k in 0..np {
            correlation[j][k] = cov.as_ref().map(|c| c[j][k] / (c[j][j] * c[k][k]).sqrt()).unwrap_or(f64::NAN);
        }
    }
    let mut contributions: Vec<(i32, f64)> = vec![];
    for (i, ob) in obs.iter().enumerate() {
        let c = (w[i] * (ob.value - fitted[i])).powi(2);
        match contributions.iter_mut().find(|x| x.0 == ob.kind) {
            Some(x) => x.1 += c,
            None => contributions.push((ob.kind, c)),
        }
    }
    contributions.sort_by_key(|x| x.0);
    let y: Vec<f64> = obs.iter().map(|o| o.value).collect();
    let measures = fit_measures(&y, &fitted, &w, ssq_start, np);
    let mut project = prj.clone();
    s.apply(&mut project, &b);
    Ok(InverseResult {
        names,
        iterations,
        stop,
        params,
        correlation,
        weights: w,
        fitted,
        ssq,
        contributions,
        measures,
        mass_balance_error: fin.mass_balance_error,
        final_results: fin.results,
        project: Some(project),
        runs: m.runs + 1,
        elapsed_secs: start.elapsed().as_secs_f64(),
        starts: vec![],
        best_start: 0,
        stages,
    })
}

/// Halton sequence (radical inverse of `i` in base `b`).
fn halton(i: usize, b: usize) -> f64 {
    let (mut f, mut r, mut k) = (1.0, 0.0, i);
    while k > 0 {
        f /= b as f64;
        r += f * (k % b) as f64;
        k /= b;
    }
    r
}

/// Initial estimates of starting point `k` (k >= 1), spread by a Halton sequence (deterministic): a parameter whose
/// constraints span at most two decades (or start at zero) is placed anywhere in its range (log-uniformly over more
/// than a decade), any other one within a factor of four of its initial estimate (and within its constraints).
pub fn starting_point(s: &InverseSettings, k: usize) -> Vec<f64> {
    const PRIMES: [usize; 16] = [2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37, 41, 43, 47, 53];
    let idx = s.fitted();
    let mut b: Vec<f64> = idx
        .iter()
        .enumerate()
        .map(|(j, &i)| {
            let p = &s.params[i];
            let u = halton(k, PRIMES[j % PRIMES.len()]);
            match p.bounds() {
                (Some(lo), Some(hi)) if hi > lo && (lo <= 0.0 || hi / lo <= 100.0) => {
                    if lo > 0.0 && hi / lo > 10.0 {
                        lo * (hi / lo).powf(u)
                    } else {
                        lo + u * (hi - lo)
                    }
                }
                _ if p.value != 0.0 => p.value * 4f64.powf(2.0 * u - 1.0),
                (Some(lo), Some(hi)) => lo + u * (hi - lo),
                _ => p.value,
            }
        })
        .collect();
    clamp_bounds(s, &idx, &mut b);
    b
}

/// Parameter estimation: Marquardt-Levenberg minimization of the weighted sum of squares (the algorithm of HYDRUS
/// 4.17, see the module documentation), followed by a final run with the fitted parameters (its results are returned)
/// and the statistics of Fit.out.
///
/// Not in HYDRUS (both off with `InverseSettings::hydrus_stop`):
/// * the iteration runs in stages on frozen time levels (`MarqOptions::frozen`). With the adaptive time steps of every
///   run the objective jumps by a few per cent of its value when a parameter changes slightly; in a narrow curved valley
///   (UPINFIL) the Marquardt iteration of HYDRUS then stops far from the minimum, at a point that depends on rounding;
/// * with `InverseSettings::extra_starts` > 0 the estimation also runs from that many further starting points
///   ([`starting_point`]), in parallel threads, and the result with the lowest objective is kept (problems with several
///   local minima).
///
/// The iterations of the first starting point (the initial estimates of the settings) are passed to `progress`; for
/// the other starting points only their end is reported (`Progress::finished`).
pub fn optimize<F: FnMut(&Progress)>(prj: &Project, s: &InverseSettings, o: &MarqOptions, cancel: Option<&AtomicBool>, mut progress: F) -> Result<InverseResult, HydrusError> {
    let started = std::time::Instant::now();
    let n = if s.hydrus_stop { 1 } else { 1 + s.extra_starts };
    let idx = s.fitted();
    let mut initial = s.initial();
    clamp_bounds(s, &idx, &mut initial);
    let points: Vec<Vec<f64>> = (0..n).map(|k| if k == 0 { initial.clone() } else { starting_point(s, k) }).collect();
    let with_values = |b: &[f64]| {
        let mut t = s.clone();
        for (j, &i) in idx.iter().enumerate() {
            t.params[i].value = b[j];
        }
        t
    };
    // Rusteau iteration: a run may take at most ten times the iterations of the run at the initial estimates (at least
    // 2000; a starting point or a trial step with extreme values can need millions of time steps)
    let max_steps = if o.frozen && !s.hydrus_stop {
        match forward(prj, s, &initial, false, cancel) {
            Ok(r) => (10 * r.work).max(2000),
            Err(_) => 0,
        }
    } else {
        0
    };
    let results: Vec<Option<Result<InverseResult, HydrusError>>> = if n == 1 {
        vec![Some(optimize_single(prj, s, o, cancel, max_steps, &mut progress))]
    } else {
        let settings: Vec<InverseSettings> = points.iter().map(|b| with_values(b)).collect();
        type Msg = (usize, Option<Progress>, Option<Result<InverseResult, HydrusError>>);
        let (tx, rx) = std::sync::mpsc::channel::<Msg>();
        // a pool of workers takes the starting points in order (the first one is always running)
        let workers = std::thread::available_parallelism().map(|x| x.get()).unwrap_or(4).clamp(2, n);
        let next = std::sync::atomic::AtomicUsize::new(0);
        std::thread::scope(|sc| {
            for _ in 0..workers {
                let tx = tx.clone();
                let (settings, next) = (&settings, &next);
                sc.spawn(move || loop {
                    let k = next.fetch_add(1, Ordering::SeqCst);
                    if k >= n {
                        break;
                    }
                    let tx2 = tx.clone();
                    let r = optimize_single(prj, &settings[k], o, cancel, max_steps, |p| {
                        if k == 0 {
                            let _ = tx2.send((0, Some(p.clone()), None));
                        }
                    });
                    let _ = tx.send((k, None, Some(r)));
                });
            }
            drop(tx);
            let mut out: Vec<Option<Result<InverseResult, HydrusError>>> = (0..n).map(|_| None).collect();
            for (k, p, r) in rx {
                if let Some(p) = p {
                    progress(&p);
                }
                if let Some(r) = r {
                    let (ssq, runs, params) = r.as_ref().map(|x| (x.ssq, x.runs, x.params.iter().map(|p| p.value).collect())).unwrap_or((f64::INFINITY, 0, vec![]));
                    progress(&Progress { iteration: 0, ssq, params, runs, start: k, finished: true });
                    out[k] = Some(r);
                }
            }
            out
        })
    };
    let mut starts = vec![];
    let mut best: Option<(usize, InverseResult)> = None;
    let mut first_err = None;
    let mut runs = 0;
    for (k, r) in results.into_iter().enumerate() {
        match r {
            Some(Ok(res)) => {
                runs += res.runs;
                starts.push(StartOutcome { initial: points[k].clone(), fitted: res.params.iter().map(|p| p.value).collect(), ssq: res.ssq, stop: res.stop, iterations: res.iterations.len() - 1, error: None });
                if best.as_ref().is_none_or(|(_, b)| res.ssq < b.ssq) {
                    best = Some((k, res));
                }
            }
            Some(Err(e)) => {
                starts.push(StartOutcome { initial: points[k].clone(), fitted: vec![], ssq: f64::INFINITY, stop: StopReason::NoReduction, iterations: 0, error: Some(e.to_string()) });
                if first_err.is_none() {
                    first_err = Some(e);
                }
            }
            None => starts.push(StartOutcome { initial: points[k].clone(), fitted: vec![], ssq: f64::INFINITY, stop: StopReason::Cancelled, iterations: 0, error: Some("not finished".into()) }),
        }
    }
    let Some((k, mut res)) = best else {
        return Err(first_err.unwrap_or_else(|| HydrusError::Numerical("the estimation failed from every starting point".into())));
    };
    let cancelled = || cancel.is_some_and(|c| c.load(Ordering::Relaxed));
    res.starts = starts;
    res.best_start = k;
    res.runs = runs;
    res.elapsed_secs = started.elapsed().as_secs_f64();
    if cancelled() {
        res.stop = StopReason::Cancelled;
    }
    Ok(res)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn student_t_quantiles() {
        // 97.5 % quantiles (Fit.out: ONESTEP 7 dof, DRAINAGE 40 dof)
        assert!((t_975(7) - 2.364624).abs() < 1e-5);
        assert!((t_975(40) - 2.021075).abs() < 1e-5);
        assert!((t_975(1) - 12.7062).abs() < 1e-3);
        assert!((t_975(100000) - 1.95998).abs() < 1e-4);
    }

    /// The measures of Fit.out recomputed from its own table (HYDRUS 4.17, example ONESTEP, no internal weighting).
    #[test]
    fn fit_measures_reproduce_hydrus() {
        let obs = [-0.0786, -0.1616, -0.2097, -0.3408, -0.4456, -0.498, -0.5614, -0.5937, -0.6824, 0.157];
        let fit = [-0.0873, -0.15815, -0.20401, -0.32826, -0.43624, -0.50039, -0.57524, -0.61561, -0.66793, 0.17166];
        let w = [1.0; 10];
        // RMSE from the residuals of the final run, AIC/BIC with the SSQ at the start of the last iteration (0.1409e-2)
        let m = fit_measures(&obs, &fit, &w, 0.1409e-2, 3);
        assert!((m.r_squared - 0.99799).abs() < 2e-5, "{}", m.r_squared);
        assert!((m.me - -0.1332e-2).abs() < 1e-6, "{}", m.me);
        assert!((m.mae - 0.1070e-1).abs() < 1e-5, "{}", m.mae);
        assert!((m.rmse - 0.1276e-1).abs() < 1e-5, "{}", m.rmse);
        assert!((m.aic - -0.8267e2).abs() < 0.01, "{}", m.aic);
        assert!((m.bic - -0.8177e2).abs() < 0.01, "{}", m.bic);
    }

    /// HYDRUS weights of the measurement sets (Fit.out of UPINFIL: retention points share the weight of the water
    /// contents; DRAINAGE: one set of water contents).
    #[test]
    fn weighting_by_standard_deviation() {
        let mut s = InverseSettings { weighting: 2, ..Default::default() };
        let th = [0.282, 0.246, 0.206, 0.09];
        for &v in &th {
            s.observations.push(Observation { x: 1.0, value: v, kind: 2, pos: 1, weight: 1.0 });
        }
        s.observations.push(Observation { x: -100.0, value: 0.3, kind: 5, pos: 1, weight: 10.0 });
        s.observations.push(Observation { x: 1.0, value: -50.0, kind: 1, pos: 1, weight: 1.0 });
        let w = s.effective_weights();
        let all = [0.282, 0.246, 0.206, 0.09, 0.3];
        let mean = all.iter().sum::<f64>() / 5.0;
        let sd = (all.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / 5.0).sqrt();
        let v = 1.0 / (5f64.sqrt() * sd);
        assert!((w[0] - v).abs() < 1e-12 && (w[4] - 10.0 * v).abs() < 1e-12);
        // a set with a single value gets weight 1
        assert_eq!(w[5], 1.0);
        s.weighting = 1;
        let w = s.effective_weights();
        assert!((w[5] - mean / 50.0).abs() < 1e-12, "mean ratio relative to the set of the smallest mean");
        s.weighting = 0;
        assert_eq!(s.effective_weights()[4], 10.0);
    }

    /// The synthetic one-step outflow example: the parameters used to produce the data are found again.
    #[test]
    fn synthetic_outflow_parameters_are_recovered() {
        let p = crate::examples::inverse_outflow();
        let s = p.inverse.clone().unwrap();
        let r = optimize(&p, &s, &MarqOptions::default(), None, |_| {}).unwrap();
        let want = [0.025, 1.7, 3.0];
        for (q, w) in r.params.iter().zip(want) {
            assert!((q.value - w).abs() / w < 0.01, "{} = {} (want {})", q.name, q.value, w);
            assert!(q.lower_95 <= q.value && q.value <= q.upper_95);
        }
        assert!(r.ssq < 1e-6, "SSQ {}", r.ssq);
        assert!(r.measures.r_squared > 0.99999);
        assert_eq!(r.fitted.len(), s.observations.len());
        assert!(r.final_results.as_ref().is_some_and(|x| !x.tlevel.is_empty()));
        // the fitted parameters are written into the returned project
        let m = &r.project.as_ref().unwrap().water.materials[0];
        assert!((m.alpha - r.params[0].value).abs() < 1e-15 && (m.ks - r.params[2].value).abs() < 1e-15);
    }

    /// A run on the time levels of another run with the same parameters reproduces it; on frozen levels the model
    /// values change smoothly with a parameter (second differences of the order of the rounding errors).
    #[test]
    fn frozen_time_levels() {
        let p = crate::examples::inverse_outflow();
        let s = p.inverse.clone().unwrap();
        let b = s.initial();
        let free = forward(&p, &s, &b, false, None).unwrap();
        let fixed = forward_on(&p, &s, &b, false, None, Some(free.levels.clone()), 0).unwrap();
        assert_eq!(free.levels.len(), fixed.levels.len());
        for (a, c) in free.values.iter().zip(&fixed.values) {
            assert!((a - c).abs() <= 1e-9 * a.abs().max(1e-9), "{a} vs {c}");
        }
        // f(b + k h) for k = -2..2 along n: the second differences are smooth (no jumps of the adaptive time steps)
        let j = 1;
        let vals: Vec<Vec<f64>> = (-2..=2)
            .map(|k| {
                let mut bb = b.clone();
                bb[j] *= 1.0 + 1e-3 * k as f64;
                forward_on(&p, &s, &bb, false, None, Some(free.levels.clone()), 0).unwrap().values
            })
            .collect();
        for i in 0..vals[0].len() {
            let d2a = vals[0][i] - 2.0 * vals[1][i] + vals[2][i];
            let d2b = vals[2][i] - 2.0 * vals[3][i] + vals[4][i];
            let scale = (vals[4][i] - vals[0][i]).abs().max(1e-12);
            assert!((d2a - d2b).abs() <= 0.05 * scale, "observation {i}: second differences {d2a} and {d2b} (range {scale})");
        }
        // a run longer than the step limit fails
        assert!(forward_on(&p, &s, &b, false, None, None, 3).is_err());
    }

    /// Further starting points lie within the constraints, differ from each other and do not depend on the call.
    #[test]
    fn starting_points() {
        let p = crate::examples::inverse_outflow();
        let s = p.inverse.clone().unwrap();
        let idx = s.fitted();
        let pts: Vec<Vec<f64>> = (1..=8).map(|k| starting_point(&s, k)).collect();
        for (k, b) in pts.iter().enumerate() {
            for (j, &i) in idx.iter().enumerate() {
                let (lo, hi) = s.params[i].bounds();
                assert!(lo.is_none_or(|l| b[j] >= l) && hi.is_none_or(|h| b[j] <= h), "start {} parameter {j}: {}", k + 1, b[j]);
            }
            assert_eq!(b, &starting_point(&s, k + 1));
        }
        for a in 0..pts.len() {
            for c in a + 1..pts.len() {
                assert_ne!(pts[a], pts[c]);
            }
        }
    }

    /// Heat transport parameters (lTempF): b1 and Cn of the synthetic soil temperature example are found again from
    /// the temperatures at three observation nodes (observation type 4).
    #[test]
    fn synthetic_heat_parameters_are_recovered() {
        let p = crate::examples::inverse_heat();
        let s = p.inverse.clone().unwrap();
        assert!(s.validate(&p).is_empty(), "{:?}", s.validate(&p));
        assert_eq!(s.params.iter().filter(|q| q.group == ParamGroup::Heat).count(), 9);
        let r = optimize(&p, &s, &MarqOptions::default(), None, |_| {}).unwrap();
        assert_eq!(r.names, ["B1", "Cn"]);
        for (q, w) in r.params.iter().zip([1.5673e16, 1.4333e14]) {
            assert!((q.value - w).abs() / w < 0.005, "{} = {} (want {})", q.name, q.value, w);
        }
        assert!(r.measures.r_squared > 0.99999);
        let m = &r.project.as_ref().unwrap().heat.materials[0];
        assert!((m.b1 - r.params[0].value).abs() < 1.0 && (m.cn - r.params[1].value).abs() < 1.0);
        // solute and heat parameters exclude each other
        let mut s2 = s.clone();
        s2.fit_solute = true;
        assert!(s2.validate(&p).iter().any(|e| e.contains("cannot be estimated together")));
        // unit conversion: temperatures stay, times and the conductivity coefficient are converted
        let mut p1 = p.clone();
        crate::convert_units(&mut p1, LengthUnit::M, TimeUnit::Hours);
        let s1 = p1.inverse.as_ref().unwrap();
        assert!((s1.observations[0].value - s.observations[0].value).abs() < 1e-12);
        assert!((s1.observations[0].x - s.observations[0].x * 24.0).abs() < 1e-12);
        let f0 = forward(&p, &s, &s.initial(), false, None).unwrap().values;
        let f1 = forward(&p1, s1, &s1.initial(), false, None).unwrap().values;
        for k in 0..f0.len() {
            assert!((f1[k] - f0[k]).abs() < 0.02, "{k}: {} vs {}", f1[k], f0[k]);
        }
    }

    /// Units: converting the project converts the settings, and the problem stays the same.
    #[test]
    fn unit_conversion_of_the_settings() {
        let p0 = crate::examples::inverse_outflow();
        let mut p1 = p0.clone();
        crate::convert_units(&mut p1, LengthUnit::Mm, TimeUnit::Days);
        let (s0, s1) = (p0.inverse.as_ref().unwrap(), p1.inverse.as_ref().unwrap());
        let ks = |s: &InverseSettings| s.params.iter().find(|q| q.index == 5).unwrap().value;
        assert!((ks(s1) - ks(s0) * 10.0 * 24.0).abs() < 1e-9);
        assert!((s1.observations[0].x - s0.observations[0].x / 24.0).abs() < 1e-12);
        assert!((s1.observations[0].value - s0.observations[0].value * 10.0).abs() < 1e-12);
        assert!((s1.observations[11].x - s0.observations[11].x * 10.0).abs() < 1e-9);
        let f0 = forward(&p0, s0, &s0.initial(), false, None).unwrap().values;
        let f1 = forward(&p1, s1, &s1.initial(), false, None).unwrap().values;
        for k in 0..f0.len() {
            let scale = if k < 11 { 10.0 } else { 1.0 };
            assert!((f1[k] - f0[k] * scale).abs() < 1e-6 * (1.0 + f1[k].abs()), "{k}: {} vs {}", f1[k], f0[k] * scale);
        }
    }
}
