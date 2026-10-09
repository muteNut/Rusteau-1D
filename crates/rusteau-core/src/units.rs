//! Conversion of a project to other length and time units.
//!
//! Every input value is stored in the project's own units (as in HYDRUS-1D).  When the length or time unit changes,
//! [`convert_units`] rescales all values with their physical dimension, so the converted project describes the same
//! problem.  The mass unit is only a label in HYDRUS and is not converted; concentrations [M/L³], bulk densities and
//! distribution coefficients change with the length unit only.
//!
//! Quantities with fixed units are left alone: temperatures [°C], the meteorological input of `METEO.IN` (radiation
//! [MJ/m²/d], wind speed [km/d], sunshine hours, crop height, wind and temperature measurement heights [cm], latitude,
//! altitude [m], interception constant [mm]) and the activation energies of the temperature dependence.  The time
//! columns of `METEO.IN` are in project units and are converted.

use crate::model::*;

/// Length units per metre.
fn per_metre(u: LengthUnit) -> f64 {
    Units { length: u, time: TimeUnit::Days, mass: String::new() }.x_conv()
}

/// Time units per second.
fn per_second(u: TimeUnit) -> f64 {
    Units { length: LengthUnit::Cm, time: u, mass: String::new() }.t_conv()
}

/// Rescales all values of `prj` from its current units to `length` / `time` and sets the new units.
pub fn convert_units(prj: &mut Project, length: LengthUnit, time: TimeUnit) {
    let fl = per_metre(length) / per_metre(prj.units.length);
    let ft = per_second(time) / per_second(prj.units.time);
    prj.units.length = length;
    prj.units.time = time;
    if fl == 1.0 && ft == 1.0 {
        return;
    }
    // value_new = value_old * fl^a * ft^b for a quantity of dimension L^a T^b
    let f = |a: i32, b: i32| fl.powi(a) * ft.powi(b);
    let s = |v: &mut f64, a: i32, b: i32| *v *= f(a, b);
    let sv = |v: &mut Vec<f64>, a: i32, b: i32| v.iter_mut().for_each(|x| *x *= f(a, b));
    let (len, time_, rate, per_len, per_time) = ((1, 0), (0, 1), (1, -1), (-1, 0), (0, -1));
    let conc = (-3, 0); // [M/L³]

    // ---- water flow
    let model = prj.water.model;
    let w = &mut prj.water;
    s(&mut w.tol_h, len.0, len.1);
    s(&mut w.h_tab1, len.0, len.1);
    s(&mut w.h_tab_n, len.0, len.1);
    for m in w.materials.iter_mut() {
        // Kosugi: the alpha column holds the median pressure head hm [L]
        let alpha = if model == SoilModel::Kosugi { len } else { per_len };
        s(&mut m.alpha, alpha.0, alpha.1);
        s(&mut m.alpha_w, alpha.0, alpha.1);
        s(&mut m.ks, rate.0, rate.1);
        s(&mut m.ks_w, rate.0, rate.1);
        match model {
            SoilModel::ModifiedVG => s(&mut m.extra[3], rate.0, rate.1),
            SoilModel::Durner => s(&mut m.extra[1], per_len.0, per_len.1),
            SoilModel::DualPorosityW => s(&mut m.extra[2], per_time.0, per_time.1),
            SoilModel::DualPorosityH => {
                s(&mut m.extra[2], per_len.0, per_len.1);
                // transfer = Ka(h) (h_f - h_im): the "Ks" of the immobile region is [1/(L T)]
                s(&mut m.extra[4], -1, -1);
            }
            SoilModel::DualPermeability => {
                s(&mut m.extra[2], per_len.0, per_len.1);
                s(&mut m.extra[4], rate.0, rate.1);
            }
            _ => {}
        }
    }
    if let Some(tabs) = w.tabs.as_mut() {
        for t in tabs.iter_mut() {
            sv(&mut t.h, len.0, len.1);
            sv(&mut t.con, rate.0, rate.1);
            sv(&mut t.cap, per_len.0, per_len.1);
        }
    }
    for d in w.dual_perm.materials.iter_mut() {
        s(&mut d.a, len.0, len.1);
        s(&mut d.ks_a, rate.0, rate.1);
        s(&mut d.omega_w, per_time.0, per_time.1);
        s(&mut d.alpha_im, per_len.0, per_len.1);
        s(&mut d.ks_im, rate.0, rate.1);
    }
    let bc = &mut w.bc;
    s(&mut bc.h_seep, len.0, len.1);
    s(&mut bc.r_top, rate.0, rate.1);
    s(&mut bc.r_bot, rate.0, rate.1);
    s(&mut bc.r_root, rate.0, rate.1);
    s(&mut bc.gwl0l, len.0, len.1);
    s(&mut bc.aqh, rate.0, rate.1);
    s(&mut bc.bqh, per_len.0, per_len.1);
    if let Some(d) = bc.drains.as_mut() {
        for v in [&mut d.z_bot, &mut d.spacing, &mut d.base_gw, &mut d.wet_perimeter, &mut d.z_interface] {
            s(v, len.0, len.1);
        }
        s(&mut d.entrance_resistance, time_.0, time_.1);
        for v in [&mut d.kh_top, &mut d.kh_bot, &mut d.kv_top, &mut d.kv_bot] {
            s(v, rate.0, rate.1);
        }
    }
    if let Some(irr) = bc.irrigation.as_mut() {
        s(&mut irr.h_trigger, len.0, len.1);
        s(&mut irr.rate, rate.0, rate.1);
        s(&mut irr.duration, time_.0, time_.1);
        s(&mut irr.lag, time_.0, time_.1);
    }

    // ---- time
    let t = &mut prj.time;
    for v in [&mut t.dt, &mut t.dt_min, &mut t.dt_max, &mut t.t_init, &mut t.t_max, &mut t.print_interval] {
        s(v, time_.0, time_.1);
    }
    sv(&mut t.print_times, time_.0, time_.1);

    // ---- atmosphere
    let lai_column = prj.atmosphere.lai_partitioning;
    let a = &mut prj.atmosphere;
    s(&mut a.h_crit_s, len.0, len.1);
    s(&mut a.interception_a, len.0, len.1);
    s(&mut a.snow_mf, rate.0, rate.1); // [L/T/°C]
    for r in a.records.iter_mut() {
        s(&mut r.t, time_.0, time_.1);
        s(&mut r.prec, rate.0, rate.1);
        s(&mut r.evap, rate.0, rate.1);
        if !lai_column {
            s(&mut r.transp, rate.0, rate.1);
        }
        s(&mut r.h_crit_a, len.0, len.1);
        s(&mut r.r_bot, rate.0, rate.1);
        s(&mut r.h_bot, len.0, len.1);
        s(&mut r.h_top, len.0, len.1);
        sv(&mut r.c_top, conc.0, conc.1);
        sv(&mut r.c_bot, conc.0, conc.1);
        s(&mut r.x_root, len.0, len.1);
    }
    if let Some(mp) = a.meteo.as_mut() {
        s(&mut mp.x_root, len.0, len.1);
        for g in mp.growth.iter_mut() {
            s(&mut g[0], time_.0, time_.1);
            s(&mut g[4], len.0, len.1);
        }
        for r in mp.records.iter_mut() {
            s(&mut r.t, time_.0, time_.1);
            if let Some(x) = r.x_root.as_mut() {
                s(x, len.0, len.1);
            }
        }
    }

    // ---- root water uptake
    let ru = &mut prj.root;
    match &mut ru.stress {
        RootStress::Feddes { p0, p2h, p2l, p3, r2h, r2l, p_optm } => {
            for v in [p0, p2h, p2l, p3] {
                s(v, len.0, len.1);
            }
            s(r2h, rate.0, rate.1);
            s(r2l, rate.0, rate.1);
            sv(p_optm, len.0, len.1);
        }
        RootStress::SShaped { p50, .. } => s(p50, len.0, len.1),
    }
    if let Some(ss) = ru.solute_stress.as_mut() {
        sv(&mut ss.a_osm, 4, 0); // osmotic head per concentration [L / (M/L³)]
        s(&mut ss.c50, conc.0, conc.1);
    }
    match &mut ru.growth {
        RootGrowthMode::FromAtmosphere => {}
        RootGrowthMode::Table(rows) => {
            for (tt, x) in rows.iter_mut() {
                s(tt, time_.0, time_.1);
                s(x, len.0, len.1);
            }
        }
        RootGrowthMode::Logistic { t_min, t_med, t_harv, x_min, x_med, x_max, period, .. } => {
            for v in [t_min, t_med, t_harv, period] {
                s(v, time_.0, time_.1);
            }
            for v in [x_min, x_med, x_max] {
                s(v, len.0, len.1);
            }
        }
    }
    sv(&mut ru.c_root_max, conc.0, conc.1);
    sv(&mut ru.r_km, conc.0, conc.1);
    sv(&mut ru.c_min, conc.0, conc.1);
    s(&mut ru.s_pot, -2, -1); // [M/L²/T]

    // ---- heat (energy in kg m² s⁻², as in HYDRUS)
    let h = &mut prj.heat;
    for m in h.materials.iter_mut() {
        s(&mut m.disp, len.0, len.1);
        for v in [&mut m.b1, &mut m.b2, &mut m.b3] {
            s(v, 1, -3); // thermal conductivity [M L T⁻³ K⁻¹]
        }
        for v in [&mut m.cn, &mut m.co, &mut m.cw] {
            s(v, -1, -2); // volumetric heat capacity [M L⁻¹ T⁻² K⁻¹]
        }
    }
    s(&mut h.period, time_.0, time_.1);

    // ---- solute transport
    let mobile_immobile = prj.solute.materials.iter().any(|m| m.th_immobile > 0.0);
    let so = &mut prj.solute;
    let dual_neq = so.l_dual_neq;
    s(&mut so.tol_abs, conc.0, conc.1);
    s(&mut so.t_pulse, time_.0, time_.1);
    s(&mut so.d_surf, len.0, len.1);
    s(&mut so.c_atm, conc.0, conc.1);
    for m in so.materials.iter_mut() {
        s(&mut m.bulk_density, conc.0, conc.1);
        s(&mut m.disp_l, len.0, len.1);
    }
    for sp in so.species.iter_mut() {
        s(&mut sp.diff_w, 2, -1);
        s(&mut sp.diff_g, 2, -1);
        s(&mut sp.c_top, conc.0, conc.1);
        s(&mut sp.c_bot, conc.0, conc.1);
        for p in sp.per_material.iter_mut() {
            // HYDRUS isotherm s = Ks c^beta / (1 + nu c^beta), c [M/L³], s [M/M]: Ks and nu in [L³/M]^beta
            let kd = fl.powf(3.0 * p.beta);
            p.ks *= kd;
            p.nu *= kd;
            // first-order decay (mu_*) and chain (gam_*) rates [1/T]; with dual non-equilibrium mu_g holds f_em [-] and
            // gam_g omega_S [1/T]
            for v in [&mut p.mu_w, &mut p.mu_s, &mut p.gam_w, &mut p.gam_s, &mut p.gam_g, &mut p.omega, &mut p.alpha_s, &mut p.omega_im] {
                s(v, per_time.0, per_time.1);
            }
            if !dual_neq {
                s(&mut p.mu_g, per_time.0, per_time.1);
            }
            // zero-order production: liquid and gas [M/L³/T], solid [M/M/T]
            s(&mut p.mu0_w, -3, -1);
            s(&mut p.mu0_s, 0, -1);
            s(&mut p.mu0_g, -3, -1);
            for v in [&mut p.r_ka1, &mut p.r_kd1, &mut p.r_ka2, &mut p.r_kd2] {
                s(v, per_time.0, per_time.1);
            }
            s(&mut p.d_c, len.0, len.1);
            s(&mut p.d_p, len.0, len.1);
        }
    }
    for wd in so.w_dep.iter_mut() {
        wd.h_ref.iter_mut().for_each(|x| *x *= fl);
    }

    // ---- profile
    let theta_init = prj.water.init_in_water_content;
    for n in prj.profile.nodes.iter_mut() {
        s(&mut n.depth, len.0, len.1);
        if !theta_init {
            s(&mut n.h, len.0, len.1);
        }
        sv(&mut n.conc, conc.0, conc.1);
        if mobile_immobile {
            sv(&mut n.sorb, conc.0, conc.1); // immobile liquid concentration
        }
    }
    for f in prj.profile.fixed_points.iter_mut() {
        s(&mut f.depth, len.0, len.1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conversion_back_and_forth_restores_the_project() {
        let p0 = crate::examples::infiltration_evaporation();
        let mut p = p0.clone();
        convert_units(&mut p, LengthUnit::M, TimeUnit::Hours);
        assert_eq!((p.units.length, p.units.time), (LengthUnit::M, TimeUnit::Hours));
        assert!((p.water.materials[0].ks - p0.water.materials[0].ks / 100.0 / 24.0).abs() < 1e-12 * p0.water.materials[0].ks);
        assert!((p.water.materials[0].alpha - p0.water.materials[0].alpha * 100.0).abs() < 1e-9);
        assert!((p.time.t_max - p0.time.t_max * 24.0).abs() < 1e-9);
        convert_units(&mut p, p0.units.length, p0.units.time);
        let a = serde_json::to_value(&p0).unwrap();
        let b = serde_json::to_value(&p).unwrap();
        fn close(a: &serde_json::Value, b: &serde_json::Value) -> bool {
            match (a, b) {
                (serde_json::Value::Number(x), serde_json::Value::Number(y)) => {
                    let (x, y) = (x.as_f64().unwrap(), y.as_f64().unwrap());
                    (x - y).abs() <= 1e-12 * x.abs().max(y.abs()).max(1e-300)
                }
                (serde_json::Value::Array(x), serde_json::Value::Array(y)) => x.len() == y.len() && x.iter().zip(y).all(|(a, b)| close(a, b)),
                (serde_json::Value::Object(x), serde_json::Value::Object(y)) => x.len() == y.len() && x.iter().all(|(k, v)| y.get(k).is_some_and(|w| close(v, w))),
                _ => a == b,
            }
        }
        assert!(close(&a, &b));
    }

    #[test]
    fn converted_project_gives_the_same_results() {
        // cm / d -> m / h: the run must describe the same physics (time levels, fluxes, heads)
        let p0 = crate::examples::infiltration_evaporation();
        let mut p1 = p0.clone();
        convert_units(&mut p1, LengthUnit::M, TimeUnit::Hours);
        let run = |p: Project| {
            let mut s = crate::Simulation::new(p).unwrap();
            s.run(|_| true);
            s.res
        };
        let (r0, r1) = (run(p0), run(p1));
        assert_eq!(r0.tlevel.len(), r1.tlevel.len());
        let (a, b) = (r0.tlevel.last().unwrap(), r1.tlevel.last().unwrap());
        assert!((a.t * 24.0 - b.t).abs() < 1e-6 * b.t);
        assert!((a.cum_v_top / 100.0 - b.cum_v_top).abs() < 1e-6 * a.cum_v_top.abs() / 100.0);
        assert!((a.volume / 100.0 - b.volume).abs() < 1e-6 * a.volume / 100.0);
        for (pa, pb) in r0.profiles.iter().zip(&r1.profiles) {
            for (na, nb) in pa.nodes.iter().zip(&pb.nodes) {
                assert!((na.h / 100.0 - nb.h).abs() < 1e-6 * (na.h.abs() / 100.0).max(1e-3), "{} {}", na.h, nb.h);
            }
        }
    }
}
