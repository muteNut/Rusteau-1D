//! Simulation driver: state container, initialisation and the main time loop
//! (port of HYDRUS.FOR, INPUT.FOR initialisation and TIME.FOR).

use crate::error::HydrusError;
use crate::heat::HeatState;
use crate::material::*;
use crate::model::*;
use crate::output::*;
use crate::solute::SoluteState;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StepStatus {
    Running,
    Finished,
    Failed,
}

pub struct Simulation {
    pub prj: Project,
    pub res: Results,
    pub n: usize,
    pub n_mat: usize,
    pub model: SoilModel,
    pub i_hyst: i32,
    pub x_conv: f64,
    pub t_conv: f64,
    // process switches
    pub l_wat: bool,
    pub l_chem: bool,
    pub l_temp: bool,
    pub sink_f: bool,
    pub l_root: bool,
    pub short_o: bool,
    pub top_inf: bool,
    pub bot_inf: bool,
    pub atm_bc: bool,
    pub w_layer: bool,
    pub free_d: bool,
    pub seep_f: bool,
    pub gwl_f: bool,
    pub q_drain: bool,
    pub l_var_bc: bool,
    pub max_it: usize,
    pub tol_th: f64,
    pub tol_h: f64,
    // materials
    pub par_d: Vec<Par>,
    pub par_w: Vec<Par>,
    pub ah_w: Vec<f64>,
    pub ath_w: Vec<f64>,
    pub ak_w: Vec<f64>,
    pub tabs: Vec<MatTable>,
    pub l_table: bool,
    pub h_tab_first: f64,
    pub h_tab_last: f64,
    /// Hydraulic property tables of GenMat (100 log-spaced pressure heads, per material)
    pub h_tab: Vec<f64>,
    pub con_tab: Vec<Vec<f64>>,
    pub cap_tab: Vec<Vec<f64>>,
    pub the_tab: Vec<Vec<f64>>,
    pub h_sat: Vec<f64>,
    pub con_sat: Vec<f64>,
    pub thr: Vec<f64>,
    pub ths: Vec<f64>,
    pub con_s_max: f64,
    // mesh & node properties (index 0 = bottom)
    pub x: Vec<f64>,
    pub mat: Vec<usize>,
    pub lay: Vec<usize>,
    pub beta: Vec<f64>,
    pub ah: Vec<f64>,
    pub ak: Vec<f64>,
    pub ath: Vec<f64>,
    // water state
    pub h_new: Vec<f64>,
    pub h_old: Vec<f64>,
    pub h_temp: Vec<f64>,
    pub th_new: Vec<f64>,
    pub th_old: Vec<f64>,
    pub th_eq: Vec<f64>,
    pub con: Vec<f64>,
    pub cap: Vec<f64>,
    pub sink: Vec<f64>,
    pub con_o: Vec<f64>,
    pub kappa: Vec<i32>,
    pub kappa_o: Vec<i32>,
    pub ath_s: Vec<f64>,
    pub th_rr: Vec<f64>,
    pub con_r: Vec<f64>,
    pub ak_s: Vec<f64>,
    pub v_old: Vec<f64>,
    pub v_new: Vec<f64>,
    // Vapor transport state
    pub l_vapor: bool,
    pub con_lt: Vec<f64>,
    pub con_vt: Vec<f64>,
    pub con_vh: Vec<f64>,
    pub th_v_new: Vec<f64>,
    pub th_v_old: Vec<f64>,
    pub v_v_new: Vec<f64>,
    pub v_v_old: Vec<f64>,
    // boundary conditions
    pub kod_top: i32,
    pub kod_bot: i32,
    pub k_top_old: i32,
    /// Dual permeability: atmospheric top boundary code and head of the MATRIX domain (it switches between flux and head
    /// independently of the fracture domain: -4 flux, 4 head at the dry limit / ponding head)
    pub kod_top_m: i32,
    pub h_top_m: f64,
    pub k_top_old_m: i32,
    pub k_bot_old: i32,
    pub r_top: f64,
    pub r_bot: f64,
    pub r_root: f64,
    pub h_top: f64,
    pub h_bot: f64,
    pub h_crit_a: f64,
    pub h_crit_s: f64,
    pub gwl0l: f64,
    pub aqh: f64,
    pub bqh: f64,
    pub prec: f64,
    pub r_soil: f64,
    pub prec_raw: f64,
    pub r_soil_raw: f64,
    pub r_root_raw: f64,
    pub cos_alf: f64,
    pub v_top: f64,
    pub v_bot: f64,
    pub h_seep: f64,
    // time control
    pub t: f64,
    pub t_old: f64,
    pub dt: f64,
    pub dt_old: f64,
    /// `lExtrap` of the Fortran main program: start each step from the extrapolated head `hNew + (hNew - hOld)*dt/dtOld`.
    pub l_extrap: bool,
    pub dt_opt: f64,
    pub dt_min: f64,
    pub dt_max: f64,
    pub d_mul: f64,
    pub d_mul2: f64,
    pub dt_init: f64,
    pub it_min: usize,
    pub it_max: usize,
    pub t_init: f64,
    pub t_max: f64,
    pub t_atm: f64,
    pub t_atm1: f64,
    pub t_atm2: f64,
    pub t_atm_old: f64,
    pub t_print: Vec<f64>,
    pub t_print1: f64,
    pub print_step: usize,
    pub print_daily: bool,
    pub print_int: f64,
    pub p_level: usize,
    pub t_level: usize,
    pub min_step: bool,
    pub iter_w: usize,
    pub iter_c: usize,
    pub it_cum: usize,
    pub convg: bool,
    pub n_noconv: usize,
    pub dt_max_c: f64,
    pub dt_max_t: f64,
    pub target_times: Vec<f64>,
    pub target_idx: usize,
    // atmosphere
    pub atm_idx: usize,
    pub meteo_idx: usize,
    pub r_root_d: f64,
    pub r_soil_d: f64,
    pub prec_d: f64,
    pub snow_layer: f64,
    pub interc_state: crate::meteo::InterceptionState,
    // cumulative water quantities
    pub cum_q: [f64; 12],
    pub w_cum_t: f64,
    pub w_cum_a: f64,
    pub w_vol_i: f64,
    pub wat_in: Vec<f64>,
    pub h_root: f64,
    pub v_root: f64,
    pub x_root: f64,
    pub rg: f64,
    pub omega_c: f64,
    pub l_end: bool,
    pub done: bool,
    pub failed: bool,
    // Dual porosity state
    pub i_dual_por: i32,
    pub th_new_im: Vec<f64>,
    pub th_old_im: Vec<f64>,
    pub sink_im: Vec<f64>,
    pub w_transf: f64,
    // Dual permeability state (Model 8)
    pub l_dual_perm: bool,
    pub w_fracture: f64,
    pub h_matrix_new: Vec<f64>,
    pub h_matrix_old: Vec<f64>,
    pub th_matrix_new: Vec<f64>,
    pub th_matrix_old: Vec<f64>,
    /// Matrix dual porosity: immobile water content of the matrix (new / old) and the mobile -> immobile water
    /// transfer per unit matrix volume
    pub th_im_mat_new: Vec<f64>,
    pub th_im_mat_old: Vec<f64>,
    pub sink_imm: Vec<f64>,
    pub matrix_dual_por: bool,
    pub con_matrix: Vec<f64>,
    pub cap_matrix: Vec<f64>,
    // sub-models
    pub heat: Option<HeatState>,
    pub sol: Option<SoluteState>,
    // Bob Lenhard air-entrapment hysteresis (iHyst = 3)
    pub lenhard_i_hyst: i32,
    /// Number of HAWPATH error branches hit (no scanning path found for a trial head)
    pub lenhard_path_failures: usize,
    pub lenhard: Option<crate::lenhard::LenhardState>,
    // Water-heat(-vapor) coupling loop (iTemp in HYDRUS.FOR) and surface energy balance
    pub i_temp: i32,
    pub l_en_bal: bool,
    pub en_bal: EnBalState,
    /// lFluxOut of HYDRUS.FOR (hard-wired .false. in Init): accumulate CumQ(9), CumQ(10)
    pub l_flux_out: bool,
    /// Index of the meteo record for which Meteo.out was written last (the A-level block re-evaluates the
    /// snow/interception state without a new SetMeteo call)
    pub met_row_key: Option<usize>,
    /// Result of the last SetMeteo call (for Meteo.out)
    pub met_pet: crate::meteo::PotEt,
    /// OmegaW (ratio of actual and potential transpiration, used by active root solute uptake)
    pub omega_w: f64,
    /// SPot: potential root solute uptake (constant from Selector.in, then the cB column of Atmosph.in)
    pub s_pot: f64,
    // Daily-generated meteorological data (lMetDaily) and meteo-derived crop state
    pub l_daily_met: bool,
    pub day_met: crate::meteo::DailyMetState,
    /// Net shortwave radiation at the current time step [MJ/m2/d] (Rst) and daily Rad
    pub rst: f64,
    pub day_rad: f64,
    pub met_lai: f64,
    pub met_scf: f64,
    // Snow / interception are evaluated once per atmospheric record from this saved state
    pub dt_snow: f64,
    pub snow_ev0: f64,
    pub interc_ev0: crate::meteo::InterceptionState,
    /// Precipitation after the snow model (before interception)
    pub prec_base: f64,
}

/// Surface energy-balance quantities passed from Evapor to UpdateEnergy.
#[derive(Clone, Copy, Debug, Default)]
pub struct EnBalState {
    pub evap: f64,      // [kg/m2/s]
    pub sens_flux: f64, // [W/m2]
    pub lat: f64,       // [J/kg]
    pub const_: f64,    // `Const` of Evapor
    pub r_h: f64,       // [s/m]
    pub rn: f64,        // net radiation [MJ/m2/d]
    pub rns: f64,       // net shortwave radiation [MJ/m2/d]
    pub rnl: f64,       // net longwave radiation [MJ/m2/d]
}

impl Simulation {
    pub fn new(prj: Project) -> Result<Simulation, HydrusError> {
        let problems = prj.validate();
        if !problems.is_empty() {
            return Err(HydrusError::Invalid(problems.join("\n")));
        }
        let n = prj.profile.nodes.len();
        let n_mat = prj.water.materials.len();
        let model = prj.water.model;
        let i_hyst = prj.water.hysteresis.code();
        let i_dual_por = match model {
            SoilModel::DualPorosityW => 1,
            SoilModel::DualPorosityH => 2,
            _ => 0,
        };

        let x_conv = prj.units.x_conv();
        let t_conv = prj.units.t_conv();
        let bc = prj.water.bc.clone();
        let l_wat = prj.processes.water_flow;
        let l_en_bal = prj.processes.heat && prj.atmosphere.meteo.as_ref().map(|m| m.l_en_bal).unwrap_or(false);
        let l_daily_met = prj.atmosphere.meteo.as_ref().map(|m| m.l_daily && !m.records.is_empty()).unwrap_or(false);
        let l_chem = prj.processes.solute;
        let l_temp = prj.processes.heat;
        let sink_f = prj.processes.root_water_uptake;
        let l_root = prj.processes.root_growth;
        let ns = prj.n_solutes();

        // ---- materials
        let mut par_d = vec![];
        let mut par_w = vec![[0.0; 11]; n_mat];
        let mut ah_w = vec![1.0; n_mat];
        let mut ath_w = vec![1.0; n_mat];
        let mut ak_w = vec![1.0; n_mat];
        for (m, mat) in prj.water.materials.iter().enumerate() {
            let mut p = par_of(mat, model, x_conv);
            if i_hyst > 0 {
                p[6] = mat.qm.max(mat.qs);
                let mut w = [0.0; 11];
                w[1] = mat.qs_w;
                w[2] = mat.alpha_w;
                w[4] = mat.ks_w;
                w[0] = p[0];
                w[3] = p[3];
                ah_w[m] = p[2] / w[2];
                ath_w[m] = (w[1] - w[0]) / (p[1] - p[0]);
                ak_w[m] = 1.0;
                if i_hyst == 2 {
                    ak_w[m] = w[4] / p[4];
                }
                w[6] = w[0] + ath_w[m] * (p[6] - p[0]);
                p[7] = p[0];
                p[8] = p[1];
                p[9] = p[4];
                w[7] = w[0];
                w[8] = w[1];
                w[9] = w[4];
                w[5] = p[5];
                par_w[m] = w;
            }
            par_d.push(p);
        }
        // MATERIAL.FOR: for iModel = 0 (van Genuchten-Mualem) FK/FC/FQ use Qm = Qs also with Kool-Parker hysteresis; the
        // thm column of the hysteresis input only feeds ParD(7) = max(thm, ths).  (The port used to evaluate these cases as
        // the modified VG model with Qm = thm, which reproduces the 1SCALING sample output of an older HYDRUS build but
        // not the compiled 2009 source: initial K of 1SCALING 9.5e-6 instead of 1.23e-7.  Verified against the compiled
        // source for 1SCALING and 2HYSTER.)

        let tabs = if let Some(ref external_tabs) = prj.water.tabs {
            external_tabs.clone()
        } else {
            Vec::new()
        };
        // GenMat / MatIn: tables of K, C and theta (NTab = 100); lTable decides from hTab1 / hTabN
        let (mut l_table, mut h1, mut hn) = (false, 0.0f64, 0.0f64);
        let mut h_tab: Vec<f64> = Vec::new();
        let (mut con_tab, mut cap_tab, mut the_tab) = (Vec::new(), Vec::new(), Vec::new());
        if prj.water.use_tables && model != SoilModel::Tabular {
            let (a, b) = (prj.water.h_tab1, prj.water.h_tab_n);
            h1 = -a.abs().min(b.abs());
            hn = -h1.abs().max(b.abs()); // (MatIn uses the already modified hTab1)
            l_table = true;
            if (h1 > -0.00001 && hn > -0.00001) || h1 == hn {
                l_table = false;
                h1 = -0.0001 * x_conv;
                hn = -100.0 * x_conv;
            }
            let nt = 100usize;
            let dlh = ((-hn).log10() - (-h1).log10()) / (nt - 1) as f64;
            h_tab = (0..nt).map(|i| -(10f64).powf((-h1).log10() + i as f64 * dlh)).collect();
            for p in par_d.iter() {
                con_tab.push(h_tab.iter().map(|&h| fk(model, h, p)).collect::<Vec<f64>>());
                cap_tab.push(h_tab.iter().map(|&h| fc(model, h, p)).collect::<Vec<f64>>());
                the_tab.push(h_tab.iter().map(|&h| fq(model, h, p)).collect::<Vec<f64>>());
            }
        }

        let h_sat: Vec<f64> = par_d.iter().map(|p| fh(model, 1.0, p)).collect();
        let con_sat: Vec<f64> = par_d.iter().map(|p| p[4]).collect();
        let thr: Vec<f64> = par_d.iter().map(|p| p[0]).collect();
        let ths: Vec<f64> = par_d.iter().map(|p| p[1]).collect();
        let con_s_max = con_sat.iter().cloned().fold(0.0, f64::max);

        // ---- profile (flip to bottom -> top)
        let nodes: Vec<&ProfileNode> = prj.profile.nodes.iter().rev().collect();
        let x: Vec<f64> = nodes.iter().map(|nd| -nd.depth).collect();
        let mat: Vec<usize> = nodes.iter().map(|nd| nd.mat - 1).collect();
        let lay: Vec<usize> = nodes.iter().map(|nd| nd.layer.max(1)).collect();
        let mut beta: Vec<f64> = nodes.iter().map(|nd| nd.beta).collect();
        let ah: Vec<f64> = nodes.iter().map(|nd| nd.ah).collect();
        let ak: Vec<f64> = nodes.iter().map(|nd| nd.ak).collect();
        let ath: Vec<f64> = nodes.iter().map(|nd| nd.ath).collect();

        // normalise root distribution (as in NodInf)
        {
            let mut sb = 0.0;
            if beta[n - 1] > 0.0 {
                sb = beta[n - 1] * (x[n - 1] - x[n - 2]) / 2.0;
            }
            for i in 1..n - 1 {
                if beta[i] > 0.0 {
                    sb += beta[i] * (x[i + 1] - x[i - 1]) / 2.0;
                }
            }
            for b in beta.iter_mut().skip(1) {
                *b = if sb > 0.0 { *b / sb } else { 0.0 };
            }
        }

        let mut h0: Vec<f64> = nodes.iter().map(|nd| nd.h).collect();
        let kappa0 = prj.water.hysteresis.initial_branch_code(prj.water.init_kappa);
        let kappa = vec![kappa0; n];

        // Lower bound on the pressure head (only meaningful when h0 really is a head)
        if !prj.water.init_in_water_content {
            for i in 0..n {
                let m = mat[i];
                if model.code() < 10 {
                    let h_bound = ah[i] * fh(model, 0.00000001, &par_d[m]);
                    if h0[i] < h_bound {
                        h0[i] = h_bound;
                    }
                }
            }
        }

        // Immobile water content from the initial condition (Fortran InitW, iDualPor > 0)
        let mut th_im_water = vec![0.0; n];

        if prj.water.init_in_water_content {
            for i in 0..n {
                let m = mat[i];
                if h0[i] <= 0.0 || h0[i] > 1.0 {
                    continue;
                }
                if i_dual_por > 0 {
                    let th_total = h0[i];
                    h0[i] = th_total * par_d[m][1] / (par_d[m][1] + par_d[m][7]);
                    th_im_water[i] = th_total - h0[i];
                }
                let (qr, qs) = if kappa[i] == -1 {
                    (par_d[m][0], par_d[m][1])
                } else {
                    (par_w[m][0], par_w[m][1])
                };
                let d_xz = if ath[i] > 0.0 { ath[i] } else { 1.0 };
                let denom = (qs - qr) * d_xz;
                if denom > 1e-12 {
                    let qe = (h0[i] - qr) / denom;
                    if qe >= 1.0 {
                        h0[i] = 0.0;
                    } else {
                        let qe_safe = qe.max(1e-6);
                        if kappa[i] == -1 {
                            h0[i] = fh(model, qe_safe, &par_d[m]) * ah[i];
                        } else {
                            h0[i] = fh(model, qe_safe, &par_w[m]) * ah[i] * ah_w[m];
                        }
                    }
                }
            }
        }

        // ---- boundary condition codes (BasInf "input modifications")
        let mut kod_top = bc.kod_top;
        let mut kod_bot = bc.kod_bot;
        let l_var_bc = kod_top == 0;
        if bc.top_time_variable {
            kod_top = if kod_top >= 0 { 3 } else { -3 };
        }
        if bc.bot_time_variable {
            kod_bot = if kod_bot >= 0 { 3 } else { -3 };
        }
        let h_crit_s = prj.atmosphere.h_crit_s;
        if bc.atmospheric && kod_top < 0 {
            kod_top = -4;
        }
        if bc.surface_layer {
            kod_top = -kod_top.abs();
        }
        if bc.gwl_flux {
            kod_bot = -7;
        }
        if bc.free_drainage {
            kod_bot = -5;
        }
        if bc.seepage_face {
            kod_bot = -2;
        }
        let const_flux_read = (!bc.top_time_variable && bc.kod_top == -1)
            || (!bc.bot_time_variable && bc.kod_bot == -1 && !bc.gwl_flux && !bc.free_drainage && !bc.seepage_face && bc.drains.is_none());
        let (r_top, r_bot, r_root) = if const_flux_read { (bc.r_top, bc.r_bot, bc.r_root.abs()) } else { (0.0, 0.0, 0.0) };

        // Initialize th_old_im and th_new_im with ths_im (par_d[m][7]) or initial condition
        let mut th_init_im = vec![0.0; n];
        for i in 0..n {
            let m = mat[i];
            th_init_im[i] = if prj.water.init_in_water_content && i_dual_por > 0 {
                th_im_water[i]
            } else {
                par_d[m][7]
            };
        }

        let l_dual_perm = model == SoilModel::DualPermeability;
        let prj_matrix_dual_por = prj.water.dual_perm.matrix_dual_por;
        let w_fracture = if l_dual_perm { prj.water.dual_perm.w.clamp(0.001, 0.999) } else { 0.0 };

        let tm = &prj.time;
        let mut t_print = tm.print_times.clone();
        t_print.push(tm.t_max);
        let th_rr: Vec<f64> = (0..n).map(|i| thr[mat[i]]).collect();

        let mut sim = Simulation {
            n,
            n_mat,
            model,
            i_hyst,
            x_conv,
            t_conv,
            l_wat,
            l_chem,
            l_temp,
            sink_f,
            l_root,
            short_o: prj.processes.short_output,
            top_inf: bc.top_time_variable,
            bot_inf: bc.bot_time_variable,
            atm_bc: bc.atmospheric,
            w_layer: bc.surface_layer,
            free_d: bc.free_drainage,
            seep_f: bc.seepage_face,
            gwl_f: bc.gwl_flux,
            q_drain: bc.drains.is_some(),
            l_var_bc,
            max_it: prj.water.max_iter,
            tol_th: prj.water.tol_th,
            tol_h: prj.water.tol_h,
            par_d,
            par_w,
            ah_w,
            ath_w,
            ak_w,
            tabs,
            l_table,
            h_tab_first: h1,
            h_tab_last: hn,
            h_tab,
            con_tab,
            cap_tab,
            the_tab,
            h_sat,
            con_sat,
            thr,
            ths,
            con_s_max,
            x,
            mat,
            lay,
            beta,
            ah,
            ak,
            ath,
            h_new: h0.clone(),
            h_old: h0.clone(),
            h_temp: h0.clone(),
            th_new: vec![0.0; n],
            th_old: vec![0.0; n],
            th_eq: vec![0.0; n],
            con: vec![0.0; n],
            cap: vec![0.0; n],
            sink: vec![0.0; n],
            con_o: vec![0.0; n],
            kappa: kappa.clone(),
            kappa_o: kappa.clone(),
            ath_s: vec![1.0; n],
            th_rr,
            con_r: vec![0.0; n],
            ak_s: vec![1.0; n],
            v_old: vec![0.0; n],
            v_new: vec![0.0; n],
            kod_top,
            kod_bot,
            k_top_old: kod_top,
            kod_top_m: kod_top,
            h_top_m: 0.0,
            k_top_old_m: kod_top,
            k_bot_old: kod_bot,
            r_top,
            r_bot,
            r_root,
            h_top: h0[n - 1],
            h_bot: h0[0],
            h_crit_a: -1e10,
            h_crit_s,
            gwl0l: bc.gwl0l,
            aqh: bc.aqh,
            bqh: bc.bqh,
            prec: 0.0,
            r_soil: 0.0,
            prec_raw: 0.0,
            r_soil_raw: 0.0,
            r_root_raw: 0.0,
            cos_alf: prj.cos_alpha,
            v_top: 0.0,
            v_bot: 0.0,
            h_seep: bc.h_seep,
            // dt is REAL*4 in the Fortran code (t double precision): see `tm_cont`
            t: tm.t_init + tm.dt as f32 as f64,
            t_old: tm.t_init,
            dt: tm.dt as f32 as f64,
            dt_old: tm.dt as f32 as f64,
            l_extrap: true, // HYDRUS.FOR: lExtrap=.true. (tested: the reference outputs need it, Test10 fails without)
            dt_opt: tm.dt as f32 as f64,
            dt_min: tm.dt_min,
            dt_max: tm.dt_max,
            d_mul: tm.d_mul,
            d_mul2: tm.d_mul2,
            dt_init: tm.dt,
            it_min: tm.it_min,
            it_max: tm.it_max,
            t_init: tm.t_init,
            t_max: tm.t_max,
            t_atm: tm.t_max,
            t_atm1: tm.t_max,
            t_atm2: tm.t_max,
            t_atm_old: tm.t_init,
            t_print,
            t_print1: tm.t_max,
            print_step: tm.print_step.max(1),
            print_daily: tm.print_at_interval,
            print_int: tm.print_interval,
            p_level: 0,
            t_level: 1,
            // DEVIATION FROM THE 2009 SOURCE, confirmed by HYDRUS 4.17: Init sets lMinStep=.true. (the first TmCont call would
            // keep dt at its initial value), the 4.17 binary grows dt from the first step: 1SCALING 0.01, 0.023, 0.0399, ...
            // and 1220 time levels, all reproduced only with lMinStep = .false. here (with .true.: 0.01, 0.02, 0.033, 1221).
            min_step: false,
            iter_w: 0,
            iter_c: 0,
            it_cum: 0,
            convg: true,
            n_noconv: 0,
            dt_max_c: 1e30,
            dt_max_t: 1e30,
            target_times: Vec::new(),
            target_idx: 0,
            atm_idx: 0,
            meteo_idx: 0,
            r_root_d: 0.0,
            r_soil_d: 0.0,
            prec_d: 0.0,
            snow_layer: 0.0,
            interc_state: crate::meteo::InterceptionState::default(),
            cum_q: [0.0; 12],
            w_cum_t: 0.0,
            w_cum_a: 0.0,
            w_vol_i: 0.0,
            wat_in: vec![0.0; n],
            h_root: 0.0,
            v_root: 0.0,
            x_root: 0.0,
            rg: 0.0,
            omega_c: prj.root.omega_c,
            l_end: false,
            done: false,
            failed: false,
            lenhard_i_hyst: 0,
            lenhard_path_failures: 0,
            lenhard: None,
            i_temp: 0,
            l_en_bal,
            en_bal: EnBalState::default(),
            l_flux_out: tm.flux_out,
            met_pet: crate::meteo::PotEt::default(),
            met_row_key: None,
            omega_w: 1.0,
            s_pot: prj.root.s_pot,
            l_daily_met,
            day_met: crate::meteo::DailyMetState::default(),
            rst: 0.0,
            day_rad: 0.0,
            met_lai: 0.0,
            met_scf: 0.0,
            dt_snow: 0.0,
            snow_ev0: 0.0,
            interc_ev0: crate::meteo::InterceptionState::default(),
            prec_base: 0.0,
            l_vapor: prj.processes.vapor,
            con_lt: vec![0.0; n],
            con_vt: vec![0.0; n],
            con_vh: vec![0.0; n],
            th_v_new: vec![0.0; n],
            th_v_old: vec![0.0; n],
            v_v_new: vec![0.0; n],
            v_v_old: vec![0.0; n],
            i_dual_por,
            th_new_im: th_init_im.clone(),
            th_old_im: th_init_im,
            sink_im: vec![0.0; n],
            w_transf: 0.0,
            l_dual_perm,
            w_fracture,
            h_matrix_new: h0.clone(),
            h_matrix_old: h0.clone(),
            th_matrix_new: vec![0.0; n],
            th_matrix_old: vec![0.0; n],
            th_im_mat_new: vec![0.0; n],
            th_im_mat_old: vec![0.0; n],
            sink_imm: vec![0.0; n],
            matrix_dual_por: l_dual_perm && prj_matrix_dual_por,
            con_matrix: vec![0.0; n],
            cap_matrix: vec![0.0; n],
            heat: None,
            sol: None,
            res: Results { n_solutes: ns, ..Default::default() },
            prj,
        };
        if sim.print_daily {
            sim.t_print1 = sim.t_init + sim.print_int;
        }
        sim.res.obs_nodes = sim.prj.profile.observation_nodes.clone();
        sim.initialise()?;
        Ok(sim)
    }

    fn initialise(&mut self) -> Result<(), HydrusError> {
        // hTop / hBot come from the initial condition at the boundary nodes
        self.h_top = self.h_new[self.n - 1];
        self.h_bot = self.h_new[0];

        // Temperature first: set_mat (vapor flow) reads it
        if self.l_temp {
            self.heat_init()?;
        }

        self.init_water_state();

        if self.l_chem {
            self.solute_init()?; // uses th_old
        }

        if self.top_inf || self.bot_inf || self.atm_bc {
            self.init_atmosphere()?;
        }

        if self.l_root {
            self.set_rg();
        }
        if self.sink_f {
            self.set_snk();
        }

        // Initial output
        self.profile_out(self.t_init);
        self.sub_reg(0);
        self.obs_nod();

        if self.l_chem || self.l_temp || self.l_vapor {
            let (ho, thn) = (self.h_old.clone(), self.th_old.clone());
            let temps: Vec<f64> = (0..self.n).map(|i| self.temp(i)).collect();
            let (v, vv) = self.veloc(&ho, &thn, &thn, &temps);
            self.v_old = v;
            self.v_v_old = vv;
        }
        self.v_new = self.v_old.clone();
        self.v_v_new = self.v_v_old.clone();
        Ok(())
    }

    /// Initial hydraulic properties, water contents and immobile water.
    fn init_water_state(&mut self) {
        if self.i_hyst == 3 {
            let ik = self.prj.water.init_kappa;
            self.lenhard_hyst(ik, 1, crate::lenhard::ThetaTarget::Old);
        } else {
            self.set_mat(0);
            self.th_old.copy_from_slice(&self.th_eq);
        }
        self.th_new.copy_from_slice(&self.th_old);

        if self.l_vapor {
            self.th_v_old = self.th_v_new.clone();
        }

        if self.l_dual_perm {
            self.set_mat_matrix();
            self.th_matrix_old.copy_from_slice(&self.th_matrix_new);
            if self.matrix_dual_por {
                // the immobile region of the matrix starts saturated (as the immobile region of the dual-porosity model)
                // InitDualPor: water-content driven (iDualPor = 1): same relative saturation as the mobile matrix water;
                // pressure-head driven (iDualPor = 2): in equilibrium with the initial matrix pressure head
                let head = self.prj.water.dual_perm.matrix_dual_por_head;
                for i in 0..self.n {
                    let m = self.mat[i];
                    let dm = self.prj.water.dual_perm.material(m);
                    let t = if self.prj.water.init_in_water_content {
                        dm.ths_im
                    } else if head {
                        let par_im: Par = [dm.thr_im, dm.ths_im, dm.alpha_im, dm.n_im, dm.ks_im, dm.l_matrix, 0.0, 0.0, 0.0, 0.0, 0.0];
                        fq(SoilModel::VanGenuchten, self.h_matrix_new[i], &par_im)
                    } else {
                        let (thr_m, ths_m) = (self.par_d[m][6], self.par_d[m][7]);
                        let se = ((self.th_matrix_new[i] - thr_m) / (ths_m - thr_m)).clamp(0.0, 1.0);
                        dm.thr_im + se * (dm.ths_im - dm.thr_im)
                    };
                    self.th_im_mat_new[i] = t;
                    self.th_im_mat_old[i] = t;
                }
            }
        }

        if self.i_dual_por > 0 && !self.prj.water.init_in_water_content {
            for i in 0..self.n {
                let p = self.par_d[self.mat[i]];
                self.th_new_im[i] = if self.i_dual_por == 1 {
                    let se = (self.th_old[i] - p[0]) / (p[1] - p[0]);
                    p[6] + se * (p[7] - p[6])
                } else {
                    let par_im: Par = [p[6], p[7], p[8], p[9], p[10], p[5], 0.0, 0.0, 0.0, 0.0, 0.0];
                    fq(SoilModel::VanGenuchten, self.h_new[i], &par_im)
                };
            }
            self.th_old_im = self.th_new_im.clone();
        }
    }

    /// First atmospheric record and derived surface fluxes.
    fn init_atmosphere(&mut self) -> Result<(), HydrusError> {
        self.atm_idx = 0;
        self.meteo_idx = 0;
        self.snow_layer = 0.0;
        self.interc_state = crate::meteo::InterceptionState::default();
        self.t_atm2 = self.t_max;
        self.set_bc()?; // also applies snow / interception (see #10)
        if self.l_daily_met {
            self.daily_met_init();
        }

        let mut next = self.t_atm1.min(self.t_atm2);
        if let Some(mp) = &self.prj.atmosphere.meteo {
            if let Some(rec) = mp.records.get(self.meteo_idx) {
                if rec.t > self.t_init {
                    next = next.min(rec.t);
                }
            }
        }
        self.t_atm = next;

        if self.l_chem {
            self.set_chem_bc();
        }
        if self.prj.atmosphere.daily_variation && !self.l_daily_met {
            self.r_root_d = self.r_root;
            self.r_soil_d = self.r_soil;
            self.daily_var_root_soil();
        }
        if self.prj.atmosphere.sinusoidal_precip {
            self.prec_d = self.prec;
            self.t_atm_old = self.t_init;
            self.sin_prec();
        }
        if self.kod_top == -4 || !self.l_var_bc {
            self.r_top = self.r_soil.abs() - self.prec.abs();
        }
        Ok(())
    }

    /// Evaluates the snow model and the canopy interception for the current atmospheric record.
    ///
    /// The result is always derived from the raw record values and from the snow /
    /// interception state stored when the record was read (`snow_ev0`, `interc_ev0`), so the
    /// function can be called repeatedly for the same record without accumulating snow or
    /// intercepted water twice. As in HYDRUS-1D, Snow is evaluated once per record with the
    /// record interval `tAtm1 - tAtmOld`.
    pub fn apply_snow_and_interception(&mut self) {
        self.prec = self.prec_raw;
        self.r_soil = self.r_soil_raw;
        self.r_root = self.r_root_raw;
        self.snow_layer = self.snow_ev0;
        if !self.l_daily_met {
            self.interc_state = self.interc_ev0;
        }

        // 1. Snow accumulation and melt
        if self.prj.atmosphere.snow {
            let temp_air = if let Some(ref mp) = self.prj.atmosphere.meteo {
                if self.meteo_idx < mp.records.len() {
                    let r = &mp.records[self.meteo_idx];
                    (r.t_max + r.t_min) / 2.0
                } else {
                    0.0
                }
            } else if self.atm_idx > 0 && self.atm_idx <= self.prj.atmosphere.records.len() {
                // the record that was read last (tTop column)
                self.prj.atmosphere.records[self.atm_idx - 1].t_top
            } else {
                0.0
            };
            let dt_s = if self.dt_snow > 0.0 { self.dt_snow } else { self.dt };
            let mut c_top_dummy = vec![0.0; self.n_species()];
            let c_t_dummy = vec![0.0; self.n_species()];

            let (p_eff, s_pack, evap_eff, min_step) = crate::meteo::calculate_snow(
                self.prec,
                temp_air,
                dt_s,
                self.prj.atmosphere.snow_mf,
                self.snow_layer,
                self.r_soil,
                self.x_conv,
                &mut c_top_dummy,
                &c_t_dummy,
            );

            self.prec = p_eff;
            self.snow_layer = s_pack;
            self.r_soil = evap_eff;
            if min_step {
                self.min_step = true;
            }
        }
        self.prec_base = self.prec;

        // 2. Canopy interception with the parameters of Meteo.in (iInterc = 1). LAI and soil
        // cover are the values returned by CropRes; Prec and TransP are handled in mm/d and
        // aInterc in mm, as in Intercept. (With daily-generated meteo this is done for every
        // time step in `day_met_values`.)
        let meteo_interc = self.prj.atmosphere.meteo.as_ref().map(|m| (m.i_interc, m.a_interc, m.l_en_bal));
        if let Some((i_interc, a_mm, mp_en_bal)) = meteo_interc {
            let k = 0.001 * self.x_conv / (24.0 * 3600.0 * self.t_conv); // [L/T] per [mm/d]
            let prec_mm = self.prec / k;
            let (mut r_interc, mut ex_interc) = (0.0, self.interc_state.excess);
            if i_interc == 1 && !self.l_daily_met {
                let (net_p, net_t, total) = crate::meteo::calculate_interception(
                    self.prec / k,
                    self.r_root / k,
                    self.met_lai,
                    a_mm,
                    self.met_scf,
                    &mut self.interc_state,
                );
                self.prec = net_p * k;
                self.r_root = net_t * k;
                r_interc = total;
                ex_interc = self.interc_state.excess;
            }
            // Meteo.out, Penman-Monteith with meteo records: one row per SetMeteo call
            if !self.l_daily_met && !mp_en_bal && self.met_row_key != Some(self.meteo_idx) {
                self.met_row_key = Some(self.meteo_idx);
                let tt_conv = 24.0 * 3600.0 * self.t_conv;
                let rec_t = self.prj.atmosphere.meteo.as_ref().and_then(|m| m.records.get(self.meteo_idx)).map(|r| r.t).unwrap_or(self.t);
                let mut day_no = (rec_t / tt_conv).rem_euclid(365.0);
                if day_no == 0.0 {
                    day_no = 365.0;
                }
                let p = self.met_pet;
                self.res.meteo_mode = 0;
                self.res.meteo.push(crate::output::MeteoRow {
                    t: day_no,
                    et: p.et_comb,
                    evap_p: p.evap_p,
                    transp_p: p.trans_p,
                    rns: p.rns,
                    rnl: p.rnl,
                    rad_term: p.rad_term,
                    aero_term: p.aero_term,
                    prec: prec_mm,
                    interc: r_interc,
                    ex_interc,
                    ..Default::default()
                });
            }
        }

        if self.atm_bc || self.top_inf {
            if self.kod_top.abs() == 4 || !self.l_var_bc {
                self.r_top = self.r_soil.abs() - self.prec.abs();
            }
        }
    }

    // ---------------------------------------------------------------- main loop
    /// Advance the simulation by one time step.
    pub fn step(&mut self) -> StepStatus {
        if self.done {
            return if self.failed { StepStatus::Failed } else { StepStatus::Finished };
        }
        // ---- water flow, root uptake and heat transport. With vapor flow or the surface
        // energy balance the water and heat problems are coupled and solved twice per time
        // step (iTemp loop of HYDRUS.FOR): the second pass uses the new temperatures.
        let coupled = self.l_temp && (self.l_vapor || self.l_en_bal);
        self.i_temp = 0;
        loop {
            if self.l_en_bal {
                self.evapor_pass();
            }
            if self.l_wat {
                self.wat_flow();
                if !self.convg {
                    self.n_noconv += 1;
                } else {
                    self.n_noconv = 0;
                }
                if !self.convg && self.n_noconv >= 10 {
                    self.res.messages.push(format!("The numerical solution has not converged (t = {:.6}).", self.t));
                    self.done = true;
                    self.failed = true;
                    self.res.failed = true;
                    return StepStatus::Failed;
                }
                if self.l_en_bal {
                    self.update_energy();
                }
            } else {
                self.iter_w = 1;
                self.it_cum += 1;
            }

            if self.l_wat && (self.l_temp || self.l_chem || self.l_vapor) {
                let (hn, thn, tho) = (self.h_new.clone(), self.th_new.clone(), self.th_old.clone());
                let temps: Vec<f64> = (0..self.n).map(|i| self.temp(i)).collect();
                let (v, vv) = self.veloc(&hn, &thn, &tho, &temps);
                self.v_new = v;
                self.v_v_new = vv;
            }
            if self.l_root {
                self.set_rg();
            }
            if self.sink_f {
                self.set_snk();
            }
            if self.l_temp {
                self.temper();
                if coupled && self.i_temp < 1 {
                    self.i_temp += 1;
                    continue;
                }
            }
            break;
        }
        if self.l_chem {
            if let Err(e) = self.solute() {
                self.res.messages.push(e.to_string());
                self.done = true;
                self.failed = true;
                self.res.failed = true;
                return StepStatus::Failed;
            }
        }

        // ---- output
        if (self.t - self.t_max).abs() <= 0.5 * self.dt_min || self.t > self.t_max {
            self.l_end = true;
        }
        let j_print = self.j_print();
        self.tl_inf(j_print);
        self.obs_nod();

        if self.print_daily && (self.t_print1 - self.t).abs() < 0.001 * self.dt {
            self.t_print1 += self.print_int;
        }
        let at_profile_print = (self.p_level < self.t_print.len() && (self.t_print[self.p_level] - self.t).abs() <= 0.01 * self.dt)
            || self.l_end;

        if at_profile_print {
            self.profile_out(self.t);
            self.sub_reg(self.p_level + 1);
            if self.p_level < self.t_print.len() && (self.t_print[self.p_level] - self.t).abs() <= 0.01 * self.dt {
                self.p_level += 1;
            }
        }

        // ---- A-level
        if (self.t - self.t_atm).abs() <= 0.001 * self.dt && (self.top_inf || self.bot_inf || self.atm_bc) {
            self.a_level();
            if (self.t - self.t_atm1).abs() <= 0.001 * self.dt {
                self.t_atm_old = self.t_atm1;
                if self.set_bc().is_err() {
                    self.res.messages.push("Error reading atmospheric data.".into());
                    self.done = true;
                    self.failed = true;
                    return StepStatus::Failed;
                }
            }
            if self.l_daily_met {
                self.daily_met_boundary();
            } else if let Err(e) = self.meteo_record_boundary() {
                self.res.messages.push(format!("{}", e));
                self.done = true;
                self.failed = true;
                return StepStatus::Failed;
            }
            let mut next_atm_time = self.t_atm1.min(self.t_atm2);
            if let Some(ref mp) = self.prj.atmosphere.meteo {
                if self.meteo_idx < mp.records.len() {
                    let t_meteo = mp.records[self.meteo_idx].t;
                    if t_meteo > self.t {
                        next_atm_time = next_atm_time.min(t_meteo);
                    }
                }
            }
            self.t_atm = next_atm_time;
            if self.l_chem {
                self.set_chem_bc();
            }
            self.apply_snow_and_interception();
            if self.prj.atmosphere.daily_variation && !self.l_daily_met {
                self.r_root_d = self.r_root;
                self.r_soil_d = self.r_soil;
            }
            if self.prj.atmosphere.sinusoidal_precip {
                self.prec_d = self.prec;
            }
        }
        if self.l_chem {
            self.sol_wlayer_mix();
        }

        // ---- time governing
        if (self.t - self.t_max).abs() <= 0.5 * self.dt_min || self.t > self.t_max {
            self.done = true;
            self.res.finished = true;
            return StepStatus::Finished;
        }
        self.t_old = self.t;
        self.dt_old = self.dt;
        self.k_top_old = self.kod_top;
        self.k_top_old_m = self.kod_top_m;
        self.k_bot_old = self.kod_bot;
        if !self.l_wat {
            self.iter_w = 1;
        }
        let iter = self.iter_w.max(self.iter_c);
        let mut dt_max_a = self.dt_max_c.min(self.dt_max_t);
        if self.prj.atmosphere.sinusoidal_precip && self.prec_d > 0.0 {
            dt_max_a = dt_max_a.min((self.t_atm1 - self.t_atm_old) / 20.0);
        }
        let mut t_target = if self.p_level < self.t_print.len() { self.t_print[self.p_level] } else { self.t_max };
        t_target = t_target.min(self.t_print1);

        while self.target_idx < self.target_times.len() && self.target_times[self.target_idx] <= self.t + 1e-6 {
            self.target_idx += 1;
        }
        if self.target_idx < self.target_times.len() {
            t_target = t_target.min(self.target_times[self.target_idx]);
        }

        self.tm_cont(iter, t_target, dt_max_a);
        self.t += self.dt;

        // Apply sub-daily variations (only modify r_top if atmospheric BC is enabled)
        if self.prj.atmosphere.daily_variation && !self.l_daily_met {
            self.daily_var_root_soil();
            if self.atm_bc || self.top_inf {
                self.r_top = self.r_soil.abs() - self.prec.abs();
            }
        }
        if self.prj.atmosphere.sinusoidal_precip {
            self.sin_prec();
            if self.atm_bc || self.top_inf {
                self.r_top = self.r_soil.abs() - self.prec.abs();
            }
        }

        if self.l_daily_met {
            self.daily_met_step(); // Meteo(3): DailyMet(4) + SetDayMet
        }

        self.t_level += 1;
        if self.t_level > 999_999 {
            self.t_level = 2;
        }
        self.update();
        StepStatus::Running
    }

    /// Run to completion, calling `progress(t_now, t_max)` after each step.
    pub fn run<F: FnMut(&Simulation) -> bool>(&mut self, mut progress: F) -> StepStatus {
        loop {
            let s = self.step();
            if s != StepStatus::Running {
                return s;
            }
            if !progress(self) {
                return StepStatus::Running;
            }
        }
    }

    fn j_print(&self) -> bool {
        let at_print = self.p_level < self.t_print.len() && (self.t_print[self.p_level] - self.t).abs() < 0.001 * self.dt;
        let at_daily = self.print_daily && (self.t_print1 - self.t).abs() < 0.001 * self.dt;
        let regular = !self.short_o && (self.t_level - 1) % self.print_step == 0;
        at_print || at_daily || regular
    }

    /// Update variables at the end of a time step (Update in Fortran).
    fn update(&mut self) {
        let n = self.n;
        let i_bot = if self.kod_bot > 0 { 1 } else { 0 };
        let i_top = if self.kod_top > 0 { n - 2 } else { n - 1 };
        let mut l_sat = true;
        if self.i_dual_por > 0 {
            self.th_old_im.copy_from_slice(&self.th_new_im);
        }
        if self.l_dual_perm {
            self.h_matrix_old.copy_from_slice(&self.h_matrix_new);
            self.th_matrix_old.copy_from_slice(&self.th_matrix_new);
            self.th_im_mat_old.copy_from_slice(&self.th_im_mat_new);
        }
        if self.l_wat {
            for i in i_bot..=i_top {
                if self.l_extrap && self.h_new[i] < 0.0 && self.h_old[i] < 0.0 {
                    self.h_temp[i] = self.h_new[i] + (self.h_new[i] - self.h_old[i]) * self.dt / self.dt_old;
                } else {
                    self.h_temp[i] = self.h_new[i];
                }
                self.h_old[i] = self.h_new[i];
                self.h_new[i] = self.h_temp[i];
            }
            if self.kod_bot > 0 {
                self.h_old[0] = self.h_new[0];
            }
        }
        for i in 0..n {
            if self.l_wat {
                self.th_old[i] = self.th_new[i];
                if self.l_temp || self.l_chem {
                    self.v_old[i] = self.v_new[i];
                }
                if self.h_new[i] < 0.0 {
                    l_sat = false;
                }
            }
            if self.l_temp {
                if let Some(h) = self.heat.as_mut() {
                    h.temp_o[i] = h.temp_n[i];
                }
            }
        }
        if self.l_wat && l_sat && (self.kod_top == -1 || self.kod_top == 4) && self.r_top >= -self.con_s_max {
            if self.kod_top == 4 {
                self.kod_top = -4;
            }
            self.kod_top_m = if self.atm_bc { -4 } else { self.kod_top };
            let v = -0.005 * self.x_conv;
            self.h_new[n - 1] = v;
            self.h_temp[n - 1] = v;
            self.h_old[n - 1] = v;
        }
        if self.l_vapor {
            self.th_v_old.copy_from_slice(&self.th_v_new);
            self.v_v_old.copy_from_slice(&self.v_v_new);
        }
    }

    // ------------------------------------------------------------- utilities
    pub fn n_species(&self) -> usize {
        self.prj.n_solutes()
    }
    pub fn depth_of(&self, i: usize) -> f64 {
        self.x[self.n - 1] - self.x[i]
    }
    pub fn theta_of(&self, m: usize, h: f64) -> f64 {
        if m < self.par_d.len() {
            crate::material::fq(self.model, h, &self.par_d[m])
        } else {
            0.0
        }
    }
}
// ------------------------------------------------------------------------------------
// Surface energy balance coupling (Evapor / UpdateEnergy calls of the main program)
// ------------------------------------------------------------------------------------
impl Simulation {
    /// `call Evapor(...)` at the top of the coupling loop: evaporation, surface heat flux
    /// and the resulting top flux boundary condition rTop = -Prec + Evap.
    pub fn evapor_pass(&mut self) {
        let n = self.n;
        let tt_conv = 86400.0 * self.t_conv;
        let period = self.prj.heat.period;
        let (temp_s, theta_top, h_top) = {
            let hs = match self.heat.as_ref() {
                Some(h) => h,
                None => return,
            };
            (0.5 * hs.temp_n[n - 1] + 0.5 * hs.temp_o[n - 1], self.th_new[n - 1], self.h_new[n - 1])
        };
        let daily = if self.l_daily_met {
            Some((self.day_met.temp_a, self.day_met.rh_a, self.rst))
        } else {
            None
        };
        let out = {
            let mp = match self.prj.atmosphere.meteo.as_ref() {
                Some(m) if !m.records.is_empty() => m,
                _ => return,
            };
            if daily.is_some() {
                // lMetDaily: temperature, humidity and radiation of DailyMet / SetDayMet
                let cur = &self.day_met.cur;
                crate::meteo::evapor(
                    mp,
                    self.t / tt_conv,
                    self.t,
                    period,
                    temp_s,
                    cur.t_max,
                    cur.t_min,
                    self.day_rad,
                    cur.sun_hours,
                    cur.rh_max,
                    cur.wind_ms,
                    h_top,
                    theta_top,
                    false,
                    self.x_conv,
                    daily,
                )
            } else {
                let idx = self.meteo_idx.min(mp.records.len() - 1);
                let rn = &mp.records[idx];
                // MeteoInt: linear interpolation between the previous and the current record
                // (Rad and SunHours are not interpolated when lEnBal is on).
                let (ro, t0) = if idx > 0 { (&mp.records[idx - 1], mp.records[idx - 1].t) } else { (rn, self.t_init) };
                let frac = if idx > 0 && rn.t > t0 { ((self.t - t0) / (rn.t - t0)).clamp(0.0, 1.0) } else { 1.0 };
                let lerp = |a: f64, b: f64| a + (b - a) * frac;
                let t_max_a = lerp(ro.t_max, rn.t_max);
                let t_min_a = lerp(ro.t_min, rn.t_min);
                let wind_ms = lerp(ro.wind_kmd, rn.wind_kmd) / 86.4;
                let mut rh = lerp(ro.rh_mean, rn.rh_mean);
                if mp.i_rel_hum == 1 {
                    // vapor pressure given: RHMean = Ea*(50/Ea(TMin)+50/Ea(TMax))
                    let ea_t = |t: f64| 0.6108 * ((17.27 * t) / (t + 237.3)).exp();
                    rh = rh * (50.0 / ea_t(t_min_a) + 50.0 / ea_t(t_max_a));
                }
                let is_hourly = mp.records[0].t - self.t_init <= 0.9999 * tt_conv;
                crate::meteo::evapor(
                    mp,
                    self.t / tt_conv,
                    self.t,
                    period,
                    temp_s,
                    t_max_a,
                    t_min_a,
                    rn.rad,
                    rn.sun_hours,
                    rh,
                    wind_ms,
                    h_top,
                    theta_top,
                    is_hourly,
                    self.x_conv,
                    None,
                )
            }
        };
        let (xc, tc) = (self.x_conv, self.t_conv);
        let evap1 = out.evap / out.row * xc / tc; // [L/T]
        self.r_top = -self.prec + evap1;
        self.en_bal = EnBalState {
            evap: out.evap,
            sens_flux: out.sens_flux,
            lat: out.lat,
            const_: out.lat * out.row / xc / tc.powi(2),
            r_h: out.r_h,
            rn: out.rn,
            rns: out.rns,
            rnl: out.rnl,
        };
        if let Some(hs) = self.heat.as_mut() {
            // negative = into the profile (HYDRUS units)
            hs.heat_fl = -out.heat_flux / tc.powi(3);
        }
    }

    /// `call UpdateEnergy(...)`: reduce the latent heat flux when the soil cannot deliver
    /// the potential evaporation and give the difference to the sensible/ground heat flux.
    /// Also writes the energy-balance rows of Meteo.out (second coupling pass only).
    pub fn update_energy(&mut self) {
        let n = self.n;
        let (xc, tc) = (self.x_conv, self.t_conv);
        let (r_top, v_top) = (self.r_top, self.v_top);
        let temp_s = match self.heat.as_ref() {
            Some(h) => 0.5 * h.temp_n[n - 1] + 0.5 * h.temp_o[n - 1],
            None => return,
        };
        if r_top > 0.0 && r_top > v_top {
            let row = (1.0 - 7.37e-6 * (temp_s - 4.0).powi(2) + 3.79e-8 * (temp_s - 4.0).powi(3)) * 1000.0;
            let d_flux = r_top - v_top.max(0.0);
            let delta_e = d_flux * self.en_bal.const_ * tc.powi(3); // [kg/s3]
            if delta_e > 0.0 && v_top.abs() > 0.0 {
                let ca = 1200.0;
                let cw = 4180000.0;
                let m = self.mat[n - 1];
                let th = self.th_new[n - 1];
                let hm = &self.prj.heat.materials[m];
                let r_lamb = (hm.b1 + hm.b2 * th + hm.b3 * th.sqrt()).max(0.0) / xc * tc.powi(3);
                let dz = self.x[n - 1] - self.x[n - 2];
                let delta_t = delta_e / (ca / self.en_bal.r_h + r_lamb / (dz / xc) - cw * (v_top * tc / xc));
                let delta_sens = ca * delta_t / self.en_bal.r_h;
                self.en_bal.sens_flux += delta_sens;
                if let Some(hs) = self.heat.as_mut() {
                    hs.heat_fl -= (delta_e - delta_sens) / tc.powi(3);
                }
                self.en_bal.evap -= d_flux / xc * tc * row;
            }
        }

        // Meteo.out (only on the last coupling pass and every nPrStep-th step)
        let n_pr = self.prj.time.print_step.max(1);
        if self.i_temp == 1 && (self.t_level + n_pr - 1) % n_pr == 0 {
            let r_conv = 1.0e6 / 86400.0; // W/m2 -> MJ/m2/d
            let heat_fl = self.heat.as_ref().map(|h| h.heat_fl).unwrap_or(0.0);
            let e = self.en_bal;
            let latent = e.lat * e.evap / r_conv;
            let sensible = e.sens_flux / r_conv;
            let heat_mj = heat_fl / r_conv * tc.powi(3);
            let (air_temp, air_rh, solar_rad) = if self.l_daily_met { (self.day_met.temp_a, self.day_met.rh_a, self.rst) } else { (0.0, 0.0, 0.0) };
            self.res.meteo_mode = if self.l_daily_met { 3 } else { 2 };
            self.res.meteo.push(crate::output::MeteoRow {
                t: self.t,
                rns: e.rns,
                rnl: e.rnl,
                rn: e.rn,
                sensible,
                latent,
                heat_flux: heat_mj,
                balance: e.rn - sensible - latent + heat_mj,
                air_temp,
                air_rh,
                solar_rad,
                ..Default::default()
            });
        }
    }
}
