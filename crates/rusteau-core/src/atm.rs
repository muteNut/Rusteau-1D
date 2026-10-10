//! Time-variable boundary conditions and time-step control (port of parts of TIME.FOR).

use crate::error::HydrusError;
use crate::model::SoilModel;
use crate::output::ALevel;
use crate::sim::Simulation;

impl Simulation {
    /// Read the next atmospheric record (SetBC).
    pub fn set_bc(&mut self) -> Result<(), HydrusError> {
        let n = self.n;
        let kod_top_old = self.kod_top;
        let recs = &self.prj.atmosphere.records;
        let n_rec = recs.len();

        if n_rec == 0 {
            return Ok(());
        }

        // When boundary cycles are enabled and the table ends, wrap back to the beginning
        if self.prj.atmosphere.bc_cycles && self.atm_idx >= n_rec {
            let t_first = recs.first().map(|r| r.t).unwrap_or(0.0);
            let t_last = recs.last().map(|r| r.t).unwrap_or(1.0);
            let cycle_period = (t_last - t_first).max(1e-6);

            self.atm_idx %= n_rec;
            // `t` reaches a record time only within the synchronisation tolerance 0.001*dt (dt is REAL*4, t double),
            // e.g. 5.9999999925 for 6: count the cycles from the synchronised time.
            let t_sync = self.t + 0.001 * self.dt;
            let num_cycles = ((t_sync - self.t_init) / cycle_period).floor();
            self.t_atm_old = self.t;
            self.t_atm1 = t_first + (num_cycles + 1.0) * cycle_period;
        }

        if self.atm_idx >= n_rec {
            // 'end' record: run stops at the last atmospheric time
            self.t_max = self.t_atm1;
            return Ok(());
        }

        let rec = recs[self.atm_idx].clone();
        self.atm_idx += 1;
        // HYDRUS 4.17 triggered irrigation: a record read during the irrigation sets its own top flux again
        self.irr_override = None;

        if self.prj.atmosphere.bc_cycles {
            let t_first = recs.first().map(|r| r.t).unwrap_or(0.0);
            let t_last = recs.last().map(|r| r.t).unwrap_or(1.0);
            let cycle_period = (t_last - t_first).max(1e-6);
            let t_sync = self.t + 0.001 * self.dt;
            let num_cycles = ((t_sync - self.t_init) / cycle_period).floor();
            self.t_atm1 = rec.t + num_cycles * cycle_period;
            if self.t_atm1 <= t_sync && self.atm_idx < n_rec {
                self.t_atm1 = recs[self.atm_idx].t + num_cycles * cycle_period;
            }
        } else {
            self.t_atm1 = rec.t;
        }

        // A new atmospheric record starts: remember the snow / interception state in front of
        // it (apply_snow_and_interception is re-evaluated from this state) and the record
        // interval used by Snow (dtSnow = tAtm1 - tAtmOld).
        self.snow_ev0 = self.snow_layer;
        self.interc_ev0 = self.interc_state;
        self.dt_snow = (self.t_atm1 - self.t_atm_old).max(0.0);

        // Store raw baseline values
        self.prec_raw = rec.prec;
        self.r_soil_raw = rec.evap;
        self.r_root_raw = rec.transp.abs();

        if self.prj.atmosphere.has_root_depth {
            self.x_root = rec.x_root;
        }
        self.atm_record_scalars(&rec);

        if self.top_inf {
            let r_top_old = self.r_top;
            self.h_crit_a = -rec.h_crit_a.abs();
            if self.l_var_bc {
                self.prec = self.prec_raw;
                self.r_top = self.prec;
                if (r_top_old - self.r_top).abs() > self.r_top.abs() * 0.2 && self.r_top < 0.0 {
                    self.min_step = true;
                }
                self.kod_top = self.r_soil_raw as i32;
                self.r_soil = 0.0;
                if self.kod_top == -1 && kod_top_old == 1 && self.prec > 0.0 && self.h_new[n - 1] > 0.0 {
                    self.h_new[n - 1] = -0.01 * self.x_conv;
                }
            } else {
                // Meteorological potential ET (Meteo.in) or LAI based partitioning of the
                // potential ET given in Atmosph.in (SetBC, lLAI)
                self.meteo_advance();
                if self.prj.atmosphere.meteo.is_some() {
                    self.meteo_potential()?;
                } else if self.prj.atmosphere.lai_partitioning {
                    // The rRoot column holds the LAI and the rSoil column the potential ET
                    let r_lai = rec.transp;
                    let r_pet = rec.evap;
                    let ext = self.prj.atmosphere.extinction.max(0.1);
                    let r_r = if r_lai > 0.0 { r_pet * (1.0 - (-ext * r_lai).exp()).max(0.0) } else { 0.0 };
                    self.r_soil_raw = r_pet - r_r;
                    self.r_root_raw = r_r.abs();
                }

                // Effective rates (snow / interception are evaluated once per atmospheric record)
                self.prec = self.prec_raw;
                self.r_soil = self.r_soil_raw;
                self.r_root = self.r_root_raw;
                self.apply_snow_and_interception();

                if (r_top_old - self.r_top).abs() > self.r_top.abs() * 0.2 && self.r_top < 0.0 {
                    self.min_step = true;
                }
                if self.r_top > 0.0 && r_top_old < 0.0 && !self.w_layer {
                    let mut x_limit = 0.0;
                    if self.model == SoilModel::VGAirEntry {
                        x_limit = -0.03 * self.x_conv;
                    }
                    if self.kod_top == 4 || self.h_new[n - 1] > x_limit {
                        if self.model != SoilModel::VGAirEntry {
                            x_limit = -0.01 * self.x_conv;
                        }
                        self.h_new[n - 1] = x_limit;
                        self.kod_top = -4;
                        self.kod_top_m = -4;
                    }
                }
            }
            if self.kod_top == 3 || self.l_var_bc {
                self.h_top = rec.h_top;
            }
        }

        if self.bot_inf {
            let mut h_b = rec.h_bot;
            if self.centrif_radius.is_some() {
                // centrifuge (SetBC of TIME.FOR): the hBot column holds the rotation speed (rpm in Rusteau), the
                // bottom pressure head is 0
                self.cos_alf = crate::sim::centrifuge_cos_alf(h_b, self.x_conv);
                h_b = 0.0;
                self.min_step = true;
            }
            if (self.r_bot - rec.r_bot).abs() > self.r_bot.abs() * 0.2 {
                self.min_step = true;
            }
            self.r_bot = rec.r_bot;
            if (self.h_bot - h_b - self.gwl0l).abs() > self.h_bot.abs() * 0.2 {
                self.min_step = true;
            }
            self.h_bot = h_b + self.gwl0l;
        }
        Ok(())
    }

    /// TmCont of TIME.FOR.  `dt`, `dtOpt`, `dtMax`, `dMul`, `dMul2` and `dtMin` are REAL*4 in the Fortran code while `t` is
    /// double precision: every value assigned to `dt` / `dt_opt` is rounded to single precision (`sp`), so that the time
    /// levels `t = t + dt` are the same double-precision numbers as in the Fortran run (verified against the compiled
    /// source and the 1INFILTR / 2HYSTER reference RUN_INF.OUT: identical time columns; HYDRUS 4.17 1SCALING: all
    /// 1220 time levels, together with the initial lMinStep = .false. of `Simulation::new`).
    pub fn tm_cont(&mut self, iter: usize, t_print: f64, dt_max_c: f64) {
        let sp = |v: f64| v as f32 as f64;
        let dt_max_w = sp(self.dt_max);
        let dt_max_c = sp(dt_max_c);
        let dt_max;
        if self.min_step {
            dt_max = dt_max_w.min(dt_max_c).min(sp(self.dt_init)).min(self.dt_opt);
            self.dt_opt = dt_max;
            self.min_step = false;
        } else {
            dt_max = dt_max_w.min(dt_max_c);
        }
        let (d_mul, d_mul2, dt_min) = (sp(self.d_mul), sp(self.d_mul2), sp(self.dt_min));
        let t_fix = t_print.min(self.t_atm).min(self.t_max);
        if iter <= self.it_min && (t_fix - self.t) >= sp(d_mul * self.dt_opt) {
            self.dt_opt = dt_max.min(sp(d_mul * self.dt_opt));
        }
        if iter >= self.it_max {
            self.dt_opt = dt_min.max(sp(d_mul2 * self.dt_opt));
        }
        let rest = sp(t_fix - self.t); // sngl(tFix-t)
        self.dt = self.dt_opt.min(rest);
        let mut i_step = 1i64;
        if self.dt > 0.0 {
            i_step = (((rest as f32) / (self.dt as f32)).round()) as i64; // anint(sngl(tFix-t)/dt)
        }
        // TIME.FOR: the step is evened out only when at most ten steps remain to the next fixed time.  (An earlier
        // version of the port applied it for every iStep >= 1 to follow the 1SCALING sample output, which was written
        // by an older HYDRUS build; the compiled source and the current reference outputs, e.g. 1INFILTR and 2HYSTER,
        // use the limit.)
        if (1..=10).contains(&i_step) {
            self.dt = sp(((rest as f32) / (i_step as f32)) as f64).min(dt_max);
        }
        if i_step == 1 {
            self.dt = rest;
            // TIME.FOR: a single remaining step that exceeds dtMax by more than dtMin is halved, also with a meteorological
            // input file.  (A former deviation skipped the halving with meteo records, fitted to the EnBal2b output of an
            // older build; the HYDRUS 4.17 EnBal2b output halves, e.g. 0.011 -> 2 x 0.0055 before t = 328.083.)
            if self.dt - dt_max > dt_min {
                self.dt = sp(self.dt / 2.0);
            }
        }
        if self.dt <= 0.0 {
            self.dt = sp(dt_min / 3.0);
        }
    }

    fn daily_factor(&self, base: f64) -> f64 {
        let t_day = self.t / self.t_conv / 86400.0;
        let rem = t_day % 1.0;
        if rem <= 0.264 || rem >= 0.736 {
            0.24 * base
        } else {
            2.75 * base * (2.0 * std::f64::consts::PI * t_day - 6.0 * std::f64::consts::PI / 12.0).sin()
        }
    }

    pub fn daily_var_root_soil(&mut self) {
        self.r_root = self.daily_factor(self.r_root_d);
        self.r_soil = self.daily_factor(self.r_soil_d);
    }

    pub fn sin_prec(&mut self) {
        let dt = self.t_atm1 - self.t_atm_old;
        if self.prec_d > 0.0 && dt > 0.0 {
            self.prec = self.prec_d * (1.0 + (2.0 * std::f64::consts::PI * (self.t - self.t_atm_old) / dt - std::f64::consts::PI).cos());
        } else {
            self.prec = 0.0;
        }
    }

    pub fn a_level(&mut self) {
        let n = self.n;
        self.res.alevel.push(ALevel {
            t: self.t,
            cum: [self.cum_q[0], self.cum_q[1], self.cum_q[2], self.cum_q[3], self.cum_q[4]],
            h_top: self.h_new[n - 1],
            h_root: self.h_root,
            h_bot: self.h_new[0],
        });
    }
}

// ------------------------------------------------------------------------------------
// Meteorological data (Meteo.in): Meteo(1/2/3) of the main program
// ------------------------------------------------------------------------------------
impl Simulation {
    /// Advance the index of the meteo record that is valid at the current time.
    pub fn meteo_advance(&mut self) {
        if self.l_daily_met {
            return; // with daily-generated data the index follows DailyMet
        }
        if let Some(ref mp) = self.prj.atmosphere.meteo {
            while self.meteo_idx + 1 < mp.records.len() && self.t >= mp.records[self.meteo_idx].t {
                self.meteo_idx += 1;
            }
        }
    }

    /// SetMeteo for the current meteo record: raw potential evaporation / transpiration.
    /// (With the energy balance SetMeteo returns before the evapotranspiration part.)
    pub fn meteo_potential(&mut self) -> Result<(), HydrusError> {
        if self.l_daily_met {
            return Ok(()); // evaluated for every time step by `day_met_values`
        }
        let tt_conv = 24.0 * 3600.0 * self.t_conv;
        let r_conv = 0.001 * self.x_conv;
        let (l_en_bal, res) = {
            let mp = match self.prj.atmosphere.meteo.as_ref() {
                Some(m) => m,
                None => return Ok(()),
            };
            let rec = match mp.records.get(self.meteo_idx) {
                Some(r) => r,
                None => return Ok(()),
            };
            if mp.l_en_bal {
                (true, None)
            } else {
                // iMetHour: the first record is less than one day after the initial time
                let is_hourly = mp.records[0].t - self.t_init <= 0.9999 * tt_conv;
                (false, Some(crate::meteo::potential_et_ex(mp, rec, self.t_conv, is_hourly)))
            }
        };
        if l_en_bal {
            self.r_root_raw = 0.0; // rSoil stays as given in Atmosph.in; Evapor sets rTop
            if let Some(mp) = self.prj.atmosphere.meteo.as_ref() {
                if let Some(rec) = mp.records.get(self.meteo_idx) {
                    if let Some(xr) = rec.x_root {
                        self.x_root = xr;
                    }
                }
            }
            return Ok(());
        }
        match res.flatten() {
            Some(p) => {
                self.r_soil_raw = p.evap_p * r_conv / tt_conv;
                self.r_root_raw = p.trans_p * r_conv / tt_conv;
                self.met_lai = p.lai;
                self.met_scf = p.scf;
                self.met_pet = p;
                if let Some(xr) = p.x_root {
                    self.x_root = xr;
                }
                Ok(())
            }
            None => Err(HydrusError::Numerical(
                "Meteo.in: the crop height exceeds the height of the wind or temperature measurement".into(),
            )),
        }
    }

    /// Meteo(2) for non-daily meteo data: a meteo record ends at the current time (which does
    /// not need to coincide with an atmospheric record) -> move on to the next record.
    pub fn meteo_record_boundary(&mut self) -> Result<(), HydrusError> {
        let hit = match self.prj.atmosphere.meteo.as_ref() {
            Some(mp) if !self.l_daily_met && !mp.records.is_empty() => {
                self.meteo_idx + 1 < mp.records.len() && (self.t - mp.records[self.meteo_idx].t).abs() <= 0.001 * self.dt
            }
            _ => false,
        };
        if hit {
            self.meteo_idx += 1;
            self.meteo_potential()?;
            self.apply_snow_and_interception();
        }
        Ok(())
    }

    /// Meteo(1) for lMetDaily: the maximum time step is limited to one hour, the first two
    /// daily records are read and the first time step is evaluated.
    pub fn daily_met_init(&mut self) {
        self.dt_max = self.dt_max.min(3600.0 * self.t_conv);
        if let Some(mp) = self.prj.atmosphere.meteo.as_ref() {
            crate::meteo::daily_met(mp, &mut self.day_met, 1, self.t, self.dt, self.t_init, self.t_max, 86400.0 * self.t_conv);
        }
        self.meteo_idx = self.day_met.idx;
        self.t_atm2 = self.day_met.cur.t;
        self.day_met_values();
        if !self.l_var_bc {
            self.r_top = self.r_soil.abs() - self.top_prec();
        }
    }

    /// Meteo(2) for lMetDaily: the current day ends -> the next day becomes the current one.
    pub fn daily_met_boundary(&mut self) {
        if (self.t - self.day_met.cur.t).abs() <= 0.001 * self.dt {
            if let Some(mp) = self.prj.atmosphere.meteo.as_ref() {
                crate::meteo::daily_met(mp, &mut self.day_met, 2, self.t, self.dt, self.t_init, self.t_max, 86400.0 * self.t_conv);
            }
            self.meteo_idx = self.day_met.idx;
            self.t_atm2 = self.day_met.cur.t;
        }
    }

    /// Meteo(3) for lMetDaily: air temperature and humidity at the new time, then SetDayMet.
    pub fn daily_met_step(&mut self) {
        if let Some(mp) = self.prj.atmosphere.meteo.as_ref() {
            crate::meteo::daily_met(mp, &mut self.day_met, 4, self.t, self.dt, self.t_init, self.t_max, 86400.0 * self.t_conv);
        }
        self.meteo_idx = self.day_met.idx;
        // At t = tEnd - 1 DailyMet already moves on to the last day inside this call (iDaily = 3),
        // so tAtm2 has to follow here as well.
        self.t_atm2 = self.day_met.cur.t;
        self.day_met_values();
        if !self.l_var_bc {
            self.r_top = self.r_soil.abs() - self.top_prec();
        }
    }

    /// SetDayMet for the current time: potential evaporation and transpiration (or the radiation
    /// input of the energy balance), interception and conversion to [L/T].
    fn day_met_values(&mut self) {
        let l_en_bal = self.l_en_bal;
        let (out, i_interc, a_mm) = {
            let mp = match self.prj.atmosphere.meteo.as_ref() {
                Some(m) => m,
                None => return,
            };
            (
                crate::meteo::set_day_met(mp, &self.day_met, self.t, self.t_conv, l_en_bal),
                mp.i_interc,
                mp.a_interc,
            )
        };
        let out = match out {
            Some(o) => o,
            None => {
                self.res.messages.push(
                    "Meteo.in: the crop height exceeds the height of the wind or temperature measurement".into(),
                );
                return;
            }
        };
        self.rst = out.rst;
        self.day_rad = out.rad;
        self.met_lai = out.lai;
        self.met_scf = out.scf;
        if let Some(xr) = out.x_root {
            self.x_root = xr;
        }
        if l_en_bal {
            return; // Evapor provides rTop and the heat flux
        }
        let k = 0.001 * self.x_conv / (24.0 * 3600.0 * self.t_conv); // [L/T] per [mm/d]
        self.r_soil = out.evap_p * k;
        let (mut r_interc, mut ex_interc) = (0.0, self.interc_state.excess);
        if i_interc == 1 {
            let (net_p, net_t, total) = crate::meteo::calculate_interception(
                self.prec_base / k,
                out.trans_p,
                out.lai,
                a_mm,
                out.scf,
                &mut self.interc_state,
            );
            self.prec = net_p * k;
            self.r_root = net_t * k;
            r_interc = total;
            ex_interc = self.interc_state.excess;
        } else {
            self.prec = self.prec_base;
            self.r_root = out.trans_p * k;
        }
        // Meteo.out (DayMeteoOut): one row per time step
        self.res.meteo_mode = 1;
        self.res.meteo.push(crate::output::MeteoRow {
            t: self.t,
            et: out.et_comb,
            evap_p: out.evap_p,
            transp_p: out.trans_p,
            rns: out.rns,
            rnl: out.rnl,
            rad_term: out.rad_term,
            aero_term: out.aero_term,
            prec: self.prec_base / k,
            interc: r_interc,
            ex_interc,
            air_temp: self.day_met.temp_a,
            air_rh: self.day_met.rh_a,
            solar_rad: out.rst,
            ..Default::default()
        });
    }
}
