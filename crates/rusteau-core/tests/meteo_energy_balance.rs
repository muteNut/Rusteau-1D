//! Integration tests: meteorological ET, daily-generated meteo (lMetDaily), the surface energy
//! balance with the water-heat coupling loop (iTemp) and the T_Level1 output (lFluxOut).

use rusteau_core::*;

fn base_project() -> Project {
    let mut p = examples::infiltration_evaporation();
    p.processes.root_water_uptake = false;
    for nd in p.profile.nodes.iter_mut() {
        nd.beta = 0.0;
        nd.h = -150.0;
        nd.temp = 15.0;
    }
    p.time.t_max = 8.0;
    p.time.print_times = vec![2.0, 4.0, 6.0, 8.0];
    p.atmosphere.records = (1..=8)
        .map(|i| AtmRecord { t: i as f64, prec: if i == 3 { 2.0 } else { 0.0 }, h_crit_a: 10000.0, ..Default::default() })
        .collect();
    let mut mp = MeteoSettings::default();
    mp.i_radiation = 1;
    mp.i_sun_sh = 3;
    mp.i_crop = 1;
    mp.crop_height = 12.0;
    mp.lai = 2.0;
    mp.i_lai = 1;
    mp.records = (1..=8)
        .map(|i| MeteoRecord {
            t: i as f64,
            rad: 14.0 + (i % 3) as f64,
            t_max: 24.0 + (i % 4) as f64,
            t_min: 12.0,
            rh_mean: 60.0,
            wind_kmd: 180.0,
            sun_hours: 0.0,
            ..Default::default()
        })
        .collect();
    p.atmosphere.meteo = Some(mp);
    p
}

fn run(p: Project) -> Results {
    let mut sim = Simulation::new(p).expect("project");
    let st = sim.run(|_| true);
    assert_eq!(st, StepStatus::Finished, "messages: {:?}", sim.res.messages);
    sim.res
}

fn check_finite(r: &Results) {
    assert!(!r.tlevel.is_empty());
    for t in &r.tlevel {
        assert!(t.v_top.is_finite() && t.h_top.is_finite() && t.cum_evap.is_finite(), "NaN at t={}", t.t);
    }
}

#[test]
fn penman_monteith_with_meteo_runs() {
    let r = run(base_project());
    check_finite(&r);
    let last = r.tlevel.last().unwrap();
    assert!(last.cum_v_top < 0.0, "evaporation expected, cum_v_top = {}", last.cum_v_top);
}

#[test]
fn daily_generated_meteo_runs_and_is_close_to_daily_values() {
    let plain = run(base_project());
    let mut p = base_project();
    p.atmosphere.meteo.as_mut().unwrap().l_daily = true;
    let daily = run(p);
    check_finite(&daily);
    let a = plain.tlevel.last().unwrap().cum_v_top;
    let b = daily.tlevel.last().unwrap().cum_v_top;
    assert!(b < 0.0);
    // daily generation redistributes the same daily totals over the day
    assert!((a - b).abs() < 0.5 * a.abs(), "plain {a} vs daily {b}");
}

#[test]
fn energy_balance_with_coupling_loop_runs() {
    let mut p = base_project();
    p.processes.heat = true;
    p.heat.materials = vec![HeatMaterial::default(); p.water.materials.len()];
    p.heat.k_top = -1;
    p.heat.t_bot = 15.0;
    p.heat.period = 1.0;
    p.atmosphere.meteo.as_mut().unwrap().l_en_bal = true;
    let r = run(p);
    check_finite(&r);
    for t in &r.tlevel {
        assert!(t.temp_top > -40.0 && t.temp_top < 80.0, "unphysical surface temperature {}", t.temp_top);
    }
}

#[test]
fn energy_balance_with_daily_meteo_runs() {
    let mut p = base_project();
    p.processes.heat = true;
    p.heat.materials = vec![HeatMaterial::default(); p.water.materials.len()];
    p.heat.period = 1.0;
    let mp = p.atmosphere.meteo.as_mut().unwrap();
    mp.l_en_bal = true;
    mp.l_daily = true;
    let r = run(p);
    check_finite(&r);
}

#[test]
fn flux_out_produces_tlevel1_and_cumulative_precip() {
    let mut p = base_project();
    p.time.flux_out = true;
    let r = run(p);
    assert!(!r.tlevel1.is_empty());
    let last = r.tlevel1.last().unwrap();
    // 2 cm of precipitation fell in the third day
    assert!(last.cum_prec > 1.0 && last.cum_prec < 3.0, "Sum(PrecP) = {}", last.cum_prec);
}

#[test]
fn flux_out_off_by_default() {
    let r = run(base_project());
    assert!(r.tlevel1.is_empty());
}

#[test]
fn meteo_out_rows_for_all_four_modes() {
    // 0: Penman-Monteith with records
    let r = run(base_project());
    assert_eq!(r.meteo_mode, 0);
    assert_eq!(r.meteo.len(), 8, "one row per meteo record");
    let m = &r.meteo[0];
    assert!(m.et > 0.0 && (m.et - (m.evap_p + m.transp_p)).abs() < 1e-9, "ET = Evap + Transp");
    assert!(m.rad_term.is_finite() && m.aero_term.is_finite() && m.prec >= 0.0);

    // 1: Penman-Monteith with daily-generated data: one row per time step
    let mut p = base_project();
    p.atmosphere.meteo.as_mut().unwrap().l_daily = true;
    let r = run(p);
    assert_eq!(r.meteo_mode, 1);
    assert!(r.meteo.len() > 50 && r.meteo.iter().all(|m| m.air_temp.is_finite() && m.solar_rad >= 0.0));

    // 2 and 3: surface energy balance (closed by construction)
    for daily in [false, true] {
        let mut p = base_project();
        p.processes.heat = true;
        p.heat.materials = vec![HeatMaterial::default(); p.water.materials.len()];
        p.heat.period = 1.0;
        let mp = p.atmosphere.meteo.as_mut().unwrap();
        mp.l_en_bal = true;
        mp.l_daily = daily;
        let r = run(p);
        assert_eq!(r.meteo_mode, if daily { 3 } else { 2 });
        assert!(!r.meteo.is_empty());
        for m in &r.meteo {
            assert!(m.balance.abs() < 0.05, "energy balance residual {} MJ/m2/d at t={}", m.balance, m.t);
        }
    }
}

#[test]
fn moisture_dependence_table_follows_rmd() {
    use rusteau_core::solute::rmd_table;
    let mut tab = MoistDepTable::default();
    tab.react = vec![[1.0, 0.0, 0.0, 0.0, 0.0, 1.0]; 13];
    tab.react[0] = [0.2, 0.1, 0.2, 0.3, 0.4, 0.5]; // ReacMin0, Theta0..Theta3, ReacMin1
    assert!((rmd_table(&tab, 1, 0.05) - 0.2).abs() < 1e-12); // below Theta0
    assert!((rmd_table(&tab, 1, 0.15) - 0.6).abs() < 1e-12); // rising ramp
    assert!((rmd_table(&tab, 1, 0.25) - 1.0).abs() < 1e-12); // plateau
    assert!((rmd_table(&tab, 1, 0.35) - 0.75).abs() < 1e-12); // falling ramp
    assert!((rmd_table(&tab, 1, 0.45) - 0.5).abs() < 1e-12); // above Theta3
    assert!((rmd_table(&tab, 2, 0.05) - 1.0).abs() < 1e-12); // Theta2 = 0: no reduction
    assert!((rmd_table(&tab, 14, 0.05) - 1.0).abs() < 1e-12); // out of range row
}
