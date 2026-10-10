//! Solute transport (port of SOLUTE.FOR): advection-dispersion with equilibrium and
//! non-equilibrium sorption (two-site / mobile-immobile), first-order decay chains,
//! zero-order production, gas-phase partitioning and passive root uptake.
//! Not ported: temperature / water-content dependent rates, virus/colloid filtration,
//! dual-porosity coupling, volatilisation boundary.

use crate::error::HydrusError;
use crate::heat::thomas;
use crate::model::*;
use crate::output::*;
use crate::sim::Simulation;

pub struct SoluteState {
    pub ns: usize,
    pub epsi: f64,
    pub upw: bool,
    pub art_d: bool,
    pub tort: bool,
    pub tort_model: i32,
    pub tol_a: f64,
    pub tol_r: f64,
    pub max_it_c: usize,
    pub pe_cr: f64,
    pub k_top: i32,
    pub k_bot: i32,
    pub t_pulse: f64,
	pub d_surf: f64,
    pub c_atm: f64,
	pub l_tdep: bool,
    pub t_dep: Vec<SpeciesTDep>,
    // parameters
    pub ro: Vec<f64>,
    pub disp_l: Vec<f64>,
    pub frac: Vec<f64>,
    pub th_im: Vec<f64>,
    /// immobile water content of the matrix domain (dual permeability)
    pub th_im_m: Vec<f64>,
    pub diff_w: Vec<f64>,
    pub diff_g: Vec<f64>,
    pub pm: Vec<Vec<SpeciesMaterial>>,
    pub c_root_max: Vec<f64>,
    pub l_linear: Vec<bool>,
    pub l_equil: bool,
    pub l_mob_im: Vec<bool>,
    // state
    pub conc: Vec<Vec<f64>>,
    pub sorb: Vec<Vec<f64>>,
    pub c_top: Vec<f64>,
    pub c_bot: Vec<f64>,
    pub c_t: Vec<f64>,
    pub c_prev_o: Vec<f64>,
    /// Concentrations of all solutes at the beginning of the time step (cPrevO of the parent solute, own old values)
    pub conc_old_step: Vec<Vec<f64>>,
    pub c_new: Vec<f64>,
    pub c_temp: Vec<f64>,
    pub sorb_n: Vec<f64>,
	pub l_moist: bool,
	pub i_moist_dep: i32,
    pub w_dep: Vec<SpeciesWDep>,
	pub moist_tables: Vec<MoistDepTable>,
    pub th_ref: Vec<Vec<[f64; 9]>>,
	pub l_dual_neq: bool,
	pub l_nequil: bool,
    // work arrays
    pub th_n: Vec<f64>,
    pub th_o: Vec<f64>,
    pub v_o: Vec<f64>,
    pub v_n: Vec<f64>,
    pub disp: Vec<f64>,
    pub retard: Vec<f64>,
    pub g0: Vec<f64>,
    pub g1: Vec<f64>,
    pub q0: Vec<f64>,
    pub q1: Vec<f64>,
    pub wc: Vec<f64>,
    pub v_corr: Vec<f64>,
    pub s_sink: Vec<f64>,
    pub mb: Vec<f64>,
    pub md: Vec<f64>,
    pub me: Vec<f64>,
    pub mf: Vec<f64>,
    pub d_mob_i: f64,
    pub f1: f64,
    pub e1: f64,
    pub d1: f64,
    pub bn: f64,
    pub dn: f64,
    pub fn_: f64,
    // fluxes
    pub cv_top: Vec<f64>,
    pub cv_bot: Vec<f64>,
    pub cv_ch0: Vec<f64>,
    pub cv_ch1: Vec<f64>,
    pub cv_chr: Vec<f64>,
    pub cv_chim: Vec<f64>,
    pub cum_ch: Vec<[f64; 10]>,
    pub c_cum_t: Vec<f64>,
    pub c_cum_a: Vec<f64>,
    pub c_root: Vec<f64>,
    pub c_vol_i: Vec<f64>,
    pub sol_in: Vec<Vec<f64>>,
    pub peclet: f64,
    pub courant: f64,
	pub l_bact: bool,
	pub l_filtr: bool,
	pub sorb2: Vec<Vec<f64>>,
	pub sorb_n2: Vec<f64>,
	pub conc_m: Vec<Vec<f64>>,
    pub sorb_m: Vec<Vec<f64>>,
    /// Dual permeability: matrix-domain boundary fluxes / reaction terms of the last step per solute
    /// (per unit matrix area / volume, +x = upward; weighted with 1-w when summed) and the
    /// fracture-matrix solute exchange Gamma_s per total volume at the nodes
    pub cvm_top: Vec<f64>,
    pub cvm_bot: Vec<f64>,
    pub cvm_ch0: Vec<f64>,
    pub cvm_ch1: Vec<f64>,
    pub cvm_chr: Vec<f64>,
    pub gam_s_dp: Vec<f64>,
    /// Matrix concentration at the start of the step (dual permeability)
    pub conc_m_old: Vec<f64>,
    /// Kinetic (two-site) sorbed concentration in the matrix (current / start of step), per solute
    pub sorb_mk: Vec<Vec<f64>>,
    pub sorb_mk_old: Vec<Vec<f64>>,
    /// Second attachment site of the colloid model in the matrix (current / start of step)
    pub sorb_mk2: Vec<Vec<f64>>,
    pub sorb_mk2_old: Vec<Vec<f64>>,
    /// Immobile-region concentration of the matrix (current / start of step)
    pub sorb_mi: Vec<Vec<f64>>,
    pub sorb_mi_old: Vec<Vec<f64>>,
    /// Colloid size exclusion in the matrix (steady flow): (accessible water content, colloid velocity)
    pub excl_m: Option<(f64, f64)>,
    /// STrans: solute transfer to the immobile domain / kinetic sites (first solute)
    pub s_trans: Vec<f64>,
    /// Water content from which colloids are excluded (ChPar(4,1) of the filtration model when
    /// water flow is off; Exclusion zeroes the immobile water content afterwards)
    pub th_c_excl: f64,
}

/// Output of `Simulation::scaled_params`.
pub(crate) struct ScaledPar {
    pub p: SpeciesMaterial,
    pub p_im: SpeciesMaterial,
    pub diff_w: f64,
    pub diff_g: f64,
    /// Gas diffusion coefficient slot (colloid diameter of the filtration model), never zeroed
    pub dc: f64,
}

impl Simulation {
    pub fn conc(&self, j: usize, i: usize) -> f64 {
        self.sol.as_ref().map(|s| s.conc.get(j).map(|c| c[i]).unwrap_or(0.0)).unwrap_or(0.0)
    }
    pub fn sorb(&self, j: usize, i: usize) -> f64 {
        self.sol.as_ref().map(|s| s.sorb.get(j).map(|c| c[i]).unwrap_or(0.0)).unwrap_or(0.0)
    }
    pub fn conc_matrix(&self, j: usize, i: usize) -> f64 {
        self.sol.as_ref().map(|s| s.conc_m.get(j).map(|c| c[i]).unwrap_or(0.0)).unwrap_or(0.0)
    }
    pub fn sorb_matrix(&self, j: usize, i: usize) -> f64 {
        self.sol.as_ref().map(|s| s.sorb_m.get(j).map(|c| c[i]).unwrap_or(0.0)).unwrap_or(0.0)
    }
    pub fn peclet_courant(&self) -> (f64, f64) {
        self.sol.as_ref().map(|s| (s.peclet, s.courant)).unwrap_or((0.0, 0.0))
    }
    pub fn set_c_root(&mut self, sums: &[f64], a: f64) {
        if let Some(s) = self.sol.as_mut() {
            for j in 0..s.ns.min(sums.len()) {
                s.c_root[j] = sums[j] / a;
            }
        }
    }

    pub fn atm_record_scalars(&mut self, rec: &AtmRecord) {
        if let Some(h) = self.heat.as_mut() {
            h.t_top = rec.t_top;
            h.t_bot = rec.t_bot;
            h.ampl = rec.ampl;
        }
        let act_rsu = self.prj.root.l_act_rsu;
        let active = self.prj.root.active_solute;
        let mut s_pot = None;
        if let Some(s) = self.sol.as_mut() {
            for j in 0..s.ns {
                s.c_t[j] = rec.c_top.get(j).copied().unwrap_or(0.0);
                s.c_bot[j] = rec.c_bot.get(j).copied().unwrap_or(0.0);
                // SetBC: `cBot(jj)=0.; SPot=cB(jj)` (2009: single solute; the cB column of the solute with active
                // uptake carries SPot)
                if act_rsu && j == active.min(s.ns - 1) {
                    s_pot = Some(s.c_bot[j]);
                    s.c_bot[j] = 0.0;
                }
            }
        }
        if let Some(v) = s_pot {
            self.s_pot = v;
        }
    }

    pub fn set_chem_bc(&mut self) {
        let (prec, r_soil, kod_top) = (self.prec, self.r_soil, self.kod_top);
        let wl = self.w_layer && self.h_new[self.n - 1] > 0.0;
        if let Some(s) = self.sol.as_mut() {
            for j in 0..s.ns {
                if wl {
                    continue;
                }
                s.c_top[j] = s.c_t[j];
                if kod_top.abs() == 4 && s.k_top <= 0 {
                    if prec - r_soil > 0.0 {
                        s.c_top[j] = s.c_t[j] * prec / (prec - r_soil);
                    } else if r_soil > 0.0 {
                        s.c_top[j] = 0.0;
                    }
                }
            }
        }
    }

    /// Mass balance in the surface layer for concentration at the top.
    pub fn sol_wlayer_mix(&mut self) {
        let m = self.n - 1;
        if !(self.w_layer && self.h_new[m] > 0.0) {
            return;
        }
        let ht = self.h_new[m];
        let dt = self.dt;
        let prec = self.prec.max(0.0);
        let r_soil = self.r_soil.max(0.0);

        if let Some(s) = self.sol.as_mut() {
            for j in 0..s.ns {
                let c_rain = s.c_t[j];
                // Previous surface layer mass + precipitation mass
                let num = ht * s.c_top[j] + dt * prec * c_rain;
                let den = ht + dt * (prec - r_soil);
                if den > 1e-12 {
                    s.c_top[j] = (num / den).max(0.0);
                }
            }
        }
    }

    pub fn solute_init(&mut self) -> Result<(), HydrusError> {
        let n = self.n;
        let so = self.prj.solute.clone();
        let ns = so.species.len();
        let n_mat = self.n_mat;
        let mut l_equil = true;
        let mut l_mob = vec![false; n_mat];
        for m in 0..n_mat {
            let mm = &so.materials[m];
            // ChemIn: lEquil = .false. for Frac < 1, ThImob > 0 or the filtration model; HYDRUS.FOR adds
            // iDualPor > 0 (lDualNEq alone does not make the problem non-equilibrium)
            if mm.frac < 1.0 || mm.th_immobile > 0.0 || so.l_bact || self.i_dual_por > 0 {
                l_equil = false;
            }
            l_mob[m] = mm.th_immobile > 0.0 && !so.l_bact;
            if !l_equil && mm.bulk_density == 0.0 {
                return Err(HydrusError::Invalid("Bulk density cannot be zero.".into()));
            }
        }
        // Exclusion (only for the filtration model with steady water flow): th_c = ChPar(4,1), then ChPar(4,1) = 0
        let mut th_c_excl = 0.0;
        if so.l_bact && !self.l_wat {
            th_c_excl = so.materials[0].th_immobile;
        }
        let mut l_linear = vec![true; ns];
        for j in 0..ns {
            for m in 0..n_mat {
                let p = &so.species[j].per_material[m];
                if p.nu.abs() > 1e-12 || (p.beta - 1.0).abs() > 0.001 {
                    l_linear[j] = false;
                }
                // filtration model: blocking makes the attachment non-linear (SMax1 or SMax2 > 0)
                if so.l_bact && (p.s_max1 > 0.0 || p.s_max2 > 0.0) {
                    l_linear[j] = false;
                }
            }
        }
        let nodes: Vec<&ProfileNode> = self.prj.profile.nodes.iter().rev().collect();
        let mut conc = vec![vec![0.0; n]; ns];
        let mut sorb = vec![vec![0.0; n]; ns];
		let mut sorb2 = vec![vec![0.0; n]; ns];
        if so.l_bact || so.l_dual_neq {
            for j in 0..ns {
                for i in 0..n {
                    sorb2[j][i] = nodes[i].sorb.get(j).copied().unwrap_or(0.0);
                }
            }
        }
        for i in 0..n {
            for j in 0..ns {
                conc[j][i] = nodes[i].conc.get(j).copied().unwrap_or(0.0);
                sorb[j][i] = nodes[i].sorb.get(j).copied().unwrap_or(0.0);
            }
        }
        let ro: Vec<f64> = so.materials.iter().map(|m| m.bulk_density).collect();
        let frac: Vec<f64> = so.materials.iter().map(|m| m.frac).collect();
        // initial mass -> concentration
        if so.mass_init {
            for j in 0..ns {
                for i in 0..n {
                    if conc[j][i] > 0.0 {
                        let m = self.mat[i];
                        let p = &so.species[j].per_material[m];
                        // Par(7) = Theta + ThetaIm (total water), Par(8) = ThSat - Par(7)
                        let th = self.th_old[i] + if self.i_dual_por > 0 { self.th_old_im[i] } else { 0.0 };
                        let tha = self.ths[m] - th;
                        let mass = conc[j][i];
                        // temperature-adjusted Ks, Nu and Henry constant at the initial temperature
                        let p = &{
                            let mut q = p.clone();
                            if so.l_tdep && j < so.t_dep.len() {
                                let td = &so.t_dep[j];
                                let tk = self.prj.profile.nodes.get(self.n - 1 - i).map(|nd| nd.temp).unwrap_or(20.0) + 273.15;
                                let tt = (tk - 293.15) / (8.314 * tk * 293.15);
                                q.ks *= (td.ks * tt).exp();
                                q.nu *= (td.nu * tt).exp();
                                q.henry *= (td.henry * tt).exp();
                            }
                            q
                        };

                        let ks_eff = if so.l_bact {
                            let mut k1 = 0.0;
                            let mut k2 = 0.0;
                            if p.r_kd1 > 0.0 && ro[m] > 0.0 {
                                k1 = th * p.r_ka1 / ro[m] / p.r_kd1;
                            }
                            if p.r_kd2 > 0.0 && ro[m] > 0.0 {
                                k2 = th * p.r_ka2 / ro[m] / p.r_kd2;
                            }
                            k1 + k2
                        } else {
                            p.ks
                        };

                        conc[j][i] = if l_linear[j] {
                            mass / (th + ro[m] * ks_eff + tha * p.henry)
                        } else {
                            let f = |c: f64| th * c + ro[m] * ks_eff * c.powf(p.beta) / (1.0 + p.nu * c.powf(p.beta)) + tha * p.henry * c;
                            let (mut lo, mut hi) = (1e-20f64, 1e20f64);
                            for _ in 0..300 {
                                let mid = (lo * hi).sqrt();
                                if f(mid) > mass {
                                    hi = mid
                                } else {
                                    lo = mid
                                }
                            }
                            (lo * hi).sqrt()
                        };
                    }
                }
            }
        }
        if !l_equil && (so.equil_init || so.mass_init) {
            // NonEqInit: the non-equilibrium phase starts in equilibrium with the liquid phase
            for j in 0..ns {
                for i in 0..n {
                    let m = self.mat[i];
                    let p = &so.species[j].per_material[m];
                    let cc = conc[j][i];
                    if l_mob[m] || self.i_dual_por > 0 {
                        sorb[j][i] = cc;
                    } else {
                        let sc = if !l_linear[j] && cc > 0.0 { cc.powf(p.beta - 1.0) / (1.0 + p.nu * cc.powf(p.beta)) } else { 1.0 };
                        sorb[j][i] = (1.0 - frac[m]) * sc * p.ks * cc;
                        if so.l_bact {
                            let th = self.th_old[i];
                            // (the Fortran uses rKa1/rKd1 for both sites; the second site uses its own rates here)
                            let ks1 = if p.r_kd1 > 0.0 && ro[m] > 0.0 { th * p.r_ka1 / ro[m] / p.r_kd1 } else { 0.0 };
                            let ks2 = if p.r_kd2 > 0.0 && ro[m] > 0.0 { th * p.r_ka2 / ro[m] / p.r_kd2 } else { 0.0 };
                            sorb[j][i] = ks1 * cc;
                            sorb2[j][i] = ks2 * cc;
                        }
                    }
                }
            }
        }
        let mut c_top = vec![0.0; ns];
        let mut c_bot = vec![0.0; ns];
        for j in 0..ns {
            c_top[j] = so.species[j].c_top;
            c_bot[j] = so.species[j].c_bot;
        }
        let z = |k: usize| vec![0.0; k];
        let conc_m = conc.clone();
        let mut sorb_m = sorb.clone();
        let mut sorb_mk = vec![vec![0.0; n]; ns];
        let mut sorb_mk2 = vec![vec![0.0; n]; ns];
        let mut sorb_mi = vec![vec![0.0; n]; ns];
        // colloid size exclusion in the matrix (steady water flow): accessible water content and colloid velocity of the
        // first node are applied to all nodes (Exclusion); the immobile water content of material 1 is then used up
        let mut excl_m: Option<(f64, f64)> = None;
        let mut th_c_m = 0.0;
        if self.l_dual_perm && so.l_bact && !self.l_wat && !self.matrix_dual_por && so.materials[0].th_im_matrix > 0.0 {
            th_c_m = so.materials[0].th_im_matrix;
            let par = &self.par_d[self.mat[0]];
            let lm = self.prj.water.dual_perm.material(self.mat[0]).l_matrix;
            excl_m = Some(exclusion(self.th_m_at(0, true), self.matrix_flux(0, 1), th_c_m, par[6], par[7], par[8], par[9], lm));
        }
        let _ = th_c_m;
        if self.l_dual_perm {
            // matrix: the non-equilibrium phase starts in equilibrium with the liquid phase (NonEqInit)
            for j in 0..ns {
                for i in 0..n {
                    let m = self.mat[i];
                    let p = &so.species[j].per_material[m];
                    let c0 = conc_m[j][i];
                    let s0 = Self::dp_iso(p, c0).0;
                    sorb_m[j][i] = s0;
                    let stat_im = if excl_m.is_some() && m == self.mat[0] { 0.0 } else { so.materials[m].th_im_matrix };
                    let imm = self.matrix_dual_por || (stat_im > 0.0 && (!so.l_bact || p.omega_im > 0.0));
                    if imm {
                        sorb_mi[j][i] = c0; // immobile concentration (NonEqInit: Sorb = Conc)
                        sorb_m[j][i] = c0;
                    }
                    if so.l_bact {
                        let th = excl_m.map(|e| e.0).unwrap_or_else(|| self.th_m_at(i, false));
                        let ks1 = if p.r_kd1 > 0.0 && ro[m] > 0.0 { th * p.r_ka1 / ro[m] / p.r_kd1 } else { 0.0 };
                        let ks2 = if p.r_kd2 > 0.0 && ro[m] > 0.0 { th * p.r_ka2 / ro[m] / p.r_kd2 } else { 0.0 };
                        sorb_mk[j][i] = ks1 * c0;
                        sorb_mk2[j][i] = ks2 * c0;
                        if !imm {
                            sorb_m[j][i] = frac[m] * s0 + sorb_mk[j][i] + sorb_mk2[j][i];
                        }
                    } else if !imm && frac[m] < 1.0 {
                        sorb_mk[j][i] = (1.0 - frac[m]) * s0; // two-site kinetic sites
                    }
                    // dual non-equilibrium kinetic sites in the mobile zone are initialised from the profile (Sorb2) in the Fortran;
                    // the matrix starts them in equilibrium with the solution
                    if imm && so.l_dual_neq && !so.l_bact {
                        let f_em = p.mu_g.clamp(0.0, 1.0);
                        sorb_mk[j][i] = frac[m] * (1.0 - f_em) * s0;
                    }
                }
            }
        }
		let mut th_ref = vec![vec![[0.0; 9]; n_mat]; ns];
        if so.l_moist {
            for j in 0..ns {
                if j < so.w_dep.len() {
                    let wdep = &so.w_dep[j];
                    for m in 0..n_mat {
                        for k in 0..9 {
                            let href = wdep.h_ref[k];
                            // Compute theta_ref using retention curve FQ for material m
                            th_ref[j][m][k] = self.theta_of(m, href);
                        }
                    }
                }
            }
        }
        self.sol = Some(SoluteState {
            ns,
            epsi: so.epsi,
            upw: so.upstream_weighting,
            art_d: so.artificial_dispersion,
            tort: so.tortuosity,
            tort_model: so.tort_model,
            tol_a: so.tol_abs,
            tol_r: so.tol_rel,
            max_it_c: so.max_iter.max(1),
            pe_cr: so.pe_cr.max(0.1),
            k_top: so.k_top,
            k_bot: so.k_bot,
            t_pulse: so.t_pulse,
            d_surf: so.d_surf,
            c_atm: so.c_atm,
			l_tdep: so.l_tdep,
            t_dep: so.t_dep.clone(),
            ro,
            disp_l: so.materials.iter().map(|m| m.disp_l).collect(),
            frac,
            th_im: so.materials.iter().enumerate().map(|(k, m)| if k == 0 && th_c_excl != 0.0 { 0.0 } else { m.th_immobile }).collect(),
            th_im_m: so.materials.iter().enumerate().map(|(k, m)| if k == 0 && excl_m.is_some() { 0.0 } else { m.th_im_matrix }).collect(),
            diff_w: so.species.iter().map(|s| s.diff_w).collect(),
            diff_g: so.species.iter().map(|s| s.diff_g).collect(),
            pm: so.species.iter().map(|s| s.per_material.clone()).collect(),
            c_root_max: (0..ns).map(|j| self.prj.root.c_root_max.get(j).copied().unwrap_or(0.0)).collect(),
            l_linear,
            l_equil,
            l_mob_im: l_mob,
            conc,
            sorb,
            c_top,
            c_bot,
            c_t: z(ns),
            c_prev_o: z(n),
            conc_old_step: vec![vec![0.0; n]; ns],
            c_new: z(n),
            c_temp: z(n),
			l_moist: so.l_moist,
			i_moist_dep: so.i_moist_dep,
			w_dep: so.w_dep.clone(),
			moist_tables: so.moist_tables.clone(),
			th_ref,
			l_dual_neq: so.l_dual_neq,
			l_nequil: so.l_nequil,
            sorb_n: z(n),
            th_n: z(n),
            th_o: z(n),
            v_o: z(n),
            v_n: z(n),
            disp: z(n),
            retard: z(n),
            g0: z(n),
            g1: z(n),
            q0: z(n),
            q1: z(n),
            wc: z(n),
            v_corr: z(n),
            s_sink: z(n),
            mb: z(n),
            md: z(n),
            me: z(n),
            mf: z(n),
            d_mob_i: 1.0,
            f1: 0.0,
            e1: 0.0,
            d1: 0.0,
            bn: 0.0,
            dn: 0.0,
            fn_: 0.0,
            cv_top: z(ns),
            cv_bot: z(ns),
            cv_ch0: z(ns),
            cv_ch1: z(ns),
            cv_chr: z(ns),
            cv_chim: z(ns),
            cum_ch: vec![[0.0; 10]; ns],
            c_cum_t: z(ns),
            c_cum_a: z(ns),
            c_root: z(ns),
            c_vol_i: z(ns),
            sol_in: vec![vec![0.0; n]; ns],
            peclet: 0.0,
            courant: 0.0,
            l_bact: so.l_bact,
            l_filtr: so.l_filtr,
            sorb2,
            sorb_n2: z(n),
			conc_m,
            sorb_m,
            s_trans: vec![0.0; n],
            cvm_top: vec![0.0; ns],
            cvm_bot: vec![0.0; ns],
            cvm_ch0: vec![0.0; ns],
            cvm_ch1: vec![0.0; ns],
            cvm_chr: vec![0.0; ns],
            gam_s_dp: vec![0.0; n],
            conc_m_old: vec![0.0; n],
            sorb_mk,
            sorb_mk_old: vec![vec![0.0; n]; ns],
            sorb_mk2,
            sorb_mk2_old: vec![vec![0.0; n]; ns],
            sorb_mi,
            sorb_mi_old: vec![vec![0.0; n]; ns],
            excl_m,
            th_c_excl,
        });
        Ok(())
    }

    pub fn solute(&mut self) -> Result<(), HydrusError> {
        let mut st = self.sol.take().expect("solute state");
        let r = self.solute_inner(&mut st);
        self.sol = Some(st);
        r
    }

    fn solute_inner(&mut self, st: &mut SoluteState) -> Result<(), HydrusError> {
        let n = self.n;
        let ns = st.ns;
        'restart: loop {
            let dt = self.dt;
            let epsi = st.epsi;
            let alf = 1.0 - epsi;
            st.peclet = 0.0;
            st.courant = 0.0;
            self.dt_max_c = 1e30;
            self.iter_c = 1;
            st.th_n.copy_from_slice(&self.th_new);
            st.th_o.copy_from_slice(&self.th_old);
            st.v_o.copy_from_slice(&self.v_old);
            st.v_n.copy_from_slice(&self.v_new);
            // Exclusion (HYDRUS.FOR: `if(lBact.and..not.lWat)`): accessible water content and colloid
            // velocity from node 1, applied to all nodes (steady flow, homogeneous profile)
            if st.l_bact && !self.l_wat && st.th_c_excl.abs() > 1e-20 && !self.par_d.is_empty() {
                let p = &self.par_d[0];
                let (th_c_n, v_c_n) = exclusion(st.th_n[0], st.v_n[0], st.th_c_excl, p[0], p[1], p[2], p[3], p[5]);
                let (th_c_o, v_c_o) = exclusion(st.th_o[0], st.v_o[0], st.th_c_excl, p[0], p[1], p[2], p[3], p[5]);
                for i in 0..n {
                    st.th_n[i] = th_c_n;
                    st.v_n[i] = v_c_n;
                    st.th_o[i] = th_c_o;
                    st.v_o[i] = v_c_o;
                }
            }
            // old concentrations of ALL solutes (the daughter needs the old concentration of its parent, not its own)
            for j in 0..ns {
                let cj = st.conc[j].clone();
                st.conc_old_step[j].copy_from_slice(&cj);
            }
            for js in 0..ns {
                st.c_prev_o.copy_from_slice(&st.conc[js]);
                let dual = self.l_dual_perm;
                if dual {
                    let cm = st.conc_m[js].clone();
                    st.conc_m_old.copy_from_slice(&cm);
                    let sk = st.sorb_mk[js].clone();
                    st.sorb_mk_old[js].copy_from_slice(&sk);
                    let sk2 = st.sorb_mk2[js].clone();
                    st.sorb_mk2_old[js].copy_from_slice(&sk2);
                    let cim = st.sorb_mi[js].clone();
                    st.sorb_mi_old[js].copy_from_slice(&cim);
                }
                // fracture and matrix domain are coupled through the exchange term: iterate the pair of solves
                let n_outer = if dual { 5 } else { 1 };
                // the linear non-equilibrium update modifies Sorb / Sorb2 in place: every pass must start from the
                // values at the beginning of the step
                let (sorb_start, sorb2_start) = (st.sorb[js].clone(), st.sorb2[js].clone());
                for outer in 0..n_outer {
                if outer > 0 {
                    let cp = st.c_prev_o.clone();
                    st.conc[js].copy_from_slice(&cp);
                    st.sorb[js].copy_from_slice(&sorb_start);
                    st.sorb2[js].copy_from_slice(&sorb2_start);
                }
                let mut iter = 0;
                st.cv_top[js] = 0.0;
                st.cv_bot[js] = 0.0;
                st.cv_ch0[js] = 0.0;
                st.cv_ch1[js] = 0.0;
                st.cv_chr[js] = 0.0;
                st.cv_chim[js] = 0.0;
                // SOLUTE.FOR: `if(t-tPulse.gt.dtMin.and..not.AtmBC)` - with the atmospheric BC the concentrations of
                // the atmospheric records stay in force after tPulse.  (A former deviation ended the pulse also for
                // AtmBC, fitted to the ROOTUPTK output of an older build; the HYDRUS 4.17 output keeps cvTop = 0.4
                // after tPulse = 100 as the source does.)
                if self.t - st.t_pulse > self.dt_min && !self.atm_bc {
                    st.c_top[js] = 0.0;
                    st.c_bot[js] = 0.0;
                }
                if st.k_bot < 0 {
                    if self.l_vapor && self.r_bot.abs() < 1e-20 {
                        st.cv_bot[js] = 0.0;
                    } else if st.v_o[0] >= 0.0 {
                        st.cv_bot[js] = alf * st.c_bot[js] * st.v_o[0];
                    } else {
                        st.cv_bot[js] = alf * st.conc[js][0] * st.v_o[0];
                    }
                } else if st.k_bot == 0 {
                    st.cv_bot[js] = alf * st.conc[js][0] * st.v_o[0];
                }
                // SOLUTE.FOR: at TLevel = 1 the old-level top flux is skipped (`kTopCh.lt.0..and.TLevel.ne.1`) and the full
                // new-level flux vN*cTop is booked instead of epsi*vN*cTop, for every top boundary.  (A former deviation used
                // the generic alf*vO + epsi*vN booking for the atmospheric BC, fitted to the ROOTUPTK output of an older
                // build; HYDRUS 4.17 gives the constant CncBalT1 of -0.0191 of the source rule for ROOTUPTK and the full
                // first-step cvTop for Test9 / TEST9a.)
                let first_step_rule = self.t_level == 1;
                if st.k_top < 0 && !first_step_rule && st.v_o[n - 1] < 0.0 {
                    st.cv_top[js] = alf * st.c_top[js] * st.v_o[n - 1];
                }
                if st.k_top == -2 {
                    let m = n - 1;
                    let mat_top = self.mat[m];
                    let henry = st.pm[js][mat_top].henry;
                    let (g_vol, g_atm) = if st.d_surf > 1e-10 {
                        (st.diff_g[js] * henry / st.d_surf, st.diff_g[js] / st.d_surf)
                    } else {
                        (0.0, 0.0)
                    };
                    st.cv_top[js] += alf * g_vol * st.conc[js][m] - g_atm * st.c_atm;
                }
                if !st.l_linear[js] {
                    st.c_new.copy_from_slice(&st.conc[js]);
                    if !st.l_equil {
                        st.sorb_n.copy_from_slice(&st.sorb[js]);
                    }
                }
                if self.sink_f {
                    self.set_s_snk(st, js);
                } else {
                    for v in st.s_sink.iter_mut() {
                        *v = 0.0;
                    }
                }
                loop {
                    iter += 1;
                    if !st.l_linear[js] {
                        st.c_temp.copy_from_slice(&st.c_new);
                    }
                    for level in 1..=2 {
                        let (pe, co, dtm) = self.sol_coeff(st, js, level, iter, dt);
                        st.peclet = st.peclet.max(pe);
                        st.courant = st.courant.max(co);
                        self.dt_max_c = self.dt_max_c.min(dtm);
                        self.sol_matset(st, js, level, dt);
                        for i in 0..n {
                            if level == 1 {
                                st.v_o[i] += st.v_corr[i];
                            } else {
                                st.v_n[i] += st.v_corr[i];
                            }
                        }
                        if level == 1 && iter == 1 {
                            self.sol_masstran(st, js, alf, false);
                        }
                    }
                    // solve
                    let mut d = st.md.clone();
                    let mut f = st.mf.clone();
                    let mut b = st.mb.clone();
                    let mut e = st.me.clone();
                    b[0] = 0.0;
                    e[n - 1] = 0.0;
                    thomas(&b, &mut d, &e, &mut f);
                    let mut lconv = true;
                    for i in 0..n {
                        if st.l_linear[js] {
                            st.conc[js][i] = f[i].max(0.0);
                            if st.conc[js][i] < 1e-30 && st.conc[js][i] > 0.0 {
                                st.conc[js][i] = 0.0;
                            }
                        } else {
                            st.c_new[i] = f[i];
                            if st.c_new[i] < 1e-30 {
                                st.c_new[i] = 0.0;
                            }
                            if (st.c_new[i] - st.c_temp[i]).abs() > st.tol_a + st.tol_r * st.conc[js][i] {
                                lconv = false;
                            }
                        }
                    }
                    if !st.l_linear[js] {
                        if !lconv {
                            if iter < st.max_it_c {
                                continue;
                            } else if dt > self.dt_min && !self.l_wat {
                                let dt_old = self.dt;
                                self.dt = ((self.dt / 3.0).max(self.dt_min)) as f32 as f64; // REAL*4 as in the Fortran code
                                self.dt_opt = self.dt;
                                self.t = self.t - dt_old + self.dt;
                                continue 'restart;
                            } else {
                                return Err(HydrusError::Numerical(format!("Solute transport did not converge at t = {}", self.t)));
                            }
                        }
                        for i in 0..n {
                            st.conc[js][i] = st.c_new[i];
                            if !st.l_equil {
                                st.sorb[js][i] = st.sorb_n[i];
                            }
                        }
                    }
                    break;
                }
                if !st.l_equil && st.l_linear[js] {
                    self.sol_sorbconc(st, js, dt);
                }
                let mut dm_change = 0.0;
                if dual {
                    dm_change = self.sol_dual_perm(st, js, dt);
                }
                self.sol_masstran(st, js, epsi, true);
                // fluxes across the boundaries
                if st.k_top > 0 {
                    st.cv_top[js] = st.fn_ - st.bn * st.conc[js][n - 2] - st.dn * st.conc[js][n - 1];
                } else {
                    if st.v_n[n - 1] < 0.0 {
                        let w = if self.t_level == 1 { 1.0 } else { epsi }; // SOLUTE.FOR: full weight at TLevel = 1
                        st.cv_top[js] += w * st.v_n[n - 1] * st.c_top[js];
                    }
                    if st.k_top == -2 {
                        let m = n - 1;
                        let mat_top = self.mat[m];
                        let henry = st.pm[js][mat_top].henry;
                        let (g_vol, g_atm) = if st.d_surf > 1e-10 {
                            (st.diff_g[js] * henry / st.d_surf, st.diff_g[js] / st.d_surf)
                        } else {
                            (0.0, 0.0)
                        };
                        st.cv_top[js] += epsi * g_vol * st.conc[js][m] - g_atm * st.c_atm;
                    }
                }
                if st.k_bot < 0 {
                    if self.l_vapor && self.r_bot.abs() < 1e-20 {
                        st.cv_bot[js] = 0.0;
                    } else if st.v_n[0] >= 0.0 {
                        st.cv_bot[js] += epsi * st.c_bot[js] * st.v_n[0];
                    } else {
                        st.cv_bot[js] += epsi * st.conc[js][0] * st.v_n[0];
                    }
                } else if st.k_bot == 0 {
                    st.cv_bot[js] += epsi * st.conc[js][0] * st.v_n[0];
                } else {
                    st.cv_bot[js] = st.d1 * st.conc[js][0] + st.e1 * st.conc[js][1] - st.f1;
                }
                self.iter_c = self.iter_c.max(iter);
                if st.cv_top[js].abs() < 1e-30 {
                    st.cv_top[js] = 0.0;
                }
                if st.cv_bot[js].abs() < 1e-30 {
                    st.cv_bot[js] = 0.0;
                }
                if dual && dm_change < 0.01 {
                    break;
                }
                }
            }
            break;
        }
        Ok(())
    }

    /// Root solute uptake (port of SetSSnk in SINK.FOR): passive uptake S_w * min(c, cRootMax)
    /// and, with `lActRSU`, Michaelis-Menten type active uptake of the potential solute uptake
    /// `SPot*OmegaW` that the passive uptake does not cover (step 2), optionally compensated for
    /// stress (step 3, `OmegaS < 1`), for the solute `RootUptake::active_solute`. `omega_act[0]` is OmegaS.
    fn set_s_snk(&self, st: &mut SoluteState, js: usize) {
        let n = self.n;
        let root = &self.prj.root;
        let act = root.l_act_rsu && js == root.active_solute.min(st.ns.saturating_sub(1)) && !root.r_km.is_empty();
        let omega_s = root.omega_act.first().copied().unwrap_or(1.0);
        let (r_km, c_min) = (root.r_km.first().copied().unwrap_or(0.0), root.c_min.first().copied().unwrap_or(0.0));
        let n_step = if act { if omega_s < 1.0 { 3 } else { 2 } } else { 1 };
        let mut compen = 1.0_f64;
        let (mut omega, mut sp_uptake, mut sa_uptake_p, mut omega1, mut a_uptake_a) = (0.0, 0.0, 0.0, 0.0, 0.0_f64);
        for v in st.s_sink.iter_mut() {
            *v = 0.0;
        }
        for i_step in 1..=n_step {
            for i in 0..n {
                if self.beta[i] > 0.0 {
                    let dxm = if i == n - 1 {
                        (self.x[i] - self.x[i - 1]) / 2.0
                    } else if i == 0 {
                        (self.x[i] - self.x[i + 1]) / 2.0
                    } else {
                        (self.x[i + 1] - self.x[i - 1]) / 2.0
                    };
                    let cc = (st.conc[js][i] - c_min).max(0.0);
                    match i_step {
                        1 => {
                            st.s_sink[i] = self.sink[i] * st.conc[js][i].min(st.c_root_max[js]).max(0.0);
                            sp_uptake += st.s_sink[i] * dxm;
                            sa_uptake_p = (self.s_pot * self.omega_w - sp_uptake).max(0.0);
                        }
                        2 => {
                            a_uptake_a = cc / (r_km + cc) * self.beta[i] * sa_uptake_p;
                            omega += a_uptake_a * dxm;
                            if n_step == 2 {
                                st.s_sink[i] += a_uptake_a;
                            }
                            if sa_uptake_p > 0.0 {
                                omega1 = omega / sa_uptake_p;
                            }
                        }
                        _ => {
                            if omega1 < omega_s && omega1 > 0.0 {
                                compen = omega_s;
                            }
                            if omega1 >= omega_s {
                                compen = omega1;
                            }
                            if compen > 0.0 {
                                a_uptake_a = cc / (r_km + cc) * self.beta[i] * sa_uptake_p / compen;
                            }
                            st.s_sink[i] += a_uptake_a;
                        }
                    }
                } else {
                    st.s_sink[i] = 0.0;
                }
            }
        }
    }

    /// Temperature- and moisture-adjusted reaction parameters of one species and material
    /// (the `ChPar * exp(TDep*TT) * f1` block of Coeff). `p` holds the mobile-phase values
    /// (moisture dependence evaluated at `th_w`), `p_im` the immobile-phase decay rates
    /// (evaluated at `th_im`, the `...i` variables of Coeff). For the filtration model the
    /// rates that Coeff sets to zero are zeroed here as well.
    fn scaled_params(&self, st: &SoluteState, js: usize, m: usize, temp: f64, th_w: f64, th_im: f64) -> ScaledPar {
        let base = &st.pm[js][m];
        let mut p = base.clone();
        let (mut dw, mut dg) = (st.diff_w[js], st.diff_g[js]);
        if st.l_tdep && js < st.t_dep.len() {
            let td = &st.t_dep[js];
            let tk = temp + 273.15;
            let tt = (tk - 293.15) / (8.314 * tk * 293.15);
            let sc = |b: f64, ea: f64| if ea.abs() > 1e-12 { b * (ea * tt).exp() } else { b };
            dw = sc(dw, td.diff_w);
            dg = sc(dg, td.diff_g);
            p.ks = sc(base.ks, td.ks);
            p.nu = sc(base.nu, td.nu);
            p.henry = sc(base.henry, td.henry);
            p.mu_w = sc(base.mu_w, td.mu_w);
            p.mu_s = sc(base.mu_s, td.mu_s);
            p.mu_g = sc(base.mu_g, td.mu_g);
            p.gam_w = sc(base.gam_w, td.gam_w);
            p.gam_s = sc(base.gam_s, td.gam_s);
            p.gam_g = sc(base.gam_g, td.gam_g);
            p.mu0_w = sc(base.mu0_w, td.mu0_w);
            p.mu0_s = sc(base.mu0_s, td.mu0_s);
            p.mu0_g = sc(base.mu0_g, td.mu0_g);
            p.omega = sc(base.omega, td.omega);
            if st.l_bact {
                // ChemIn slots of the filtration model: 15 SMax2, 16 rKa2, 17 rKd2, 18 SMax1, 19 rKa1, 20 rKd1
                p.s_max2 = sc(base.s_max2, td.gam_s);
                p.r_ka2 = sc(base.r_ka2, td.gam_g);
                p.r_kd2 = sc(base.r_kd2, td.mu0_w);
                p.s_max1 = sc(base.s_max1, td.mu0_s);
                p.r_ka1 = sc(base.r_ka1, td.mu0_g);
                p.r_kd1 = sc(base.r_kd1, td.omega);
                if st.l_filtr {
                    p.d_c = sc(base.d_c, td.mu_g);
                    p.d_p = sc(base.d_p, td.gam_w);
                }
            }
        }
        let mut p_im = p.clone();
        if st.l_moist {
            // k = iReact - 1 (index of the Walker parameters), jr = jReact (row of the MoistDep.in table)
            let scale_m = |rate: f64, k: usize, jr: usize, theta: f64| -> f64 {
                if st.i_moist_dep == 2 {
                    match st.moist_tables.get(m * st.ns + js) {
                        Some(tab) => rate * rmd_table(tab, jr, theta),
                        None => rate,
                    }
                } else if js < st.w_dep.len() {
                    // Walker's formula: only below the reference water content
                    let b = st.w_dep[js].exp_b[k];
                    let th_ref = st.th_ref[js][m][k];
                    if th_ref > theta && th_ref > 0.0 {
                        rate * (theta.max(0.0) / th_ref).powf(b)
                    } else {
                        rate
                    }
                } else {
                    rate
                }
            };
            p.mu_w = scale_m(p.mu_w, 0, 1, th_w);
            p.mu_s = scale_m(p.mu_s, 1, 2, th_w);
            p.mu_g = scale_m(p.mu_g, 2, 3, th_w);
            p.gam_w = scale_m(p.gam_w, 3, 4, th_w);
            p.gam_s = scale_m(p.gam_s, 4, 5, th_w);
            p.gam_g = scale_m(p.gam_g, 5, 6, th_w);
            p.mu0_w = scale_m(p.mu0_w, 6, 7, th_w);
            p.mu0_s = scale_m(p.mu0_s, 7, 8, th_w);
            p.mu0_g = scale_m(p.mu0_g, 8, 9, th_w);
            // immobile-phase rates: jReact 10..13 with the iReact of the mobile counterpart, evaluated at ThImob
            p_im.mu_w = scale_m(p_im.mu_w, 0, 10, th_im);
            p_im.mu_s = scale_m(p_im.mu_s, 1, 11, th_im);
            p_im.gam_w = scale_m(p_im.gam_w, 3, 12, th_im);
            p_im.gam_s = scale_m(p_im.gam_s, 4, 13, th_im);
        }
        let dc = dg; // colloid diameter / gas diffusion coefficient slot of the filtration model
        if st.l_bact {
            for q in [&mut p, &mut p_im] {
                q.mu_g = 0.0;
                q.gam_w = 0.0;
                q.gam_s = 0.0;
                q.gam_g = 0.0;
                q.mu0_w = 0.0;
                q.mu0_s = 0.0;
                q.mu0_g = 0.0;
                q.omega = 0.0;
            }
            dg = 0.0;
        }
        ScaledPar { p, p_im, diff_w: dw, diff_g: dg, dc }
    }

    /// Dispersion coefficients, retardation, source/decay coefficients (Coeff + Disper + NEquil + PeCour).
    fn sol_coeff(&self, st: &mut SoluteState, js: usize, level: usize, iter: usize, dt: f64) -> (f64, f64, f64) {
        let n = self.n;
        let last = level == 2;
        let epsi = st.epsi;
        let mut peclet = 0.0f64;
        let mut courant = 0.0f64;
        let mut dt_max_c = 1e30f64;
        let l_bact = st.l_bact;
        let dual_neq = st.l_dual_neq;
        let dual_por = self.i_dual_por > 0;
        let temp_n = |i: usize| self.heat.as_ref().map(|h| h.temp_n[i]).unwrap_or(20.0);
        let temp_o = |i: usize| self.heat.as_ref().map(|h| h.temp_o[i]).unwrap_or(20.0);

        for i in (0..n).rev() {
            let j = i + 1;
            let m = self.mat[i];
            let mob = st.l_mob_im[m];
            // (lMobIm(M).and.iDualPor.eq.0.or.lBact): immobile water is subtracted from theta
            let sub_im = (mob && !dual_por) || l_bact;
            // mobile-immobile model (incl. dual porosity), not for the filtration model
            let mobim = (mob || dual_por) && !l_bact;
            let th_imob_n = if dual_por { self.th_new_im[i] } else { st.th_im[m] };
            let th_imob_o = if dual_por { self.th_old_im[i] } else { st.th_im[m] };
            let th_imob = if last { th_imob_n } else { th_imob_o };

            let (th_raw, v, vj, thj_raw) = {
                let (th_arr, v_arr) = if last { (&st.th_n, &st.v_n) } else { (&st.th_o, &st.v_o) };
                let has_j = i != n - 1;
                (th_arr[i], v_arr[i], if has_j { v_arr[j] } else { 0.0 }, if has_j { th_arr[j] } else { 0.0 })
            };
            let th_g = (self.ths[m] - th_raw).max(0.0);
            let mut th_w = th_raw;
            let mut thj = thj_raw;
            if sub_im {
                th_w = (th_w - st.th_im[m]).max(0.001);
                if i != n - 1 {
                    thj = (thj - st.th_im[m]).max(0.001);
                }
            }
            let th_wo_raw = st.th_o[i]; // ThWO = ThO(i) (not reduced by ThImob)

            let ro = st.ro[m];
            let frac = st.frac[m];
            let t_cur = if last { temp_n(i) } else { temp_o(i) };
            let sp = self.scaled_params(st, js, m, t_cur, th_w, th_imob);
            let p = sp.p.clone();
            let p_im = sp.p_im.clone();
            let (diff_w, diff_g) = (sp.diff_w, sp.diff_g);
            // values at the old time level (only needed on the last level of a non-equilibrium problem)
            let spo = if last && (!st.l_equil || st.l_bact) {
                Some(self.scaled_params(st, js, m, temp_o(i), th_wo_raw, th_imob_o))
            } else {
                None
            };
            let (po, po_im) = match &spo {
                Some(s) => (s.p.clone(), s.p_im.clone()),
                None => (p.clone(), p_im.clone()),
            };
            // change of Ks and Nu over the time step caused by the temperature change
            let (dks, dnu) = if st.l_tdep {
                let a = self.scaled_params(st, js, m, temp_n(i), th_w, th_imob);
                let b = self.scaled_params(st, js, m, temp_o(i), th_w, th_imob);
                ((a.p.ks - b.p.ks) / dt, (a.p.nu - b.p.nu) / dt)
            } else {
                (0.0, 0.0)
            };
            // parent (previous solute in the chain) parameters at the node's temperature
            let parent = if js > 0 { Some(self.scaled_params(st, js - 1, m, t_cur, th_w, th_imob)) } else { None };

            let (xks, xnu, fexp, henry) = (p.ks, p.nu, p.beta, p.henry);
            // With lDualNEq the SinkG1 / SinkG1' columns hold f_em and omega_S. The Fortran also uses them as gas-phase decay
            // rates GamG / GamG1 (a side effect of the shared slots); that leak is not reproduced.
            let (gam_l, gam_s, gam_l1, gam_s1) = (p.mu_w, p.mu_s, p.gam_w, p.gam_s);
            let (gam_g, gam_g1) = if dual_neq { (0.0, 0.0) } else { (p.mu_g, p.gam_g) };
            let (gam_li, gam_si, gam_l1i, gam_s1i) = (p_im.mu_w, p_im.mu_s, p_im.gam_w, p_im.gam_s);
            let (xmu_l, xmu_s, xmu_g, omega) = (p.mu0_w, p.mu0_s, p.mu0_g, p.omega);
            // lDualNEq: f_em = ChPar(13) (SinkG1 column), OmegaS = ChPar(16) (SinkG1' column)
            let f_em = if dual_neq { p.mu_g } else { 1.0 };
            let omega_s = if dual_neq { p.gam_g } else { 0.0 };

            // ---- effects of (non)linear adsorption
            let (mut dsconc, mut dconc, mut sconc, mut sconc_o) = (1.0, 1.0, 1.0, 1.0);
            let (mut sconc_s, mut dsconc_s, mut sconc_os) = (1.0, 1.0, 1.0);
            let (mut dretard, mut dretard_s) = (0.0, 0.0);
            let (mut sconc_p, mut sconc_ps) = (1.0, 1.0);
            let mut c_mid = 0.0;
            let c_old = st.conc[js][i];
            let s_old = st.sorb[js][i];
            let d_retard_fn = |cm: f64| -> f64 {
                let cf = cm.powf(fexp);
                cf / (1.0 + xnu * cf) * dks - xks * cm.powf(2.0 * fexp) / (1.0 + xnu * cf).powi(2) * dnu
            };
            if !st.l_linear[js] {
                let c_new_i = st.c_new[i];
                c_mid = (c_old + c_new_i) / 2.0;
                let cc = if last { c_new_i } else { c_old };
                if cc > 0.0 {
                    dsconc = fexp * cc.powf(fexp - 1.0) / (1.0 + xnu * cc.powf(fexp)).powi(2);
                    sconc = cc.powf(fexp - 1.0) / (1.0 + xnu * cc.powf(fexp));
                }
                if c_mid > 0.0 {
                    dconc = fexp * c_mid.powf(fexp - 1.0) / (1.0 + xnu * c_mid.powf(fexp)).powi(2);
                    dretard = d_retard_fn(c_mid);
                }
                if last && !st.l_equil && c_old > 0.0 {
                    sconc_o = c_old.powf(po.beta - 1.0) / (1.0 + po.nu * c_old.powf(po.beta));
                }
                if mob || dual_por {
                    let s_mid = (s_old + st.sorb_n[i]) / 2.0;
                    let ss = if last { st.sorb_n[i] } else { s_old };
                    if ss > 0.0 {
                        dsconc_s = fexp * ss.powf(fexp - 1.0) / (1.0 + xnu * ss.powf(fexp)).powi(2);
                        sconc_s = ss.powf(fexp - 1.0) / (1.0 + xnu * ss.powf(fexp));
                    }
                    if s_mid > 0.0 {
                        dretard_s = d_retard_fn(s_mid);
                    }
                    if last && !st.l_equil && s_old > 0.0 {
                        sconc_os = s_old.powf(po.beta - 1.0) / (1.0 + po.nu * s_old.powf(po.beta));
                    }
                }
            } else {
                if c_old > 0.0 {
                    dretard = c_old * dks;
                }
                if (mob || dual_por) && s_old > 0.0 {
                    dretard_s = s_old * dks;
                }
            }
            if let Some(pp) = &parent {
                if !st.l_linear[js - 1] {
                    let cprev = if last { st.conc[js - 1][i] } else { st.conc_old_step[js - 1][i] };
                    if cprev > 0.0 {
                        sconc_p = cprev.powf(pp.p.beta - 1.0) / (1.0 + pp.p.nu * cprev.powf(pp.p.beta));
                    }
                    let sp1 = st.sorb[js - 1][i];
                    if sp1 > 0.0 {
                        sconc_ps = sp1.powf(pp.p.beta - 1.0) / (1.0 + pp.p.nu * sp1.powf(pp.p.beta));
                    }
                }
            }

            // ---- retardation factor
            let retard = (ro * frac * f_em * xks * dconc + th_g * henry) / th_w + 1.0;
            st.retard[i] = retard;

            // ---- dispersion (Disper)
            let (mut tau_w, mut tau_g) = (1.0, 1.0);
            if st.tort {
                let mut ths = self.ths[m];
                if sub_im {
                    ths = (ths - st.th_im[m]).max(0.001);
                }
                if dual_por {
                    ths = self.ths[m] + th_imob;
                }
                if st.tort_model == 0 {
                    tau_w = th_w.powf(7.0 / 3.0) / ths.powi(2);
                    tau_g = th_g.powf(7.0 / 3.0) / ths.powi(2);
                } else {
                    tau_w = 0.66 * (th_w / ths).powf(8.0 / 3.0);
                    tau_g = th_g.powf(1.5) / ths;
                }
            }
            let mut disp = st.disp_l[m] * v.abs() / th_w + diff_w * tau_w + th_g / th_w * diff_g * henry * tau_g;
            if !st.art_d && !st.upw {
                let mut fi = 0.0;
                if c_mid > 0.0 {
                    fi = 6.0 * th_w * ro * xks * c_mid.powf(fexp - 1.0) * (fexp / (1.0 + xnu * c_mid.powf(fexp)).powi(2) - 1.0 / (1.0 + xnu * c_mid.powf(fexp)));
                }
                let dpom = (dt / (6.0 * th_w * (th_w + ro * frac * xks * dsconc + th_g * henry) + fi)).max(0.0);
                if !last {
                    disp += v * v * dpom;
                } else {
                    disp = (disp - v * v * dpom).max(disp / 2.0);
                }
            }
            if st.art_d {
                let mut dd = 0.0;
                if st.pe_cr != 0.0 && v.abs() > 1e-15 {
                    dd = v * v * dt / th_w / th_w / retard / st.pe_cr;
                }
                if dd > disp {
                    disp = dd;
                }
            }
            st.disp[i] = disp;

            // ---- filtration model: deposition coefficients and blocking functions
            let z_dist = self.depth_of(i);
            let (mut r_ka1, mut r_ka2) = (p.r_ka1, p.r_ka2);
            let (mut r_ka1_o, mut r_ka2_o) = (po.r_ka1, po.r_ka2);
            let (mut psi1, mut psi2, mut psi1_o, mut psi2_o) = (1.0, 1.0, 1.0, 1.0);
            if l_bact {
                if st.l_filtr {
                    let (a, b) = deposit(p.d_c, p.d_p, p.r_ka1, p.r_ka2, th_w, v, temp_n(i), self.x_conv, self.t_conv);
                    r_ka1 = a;
                    r_ka2 = b;
                    if last {
                        let v_o = st.v_o[i];
                        let (a, b) = deposit(po.d_c, po.d_p, po.r_ka1, po.r_ka2, th_wo_raw, v_o, temp_o(i), self.x_conv, self.t_conv);
                        r_ka1_o = a;
                        r_ka2_o = b;
                    }
                }
                let ss1 = if last { st.sorb_n[i] } else { st.sorb[js][i] };
                let ss2 = if last { st.sorb_n2[i] } else { st.sorb2[js][i] };
                if p.i_psi1 > 0 {
                    psi1 = blocking(p.i_psi1, p.s_max1, z_dist, ss1, sp.dc, p.s_max2);
                }
                if p.i_psi2 > 0 {
                    psi2 = blocking(p.i_psi2, p.s_max2, z_dist, ss2, sp.dc, p.s_max2);
                }
                if last {
                    if po.i_psi1 > 0 {
                        psi1_o = blocking(po.i_psi1, po.s_max1, z_dist, st.sorb[js][i], sp.dc, po.s_max2);
                    }
                    if po.i_psi2 > 0 {
                        psi2_o = blocking(po.i_psi2, po.s_max2, z_dist, st.sorb2[js][i], sp.dc, po.s_max2);
                    }
                }
            }

            // parent-solute reaction rates (immobile and mobile versions)
            let (xks_p, gam_l1p, gam_s1p, gam_g1p, gam_l1pi, gam_s1pi) = match &parent {
                Some(pp) => (pp.p.ks, pp.p.gam_w, pp.p.gam_s, if dual_neq { 0.0 } else { pp.p.gam_g }, pp.p_im.gam_w, pp.p_im.gam_s),
                None => (0.0, 0.0, 0.0, 0.0, 0.0, 0.0),
            };
            let henry_p = parent.as_ref().map(|pp| pp.p.henry).unwrap_or(0.0);

            // ---- NEquil: sorbed / immobile concentration before the matrix equation is solved
            let fl_macro = if dual_por {
                let si = self.sink_im[i];
                if si > 0.0 { si * c_old } else { si * s_old }
            } else {
                0.0
            };
            let mut ssorb = st.sorb[js][i];
            let mut ssorb2 = st.sorb2[js][i];
            if !st.l_equil && last {
                let cc_new = st.c_new[i];
                if mobim {
                    let a_mob = (th_imob_n + th_imob_o) / 2.0 + (1.0 - frac) * ro * xks * dsconc_s;
                    let d_theta = th_imob_n - th_imob_o;
                    let mut e_mob = th_imob_n * xmu_l + (1.0 - frac) * ro * xmu_s - (1.0 - frac) * ro * dretard_s;
                    let mut e_mob_o = th_imob_o * po.mu0_w + (1.0 - frac) * ro * po.mu0_s - (1.0 - frac) * ro * dretard_s;
                    e_mob += fl_macro;
                    e_mob_o += fl_macro;
                    if st.l_nequil && js > 0 {
                        let cs = st.sorb[js - 1][i] * (th_imob_n * gam_l1pi + (1.0 - frac) * ro * gam_s1pi * xks_p * sconc_ps);
                        e_mob += cs;
                        e_mob_o += cs;
                    }
                    let b_mob = th_imob_n * (gam_li + gam_l1i) + (1.0 - frac) * ro * (gam_si + gam_s1i) * xks * sconc_s;
                    let b_mob_o = th_imob_o * (po_im.mu_w + po_im.gam_w) + (1.0 - frac) * ro * (po_im.mu_s + po_im.gam_s) * po.ks * sconc_os;
                    if st.l_linear[js] {
                        let d_mob = 2.0 * a_mob + dt * (omega + b_mob) + d_theta;
                        let g_mob0 = (2.0 * a_mob - dt * (po.omega + b_mob_o) - d_theta) / d_mob;
                        st.d_mob_i = d_mob;
                        st.sorb[js][i] = s_old * g_mob0 + dt * (po.omega * c_old + e_mob + e_mob_o) / d_mob;
                        ssorb = st.sorb[js][i];
                    } else {
                        st.sorb_n[i] = s_old
                            + dt / a_mob
                                * (epsi * (omega * (cc_new - st.sorb_n[i]) - b_mob * st.sorb_n[i] + e_mob)
                                    + (1.0 - epsi) * (po.omega * (c_old - s_old) - b_mob_o * s_old + e_mob_o));
                        ssorb = st.sorb_n[i];
                    }
                    if dual_neq {
                        let mut cs = 0.0;
                        if st.l_nequil && js > 0 {
                            cs = gam_s1pi * st.sorb2[js - 1][i] * dt * 2.0;
                        }
                        let s2_old = st.sorb2[js][i];
                        if st.l_linear[js] {
                            st.sorb2[js][i] = ((2.0 - (omega_s + po_im.mu_s + po_im.gam_s) * dt) * s2_old
                                + dt * frac * (1.0 - f_em) * omega_s * po.ks * c_old
                                + dt * frac * (1.0 - f_em) * (po.mu0_s + xmu_s)
                                + cs)
                                / (2.0 + dt * (omega_s + gam_si + gam_s1i));
                            ssorb2 = st.sorb2[js][i];
                        } else {
                            st.sorb_n2[i] = s2_old
                                + dt * (epsi * (omega_s * (frac * (1.0 - f_em) * sconc * xks * cc_new - st.sorb_n2[i]) - (gam_si + gam_s1i) * st.sorb_n2[i] + frac * (1.0 - f_em) * xmu_s)
                                    + (1.0 - epsi) * (omega_s * (frac * (1.0 - f_em) * sconc_o * po.ks * c_old - s2_old) - (po_im.mu_s + po_im.gam_s) * s2_old + frac * (1.0 - f_em) * po.mu0_s));
                            ssorb2 = st.sorb_n2[i];
                        }
                    }
                } else if !l_bact {
                    // two-site sorption model
                    let mut cs = 0.0;
                    if st.l_nequil && js > 0 {
                        cs = gam_s1pi * st.sorb[js - 1][i] * dt * 2.0;
                    }
                    if st.l_linear[js] {
                        st.sorb[js][i] = ((2.0 - (po.omega + po_im.mu_s + po_im.gam_s) * dt) * s_old
                            + dt * (1.0 - frac) * po.omega * po.ks * c_old
                            + dt * (1.0 - frac) * (po.mu0_s + xmu_s)
                            + cs)
                            / (2.0 + dt * (omega + gam_si + gam_s1i));
                        ssorb = st.sorb[js][i];
                    } else {
                        st.sorb_n[i] = s_old
                            + dt * (epsi * (omega * ((1.0 - frac) * sconc * xks * cc_new - st.sorb_n[i]) - (gam_si + gam_s1i) * st.sorb_n[i] + (1.0 - frac) * xmu_s)
                                + (1.0 - epsi) * (po.omega * ((1.0 - frac) * sconc_o * po.ks * c_old - s_old) - (po_im.mu_s + po_im.gam_s) * s_old + (1.0 - frac) * po.mu0_s));
                        ssorb = st.sorb_n[i];
                    }
                } else {
                    // filtration model (two attachment sites)
                    let (gso, s2_old) = (po_im.mu_s + po_im.gam_s, st.sorb2[js][i]);
                    let gs = gam_si + gam_s1i;
                    if st.l_linear[js] {
                        st.sorb[js][i] = ((2.0 - dt * (po.r_kd1 + gso)) * s_old + dt * r_ka1_o * th_w * c_old / ro) / (2.0 + dt * (p.r_kd1 + gs));
                        ssorb = st.sorb[js][i];
                        st.sorb2[js][i] = ((2.0 - dt * (po.r_kd2 + gso)) * s2_old + dt * r_ka2_o * th_w * c_old / ro) / (2.0 + dt * (p.r_kd2 + gs));
                        ssorb2 = st.sorb2[js][i];
                    } else {
                        st.sorb_n[i] = s_old
                            + dt * (epsi * (r_ka1 * th_w / ro * psi1 * cc_new - (p.r_kd1 + gs) * st.sorb_n[i])
                                + (1.0 - epsi) * (r_ka1_o * th_wo_raw / ro * psi1_o * c_old - (po.r_kd1 + gso) * s_old));
                        ssorb = st.sorb_n[i];
                        st.sorb_n2[i] = s2_old
                            + dt * (epsi * (r_ka2 * th_w / ro * psi2 * cc_new - (p.r_kd2 + gs) * st.sorb_n2[i])
                                + (1.0 - epsi) * (r_ka2_o * th_wo_raw / ro * psi2_o * c_old - (po.r_kd2 + gso) * s2_old));
                        ssorb2 = st.sorb_n2[i];
                    }
                }
            }

            // ---- zero-order coefficient g0
            let mut g0 = xmu_l * th_w + frac * f_em * ro * xmu_s + th_g * xmu_g - st.s_sink[i];
            let mut q0 = xmu_l * th_w + ro * xmu_s + th_g * xmu_g;
            if self.l_dual_perm {
                let w_f = self.w_fracture.max(0.001);
                let c_matrix = if last { st.conc_m[js][i] } else { st.conc_m_old[i] };
                let gamma_w = self.sink_im[i];
                // Gamma_s = alpha_s (1-w) theta_m (c_f - c_m) + Gamma_w c*   (per total volume), divided by w here
                let alpha_eff = p.alpha_s * (1.0 - w_f) * self.th_m_eff(st, i, last) / w_f;
                let gamma_adv = if gamma_w < 0.0 { -gamma_w * c_matrix / w_f } else { 0.0 };
                g0 += alpha_eff * c_matrix + gamma_adv;
            }
            if !st.l_equil {
                if mobim {
                    g0 += omega * ssorb;
                    if dual_por && self.sink_im[i] <= 0.0 {
                        g0 -= fl_macro;
                    }
                    if dual_neq {
                        g0 += omega_s * ro * ssorb2;
                    }
                } else if !l_bact {
                    g0 += omega * ro * ssorb;
                } else {
                    g0 += p.r_kd1 * ro * ssorb + p.r_kd2 * ro * ssorb2;
                }
            }
            if let Some(_pp) = &parent {
                let cprev = if last { st.conc[js - 1][i] } else { st.conc_old_step[js - 1][i] };
                let mut cg = cprev * (gam_l1p * th_w + ro * frac * f_em * xks_p * gam_s1p * sconc_p + th_g * henry_p * gam_g1p);
                let mut cg1 = cg;
                if !st.l_equil {
                    if mobim {
                        let aa = st.sorb[js - 1][i] * (th_imob * gam_l1pi + (1.0 - frac) * ro * gam_s1pi * xks_p * sconc_ps);
                        if !st.l_nequil {
                            cg += aa;
                        }
                        cg1 += aa;
                        if dual_neq {
                            let aa2 = gam_s1pi * ro * st.sorb2[js - 1][i];
                            if !st.l_nequil {
                                cg += aa2;
                            }
                            cg1 += aa2;
                        }
                    } else if !l_bact {
                        let aa = gam_s1pi * ro * st.sorb[js - 1][i];
                        if !st.l_nequil {
                            cg += aa;
                        }
                        cg1 += aa;
                    }
                }
                g0 += cg;
                q0 += cg1;
            }
            if c_mid > 0.0 {
                g0 -= ro * frac * f_em * dretard;
            }
            st.g0[i] = g0;
            st.q0[i] = q0;

            // ---- first-order coefficient g1
            let mut g1 = -(gam_l + gam_l1) * th_w - (gam_s + gam_s1) * ro * frac * f_em * xks * sconc - (gam_g + gam_g1) * th_g * henry;
            if self.l_dual_perm {
                let w_f = self.w_fracture.max(0.001);
                let gamma_w = self.sink_im[i];
                let alpha_eff = p.alpha_s * (1.0 - w_f) * self.th_m_eff(st, i, last) / w_f;
                let gamma_adv = if gamma_w > 0.0 { gamma_w / w_f } else { 0.0 };
                g1 -= alpha_eff + gamma_adv;
            }
            if !st.l_equil {
                if mobim {
                    g1 -= omega;
                    if dual_por && self.sink_im[i] > 0.0 {
                        g1 -= self.sink_im[i];
                    }
                    if last && st.l_linear[js] {
                        g1 += omega * dt * omega / st.d_mob_i;
                    }
                    if dual_neq {
                        g1 -= omega_s * ro * frac * (1.0 - f_em) * sconc * xks;
                        if last && st.l_linear[js] {
                            g1 += omega_s * ro * (dt * omega_s * frac * (1.0 - f_em) * xks / (2.0 + dt * (omega_s + gam_si + gam_s1i)));
                        }
                    }
                } else if !l_bact {
                    g1 -= omega * ro * (1.0 - frac) * sconc * xks;
                    if last && st.l_linear[js] {
                        g1 += omega * ro * (dt * omega * (1.0 - frac) * xks / (2.0 + dt * (omega + gam_si + gam_s1i)));
                    }
                } else {
                    g1 -= th_w * (r_ka1 * psi1 + r_ka2 * psi2);
                    if last && st.l_linear[js] {
                        g1 += dt * th_w * (p.r_kd1 * r_ka1 / (2.0 + dt * (p.r_kd1 + gam_si)) + p.r_kd2 * r_ka2 / (2.0 + dt * (p.r_kd2 + gam_si)));
                    }
                }
            }
            st.g1[i] = g1;

            let mut q1 = (-(gam_l + gam_l1) * th_w - (gam_s + gam_s1) * ro * frac * f_em * xks * sconc - (gam_g + gam_g1) * th_g * henry) * c_old;
            if !st.l_equil {
                if mobim {
                    q1 -= s_old * (th_imob * (gam_li + gam_l1i) + (1.0 - frac) * ro * xks * sconc_s * (gam_si + gam_s1i));
                    if dual_neq {
                        q1 -= (gam_si + gam_s1i) * ro * st.sorb2[js][i];
                    }
                } else if !l_bact {
                    q1 -= (gam_si + gam_s1i) * ro * s_old;
                } else {
                    q1 -= ro * (gam_si + gam_s1i) * (s_old + st.sorb2[js][i]);
                }
            }
            st.q1[i] = q1;

            // ---- velocity correction (gas-phase diffusion, varying Henry constant with temperature)
            let henry_at = |idx: usize| -> f64 {
                let t_idx = if last { temp_n(idx) } else { temp_o(idx) };
                self.scaled_params(st, js, self.mat[idx], t_idx, th_w, th_imob).p.henry
            };
            let der_k;
            if i == 0 {
                der_k = (henry_at(j) - henry) / (self.x[1] - self.x[0]);
            } else if i == n - 1 {
                der_k = (henry - henry_at(i - 1)) / (self.x[n - 1] - self.x[n - 2]);
            } else {
                der_k = (henry_at(j) - henry_at(i - 1)) / ((self.x[j] - self.x[i - 1]) / 2.0);
            }
            let vcorr = th_g * diff_g * tau_g * der_k;
            st.v_corr[i] = vcorr;
            if level == 1 {
                st.v_o[i] -= vcorr;
            } else {
                st.v_n[i] -= vcorr;
            }

            // ---- Peclet / Courant / upwind weights
            if i != n - 1 {
                let dx = self.x[j] - self.x[i];
                let mut vv = 0.0;
                let mut vv1 = 0.0;
                if th_w > 1e-6 && thj > 1e-6 {
                    vv = (v.abs() / th_w + vj.abs() / thj) / 2.0;
                    vv1 = (v / th_w + vj / thj) / 2.0;
                }
                let dd = (st.disp[i] + st.disp[j]) / 2.0;
                if last {
                    let mut pec = 99999.0;
                    let mut dtmax = 1e30;
                    let vmax = (v.abs() + vj.abs()) / (th_w + thj);
                    let rmin = st.retard[i].min(st.retard[j]);
                    if dd > 0.0 {
                        pec = vv.abs() * dx / dd;
                    }
                    let cour = vmax * dt / dx / rmin;
                    peclet = peclet.max(pec);
                    courant = courant.max(cour);
                    let mut cour1 = 1.0;
                    if !st.upw && !st.art_d && pec != 99999.0 {
                        cour1 = (1.0f64).min(st.pe_cr / pec.max(0.5));
                    }
                    if epsi < 1.0 && vmax > 1e-20 {
                        dtmax = cour1 * dx * rmin / vmax;
                    }
                    dt_max_c = dt_max_c.min(dtmax);
                } else if st.upw && iter == 1 {
                    let mut pe2 = 11.0;
                    if dd > 0.0 {
                        pe2 = dx * vv1 / dd / 2.0;
                    }
                    st.wc[i] = if vv.abs() < 1e-30 {
                        0.0
                    } else if pe2.abs() > 10.0 {
                        if vv1 > 0.0 { 1.0 } else { -1.0 }
                    } else {
                        (1.0 / pe2.tanh() - 1.0 / pe2).clamp(-1.0, 1.0)
                    };
                }
            }
        }
        (peclet, courant, dt_max_c)
    }

    /// Assemble the tridiagonal system (MatSet).
    fn sol_matset(&self, st: &mut SoluteState, js: usize, level: usize, dt: f64) {
        let n = self.n;
        let epsi = st.epsi;
        let alf = 1.0 - epsi;
        let mut tho = st.th_o.clone();
        let mut thn = st.th_n.clone();
        for i in 0..n {
            let m = self.mat[i];
            if (st.l_mob_im[m] && self.i_dual_por == 0) || st.l_bact {
                tho[i] = (tho[i] - st.th_im[m]).max(0.001);
                thn[i] = (thn[i] - st.th_im[m]).max(0.001);
            }
        }
        let (vo, vn, wc, disp, retard, g0, g1) = (&st.v_o, &st.v_n, &st.wc, &st.disp, &st.retard, &st.g0, &st.g1);
        let c = &st.conc[js];
        let x = &self.x;
        let (mut b, mut d, mut e, mut f) = (st.mb.clone(), st.md.clone(), st.me.clone(), st.mf.clone());
        let mut b1 = x[1] - x[0];
        let (mut f1, mut e1, mut d1) = (st.f1, st.e1, st.d1);
        let (mut bn, mut dn, mut fnn) = (st.bn, st.dn, st.fn_);
        if level == 1 {
            f1 = c[0] * (b1 / 2.0 / dt * tho[0] * retard[0]
                + alf * (-(tho[0] * disp[0] + tho[1] * disp[1]) / b1 / 2.0 - ((2.0 + 3.0 * wc[0]) * vo[0] + vo[1]) / 6.0 + b1 / 12.0 * (3.0 * g1[0] + g1[1])))
                + c[1] * alf * ((tho[0] * disp[0] + tho[1] * disp[1]) / b1 / 2.0 - (vo[0] + (2.0 - 3.0 * wc[0]) * vo[1]) / 6.0 + b1 / 12.0 * (g1[0] + g1[1]))
                + alf * b1 / 6.0 * (2.0 * g0[0] + g0[1]);
            if st.k_bot == -1 {
                f[0] = f1 + alf * st.c_bot[js] * vo[0];
            }
        } else {
            e1 = epsi * (-(thn[0] * disp[0] + thn[1] * disp[1]) / b1 / 2.0 + (vn[0] + (2.0 - 3.0 * wc[0]) * vn[1]) / 6.0 - b1 / 12.0 * (g1[0] + g1[1]));
            d1 = b1 / 2.0 / dt * thn[0] * retard[0]
                + epsi * ((thn[0] * disp[0] + thn[1] * disp[1]) / b1 / 2.0 + ((2.0 + 3.0 * wc[0]) * vn[0] + vn[1]) / 6.0 - b1 / 12.0 * (3.0 * g1[0] + g1[1]));
            let f2 = epsi * b1 / 6.0 * (2.0 * g0[0] + g0[1]);
            f1 += f2;
            if st.k_bot == 1 {
                d[0] = 1.0;
                e[0] = 0.0;
                f[0] = st.c_bot[js];
            }
            if st.k_bot == -1 {
                // MatSet: `if(vN(1).gt.0..or.(lVapor.and.rBot.eq.0.))` - with vapor flow and a no-flux
                // bottom the liquid flux at node 1 is not zero (it is balanced by the vapor flux), but no
                // solute may leave through the bottom: the third-type condition (prescribed solute flux
                // vN(1)*cBot) is used for either flow direction instead of the zero-gradient outflow condition.
                if vn[0] > 0.0 || (self.l_vapor && self.r_bot.abs() < 1e-20) {
                    e[0] = e1;
                    d[0] = d1;
                    f[0] = f[0] + f2 + epsi * st.c_bot[js] * vn[0];
                } else {
                    d[0] = -1.0;
                    e[0] = 1.0;
                    f[0] = 0.0;
                }
            }
            if st.k_bot == 0 {
                d[0] = -1.0;
                e[0] = 1.0;
                f[0] = 0.0;
            }
        }
        for i in 1..n - 1 {
            let a1 = b1;
            b1 = x[i + 1] - x[i];
            let dx = (x[i + 1] - x[i - 1]) / 2.0;
            if level == 1 {
                f[i] = c[i - 1]
                    * alf
                    * ((tho[i - 1] * disp[i - 1] + tho[i] * disp[i]) / a1 / 2.0 + ((2.0 + 3.0 * wc[i - 1]) * vo[i - 1] + vo[i]) / 6.0 + a1 / 12.0 * (g1[i - 1] + g1[i]))
                    + c[i]
                        * (dx / dt * tho[i] * retard[i]
                            + alf
                                * (-(tho[i - 1] * disp[i - 1] + tho[i] * disp[i]) / a1 / 2.0 - (tho[i + 1] * disp[i + 1] + tho[i] * disp[i]) / b1 / 2.0
                                    - (vo[i + 1] + 3.0 * (wc[i - 1] + wc[i]) * vo[i] - vo[i - 1]) / 6.0
                                    + (a1 * (g1[i - 1] + 3.0 * g1[i]) + b1 * (3.0 * g1[i] + g1[i + 1])) / 12.0))
                    + c[i + 1]
                        * alf
                        * ((tho[i + 1] * disp[i + 1] + tho[i] * disp[i]) / b1 / 2.0 - (vo[i] + (2.0 - 3.0 * wc[i]) * vo[i + 1]) / 6.0 + b1 / 12.0 * (g1[i] + g1[i + 1]))
                    + alf * (a1 * (g0[i - 1] + 2.0 * g0[i]) + b1 * (2.0 * g0[i] + g0[i + 1])) / 6.0;
            } else {
                b[i] = epsi * (-(thn[i - 1] * disp[i - 1] + thn[i] * disp[i]) / a1 / 2.0 - ((2.0 + 3.0 * wc[i - 1]) * vn[i - 1] + vn[i]) / 6.0 - a1 / 12.0 * (g1[i - 1] + g1[i]));
                d[i] = dx / dt * thn[i] * retard[i]
                    + epsi
                        * ((thn[i - 1] * disp[i - 1] + thn[i] * disp[i]) / a1 / 2.0 + (thn[i + 1] * disp[i + 1] + thn[i] * disp[i]) / b1 / 2.0
                            + (vn[i + 1] + 3.0 * (wc[i - 1] + wc[i]) * vn[i] - vn[i - 1]) / 6.0
                            - (a1 * (g1[i - 1] + 3.0 * g1[i]) + b1 * (3.0 * g1[i] + g1[i + 1])) / 12.0);
                e[i] = epsi * (-(thn[i + 1] * disp[i + 1] + thn[i] * disp[i]) / b1 / 2.0 + (vn[i] + (2.0 - 3.0 * wc[i]) * vn[i + 1]) / 6.0 - b1 / 12.0 * (g1[i] + g1[i + 1]));
                f[i] += epsi * (a1 * (g0[i - 1] + 2.0 * g0[i]) + b1 * (2.0 * g0[i] + g0[i + 1])) / 6.0;
            }
        }
        let m = n - 1;
        let mat_top = self.mat[m];
        let henry_top = st.pm[js][mat_top].henry;
        let (g_vol, g_atm) = if st.k_top == -2 && st.d_surf > 1e-10 {
            (st.diff_g[js] * henry_top / st.d_surf, st.diff_g[js] / st.d_surf)
        } else {
            (0.0, 0.0)
        };

        if level == 1 {
            fnn = c[m - 1]
                * alf
                * ((tho[m - 1] * disp[m - 1] + tho[m] * disp[m]) / b1 / 2.0 + ((2.0 + 3.0 * wc[m - 1]) * vo[m - 1] + vo[m]) / 6.0 + b1 / 12.0 * (g1[m - 1] + g1[m]))
                + c[m]
                    * (b1 / 2.0 / dt * tho[m] * retard[m]
                        + alf
                            * (-(tho[m - 1] * disp[m - 1] + tho[m] * disp[m]) / b1 / 2.0 + (vo[m - 1] + (2.0 - 3.0 * wc[m - 1]) * vo[m]) / 6.0
                                + b1 / 12.0 * (g1[m - 1] + 3.0 * g1[m])))
                + alf * b1 / 6.0 * (g0[m - 1] + 2.0 * g0[m]);
            if st.k_top <= 0 {
                f[m] = fnn;
                if vo[m] < 0.0 {
                    f[m] -= alf * vo[m] * st.c_top[js];
                }
                if st.k_top == -2 {
                    f[m] += -alf * g_vol * c[m] + g_atm * st.c_atm;
                }
            }
        } else {
            bn = epsi * (-(thn[m - 1] * disp[m - 1] + thn[m] * disp[m]) / b1 / 2.0 - ((2.0 + 3.0 * wc[m - 1]) * vn[m - 1] + vn[m]) / 6.0 - b1 / 12.0 * (g1[m - 1] + g1[m]));
            dn = b1 / 2.0 / dt * thn[m] * retard[m]
                + epsi * ((thn[m - 1] * disp[m - 1] + thn[m] * disp[m]) / b1 / 2.0 - (vn[m - 1] + (2.0 - 3.0 * wc[m - 1]) * vn[m]) / 6.0 - b1 / 12.0 * (g1[m - 1] + 3.0 * g1[m]));
            let fe = epsi * b1 / 6.0 * (g0[m - 1] + 2.0 * g0[m]);
            fnn += fe;
            if st.k_top > 0 {
                b[m] = 0.0;
                d[m] = 1.0;
                f[m] = st.c_top[js];
            } else {
                b[m] = bn;
                d[m] = dn;
                f[m] += fe;
                if vn[m] < 0.0 {
                    f[m] -= epsi * vn[m] * st.c_top[js];
                }
                if st.k_top == -2 {
                    d[m] += epsi * g_vol;
                }
            }
        }
        st.mb = b;
        st.md = d;
        st.me = e;
        st.mf = f;
        st.f1 = f1;
        st.e1 = e1;
        st.d1 = d1;
        st.bn = bn;
        st.dn = dn;
        st.fn_ = fnn;
    }

    /// Mass-transfer fluxes (MassTran). `epsi` is the time weight of this call; `new_t` selects
    /// the temperature of the new (true) or the old (false) time level.
    fn sol_masstran(&self, st: &mut SoluteState, js: usize, epsi: f64, new_t: bool) {
        let n = self.n;
        let dual_por = self.i_dual_por > 0;
        let temp = |i: usize| -> f64 {
            self.heat.as_ref().map(|h| if new_t { h.temp_n[i] } else { h.temp_o[i] }).unwrap_or(20.0)
        };
        for i in 0..n - 1 {
            let j = i + 1;
            let dx = self.x[j] - self.x[i];
            st.cv_ch0[js] += epsi * dx * (st.q0[i] + st.q0[j]) / 2.0;
            st.cv_ch1[js] += epsi * dx * (st.q1[i] + st.q1[j]) / 2.0;
            st.cv_chr[js] += epsi * dx * (st.s_sink[i] + st.s_sink[j]) / 2.0;

            if self.l_dual_perm {
                // fracture-matrix solute exchange Gamma_s (per total volume), reported as transfer
                st.cv_chim[js] += epsi * dx / 2.0 * (st.gam_s_dp[i] + st.gam_s_dp[j]);
            }
            if !st.l_equil {
                let (mi, mj) = (self.mat[i], self.mat[j]);
                let th_ref = if new_t { &st.th_n } else { &st.th_o };
                let (thi_raw, thj_raw) = (th_ref[i], th_ref[j]);
                let pi_ = self.scaled_params(st, js, mi, temp(i), thi_raw, 0.0).p;
                let pj = self.scaled_params(st, js, mj, temp(j), thj_raw, 0.0).p;
                let (ci, cj) = (st.conc[js][i], st.conc[js][j]);
                let (si, sj) = (st.sorb[js][i], st.sorb[js][j]);
                let (roi, roj) = (st.ro[mi], st.ro[mj]);
                let (fri, frj) = (st.frac[mi], st.frac[mj]);
                let e_of = |cc: f64, p: &SpeciesMaterial| if cc > 0.0 { p.ks * cc.powf(p.beta) / (1.0 + p.nu * cc.powf(p.beta)) } else { 0.0 };
                if (st.l_mob_im[mi] || dual_por) && !st.l_bact {
                    let fl = |idx: usize| -> f64 {
                        if dual_por {
                            let sk = self.sink_im[idx];
                            if sk > 0.0 { sk * st.conc[js][idx] } else { sk * st.sorb[js][idx] }
                        } else {
                            0.0
                        }
                    };
                    let (fli, flj) = (fl(i), fl(j));
                    st.cv_chim[js] += epsi * dx / 2.0 * (pi_.omega * (ci - si) + pj.omega * (cj - sj) + fli + flj);
                    if js == 0 && !st.l_linear[js] {
                        st.s_trans[i] = epsi * (pi_.omega * (ci - si) + fli);
                    }
                    if st.l_dual_neq {
                        // f_em = ChPar(13), OmegaS = ChPar(16)
                        let se_i = fri * (1.0 - pi_.mu_g) * e_of(ci, &pi_);
                        let se_j = frj * (1.0 - pj.mu_g) * e_of(cj, &pj);
                        let (s2i, s2j) = (st.sorb2[js][i], st.sorb2[js][j]);
                        st.cv_chim[js] += epsi * dx / 2.0 * (roi * pi_.gam_g * (se_i - s2i) + roj * pj.gam_g * (se_j - s2j));
                        if js == 0 && !st.l_linear[js] {
                            st.s_trans[i] += epsi * roi * pi_.gam_g * (se_i - s2i);
                        }
                    }
                } else if !st.l_bact {
                    let sei = (1.0 - fri) * e_of(ci, &pi_);
                    let sej = (1.0 - frj) * e_of(cj, &pj);
                    st.cv_chim[js] += epsi * dx / 2.0 * (roi * pi_.omega * (sei - si) + roj * pj.omega * (sej - sj));
                    if js == 0 {
                        st.s_trans[i] = epsi * roi * pi_.omega * (sei - si);
                    }
                } else {
                    // filtration model; the rates of node i (Deposit is evaluated for node i only)
                    let (s2i, s2j) = (st.sorb2[js][i], st.sorb2[js][j]);
                    let thi = thi_raw - st.th_im[mi];
                    let thj = thj_raw - st.th_im[mj];
                    let (mut ka1i, mut ka1j, mut ka2i, mut ka2j) = (pi_.r_ka1, pj.r_ka1, pi_.r_ka2, pj.r_ka2);
                    if st.l_filtr {
                        let v_ref = if new_t { &st.v_n } else { &st.v_o };
                        let (a, b) = deposit(pi_.d_c, pi_.d_p, pi_.r_ka1, pi_.r_ka2, thi, v_ref[i], temp(i), self.x_conv, self.t_conv);
                        ka1i = a;
                        ka1j = a;
                        ka2i = b;
                        ka2j = b;
                    }
                    let (zi, zj) = (self.depth_of(i), self.depth_of(j));
                    let dci = self.scaled_params(st, js, mi, temp(i), thi_raw, 0.0).dc;
                    let psi = |ip: i32, smax: f64, z: f64, ss: f64, smax2: f64| if ip > 0 { blocking(ip, smax, z, ss, dci, smax2) } else { 1.0 };
                    let (psi1i, psi1j) = (psi(pi_.i_psi1, pi_.s_max1, zi, si, pi_.s_max2), psi(pi_.i_psi1, pj.s_max1, zj, sj, pi_.s_max2));
                    let (psi2i, psi2j) = (psi(pi_.i_psi2, pi_.s_max2, zi, s2i, pi_.s_max2), psi(pi_.i_psi2, pj.s_max2, zj, s2j, pi_.s_max2));
                    st.cv_chim[js] += epsi * dx / 2.0
                        * (ci * thi * (psi1i * ka1i + psi2i * ka2i) + cj * thj * (psi1j * ka1j + psi2j * ka2j)
                            - roi * (si * pi_.r_kd1 + s2i * pi_.r_kd2)
                            - roj * (sj * pj.r_kd1 + s2j * pj.r_kd2));
                    if js == 0 {
                        st.s_trans[i] = epsi * (ci * thi * (psi1i * ka1i + psi2i * ka2i) - roi * (si * pi_.r_kd1 + s2i * pi_.r_kd2));
                    }
                }
            }
        }
    }

    /// Sorbed concentration at the end of the step for linear non-equilibrium transport
    /// (port of SorbConc in SOLUTE.FOR). The explicit part of the update was already applied in
    /// `sol_coeff` (NEquil); this adds the implicit increment.
    fn sol_sorbconc(&self, st: &mut SoluteState, js: usize, dt: f64) {
        let dual_por = self.i_dual_por > 0;
        for i in 0..self.n {
            let m = self.mat[i];
            let th_raw = st.th_n[i]; // ThW(i) of SorbConc is thN
            let temp = self.heat.as_ref().map(|h| h.temp_n[i]).unwrap_or(20.0);
            let th_imob_n = if dual_por { self.th_new_im[i] } else { st.th_im[m] };
            let th_imob_o = if dual_por { self.th_old_im[i] } else { st.th_im[m] };
            let (frac, ro) = (st.frac[m], st.ro[m]);
            let sp = self.scaled_params(st, js, m, temp, th_raw, th_imob_n);
            let (p, p_im) = (&sp.p, &sp.p_im);
            let c = st.conc[js][i];
            if (st.l_mob_im[m] || dual_por) && !st.l_bact {
                // mobile-immobile model
                let d_theta = th_imob_n - th_imob_o;
                let a_mob = (th_imob_n + th_imob_o) / 2.0 + (1.0 - frac) * ro * p.ks;
                let b_mob = th_imob_n * (p_im.mu_w + p_im.gam_w) + (p_im.mu_s + p_im.gam_s) * ro * (1.0 - frac) * p.ks;
                let d_mob = 2.0 * a_mob + dt * (p.omega + b_mob) + d_theta;
                st.sorb[js][i] += dt * p.omega * c / d_mob;
                let fl = if dual_por {
                    let sk = self.sink_im[i];
                    if sk > 0.0 { sk * c } else { sk * st.sorb[js][i] }
                } else {
                    0.0
                };
                if js == 0 {
                    st.s_trans[i] = p.omega * (c - st.sorb[js][i]) + fl;
                }
                if st.l_dual_neq {
                    // f_em = ChPar(13), OmegaS = ChPar(16)
                    let (f_em, omega_s) = (p.mu_g, p.gam_g);
                    st.sorb2[js][i] += dt * omega_s * frac * (1.0 - f_em) * p.ks * c / (2.0 + dt * (omega_s + p_im.mu_s + p_im.gam_s));
                }
            } else if !st.l_bact {
                // two-site sorption model
                let gs = self.scaled_params(st, js, m, temp, th_raw, th_raw).p_im; // GamS, GamS1: jReact 11 / 13 at ThW(i)
                st.sorb[js][i] += dt * p.omega * (1.0 - frac) * p.ks * c / (2.0 + dt * (p.omega + gs.mu_s + gs.gam_s));
            } else {
                // filtration model
                let theta = th_raw - st.th_im[m];
                let (mut ka1, mut ka2) = (p.r_ka1, p.r_ka2);
                if st.l_filtr {
                    let (a, b) = deposit(p.d_c, p.d_p, p.r_ka1, p.r_ka2, theta, st.v_n[i], temp, self.x_conv, self.t_conv);
                    ka1 = a;
                    ka2 = b;
                }
                let gs = p.mu_s + p.gam_s; // GamS1 = 0
                st.sorb[js][i] += dt * ka1 * theta * c / ro / (2.0 + dt * (p.r_kd1 + gs));
                st.sorb2[js][i] += dt * ka2 * theta * c / ro / (2.0 + dt * (p.r_kd2 + gs));
            }
        }
    }

    /// Equilibrium sorbed concentration s(c) = Ks c^beta / (1 + nu c^beta) and its derivative.
    fn dp_iso(p: &SpeciesMaterial, c: f64) -> (f64, f64) {
        if c <= 0.0 {
            return (0.0, if (p.beta - 1.0).abs() < 1e-12 { p.ks } else { 0.0 });
        }
        let cb = c.powf(p.beta);
        let den = 1.0 + p.nu * cb;
        (p.ks * cb / den, p.ks * p.beta * c.powf(p.beta - 1.0) / (den * den))
    }

    /// Matrix water content at node i (new / old time level) with the fall-backs used before the first solve.
    pub(crate) fn th_m_at(&self, i: usize, new: bool) -> f64 {
        let m = self.mat[i];
        let fallback = if m < self.par_d.len() && self.par_d[m][7] > 0.001 { self.par_d[m][7] } else { self.ths[m] };
        let arr = if new { &self.th_matrix_new } else { &self.th_matrix_old };
        if i < arr.len() && arr[i] > 0.001 { arr[i] } else { fallback }
    }

    /// Matrix water content at node `i` as seen by the solute transport (colloid exclusion replaces it by the accessible
    /// water content).
    pub(crate) fn th_m_eff(&self, st: &SoluteState, i: usize, new: bool) -> f64 {
        match st.excl_m {
            Some((thc, _)) => thc,
            None => self.th_m_at(i, new),
        }
    }

    /// Classification of a matrix node: (immobile region present, water content subtracted from the matrix water,
    /// immobile water content new/old, kinetic-site kind 0 none / 1 two-site / 2 colloid / 3 dual non-equilibrium).
    fn dp_node_kind(&self, st: &SoluteState, js: usize, i: usize) -> (bool, f64, f64, f64, u8) {
        let m = self.mat[i];
        let l_bact = st.l_bact;
        let stat = st.th_im_m[m];
        let dynamic = self.matrix_dual_por;
        let imm = dynamic || (stat > 0.0 && (!l_bact || st.pm[js][m].omega_im > 0.0));
        let th_sub = if dynamic { 0.0 } else { stat };
        let (new, old) = if dynamic {
            (self.th_im_mat_new[i], self.th_im_mat_old[i])
        } else if imm {
            (stat, stat)
        } else {
            (0.0, 0.0)
        };
        let kin = if l_bact { 2 } else if imm && st.l_dual_neq { 3 } else if !imm && st.frac[m] < 1.0 { 1 } else { 0 };
        (imm, th_sub, new, old, kin)
    }

    /// Total solute mass per unit matrix volume at node `i` (all phases of the matrix domain): mobile liquid, gas,
    /// equilibrium sorption and, depending on the model, kinetic sites / attached colloids / the immobile region.
    pub(crate) fn dp_node_mass(&self, js: usize, i: usize) -> f64 {
        let st = self.sol.as_ref().unwrap();
        let m = self.mat[i];
        let th_new = self.th_m_eff(st, i, true);
        let (imm, th_sub, th_im, _, kin) = self.dp_node_kind(st, js, i);
        let thw = if th_sub > 0.0 { (th_new - th_sub).max(0.001) } else { th_new };
        let ths_m = if self.par_d[m][7] > 0.001 { self.par_d[m][7] } else { self.ths[m] };
        let thg = (ths_m - th_new).max(0.0);
        let temp = self.heat.as_ref().map(|h| h.temp_n[i]).unwrap_or(20.0);
        let sp = self.scaled_params(st, js, m, temp, thw, th_im);
        let (f, ro) = (st.frac[m], st.ro[m]);
        let f_em = if kin == 3 { sp.p.mu_g.clamp(0.0, 1.0) } else { 1.0 };
        let c = st.conc_m[js][i];
        let mut mass = (thw + thg * sp.p.henry) * c + ro * f * f_em * Self::dp_iso(&sp.p, c).0;
        match kin {
            2 => mass += ro * (st.sorb_mk[js][i] + st.sorb_mk2[js][i]),
            1 | 3 => mass += ro * st.sorb_mk[js][i],
            _ => {}
        }
        if imm {
            let cim = st.sorb_mi[js][i];
            mass += th_im * cim + (1.0 - f) * ro * Self::dp_iso(&sp.p, cim).0;
        }
        mass
    }

    /// Solute transport in the matrix domain of the dual-permeability model (Gerke & van Genuchten 1993).
    ///
    /// Conservative finite-volume advection-dispersion equation per unit matrix volume
    ///   d(M_mobile)/dt = d/dz(E dc/dz) - d(q' c)/dz - decay - uptake + Gamma_s/(1-w) + production + transfer
    /// with M_mobile = (theta_w + theta_g H) c + rho f f_em s(c) [+ rho S_kinetic].  The matrix supports the same
    /// processes as the fracture domain:
    ///  * liquid and gas phase (Henry's law, gas diffusion, gas decay/production, temperature dependent Henry
    ///    constant -> velocity correction, volatilisation boundary k_top = -2),
    ///  * equilibrium sorption with an optional kinetic fraction (two-site model, backward Euler),
    ///  * mobile-immobile water: constant (`th_im_matrix`) or transient (`matrix_dual_por`, with the water-transfer term
    ///    FlMacro of SOLUTE.FOR) immobile water, immobile concentration c_im with transfer omega (c - c_im), sorption sites
    ///    (1-f) in contact with the immobile water, backward-Euler Newton solve per node,
    ///  * dual non-equilibrium (lDualNEq): kinetic sites f (1-f_em) in the mobile zone (f_em = mu_g slot, omega_S = gamma_g
    ///    slot, as in the fracture domain),
    ///  * the colloid / bacteria filtration model (two attachment sites, blocking, Deposit), alone, with colloid size
    ///    exclusion (steady flow) or together with an immobile region (transfer coefficient `omega_im`),
    ///  * temperature and water-content dependence of all parameters, decay-chain production.
    /// Exchange with the fracture: Gamma_s = alpha_s (1-w) theta_m (c_f - c_m) + Gamma_w c*  (c* = c_f for Gamma_w >= 0,
    /// else c_m).
    /// Returns the largest scaled change of c_m with respect to the previous call.
    fn sol_dual_perm(&self, st: &mut SoluteState, js: usize, dt: f64) -> f64 {
        let n = self.n;
        let w = self.w_fracture.clamp(0.001, 0.999);
        let wm = 1.0 - w;
        let eps = st.epsi.clamp(0.0, 1.0);
        let x = &self.x;
        let l_bact = st.l_bact;
        let vol: Vec<f64> = (0..n)
            .map(|i| if i == 0 { (x[1] - x[0]) / 2.0 } else if i == n - 1 { (x[n - 1] - x[n - 2]) / 2.0 } else { (x[i + 1] - x[i - 1]) / 2.0 })
            .collect();
        let th_new: Vec<f64> = (0..n).map(|i| self.th_m_eff(st, i, true)).collect();
        let th_old: Vec<f64> = (0..n).map(|i| self.th_m_eff(st, i, false)).collect();
        let temp_n = |i: usize| self.heat.as_ref().map(|h| h.temp_n[i]).unwrap_or(20.0);

        // node classification: immobile region, subtracted water, immobile water content, kinetic sites
        let kinds: Vec<(bool, f64, f64, f64, u8)> = (0..n).map(|i| self.dp_node_kind(st, js, i)).collect();
        let imm: Vec<bool> = kinds.iter().map(|k| k.0).collect();
        let th_sub: Vec<f64> = kinds.iter().map(|k| k.1).collect();
        let th_im_new: Vec<f64> = kinds.iter().map(|k| k.2).collect();
        let th_im_old: Vec<f64> = kinds.iter().map(|k| k.3).collect();
        let kin: Vec<u8> = kinds.iter().map(|k| k.4).collect();
        let dyn_im = self.matrix_dual_por;
        let gam_im: Vec<f64> = (0..n).map(|i| if dyn_im { self.sink_imm[i] } else { 0.0 }).collect();

        let reduce = |th: f64, s: f64| if s > 0.0 { (th - s).max(0.001) } else { th };
        let thw_new: Vec<f64> = (0..n).map(|i| reduce(th_new[i], th_sub[i])).collect();
        let thw_old: Vec<f64> = (0..n).map(|i| reduce(th_old[i], th_sub[i])).collect();
        let ths_m: Vec<f64> = (0..n)
            .map(|i| {
                let m = self.mat[i];
                if self.par_d[m][7] > 0.001 { self.par_d[m][7] } else { self.ths[m] }
            })
            .collect();
        let thg_new: Vec<f64> = (0..n).map(|i| (ths_m[i] - th_new[i]).max(0.0)).collect();
        let thg_old: Vec<f64> = (0..n).map(|i| (ths_m[i] - th_old[i]).max(0.0)).collect();

        let par: Vec<ScaledPar> = (0..n).map(|i| self.scaled_params(st, js, self.mat[i], temp_n(i), thw_new[i], th_im_new[i])).collect();
        let parent: Option<Vec<ScaledPar>> = if js > 0 {
            Some((0..n).map(|i| self.scaled_params(st, js - 1, self.mat[i], temp_n(i), thw_new[i], th_im_new[i])).collect())
        } else {
            None
        };
        let c_parent: Vec<f64> = if js > 0 { st.conc_m[js - 1].clone() } else { Vec::new() };
        let sk_parent: Vec<f64> = if js > 0 { st.sorb_mk[js - 1].clone() } else { Vec::new() };
        let cim_parent: Vec<f64> = if js > 0 { st.sorb_mi[js - 1].clone() } else { Vec::new() };
        let c_m_old = st.conc_m_old.clone();
        let sk1_old = st.sorb_mk_old[js].clone();
        let sk2_old = st.sorb_mk2_old[js].clone();
        let cim_old = st.sorb_mi_old[js].clone();
        let c_f_new = st.conc[js].clone();
        let c_f_old = st.conc_old_step[js].clone();
        let c_prev_iter = st.conc_m[js].clone();
        let frac: Vec<f64> = (0..n).map(|i| st.frac[self.mat[i]]).collect();
        let ro: Vec<f64> = (0..n).map(|i| st.ro[self.mat[i]]).collect();
        // dual non-equilibrium: f_em (SinkG1 slot) and omega_S (SinkG1' slot); equilibrium mobile sites f*f_em,
        // kinetic mobile sites f*(1-f_em)
        let f_em: Vec<f64> = (0..n).map(|i| if kin[i] == 3 { par[i].p.mu_g.clamp(0.0, 1.0) } else { 1.0 }).collect();
        let feq: Vec<f64> = (0..n).map(|i| frac[i] * f_em[i]).collect();
        let fk: Vec<f64> = (0..n).map(|i| match kin[i] { 1 => 1.0 - frac[i], 3 => frac[i] * (1.0 - f_em[i]), _ => 0.0 }).collect();
        // mobile-immobile transfer coefficient (omega_im for the colloid model whose omega slot holds a rate)
        let om_im: Vec<f64> = (0..n).map(|i| if l_bact { par[i].p.omega_im } else { par[i].p.omega }).collect();

        // Darcy flux of the matrix at the faces / nodes; steady-flow colloid exclusion uses the accelerated velocity
        let q_face: Vec<f64> = match st.excl_m {
            Some((_, vc)) => vec![vc; n - 1],
            None => (0..n - 1).map(|f| self.matrix_flux(f, f + 1)).collect(),
        };
        let q_node = |i: usize| -> f64 {
            if i == 0 { q_face[0] } else if i == n - 1 { q_face[n - 2] } else { 0.5 * (q_face[i - 1] + q_face[i]) }
        };
        // attachment rates of the colloid model
        let (mut ka1_v, mut ka2_v) = (vec![0.0; n], vec![0.0; n]);
        if l_bact {
            for i in 0..n {
                let p = &par[i].p;
                if st.l_filtr {
                    let (a, b) = deposit(p.d_c, p.d_p, p.r_ka1, p.r_ka2, thw_new[i], q_node(i), temp_n(i), self.x_conv, self.t_conv);
                    ka1_v[i] = a;
                    ka2_v[i] = b;
                } else {
                    ka1_v[i] = p.r_ka1;
                    ka2_v[i] = p.r_ka2;
                }
            }
        }

        // tortuosity factors
        let taus = |thw: f64, thg: f64, ths: f64| -> (f64, f64) {
            if !st.tort {
                (1.0, 1.0)
            } else if st.tort_model == 0 {
                (thw.powf(7.0 / 3.0) / (ths * ths), thg.powf(7.0 / 3.0) / (ths * ths))
            } else {
                (0.66 * (thw / ths).powf(8.0 / 3.0), thg.powf(1.5) / ths)
            }
        };
        // saturated water content for the tortuosity: reduced by the immobile water, enlarged by the transient one
        let ths_tort = |i: usize| -> f64 {
            if dyn_im {
                ths_m[i] + th_im_new[i]
            } else if th_sub[i] > 0.0 {
                (ths_m[i] - th_sub[i]).max(0.001)
            } else {
                ths_m[i]
            }
        };

        // face coefficients: J_f = a c_i + b c_{i+1} (+x = upward); advection with the velocity correction of a
        // temperature dependent Henry constant
        let mut a_f = vec![0.0; n - 1];
        let mut b_f = vec![0.0; n - 1];
        for f in 0..n - 1 {
            let dx = x[f + 1] - x[f];
            let (i0, i1) = (f, f + 1);
            let thw_f = 0.5 * (thw_new[i0] + thw_new[i1]);
            let thg_f = 0.5 * (thg_new[i0] + thg_new[i1]);
            let m = self.mat[f];
            let (tau_w, tau_g) = taus(thw_f, thg_f, ths_tort(i0));
            let dw = 0.5 * (par[i0].diff_w + par[i1].diff_w);
            let dg = 0.5 * (par[i0].diff_g + par[i1].diff_g);
            let h_f = 0.5 * (par[i0].p.henry + par[i1].p.henry);
            let q = q_face[f];
            let dh_dx = (par[i1].p.henry - par[i0].p.henry) / dx;
            let q_adv = q - thg_f * dg * tau_g * dh_dx;
            let e = st.disp_l[m] * q.abs() + thw_f * tau_w * dw + thg_f * tau_g * dg * h_f;
            let pe = if e > 1e-300 { q_adv.abs() * dx / e } else { f64::INFINITY };
            if pe <= 2.0 {
                a_f[f] = q_adv / 2.0 + e / dx;
                b_f[f] = q_adv / 2.0 - e / dx;
            } else if q_adv >= 0.0 {
                a_f[f] = q_adv + e / dx;
                b_f[f] = -e / dx;
            } else {
                a_f[f] = e / dx;
                b_f[f] = q_adv - e / dx;
            }
        }

        // boundary water fluxes of the matrix domain (per unit matrix area, upward positive)
        let (dx_t, dx_b) = (x[n - 1] - x[n - 2], x[1] - x[0]);
        let (q_t, q_b) = match st.excl_m {
            Some((_, vc)) => (vc, vc),
            None => {
                let vt_m = self.matrix_flux(n - 2, n - 1)
                    - (self.th_matrix_new[n - 1] - self.th_matrix_old[n - 1]) * dx_t / 2.0 / dt
                    - self.sink[n - 1] * dx_t / 2.0
                    - self.sink_imm[n - 1] * dx_t / 2.0;
                let vb_m = self.matrix_flux(0, 1)
                    + (self.th_matrix_new[0] - self.th_matrix_old[0]) * dx_b / 2.0 / dt
                    + self.sink[0] * dx_b / 2.0
                    + self.sink_imm[0] * dx_b / 2.0;
                (vt_m + self.sink_im[n - 1] * dx_t / 2.0 / wm, vb_m - self.sink_im[0] * dx_b / 2.0 / wm)
            }
        };

        let (c_top, c_bot) = (st.c_top[js], st.c_bot[js]);
        let top_dirichlet = st.k_top > 0;
        let bot_dirichlet = st.k_bot > 0;
        let c_root_max = st.c_root_max.get(js).copied().unwrap_or(1e30);
        // volatilisation through the gas boundary layer at the top (k_top = -2)
        let (g_vol, g_atm) = if st.k_top == -2 && st.d_surf > 1e-10 {
            (par[n - 1].diff_g * par[n - 1].p.henry / st.d_surf, par[n - 1].diff_g / st.d_surf)
        } else {
            (0.0, 0.0)
        };

        // kinetic sites: two-site (kind 1), dual non-equilibrium (kind 3) or attached colloids (kind 2).
        // Returns (S1, S2, d(S1+S2)/dc).
        let kin_of = |i: usize, c: f64, s1_0: f64, s2_0: f64, s1_it: f64, s2_it: f64| -> (f64, f64, f64) {
            let p = &par[i].p;
            match kin[i] {
                1 | 3 => {
                    let (s, ds) = Self::dp_iso(p, c);
                    let om = if kin[i] == 3 { p.gam_g } else { p.omega };
                    let ak = 1.0 / (1.0 + dt * (om + p.mu_s + p.gam_s));
                    (ak * (s1_0 + dt * fk[i] * (om * s + p.mu0_s)), 0.0, ak * dt * fk[i] * om * ds)
                }
                2 => {
                    let z = self.depth_of(i);
                    let psi1 = if p.i_psi1 > 0 { blocking(p.i_psi1, p.s_max1, z, s1_it, par[i].dc, p.s_max2) } else { 1.0 };
                    let psi2 = if p.i_psi2 > 0 { blocking(p.i_psi2, p.s_max2, z, s2_it, par[i].dc, p.s_max2) } else { 1.0 };
                    let a1 = 1.0 / (1.0 + dt * (p.r_kd1 + p.mu_s + p.gam_s));
                    let a2 = 1.0 / (1.0 + dt * (p.r_kd2 + p.mu_s + p.gam_s));
                    let k1 = dt * ka1_v[i] * thw_new[i] * psi1 / ro[i];
                    let k2 = dt * ka2_v[i] * thw_new[i] * psi2 / ro[i];
                    (a1 * (s1_0 + k1 * c), a2 * (s2_0 + k2 * c), a1 * k1 + a2 * k2)
                }
                _ => (0.0, 0.0, 0.0),
            }
        };
        // immobile region: backward-Euler balance of the immobile water including the water-transfer term
        //   d(theta_im c_im + (1-f) rho s(c_im))/dt = omega (c - c_im) + FlMacro - decay + production,
        //   FlMacro = Gamma c (Gamma >= 0) or Gamma c_im (Gamma < 0).
        // Returns (c_im, d c_im / d c).
        let im_solve = |i: usize, c: f64, cim_0: f64| -> (f64, f64) {
            let p = &par[i].p;
            let pi_ = &par[i].p_im;
            let (f, r, om, gm) = (frac[i], ro[i], om_im[i], gam_im[i]);
            let (th_n, th_o) = (th_im_new[i], th_im_old[i]);
            let m_old = th_o * cim_0 + (1.0 - f) * r * Self::dp_iso(p, cim_0).0;
            let prod = th_n * p.mu0_w + (1.0 - f) * r * p.mu0_s;
            let mut xi = cim_0;
            let mut dg_dx = th_n + dt * om;
            for _ in 0..30 {
                let (s, ds) = Self::dp_iso(p, xi);
                let dec = th_n * (pi_.mu_w + pi_.gam_w) * xi + (1.0 - f) * r * (pi_.mu_s + pi_.gam_s) * s;
                let fl = if gm >= 0.0 { gm * c } else { gm * xi };
                let g = th_n * xi + (1.0 - f) * r * s - m_old - dt * (om * (c - xi) + fl - dec + prod);
                dg_dx = th_n + (1.0 - f) * r * ds + dt * (om + (-gm).max(0.0) + th_n * (pi_.mu_w + pi_.gam_w) + (1.0 - f) * r * (pi_.mu_s + pi_.gam_s) * ds);
                let dxi = g / dg_dx.max(1e-300);
                xi = (xi - dxi).max(0.0);
                if dxi.abs() < 1e-12 * (1.0 + xi.abs()) {
                    break;
                }
            }
            (xi, dt * (om + gm.max(0.0)) / dg_dx.max(1e-300))
        };
        // mobile source from the immobile region: S_tr(c) = a c_im - b c
        let tr_ab = |i: usize| -> (f64, f64) {
            let (om, gm) = (om_im[i], gam_im[i]);
            (om + (-gm).max(0.0), om + gm.max(0.0))
        };

        struct Coef {
            k_dec: Vec<f64>,    // first-order decay [1/T] per unit matrix volume (positive): mobile liquid, gas, equilibrium sites
            k_upt: Vec<f64>,    // passive root uptake
            k_ex: Vec<f64>,     // implicit exchange coefficient (on c_m)
            s0_prod: Vec<f64>,  // zero-order production
            s0_chain: Vec<f64>, // production from the parent solute
            s0_ex: Vec<f64>,    // exchange source (on c_f)
            s0_upt: Vec<f64>,   // uptake cap correction (c > cRootMax)
        }
        let c_f_w: Vec<f64> = (0..n).map(|i| eps * c_f_new[i] + (1.0 - eps) * c_f_old[i]).collect();
        let build = |cc: &Vec<f64>, thw: &Vec<f64>, thg: &Vec<f64>| -> Coef {
            let z = vec![0.0; n];
            let mut co = Coef { k_dec: z.clone(), k_upt: z.clone(), k_ex: z.clone(), s0_prod: z.clone(), s0_chain: z.clone(), s0_ex: z.clone(), s0_upt: z };
            for i in 0..n {
                let p = &par[i].p;
                let (s, _) = Self::dp_iso(p, cc[i]);
                let r_s = if cc[i] > 1e-12 { s / cc[i] } else { Self::dp_iso(p, 1e-12).0 / 1e-12 };
                // (with dual non-equilibrium the SinkG1 / SinkG1' slots hold f_em and omega_S, not gas decay rates)
                let gas_dec = if kin[i] == 3 { 0.0 } else { (p.mu_g + p.gam_g) * thg[i] * p.henry };
                co.k_dec[i] = (p.mu_w + p.gam_w) * thw[i] + (p.mu_s + p.gam_s) * ro[i] * feq[i] * r_s + gas_dec;
                // production on the mobile/equilibrium compartments; the kinetic and immobile compartments account for
                // their own production and decay
                co.s0_prod[i] = p.mu0_w * thw[i] + ro[i] * feq[i] * p.mu0_s + p.mu0_g * thg[i];
                if let Some(pp) = &parent {
                    let pq = &pp[i].p;
                    let pqi = &pp[i].p_im;
                    let cpar = c_parent[i];
                    let (sp_, _) = Self::dp_iso(pq, cpar);
                    let r_sp = if cpar > 1e-12 { sp_ / cpar } else { Self::dp_iso(pq, 1e-12).0 / 1e-12 };
                    let feq_p = if kin[i] == 3 { frac[i] * pq.mu_g.clamp(0.0, 1.0) } else { frac[i] };
                    let gas_p = if kin[i] == 3 { 0.0 } else { thg[i] * pq.henry * pq.gam_g };
                    let mut cg = cpar * (pq.gam_w * thw[i] + ro[i] * feq_p * pq.gam_s * r_sp + gas_p);
                    if kin[i] == 1 || kin[i] == 3 {
                        cg += ro[i] * pq.gam_s * sk_parent[i];
                    }
                    if imm[i] {
                        let cim_p = cim_parent[i];
                        let (si_, _) = Self::dp_iso(pq, cim_p);
                        let r_si = if cim_p > 1e-12 { si_ / cim_p } else { Self::dp_iso(pq, 1e-12).0 / 1e-12 };
                        cg += cim_p * (th_im_new[i] * pqi.gam_w + (1.0 - frac[i]) * ro[i] * pqi.gam_s * r_si);
                    }
                    co.s0_chain[i] = cg;
                }
                let sk_ = self.sink[i];
                if cc[i] <= c_root_max {
                    co.k_upt[i] = sk_;
                } else {
                    co.s0_upt[i] = -sk_ * c_root_max;
                }
                let gw = self.sink_im[i];
                let a_s = p.alpha_s * th_new[i];
                co.k_ex[i] = a_s + if gw < 0.0 { -gw / wm } else { 0.0 };
                co.s0_ex[i] = (a_s + if gw >= 0.0 { gw / wm } else { 0.0 }) * c_f_w[i];
            }
            co
        };
        // operator F c (flux form, per node, includes volume)
        let apply = |cc: &Vec<f64>, co: &Coef| -> Vec<f64> {
            let mut out = vec![0.0; n];
            for f in 0..n - 1 {
                let j = a_f[f] * cc[f] + b_f[f] * cc[f + 1];
                out[f] -= j;
                out[f + 1] += j;
            }
            if !bot_dirichlet {
                if st.k_bot == -1 && q_b > 0.0 {
                    out[0] += q_b * c_bot;
                } else {
                    out[0] += q_b * cc[0];
                }
            }
            if !top_dirichlet {
                if st.k_top < 0 && q_t < 0.0 {
                    out[n - 1] += -q_t * c_top;
                }
                if st.k_top == -2 {
                    out[n - 1] -= g_vol * cc[n - 1] - g_atm * st.c_atm;
                }
            }
            for i in 0..n {
                out[i] += vol[i]
                    * (-(co.k_dec[i] + co.k_upt[i] + co.k_ex[i]) * cc[i] + co.s0_prod[i] + co.s0_chain[i] + co.s0_ex[i] + co.s0_upt[i]);
            }
            out
        };
        let operator_tri = |co: &Coef| -> (Vec<f64>, Vec<f64>, Vec<f64>) {
            let (mut lo, mut di, mut up) = (vec![0.0; n], vec![0.0; n], vec![0.0; n]);
            for f in 0..n - 1 {
                di[f] -= a_f[f];
                up[f] -= b_f[f];
                lo[f + 1] += a_f[f];
                di[f + 1] += b_f[f];
            }
            if !bot_dirichlet && !(st.k_bot == -1 && q_b > 0.0) {
                di[0] += q_b;
            }
            if !top_dirichlet && st.k_top == -2 {
                di[n - 1] -= g_vol;
            }
            for i in 0..n {
                di[i] -= vol[i] * (co.k_dec[i] + co.k_upt[i] + co.k_ex[i]);
            }
            (lo, di, up)
        };

        // stored mass of the mobile balance at the old level: (theta_w + theta_g H) c + rho feq s(c) + rho (S1 + S2)
        let kin_sum = |i: usize, s1: f64, s2: f64| if kin[i] != 0 { s1 + s2 } else { 0.0 };
        let m_old: Vec<f64> = (0..n)
            .map(|i| {
                let p = &par[i].p;
                (thw_old[i] + thg_old[i] * p.henry) * c_m_old[i] + ro[i] * (feq[i] * Self::dp_iso(p, c_m_old[i]).0 + kin_sum(i, sk1_old[i], sk2_old[i]))
            })
            .collect();
        let co_old = build(&c_m_old, &thw_old, &thg_old);
        let f_old = apply(&c_m_old, &co_old);

        let mut c = c_prev_iter.clone();
        let (mut s1, mut s2) = (sk1_old.clone(), sk2_old.clone());
        let mut cim = cim_old.clone();
        for _picard in 0..30 {
            // kinetic sites / immobile region at the current iterate (blocking lagged)
            let mut dsk = vec![0.0; n];
            let mut k_mi = vec![0.0; n];
            let mut tr_const = vec![0.0; n];
            for i in 0..n {
                if kin[i] != 0 {
                    let (a, b, d) = kin_of(i, c[i], sk1_old[i], sk2_old[i], s1[i], s2[i]);
                    s1[i] = a;
                    s2[i] = b;
                    dsk[i] = d;
                }
                if imm[i] {
                    let (ci, dci) = im_solve(i, c[i], cim_old[i]);
                    cim[i] = ci;
                    let (a, b) = tr_ab(i);
                    k_mi[i] = b - a * dci;
                    tr_const[i] = a * ci - b * c[i] + k_mi[i] * c[i];
                }
            }
            let co = build(&c, &thw_new, &thg_new);
            let (lo, di, up) = operator_tri(&co);
            let mut sub = vec![0.0; n];
            let mut dd = vec![0.0; n];
            let mut sup = vec![0.0; n];
            let mut rhs = vec![0.0; n];
            let f_const = apply(&vec![0.0; n], &co);
            for i in 0..n {
                let p = &par[i].p;
                let (s_k, ds_k) = Self::dp_iso(p, c[i]);
                let h_fac = thw_new[i] + thg_new[i] * p.henry;
                let mk = h_fac * c[i] + ro[i] * (feq[i] * s_k + kin_sum(i, s1[i], s2[i]));
                let mp = h_fac + ro[i] * (feq[i] * ds_k + dsk[i]);
                sub[i] = -eps * lo[i];
                dd[i] = vol[i] * (mp / dt + k_mi[i]) - eps * di[i];
                sup[i] = -eps * up[i];
                rhs[i] = vol[i] * ((mp * c[i] - mk + m_old[i]) / dt + tr_const[i]) + eps * f_const[i] + (1.0 - eps) * f_old[i];
            }
            if bot_dirichlet {
                sub[0] = 0.0;
                dd[0] = 1.0;
                sup[0] = 0.0;
                rhs[0] = c_bot;
            }
            if top_dirichlet {
                sub[n - 1] = 0.0;
                dd[n - 1] = 1.0;
                sup[n - 1] = 0.0;
                rhs[n - 1] = c_top;
            }
            sub[0] = 0.0;
            sup[n - 1] = 0.0;
            thomas(&sub, &mut dd, &sup, &mut rhs);
            let mut dmax = 0.0f64;
            for i in 0..n {
                let cn = rhs[i].max(0.0);
                dmax = dmax.max((cn - c[i]).abs() / (1e-12 + st.tol_a + st.tol_r * cn.abs().max(c[i].abs())));
                c[i] = cn;
            }
            if dmax < 1.0 {
                break;
            }
        }
        // final states at the converged concentration
        let mut tr_flux = vec![0.0; n]; // net transfer from the immobile region into the mobile water
        for i in 0..n {
            if kin[i] != 0 {
                let (a, b, _) = kin_of(i, c[i], sk1_old[i], sk2_old[i], s1[i], s2[i]);
                s1[i] = a;
                s2[i] = b;
            }
            if imm[i] {
                cim[i] = im_solve(i, c[i], cim_old[i]).0;
                let (a, b) = tr_ab(i);
                tr_flux[i] = a * cim[i] - b * c[i];
            }
        }
        let co = build(&c, &thw_new, &thg_new);

        // ---- boundary fluxes (+x) and reaction terms for the balance
        let f_new = apply(&c, &co);
        let store_dm: Vec<f64> = (0..n)
            .map(|i| {
                let p = &par[i].p;
                vol[i] * (((thw_new[i] + thg_new[i] * p.henry) * c[i] + ro[i] * (feq[i] * Self::dp_iso(p, c[i]).0 + kin_sum(i, s1[i], s2[i]))) - m_old[i]) / dt
            })
            .collect();
        // net mass created inside the kinetic sites during the step: zero-order production minus decay
        let kin_net: Vec<f64> = (0..n)
            .map(|i| {
                let p = &par[i].p;
                let prod_k = if kin[i] == 1 || kin[i] == 3 { fk[i] * p.mu0_s } else { 0.0 };
                vol[i] * ro[i] * (prod_k - (p.mu_s + p.gam_s) * kin_sum(i, s1[i], s2[i]))
            })
            .collect();
        let ncell_avg = |i: usize| eps * f_new[i] + (1.0 - eps) * f_old[i] + kin_net[i] + vol[i] * tr_flux[i];
        let vol_avg_c = |i: usize| eps * c[i] + (1.0 - eps) * c_m_old[i];
        let top_vol = if st.k_top == -2 { g_vol * vol_avg_c(n - 1) - g_atm * st.c_atm } else { 0.0 };
        let top_in = if st.k_top < 0 && q_t < 0.0 { q_t * c_top } else { 0.0 };
        let j_top = if top_dirichlet { ncell_avg(n - 1) - store_dm[n - 1] } else { top_in + top_vol };
        let j_bot = if bot_dirichlet {
            store_dm[0] - ncell_avg(0)
        } else if st.k_bot == -1 && q_b > 0.0 {
            q_b * c_bot
        } else {
            q_b * vol_avg_c(0)
        };
        let (mut ch0, mut ch1, mut chr) = (0.0, 0.0, 0.0);
        for i in 0..n {
            let p = &par[i].p;
            ch0 += vol[i] * (eps * (co.s0_prod[i] + co.s0_chain[i]) + (1.0 - eps) * (co_old.s0_prod[i] + co_old.s0_chain[i]));
            ch1 -= vol[i] * (eps * co.k_dec[i] * c[i] + (1.0 - eps) * co_old.k_dec[i] * c_m_old[i]);
            chr += vol[i] * (eps * (co.k_upt[i] * c[i] - co.s0_upt[i]) + (1.0 - eps) * (co_old.k_upt[i] * c_m_old[i] - co_old.s0_upt[i]));
            if kin[i] == 1 || kin[i] == 3 {
                ch0 += vol[i] * ro[i] * fk[i] * p.mu0_s;
            }
            if kin[i] != 0 {
                ch1 -= vol[i] * ro[i] * (p.mu_s + p.gam_s) * kin_sum(i, s1[i], s2[i]);
            }
            if imm[i] {
                let pi_ = &par[i].p_im;
                let (f, th_i, r) = (frac[i], th_im_new[i], ro[i]);
                ch0 += vol[i] * (th_i * p.mu0_w + (1.0 - f) * r * p.mu0_s);
                ch1 -= vol[i] * (th_i * (pi_.mu_w + pi_.gam_w) * cim[i] + (1.0 - f) * r * (pi_.mu_s + pi_.gam_s) * Self::dp_iso(p, cim[i]).0);
            }
        }
        st.cvm_top[js] = j_top;
        st.cvm_bot[js] = j_bot;
        st.cvm_ch0[js] = ch0;
        st.cvm_ch1[js] = ch1;
        st.cvm_chr[js] = chr;

        // exchange rate per total volume (reporting) and state update
        let mut change = 0.0f64;
        for i in 0..n {
            let p = &par[i].p;
            let gw = self.sink_im[i];
            let cmw = eps * c[i] + (1.0 - eps) * c_m_old[i];
            let c_star = if gw >= 0.0 { c_f_w[i] } else { cmw };
            st.gam_s_dp[i] = p.alpha_s * wm * th_new[i] * (c_f_w[i] - cmw) + gw * c_star;
            change = change.max((c[i] - c_prev_iter[i]).abs() / (1e-12 + st.tol_a + st.tol_r * c[i].abs()));
            st.conc_m[js][i] = c[i];
            st.sorb_mk[js][i] = s1[i];
            st.sorb_mk2[js][i] = s2[i];
            st.sorb_mi[js][i] = if imm[i] { cim[i] } else { 0.0 };
            // reported sorbed concentration: immobile concentration for an immobile region, else the sorbed phase
            st.sorb_m[js][i] = if imm[i] { cim[i] } else { feq[i] * Self::dp_iso(p, c[i]).0 + s1[i] + s2[i] };
        }
        change
    }

    /// Cumulative boundary fluxes (TLInf, solute part).
    pub fn solute_cumulate(&mut self, run_off: f64) -> Vec<SoluteTLevel> {
        let dt = self.dt;
        let n = self.n;
        let dual_w = if self.l_dual_perm { Some(self.w_fracture) } else { None };
        let (h_save, x_save) = (self.h_new.clone(), self.x.clone());
        let st = self.sol.as_mut().unwrap();
        let mut out = vec![];
        if let Some(w) = dual_w {
            // total fluxes per unit soil area: w * fracture + (1 - w) * matrix
            for j in 0..st.ns {
                st.cv_top[j] = w * st.cv_top[j] + (1.0 - w) * st.cvm_top[j];
                st.cv_bot[j] = w * st.cv_bot[j] + (1.0 - w) * st.cvm_bot[j];
                st.cv_ch0[j] = w * st.cv_ch0[j] + (1.0 - w) * st.cvm_ch0[j];
                st.cv_ch1[j] = w * st.cv_ch1[j] + (1.0 - w) * st.cvm_ch1[j];
                st.cv_chr[j] = w * st.cv_chr[j] + (1.0 - w) * st.cvm_chr[j];
            }
        }
        for j in 0..st.ns {
            st.cum_ch[j][0] -= st.cv_top[j] * dt;
            st.cum_ch[j][1] += st.cv_bot[j] * dt;
            st.cum_ch[j][2] += st.cv_ch0[j] * dt;
            st.cum_ch[j][3] += st.cv_ch1[j] * dt;
            st.cum_ch[j][4] += st.cv_chr[j] * dt;
            st.cum_ch[j][5] += st.cv_chim[j] * dt;
            st.c_cum_t[j] += (st.cv_top[j] - st.cv_bot[j] - st.cv_ch0[j] - st.cv_ch1[j] + st.cv_chr[j]) * dt;
            st.c_cum_a[j] += (st.cv_bot[j].abs() + st.cv_top[j].abs() + st.cv_ch0[j].abs() + st.cv_ch1[j].abs() + st.cv_chr[j].abs()) * dt;
            let c_run_off = run_off * st.c_top[j];
            st.cum_ch[j][9] += c_run_off * dt;
            // average concentration of the groundwater layer: contiguous saturated elements above the bottom
            let (mut c_gwl, mut d_gwl) = (0.0, 0.0);
            for i in 0..n - 1 {
                if h_save[i + 1] > 0.0 {
                    let dx = x_save[i + 1] - x_save[i];
                    c_gwl += (st.conc[j][i] + st.conc[j][i + 1]) / 2.0 * dx;
                    d_gwl += dx;
                } else {
                    break;
                }
            }
            if d_gwl > 0.0 {
                c_gwl /= d_gwl;
            }
            out.push(SoluteTLevel {
                cv_top: -st.cv_top[j],
                cv_bot: st.cv_bot[j],
                cum_top: st.cum_ch[j][0],
                cum_bot: st.cum_ch[j][1],
                cum_ch0: st.cum_ch[j][2],
                cum_ch1: st.cum_ch[j][3],
                c_top: st.conc[j][n - 1],
                c_root: st.c_root[j],
                c_bot: st.conc[j][0],
                cv_root: st.cv_chr[j],
                cum_root: st.cum_ch[j][4],
                cum_neq: st.cum_ch[j][5],
                c_gwl,
                c_run_off,
                cum_run_off: st.cum_ch[j][9],
            });
        }
        out
    }

    /// Solute mass in sub-regions and solute mass balance (SubReg, solute part).
    pub fn sub_reg_solute(&mut self, p_level: usize, rec: &mut BalanceOut) {
        let n = self.n;
        let dual_por = self.i_dual_por > 0;
        let temp_at = |i: usize| self.heat.as_ref().map(|h| h.temp_n[i]).unwrap_or(20.0);
        let mass_matrix: Vec<Vec<f64>> = if self.l_dual_perm {
            (0..self.sol.as_ref().unwrap().ns).map(|js| (0..n).map(|i| self.dp_node_mass(js, i)).collect()).collect()
        } else {
            Vec::new()
        };
        let st = self.sol.as_mut().unwrap();
        let n_lay = rec.sub.len();
        let ns = st.ns;
        let (l_equil, l_bact, dual_neq) = (st.l_equil, st.l_bact, st.l_dual_neq);
        for s in rec.sub.iter_mut() {
            s.c_vol = vec![0.0; ns];
            s.c_mean = vec![0.0; ns];
            s.c_vol_im = vec![0.0; ns];
            s.c_mean_im = vec![0.0; ns];
        }
        rec.total.c_vol = vec![0.0; ns];
        rec.total.c_mean = vec![0.0; ns];
        rec.total.c_vol_im = vec![0.0; ns];
        rec.total.c_mean_im = vec![0.0; ns];
        rec.sol_bal_t = vec![0.0; ns];
        rec.sol_bal_r = vec![0.0; ns];

        let w_f = if self.l_dual_perm { self.w_fracture } else { 1.0 };
        let w_m = (1.0 - w_f).max(0.0);

        // DEVIATION FROM THE 2009 SOURCE, confirmed by HYDRUS 4.17: the 2009 SubReg accumulates DeltC with the concentration of
        // solute 1 only (`if(jS.eq.1) cEl=cNewi`) and uses it for every solute; 4.17 uses the change of each solute's own mass
        // (TEST3 CncBalR2: 0.252 % at t = 50 and 0.125 % at t = 100; with the solute-1 DeltC 0.055 % / 0.062 %).
        for js in 0..ns {
            let (mut vol, mut vol_im, mut vol_im2) = (0.0, 0.0, 0.0);
            let (mut delt_c, mut c_tot, mut c_tot_im, mut a_tot) = (0.0, 0.0, 0.0, 0.0);
            let mut sub_v = vec![0.0; n_lay];
            let mut sub_vim = vec![0.0; n_lay];
            let mut sub_c = vec![0.0; n_lay];
            let mut sub_cim = vec![0.0; n_lay];
            let mut sub_a = vec![0.0; n_lay];

            for i in (0..n - 1).rev() {
                let j = i + 1;
                let (mi, mj) = (self.mat[i], self.mat[j]);
                let lay = self.lay[i] - 1;
                let dx = self.x[j] - self.x[i];
                let tt_mean = (temp_at(i) + temp_at(j)) / 2.0 + 273.15;
                let tt = |t: f64| (t + 273.15 - 293.15) / 8.314 / tt_mean / 293.15;
                // temperature-adjusted Ks, Nu, Beta, Henry at both nodes (the Fortran divides by the mean temperature)
                let adj = |idx: usize, m: usize| -> (f64, f64, f64, f64) {
                    let b = &st.pm[js][m];
                    if st.l_tdep && js < st.t_dep.len() {
                        let (td, f) = (&st.t_dep[js], tt(temp_at(idx)));
                        (b.ks * (td.ks * f).exp(), b.nu * (td.nu * f).exp(), b.beta, b.henry * (td.henry * f).exp())
                    } else {
                        (b.ks, b.nu, b.beta, b.henry)
                    }
                };
                let (ks_i, nu_i, fe_i, he_i) = adj(i, mi);
                let (ks_j, nu_j, fe_j, he_j) = adj(j, mj);
                let (ci, cj) = (st.conc[js][i], st.conc[js][j]);
                let (mut c1, mut c2) = (1.0, 1.0);
                if !st.l_linear[js] {
                    if ci > 0.0 {
                        c1 = ci.powf(fe_i - 1.0) / (1.0 + nu_i * ci.powf(fe_i));
                    }
                    if cj > 0.0 {
                        c2 = cj.powf(fe_j - 1.0) / (1.0 + nu_j * cj.powf(fe_j));
                    }
                }
                let (mut thw_i, mut thw_j) = (self.th_new[i], self.th_new[j]);
                if l_bact {
                    thw_i = st.th_n[i];
                    thw_j = st.th_n[j];
                }
                let thg_i = (self.ths[mi] - thw_i).max(0.0);
                let thg_j = (self.ths[mj] - thw_j).max(0.0);
                let (thim_i, thim_j) = if dual_por { (self.th_new_im[i], self.th_new_im[j]) } else { (st.th_im[mi], st.th_im[mj]) };
                if (st.l_mob_im[mi] && !dual_por) || l_bact {
                    thw_i = (thw_i - thim_i).max(0.001);
                }
                if (st.l_mob_im[mj] && !dual_por) || l_bact {
                    thw_j = (thw_j - thim_j).max(0.001);
                }
                // f_em of material i is used for both nodes of the element (as in SubReg)
                let f_em = if dual_neq { st.pm[js][mi].mu_g } else { 1.0 };
                let (ro_i, ro_j, fr_i, fr_j) = (st.ro[mi], st.ro[mj], st.frac[mi], st.frac[mj]);
                let cnew_f = dx / 2.0
                    * (ci * w_f * (thw_i + f_em * fr_i * ro_i * ks_i * c1 + thg_i * he_i)
                        + cj * w_f * (thw_j + f_em * fr_j * ro_j * ks_j * c2 + thg_j * he_j));
                vol += cnew_f;
                sub_v[lay] += cnew_f;

                // matrix continuum (dual permeability, no Fortran counterpart)
                if self.l_dual_perm {
                    let mass_m = dx / 2.0 * w_m * (mass_matrix[js][i] + mass_matrix[js][j]);
                    vol += mass_m;
                    sub_v[lay] += mass_m;
                }

                if !l_equil {
                    let (si, sj) = (st.sorb[js][i], st.sorb[js][j]);
                    let c_im;
                    if (st.l_mob_im[mi] || dual_por) && !l_bact || (st.l_mob_im[mi] || dual_por) && dual_neq {
                        let (mut s1, mut s2) = (1.0, 1.0);
                        if !st.l_linear[js] {
                            if si > 0.0 {
                                s1 = si.powf(fe_i - 1.0) / (1.0 + nu_i * si.powf(fe_i));
                            }
                            if sj > 0.0 {
                                s2 = sj.powf(fe_j - 1.0) / (1.0 + nu_j * sj.powf(fe_j));
                            }
                        }
                        c_im = dx / 2.0
                            * (si * (thim_i + (1.0 - fr_i) * ro_i * ks_i * s1) + sj * (thim_j + (1.0 - fr_j) * ro_j * ks_j * s2));
                        if dual_neq {
                            vol_im2 += w_f * dx / 2.0 * (ro_i * st.sorb2[js][i] + ro_j * st.sorb2[js][j]);
                        }
                    } else {
                        // two-site sorption model
                        c_im = dx / 2.0 * (ro_i * si + ro_j * sj);
                        if l_bact {
                            vol_im2 += w_f * dx / 2.0 * (ro_i * st.sorb2[js][i] + ro_j * st.sorb2[js][j]);
                        }
                    }
                    vol_im += c_im * w_f;
                    let c_e_im = (si + sj) / 2.0;
                    sub_vim[lay] += w_f * (c_im + if dual_neq || l_bact { dx / 2.0 * (ro_i * st.sorb2[js][i] + ro_j * st.sorb2[js][j]) } else { 0.0 });
                    sub_cim[lay] += c_e_im * dx;
                    c_tot_im += c_e_im * dx;
                }

                let ce = (ci + cj) / 2.0;
                c_tot += ce * dx;
                a_tot += dx;
                sub_c[lay] += ce * dx;
                sub_a[lay] += dx;

                if p_level == 0 {
                    st.sol_in[js][i] = cnew_f;
                } else {
                    delt_c += (st.sol_in[js][i] - cnew_f).abs();
                }
            }

            for l in 0..n_lay {
                rec.sub[l].c_vol[js] = sub_v[l];
                rec.sub[l].c_vol_im[js] = sub_vim[l];
                if sub_a[l] > 0.0 {
                    rec.sub[l].c_mean[js] = sub_c[l] / sub_a[l];
                    rec.sub[l].c_mean_im[js] = sub_cim[l] / sub_a[l];
                }
            }
            rec.total.c_vol[js] = vol;
            rec.total.c_vol_im[js] = vol_im + vol_im2;
            if a_tot > 0.0 {
                rec.total.c_mean[js] = c_tot / a_tot;
                rec.total.c_mean_im[js] = c_tot_im / a_tot;
            }

            // mass balance: mobile + immobile + second non-equilibrium phase
            let total = vol + vol_im + vol_im2;
            if p_level == 0 {
                st.c_vol_i[js] = total;
            } else {
                let bal_t = total - st.c_vol_i[js] + st.c_cum_t[js];
                rec.sol_bal_t[js] = bal_t;
                let cc = delt_c.max(st.c_cum_a[js]);
                if cc > 1e-25 {
                    rec.sol_bal_r[js] = bal_t.abs() / cc * 100.0;
                }
            }
        }
    }

    /// Flux concentration of a species (port of FluxConc in SOLUTE.FOR).
    pub fn flux_conc(&self, js: usize) -> Vec<f64> {
        let n = self.n;
        let mut conc_f = vec![0.0; n];
        let Some(st) = self.sol.as_ref() else {
            return conc_f;
        };
        let dual_por = self.i_dual_por > 0;
        for i in 0..n {
            let m = self.mat[i];
            let mut th_w = st.th_n[i];
            let th_g = (self.ths[m] - th_w).max(0.0);
            let mut th_imob = 0.0;
            if st.l_mob_im[m] {
                if !dual_por {
                    th_imob = st.th_im[m];
                    th_w = (th_w - th_imob).max(0.001);
                } else {
                    th_imob = self.th_new_im[i];
                }
            }
            let temp = self.heat.as_ref().map(|h| h.temp_n[i]).unwrap_or(20.0);
            let sp = self.scaled_params(st, js, m, temp, th_w, th_imob);
            let (dw, dg, henry) = (sp.diff_w, sp.dc, sp.p.henry);
            let (tau_w, tau_g) = if st.tort {
                let mut ths = self.ths[m];
                if st.l_mob_im[m] && !dual_por {
                    ths = (ths - th_imob).max(0.001);
                }
                if dual_por {
                    ths = self.ths[m] + th_imob;
                }
                (th_w.powf(7.0 / 3.0) / ths.powi(2), th_g.powf(7.0 / 3.0) / ths.powi(2))
            } else {
                (1.0, 1.0)
            };
            let q_w = st.v_n[i];
            let disp = st.disp_l[m] * q_w.abs() / th_w + dw * tau_w + th_g / th_w * dg * henry * tau_g;
            let c_grad = if i == 0 {
                (st.conc[js][1] - st.conc[js][0]) / (self.x[1] - self.x[0])
            } else if i == n - 1 {
                (st.conc[js][n - 1] - st.conc[js][n - 2]) / (self.x[n - 1] - self.x[n - 2])
            } else {
                (st.conc[js][i + 1] - st.conc[js][i - 1]) / (self.x[i + 1] - self.x[i - 1])
            };
            let mut cf = st.conc[js][i];
            if q_w != 0.0 {
                cf -= disp * th_w / q_w * c_grad;
            }
            conc_f[i] = cf;
        }
        conc_f
    }
}

/// Calculates blocking factor psi for attachment/straining (port of Blocking in SOLUTE.FOR).
pub fn blocking(i_psi: i32, s_max: f64, x_depth: f64, ss: f64, d_c: f64, s_max2: f64) -> f64 {
	let mut psi = 1.0;
	match i_psi {
		1 => {
			// Langmuirian blocking
			if s_max > 0.0 {
				psi = (1.0 - ss / s_max).max(0.0);
			}
		}
		2 => {
			// Ripening
			if s_max > 0.0 {
				psi = (ss.powf(s_max)).max(psi);
			}
		}
		3 => {
			// Random Sequential Adsorption (RSA) - SOLUTE.FOR line 1032
			if s_max > 0.0 {
				let b_inf = 1.0 / s_max;
				let s_inf = 0.546;
				let m_inf = d_c;
				let const_val = s_inf * b_inf * ss;
				if ss <= 0.8 * s_max {
					psi = 1.0 - 4.0 * const_val + 3.08 * const_val.powi(2) + 1.4069 * const_val.powi(3);
				} else {
					let num = (1.0 - b_inf * ss).max(0.0).powi(3);
					psi = num / (2.0 * m_inf.powi(2) * b_inf.powi(3));
				}
				psi = psi.max(0.0);
			}
		}
		4 => {
			// Depth-dependent straining
			if s_max > 0.0 && d_c > 0.0 {
				psi = ((x_depth.abs() + d_c) / d_c).powf(-s_max);
			}
		}
		5 => {
			// Combined straining and Langmuirian blocking
			if s_max > 0.0 && d_c > 0.0 {
				psi = ((x_depth.abs() + d_c) / d_c).powf(-s_max);
			}
			if s_max2 > 0.0 {
				psi *= (1.0 - ss / s_max2).max(0.0);
			}
		}
		_ => {}
	}
	psi
}

/// Single-collector clean-bed filtration deposition rate (port of Deposit in SOLUTE.FOR).
pub fn deposit(
	d_c1: f64,
	d_p1: f64,
	alfa1: f64,
	alfa2: f64,
	theta: f64,
	q: f64,
	temp_c: f64,
	x_conv: f64,
	t_conv: f64,
) -> (f64, f64) {
	let d_c = d_c1 / x_conv;
	let d_p = d_p1 / x_conv;
	if d_p <= 0.0 && d_c <= 0.0 {
		return (0.0, 0.0);
	}
	let pi = 3.1415; // as in SOLUTE.FOR
	let mu = 0.00093; // Pa·s
	let bk = 1.38048e-23; // J/K
	let h_hamaker = 1e-20; // J
	let g = 9.81; // m/s^2
	let ro_p = 1080.0; // kg/m^3
	let ro_f = 998.0; // kg/m^3

	let veloc = (q / x_conv * t_conv).abs();
	let p_veloc = if theta > 1e-6 { veloc / theta } else { 0.0 };

	let eta = if veloc > 0.0 {
		let gamma = (1.0 - theta).max(0.0).powf(1.0 / 3.0);
		let as_corr = 2.0 * (1.0 - gamma.powi(5)) / (2.0 - 3.0 * gamma + 3.0 * gamma.powi(5) - 2.0 * gamma.powi(6));

		let n_pe = 3.0 * pi * mu * d_p * d_c * veloc / (bk * (temp_c + 273.15));
		let e_diff = 4.0 * as_corr.powf(1.0 / 3.0) * n_pe.powf(-2.0 / 3.0);

		let n_lo = 4.0 * h_hamaker / (9.0 * pi * mu * d_p.powi(2) * veloc);
		let n_r = d_p / d_c;
		let e_inter = as_corr * n_lo.powf(1.0 / 8.0) * n_r.powf(15.0 / 8.0);

		let n_g = g * (ro_p - ro_f) * d_p.powi(2) / (18.0 * mu * veloc);
		let e_grav = 0.00338 * as_corr * n_g.powf(1.2) * n_r.powf(-0.4);

		e_diff + e_inter + e_grav
	} else {
		0.0
	};

	let ka1 = 3.0 * (1.0 - theta) / (2.0 * d_c) * eta * alfa1 * p_veloc / t_conv;
	let ka2 = 3.0 * (1.0 - theta) / (2.0 * d_c) * eta * alfa2 * p_veloc / t_conv;
	(ka1, ka2)
}
/// Calculates accessible water content and accelerated colloid velocity via Burdine model
/// (port of subroutine Exclusion in SOLUTE.FOR).
pub fn exclusion(
    theta: f64,
    v: f64,
    th_c_param: f64,
    qr: f64,
    qs: f64,
    _alpha: f64,
    n: f64,
    l_param: f64,
) -> (f64, f64) {
    if th_c_param.abs() < 1e-20 || qs <= 0.0 {
        return (theta, v);
    }

    let swr = (qr / qs).clamp(0.0, 1.0);
    let sw_c = (th_c_param / qs).clamp(0.0, 1.0);

    // Accessible water content to colloid
    let th_c = (theta - qs * sw_c).max(0.001);

    let sw = (theta / qs).clamp(0.0, 1.0);
    let sw_eff = ((sw - swr) / (1.0 - swr)).clamp(0.0, 1.0);
    let sw_c_eff = ((sw_c - swr) / (1.0 - swr)).clamp(0.0, 1.0);

    let vgn1 = n;
    let vgm1 = 1.0 - 1.0 / vgn1;
    let vgn2 = vgn1 + 1.0;
    let vgm2 = 1.0 - 2.0 / vgn2;

    // Colloid relative permeability according to Burdine model
    let kr_c = if sw_eff > sw_c_eff && vgm2 > 0.0 {
        let term1 = (1.0 - sw_c_eff.powf(1.0 / vgm2)).max(0.0).powf(vgm2);
        let term2 = (1.0 - sw_eff.powf(1.0 / vgm2)).max(0.0).powf(vgm2);
        sw_eff.powi(2) * (term1 - term2)
    } else {
        0.0
    };

    // Mualem water relative permeability
    let vel_c = if sw_eff > 0.0 && vgm1 > 0.0 {
        let term_w = 1.0 - (1.0 - sw_eff.powf(1.0 / vgm1)).max(0.0).powf(vgm1);
        let krw = sw_eff.powf(l_param) * term_w.powi(2);
        if krw > 1e-30 {
            v * kr_c / krw
        } else {
            v
        }
    } else {
        v
    };

    (th_c, vel_c)
}

/// Relative reaction rate as a function of the water content (function rMD of SOLUTE.FOR, iMoistDep = 2).
/// `jreact` = 1..13.
pub fn rmd_table(tab: &MoistDepTable, jreact: usize, theta: f64) -> f64 {
    let Some(p) = tab.react.get(jreact.wrapping_sub(1)) else {
        return 1.0;
    };
    let (rm0, t0, t1, t2, t3, rm1) = (p[0], p[1], p[2], p[3], p[4], p[5]);
    if t2.abs() < 0.001 {
        return 1.0;
    }
    if theta <= t0 {
        rm0
    } else if theta <= t1 {
        rm0 + (theta - t0) / (t1 - t0) * (1.0 - rm0)
    } else if theta <= t2 {
        1.0
    } else if theta <= t3 {
        rm1 + (theta - t3) / (t2 - t3) * (1.0 - rm1)
    } else {
        rm1
    }
}
