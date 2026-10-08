//! Collection of output records (TLInf, NodOut, ObsNod, SubReg).

use crate::output::*;
use crate::sim::Simulation;

impl Simulation {
    /// Water-flow information for the current time level; also updates cumulative fluxes.
    pub fn tl_inf(&mut self, j_print: bool) {
        let n = self.n;
        let m = n - 1;
        let grav = self.cos_alf;
        let dxn = self.x[m] - self.x[m - 1];
        let dt = self.dt;
        let v_t = -(self.con[m] + self.con[m - 1]) / 2.0 * ((self.h_new[m] - self.h_new[m - 1]) / dxn + grav)
            - (self.th_new[m] - self.th_old[m]) * dxn / 2.0 / dt
            - self.sink[m] * dxn / 2.0;
        let dx1 = self.x[1] - self.x[0];
        let mut v_b = -(self.con[0] + self.con[1]) / 2.0 * ((self.h_new[1] - self.h_new[0]) / dx1 + grav)
            + (self.th_new[0] - self.th_old[0]) * dx1 / 2.0 / dt
            + self.sink[0] * dx1 / 2.0;

        let mut v_t = v_t;
        if self.i_dual_por > 0 {
            v_t -= self.sink_im[m] * dxn / 2.0;
        }
        if self.l_dual_perm {
            // total flux = w * (fracture flux) + (1 - w) * (matrix flux); the exchange terms cancel
            let w = self.w_fracture;
            let (dxm, dx0) = (dxn, self.x[1] - self.x[0]);
            let vt_m = self.matrix_flux(m - 1, m)
                - (self.th_matrix_new[m] - self.th_matrix_old[m]) * dxm / 2.0 / dt
                - self.sink[m] * dxm / 2.0
                - self.sink_imm[m] * dxm / 2.0;
            let vb_m = self.matrix_flux(0, 1)
                + (self.th_matrix_new[0] - self.th_matrix_old[0]) * dx0 / 2.0 / dt
                + self.sink[0] * dx0 / 2.0
                + self.sink_imm[0] * dx0 / 2.0;
            let gamma_t = self.sink_im[m] * dxm / 2.0;
            let gamma_b = self.sink_im[0] * dx0 / 2.0;
            v_t = w * (v_t - gamma_t / w) + (1.0 - w) * (vt_m + gamma_t / (1.0 - w));
            v_b = w * (v_b + gamma_b / w) + (1.0 - w) * (vb_m - gamma_b / (1.0 - w));
        }
        if self.prj.water.l_w_dep {
            v_t -= (self.con_lt[m] + self.con_lt[m - 1]) / 2.0 * (self.temp(m) - self.temp(m - 1)) / dxn;
            v_b -= (self.con_lt[0] + self.con_lt[1]) / 2.0 * (self.temp(1) - self.temp(0)) / dx1;
        }
        let v_top_w = v_t; // vTopW: liquid flux before the vapor terms
        if self.l_vapor {
            v_t -= (self.con_vh[m] + self.con_vh[m - 1]) / 2.0 * (self.h_new[m] - self.h_new[m - 1]) / dxn
                + (self.con_vt[m] + self.con_vt[m - 1]) / 2.0 * (self.temp(m) - self.temp(m - 1)) / dxn
                + (self.th_v_new[m] - self.th_v_old[m]) * dxn / 2.0 / dt;
            v_b -= (self.con_vh[0] + self.con_vh[1]) / 2.0 * (self.h_new[1] - self.h_new[0]) / dx1
                + (self.con_vt[0] + self.con_vt[1]) / 2.0 * (self.temp(1) - self.temp(0)) / dx1
                - (self.th_v_new[0] - self.th_v_old[0]) * dx1 / 2.0 / dt;
        }

        let v_top_v = v_t - v_top_w; // vTopV: vapor flux
        self.v_top = v_t;
        self.v_bot = v_b;
        let mut run_off = 0.0;
        // DEVIATION FROM THE 2009 SOURCE (TLInf: runoff = |rTop - vTop| whenever rTop < 0), confirmed by HYDRUS 4.17: runoff
        // only while the top boundary is a head condition (KodTop > 0, ponding / hCritS).  With a flux top the difference
        // rTop - vTop is the storage term of the step after a change of the boundary flux, not runoff (5SEASON: 4.17 prints 0
        // where the source gives 1.56e-3 at t = 95.01; every runoff value of 2HYSTER / 2NOHYSTR has KodTop > 0).
        if (!self.w_layer || (self.w_layer && self.h_new[m] >= self.h_crit_s)) && self.r_top < 0.0 && self.kod_top > 0 {
            run_off = (self.r_top - v_t).abs();
        }
        if run_off < 1e-5 {
            run_off = 0.0;
        }
        let (mut r_infil, mut r_evap) = (0.0, 0.0);
        let (prec, r_soil) = (self.prec, self.r_soil);
        if v_t < 0.0 && (prec > 0.0 || (self.w_layer && self.h_new[m] > 0.0)) {
            r_infil = -v_t + r_soil;
        }
        if v_t >= 0.0 && prec > 0.0 {
            r_infil = prec;
        }
        if v_t > 0.0 {
            r_evap = v_t + prec;
        }
        if v_t <= 0.0 && r_soil > 0.0 && prec > 0.0 {
            r_evap = r_soil;
        }
        if v_t < 0.0 && self.w_layer && self.h_new[m] > 0.0 {
            r_evap = r_soil;
        }
        // DEVIATION FROM THE 2009 SOURCE (OUTPUT.FOR TLInf books infiltration only with Prec > 0, a surface layer or a
        // head top), confirmed by HYDRUS 4.17: with a constant-flux top (KodTop = -1) the inflow is booked as infiltration
        // too (1SCALING, rTop = -0.016667: sum(Infil) = 1.0312e-3 at t = 0.06187 ... 40.0 at t = 2400).
        if v_t < 0.0 && self.kod_top == -1 {
            r_infil = -v_t;
        }
        if v_t < 0.0 && self.kod_top > 0 {
            r_infil = -v_t;
        }
        let q = &mut self.cum_q;
        q[0] += self.r_top * dt;
        q[1] += self.r_root * dt;
        q[2] += v_t * dt;
        q[3] += self.v_root * dt;
        q[4] += v_b * dt;
        q[5] += run_off * dt;
        q[6] += r_infil * dt;
        q[7] += r_evap * dt;
        if self.l_flux_out {
            // CumQ(9), CumQ(10): cumulative Prec / rSoil (only when lFluxOut, as in TLInf)
            q[8] += self.prec * dt;
            q[9] += self.r_soil * dt;
        }
        q[10] += self.w_transf * dt; // CumQ(11): cumulative mobile/immobile water transfer
        self.w_cum_t += (v_b - v_t - self.v_root) * dt;
        self.w_cum_a += (v_b.abs() + v_t.abs() + self.v_root.abs()) * dt;
        let sol_extra = if self.l_chem { self.solute_cumulate(run_off) } else { vec![] };
        // TLInf writes the T_Level.out line only `if(lWat.or.TLevel.eq.1.or.lEnd)`, but the Solute<i>.out and Run_Inf.out
        // lines at every printed level.  Results keep one record per printed level for all of them, so with steady-state
        // water flow (lWat = f) the solute series are complete (3SELIM, TEST3, TEST4, TEST5, 3LAIJURI: the records used to
        // stop after the first level); the water values of the extra records are the steady values.
        if !j_print {
            return;
        }
        let mut volume = 0.0;
        for i in 0..n - 1 {
            let dx = self.x[i + 1] - self.x[i];
            let mut v = dx * (self.th_new[i] + self.th_new[i + 1]) / 2.0;
            if self.l_vapor {
                v += dx * (self.th_v_new[i] + self.th_v_new[i + 1]) / 2.0;
            }
            if self.i_dual_por > 0 {
                v += dx * (self.th_new_im[i] + self.th_new_im[i + 1]) / 2.0;
            }
            volume += v;
        }
        let rec = TLevel {
            tlevel: self.t_level,
            t: self.t,
            dt,
            iter_w: self.iter_w,
            iter_c: self.iter_c,
            it_cum: self.it_cum,
            kod_top: self.kod_top,
            kod_bot: self.kod_bot,
            converged: self.convg,
            r_top: self.r_top,
            r_root: self.r_root,
            v_top: v_t,
            v_root: self.v_root,
            v_bot: v_b,
            cum_r_top: self.cum_q[0],
            cum_r_root: self.cum_q[1],
            cum_v_top: self.cum_q[2],
            cum_v_root: self.cum_q[3],
            cum_v_bot: self.cum_q[4],
            h_top: self.h_new[m],
            h_root: self.h_root,
            h_bot: self.h_new[0],
            run_off,
            cum_run_off: self.cum_q[5],
            volume,
            cum_infil: self.cum_q[6],
            cum_evap: self.cum_q[7],
            precip: prec,
            peclet: self.peclet_courant().0,
            courant: self.peclet_courant().1,
            temp_top: self.temp(m),
            temp_bot: self.temp(0),
            cum_w_transf: self.cum_q[10],
            snow_layer: self.snow_layer,
            solutes: sol_extra,
        };
        self.res.tlevel.push(rec);
        if self.l_flux_out {
            // T_Level1.OUT (lFluxOut): potential and actual fluxes with the cumulative values
            self.res.tlevel1.push(TLevel1 {
                t: self.t,
                prec,
                r_soil,
                r_top: self.r_top,
                v_top: v_t,
                r_infil,
                r_evap,
                r_root: self.r_root,
                v_root: self.v_root,
                v_bot: v_b,
                run_off,
                cum_prec: self.cum_q[8],
                cum_evap_p: self.cum_q[9],
                cum_r_top: self.cum_q[0],
                cum_v_top: self.cum_q[2],
                cum_infil: self.cum_q[6],
                cum_evap: self.cum_q[7],
                cum_r_root: self.cum_q[1],
                cum_v_root: self.cum_q[3],
                cum_v_bot: self.cum_q[4],
                cum_run_off: self.cum_q[5],
                volume,
                snow_layer: self.snow_layer,
                v_top_w,
                v_top_v,
            });
        }
    }

    /// Nodal profile output (NodOut). Fluxes use the NodOut formula.
    pub fn profile_out(&mut self, t: f64) {
        let n = self.n;
        let grav = self.cos_alf;
        let ns = self.n_species();
        let con_sn = self.con_sat[self.mat[n - 1]] * self.ak[n - 1];

        let flux_concs = if self.l_chem && self.prj.solute.i_conc_type == 2 {
            let mut fc = vec![vec![0.0; ns]; n];
            for js in 0..ns {
                let f_js = self.flux_conc(js);
                for i in 0..n {
                    fc[i][js] = f_js[i];
                }
            }
            Some(fc)
        } else {
            None
        };

        let l_w_dep = self.prj.water.l_w_dep;
        let l_vapor = self.l_vapor;
        let mut nodes = Vec::with_capacity(n);
        for i in (0..n).rev() {
            // Liquid flux vi (incl. temperature-dependent part) and vapor flux vVi, as in NodOut
            let mut vi;
            let mut v_vap = 0.0;
            let (mut v_iso, mut v_term) = (0.0, 0.0);
            if i == 0 {
                let dx = self.x[1] - self.x[0];
                vi = -(self.con[0] + self.con[1]) / 2.0 * ((self.h_new[1] - self.h_new[0]) / dx + grav);
                if l_w_dep {
                    vi -= (self.con_lt[0] + self.con_lt[1]) / 2.0 * (self.temp(1) - self.temp(0)) / dx;
                }
                if l_vapor {
                    let v_h = -(self.con_vh[0] + self.con_vh[1]) / 2.0 * (self.h_new[1] - self.h_new[0]) / dx;
                    let v_t = -(self.con_vt[0] + self.con_vt[1]) / 2.0 * (self.temp(1) - self.temp(0)) / dx;
                    v_iso = v_h;
                    v_term = v_t;
                    v_vap = v_h + v_t;
                }
            } else if i == n - 1 {
                let dx = self.x[n - 1] - self.x[n - 2];
                // NodOut: `-(ThN(N)-ThOldT)*dx/2/dt - Sink(N)*dx/2`: with the storage term the printed flux of the top node equals the
                // boundary flux (2HYSTER: -9). The reference files contain solver noise in this one value (1DRAINAG prints -0.055 for a
                // no-flux top, 2NOHYSTR -8.69 for a flux of -9), so the comparison does not check the top-node flux.
                vi = -(self.con[n - 1] + self.con[n - 2]) / 2.0 * ((self.h_new[n - 1] - self.h_new[n - 2]) / dx + grav)
                    - (self.th_new[n - 1] - self.th_old[n - 1]) * dx / 2.0 / self.dt
                    - self.sink[n - 1] * dx / 2.0;
                if l_w_dep {
                    vi -= (self.con_lt[n - 1] + self.con_lt[n - 2]) / 2.0 * (self.temp(n - 1) - self.temp(n - 2)) / dx;
                }
                if l_vapor {
                    let v_h = -(self.con_vh[n - 1] + self.con_vh[n - 2]) / 2.0 * (self.h_new[n - 1] - self.h_new[n - 2]) / dx;
                    let v_t = -(self.con_vt[n - 1] + self.con_vt[n - 2]) / 2.0 * (self.temp(n - 1) - self.temp(n - 2)) / dx;
                    v_iso = v_h;
                    v_term = v_t;
                    v_vap = v_h + v_t;
                }
            } else {
                let dxa = self.x[i + 1] - self.x[i];
                let dxb = self.x[i] - self.x[i - 1];
                let va = -(self.con[i] + self.con[i + 1]) / 2.0 * ((self.h_new[i + 1] - self.h_new[i]) / dxa + grav);
                let vb = -(self.con[i] + self.con[i - 1]) / 2.0 * ((self.h_new[i] - self.h_new[i - 1]) / dxb + grav);
                vi = (va * dxa + vb * dxb) / (dxa + dxb);
                if l_w_dep {
                    let vat = -(self.con_lt[i] + self.con_lt[i + 1]) / 2.0 * (self.temp(i + 1) - self.temp(i)) / dxa;
                    let vbt = -(self.con_lt[i] + self.con_lt[i - 1]) / 2.0 * (self.temp(i) - self.temp(i - 1)) / dxb;
                    vi += (vat * dxa + vbt * dxb) / (dxa + dxb);
                }
                if l_vapor {
                    let vva = -(self.con_vh[i] + self.con_vh[i + 1]) / 2.0 * (self.h_new[i + 1] - self.h_new[i]) / dxa;
                    let vvb = -(self.con_vh[i] + self.con_vh[i - 1]) / 2.0 * (self.h_new[i] - self.h_new[i - 1]) / dxb;
                    let v_h = (vva * dxa + vvb * dxb) / (dxa + dxb);
                    let vvat = -(self.con_vt[i] + self.con_vt[i + 1]) / 2.0 * (self.temp(i + 1) - self.temp(i)) / dxa;
                    let vvbt = -(self.con_vt[i] + self.con_vt[i - 1]) / 2.0 * (self.temp(i) - self.temp(i - 1)) / dxb;
                    let v_t = (vvat * dxa + vvbt * dxb) / (dxa + dxb);
                    v_iso = v_h;
                    v_term = v_t;
                    v_vap = v_h + v_t;
                }
            }

            let sorb2 = (0..ns).map(|j| {
                self.sol.as_ref().map(|s| s.sorb2.get(j).map(|c| c[i]).unwrap_or(0.0)).unwrap_or(0.0)
            }).collect();

            let h_matrix = if self.l_dual_perm { Some(self.h_matrix_new[i]) } else { None };
            let th_matrix = if self.l_dual_perm { Some(self.th_matrix_new[i]) } else { None };
            let conc_matrix = if self.l_dual_perm {
                Some((0..ns).map(|j| self.conc_matrix(j, i)).collect())
            } else {
                None
            };
            let sorb_matrix = if self.l_dual_perm {
                Some((0..ns).map(|j| self.sorb_matrix(j, i)).collect())
            } else {
                None
            };

			let theta_val = if (t - self.t_init).abs() < 1e-6 {
				self.th_old[i]
			} else {
				self.th_new[i]
			};
            nodes.push(NodeOut {
                node: n - i,
                depth: self.x[n - 1] - self.x[i],
                h: self.h_new[i],
                theta: theta_val,
                k: self.con[i],
                c: self.cap[i],
                flux: vi + v_vap,
                flux_vapor: v_vap,
                sink: self.sink[i],
                sink_im: if self.i_dual_por > 0 { self.sink_im[i] } else { 0.0 },
                th_im: if self.i_dual_por > 0 { self.th_new_im[i] } else { 0.0 },
                s_trans: self.sol.as_ref().map(|s| s.s_trans[i]).unwrap_or(0.0),
                con_lt: self.con_lt.get(i).copied().unwrap_or(0.0),
                con_vh: self.con_vh.get(i).copied().unwrap_or(0.0),
                con_vt: self.con_vt.get(i).copied().unwrap_or(0.0),
                v_vap_iso: v_iso,
                v_vap_term: v_term,
                kappa: self.kappa[i],
                v_over_ks: vi / con_sn,
                temp: self.temp(i),
                conc: (0..ns).map(|j| self.conc(j, i)).collect(),
                sorb: (0..ns).map(|j| self.sorb(j, i)).collect(),
                sorb2,
                h_matrix,
                th_matrix,
                conc_matrix,
                sorb_matrix,
                flux_conc: flux_concs.as_ref().map(|fc| fc[i].clone()),
            });
        }
        self.res.profiles.push(ProfileOut { t, nodes });
    }

    pub fn obs_nod(&mut self) {
        if self.prj.profile.observation_nodes.is_empty() {
            return;
        }
        let n = self.n;
        let ns = self.n_species();

        let flux_concs = if self.l_chem && self.prj.solute.i_conc_type == 2 {
            let mut fc = vec![vec![0.0; ns]; n];
            for js in 0..ns {
                let f_js = self.flux_conc(js);
                for i in 0..n {
                    fc[i][js] = f_js[i];
                }
            }
            Some(fc)
        } else {
            None
        };

        let mut pts = vec![];
        for &nd in &self.prj.profile.observation_nodes {
            if nd < 1 || nd > n {
                continue;
            }
            let i = n - nd;

            let h_matrix = if self.l_dual_perm { Some(self.h_matrix_new[i]) } else { None };
            let th_matrix = if self.l_dual_perm { Some(self.th_matrix_new[i]) } else { None };
            let conc_matrix = if self.l_dual_perm {
                Some((0..ns).map(|j| self.conc_matrix(j, i)).collect())
            } else {
                None
            };

            pts.push(ObsPoint {
                node: nd,
                h: self.h_new[i],
                // Th(i) = ThNew + ThNewIm; the flux includes the vapor flux when no solute transport is simulated
                theta: self.th_new[i] + if self.i_dual_por > 0 { self.th_new_im[i] } else { 0.0 },
                temp: self.temp(i),
                flux: self.v_new.get(i).copied().unwrap_or(0.0)
                    + if !self.l_chem && self.l_vapor { self.v_v_new.get(i).copied().unwrap_or(0.0) } else { 0.0 },
                conc: (0..ns).map(|j| self.conc(j, i)).collect(),
                flux_conc: flux_concs.as_ref().map(|fc| fc[i].clone()),
                h_matrix,
                th_matrix,
                conc_matrix,
            });
        }
        // the call at initialisation (before the first step) is stamped with the initial time
        let t_obs = if self.res.tlevel.is_empty() && self.res.obs.is_empty() { self.t_init } else { self.t };
        self.res.obs.push(ObsOut { t: t_obs, points: pts });
    }

    /// Mass balance and sub-region summary (SubReg).
    pub fn sub_reg(&mut self, p_level: usize) {
        let n = self.n;
        let dt = self.dt;
        let n_lay = self.lay.iter().cloned().max().unwrap_or(1);
        let mut sub: Vec<SubRegion> = vec![SubRegion::default(); n_lay];
        let mut tot = SubRegion::default();
        let mut a_tot = 0.0;
        let mut delt_w = 0.0;
        let mut h_tot = 0.0;
        let mut volume_im = 0.0;
        // SubReg: `Volume` (printed as "W-volume") holds the mobile water only, whereas SubVol/Change/WatIn
        // (sub-regions, In-flow, balance) include the immobile region; the latter is printed as "W-volumeI".
        let mut volume_mobile = 0.0;
        let (mut t_vol, mut t_tot) = (0.0, 0.0);
        let l_temp = self.l_temp && self.heat.is_some();
        for i in (0..n - 1).rev() {
            let j = i + 1;
            let lay = self.lay[i] - 1;
            let dx = self.x[j] - self.x[i];
            sub[lay].area += dx;
            a_tot += dx;
            let he = (self.h_new[i] + self.h_new[j]) / 2.0;
            let mut vnew = dx * (self.th_new[i] + self.th_new[j]) / 2.0;
            let mut vold = dx * (self.th_old[i] + self.th_old[j]) / 2.0;

            if self.l_vapor {
                vnew += dx * (self.th_v_new[i] + self.th_v_new[j]) / 2.0;
                vold += dx * (self.th_v_old[i] + self.th_v_old[j]) / 2.0;
            }
            let mut v_mob = vnew;
            if self.i_dual_por > 0 {
                vnew += dx * (self.th_new_im[i] + self.th_new_im[j]) / 2.0;
                vold += dx * (self.th_old_im[i] + self.th_old_im[j]) / 2.0;
                volume_im += dx * (self.th_new_im[i] + self.th_new_im[j]) / 2.0; // VolumeIm
            }
            if self.l_dual_perm {
                // theta = w * theta_f + (1 - w) * theta_m
                let w = self.w_fracture;
                vnew = w * vnew + (1.0 - w) * dx * (self.th_matrix_new[i] + self.th_matrix_new[j]) / 2.0;
                vold = w * vold + (1.0 - w) * dx * (self.th_matrix_old[i] + self.th_matrix_old[j]) / 2.0;
                if self.matrix_dual_por {
                    vnew += (1.0 - w) * dx * (self.th_im_mat_new[i] + self.th_im_mat_new[j]) / 2.0;
                    vold += (1.0 - w) * dx * (self.th_im_mat_old[i] + self.th_im_mat_old[j]) / 2.0;
                }
            }
            if self.l_dual_perm {
                v_mob = vnew; // dual-permeability: the weighted fracture/matrix volume is reported as before
            }
            volume_mobile += v_mob;
            tot.change += (vnew - vold) / dt;
            sub[lay].change += (vnew - vold) / dt;
            // Sub-region W-volume as the total: mobile water only (dual porosity), the immobile water goes to W-volumeI.
            // DEVIATION FROM THE 2009 SOURCE (SubReg adds ThNewIm to SubVol), confirmed by HYDRUS 4.17: Test9 / TEST9a
            // print the same W-volume for the single sub-region as for the total (2.0599 at t = 0, not 6.833).
            sub[lay].volume += v_mob;
            h_tot += he * dx;
            sub[lay].h_mean += he * dx;

            if l_temp {
                // TNewE: heat content (K * volumetric heat capacity), TE: mean temperature
                let (mi, mj) = (self.mat[i], self.mat[j]);
                let hm = &self.prj.heat.materials;
                let cap_i = hm[mi].qn * hm[mi].cn + hm[mi].qo * hm[mi].co + hm[mi].cw * self.th_new[i];
                let cap_j = hm[mj].qn * hm[mj].cn + hm[mj].qo * hm[mj].co + hm[mj].cw * self.th_new[j];
                let (ti, tj) = (self.temp(i), self.temp(j));
                let t_new_e = dx * ((ti + 273.15) * cap_i + (tj + 273.15) * cap_j) / 2.0;
                let te = (ti + tj) / 2.0;
                t_vol += t_new_e;
                sub[lay].heat_vol += t_new_e;
                t_tot += te * dx;
                sub[lay].t_mean += te * dx;
            }
            if p_level == 0 {
                self.wat_in[i] = vnew;
            } else {
                delt_w += (self.wat_in[i] - vnew).abs();
            }
        }
        for s in sub.iter_mut() {
            if s.area > 0.0 {
                s.h_mean /= s.area;
                if l_temp {
                    s.t_mean = s.t_mean / s.area; // the Fortran prints the mean temperature in degrees C (label [K] notwithstanding)
                }
            }
        }
        if a_tot > 0.0 {
            h_tot /= a_tot;
            t_tot /= a_tot;
        }
        tot.area = a_tot;
        tot.h_mean = h_tot;
        tot.volume = volume_mobile;
        tot.volume_im = volume_im;
        if l_temp {
            tot.heat_vol = t_vol;
            tot.t_mean = t_tot;
        }
        // the initial record (PLevel 0) belongs to the initial time, not to the first time step
        let mut rec = BalanceOut { t: if p_level == 0 { self.t_init } else { self.t }, total: tot.clone(), sub, ..Default::default() };

        // Fluxes at the boundaries (v1 at the bottom, vN at the top), including the
        // temperature-dependent liquid flux and the vapor flux terms of SubReg
        let grav = self.cos_alf;
        let dx1 = self.x[1] - self.x[0];
        let mut v1 = -(self.con[0] + self.con[1]) / 2.0 * ((self.h_new[1] - self.h_new[0]) / dx1 + grav);
        let dxn = self.x[n - 1] - self.x[n - 2];
        let mut vn = -(self.con[n - 1] + self.con[n - 2]) / 2.0 * ((self.h_new[n - 1] - self.h_new[n - 2]) / dxn + grav);
        if self.prj.water.l_w_dep {
            v1 -= (self.con_lt[0] + self.con_lt[1]) / 2.0 * (self.temp(1) - self.temp(0)) / dx1;
            vn -= (self.con_lt[n - 1] + self.con_lt[n - 2]) / 2.0 * (self.temp(n - 1) - self.temp(n - 2)) / dxn;
        }
        if self.l_vapor {
            v1 -= (self.con_vh[0] + self.con_vh[1]) / 2.0 * (self.h_new[1] - self.h_new[0]) / dx1;
            vn -= (self.con_vh[n - 1] + self.con_vh[n - 2]) / 2.0 * (self.h_new[n - 1] - self.h_new[n - 2]) / dxn;
            v1 -= (self.con_vt[0] + self.con_vt[1]) / 2.0 * (self.temp(1) - self.temp(0)) / dx1;
            vn -= (self.con_vt[n - 1] + self.con_vt[n - 2]) / 2.0 * (self.temp(n - 1) - self.temp(n - 2)) / dxn;
        }
        if self.l_dual_perm {
            let w = self.w_fracture;
            v1 = w * v1 + (1.0 - w) * self.matrix_flux(0, 1);
            vn = w * vn + (1.0 - w) * self.matrix_flux(n - 2, n - 1);
        }
        rec.bot_flux = v1;
        rec.top_flux = vn;

        // wVolI = Volume + VolumeIm; WatBalT = Volume + VolumeIm - wVolI - wCumT
        let vol_bal = tot.volume + if self.l_dual_perm { 0.0 } else { tot.volume_im };
        if p_level == 0 {
            self.w_vol_i = vol_bal;
        } else {
            rec.wat_bal_t = vol_bal - self.w_vol_i - self.w_cum_t;
            let ww = delt_w.max(self.w_cum_a);
            if ww > 1e-25 {
                rec.wat_bal_r = rec.wat_bal_t.abs() / ww * 100.0;
            }
        }
        if self.l_chem {
            self.sub_reg_solute(p_level, &mut rec);
        }
        self.res.balance.push(rec);
    }
}
