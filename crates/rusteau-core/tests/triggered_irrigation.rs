//! Triggered irrigation (HYDRUS 4.17 `lIrrig`), rules taken from the 4.17 run of 5SEASON with triggered irrigation
//! (observation node 1, -200 cm, 2 cm/d, duration 1 d, lag 1 d; Irrig.out, T_Level.out, A_Level.out):
//! - the irrigation is triggered at the first time level with the head at the observation node below the threshold and
//!   not again before it has ended; it starts `lag` later and ends `duration` after the start (both are time levels and
//!   have an A_Level line);
//! - it replaces the precipitation in the potential top flux, rTop = |rSoil| - rate;
//! - an atmospheric record read during the irrigation sets its own top flux, and the end sets rTop = |rSoil| until the
//!   next record (the precipitation of that record is dropped from rTop).
use rusteau_core::*;

fn project() -> Project {
    let mut p = examples::infiltration_evaporation();
    p.processes.root_water_uptake = false;
    for nd in p.profile.nodes.iter_mut() {
        nd.beta = 0.0;
    }
    p.profile.observation_nodes = vec![6, 51];
    p.time.t_max = 12.0;
    p.time.dt_max = 0.25;
    p.time.print_times = vec![6.0, 12.0];
    p.atmosphere.records =
        (1..=12).map(|d| AtmRecord { t: d as f64, prec: 0.1, evap: 0.6, h_crit_a: 10000.0, ..Default::default() }).collect();
    p.water.bc.irrigation =
        Some(TriggeredIrrigation { obs_node: 1, h_trigger: -250.0, rate: 3.0, duration: 2.0, lag: 0.5, mode: IrrigationMode::Hydrus417 });
    p
}

fn near(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-6
}

#[test]
fn triggered_irrigation_follows_hydrus_4_17() {
    let mut sim = Simulation::new(project()).expect("project");
    assert_eq!(sim.run(|_| true), StepStatus::Finished, "{:?}", sim.res.messages);
    let r = &sim.res;
    assert!(!r.irrigation.is_empty(), "no irrigation was triggered");

    let obs_h = |k: usize| r.obs[k].points[0].h;
    let mut previous_end = f64::NEG_INFINITY;
    for ev in &r.irrigation {
        assert!(near(ev.t_start, ev.t_trigger + 0.5) && near(ev.t_end, ev.t_start + 2.0) && near(ev.amount, 6.0), "{ev:?}");
        // first time level below the threshold after the previous irrigation
        let k = r.obs.iter().position(|o| near(o.t, ev.t_trigger)).expect("observation at the trigger time");
        assert!(obs_h(k) < -250.0, "head at the trigger {}", obs_h(k));
        assert!(k == 0 || obs_h(k - 1) >= -250.0 || r.obs[k - 1].t <= previous_end + 1e-6, "not the first level below -250");
        assert!(ev.t_trigger >= previous_end - 1e-6, "triggered before the previous irrigation ended");
        previous_end = ev.t_end;

        // start and end are time levels with an A_Level line
        for te in [ev.t_start, ev.t_end] {
            if te < 12.0 {
                assert!(r.tlevel.iter().any(|tl| near(tl.t, te)), "no time level at {te}");
                assert!(r.alevel.iter().any(|a| near(a.t, te)), "no A_Level line at {te}");
            }
        }
        // potential top flux: rate in place of the precipitation until the next record, the record's own value
        // afterwards, and |rSoil| (no precipitation) from the end to the next record
        let next_record = ev.t_start.floor() + 1.0;
        for tl in r.tlevel.iter().filter(|tl| tl.t > ev.t_start + 1e-6 && tl.t <= ev.t_end.min(12.0) + 1e-6) {
            let expected = if tl.t <= next_record + 1e-6 { 0.6 - 3.0 } else { 0.6 - 0.1 };
            assert!(near(tl.r_top, expected), "rTop {} at t = {} (expected {expected})", tl.r_top, tl.t);
        }
        let after = ev.t_end.floor() + 1.0;
        for tl in r.tlevel.iter().filter(|tl| tl.t > ev.t_end + 1e-6 && tl.t <= after.min(12.0) + 1e-6) {
            assert!(near(tl.r_top, 0.6), "rTop {} at t = {} after the end (expected 0.6)", tl.r_top, tl.t);
        }
    }
}

#[test]
fn irrigation_needs_an_atmospheric_top() {
    // constant-flux top: the head at the observation node is below the threshold (0) all the time, but 4.17 offers
    // triggered irrigation only with the atmospheric boundary condition
    let mut p = project();
    let irr = TriggeredIrrigation { h_trigger: 0.0, ..p.water.bc.irrigation.clone().unwrap() };
    p.water.bc = WaterBc { kod_top: -1, r_top: -0.2, free_drainage: true, kod_bot: -1, irrigation: Some(irr), ..Default::default() };
    let mut sim = Simulation::new(p).expect("project");
    assert_eq!(sim.run(|_| true), StepStatus::Finished, "{:?}", sim.res.messages);
    assert!(sim.res.irrigation.is_empty());
    assert!(sim.res.tlevel.iter().all(|tl| (tl.r_top + 0.2).abs() < 1e-9));
}

#[test]
fn full_duration_irrigation_adds_to_the_rain() {
    // IrrigationMode::FullDuration: the rate is added to the precipitation from the start to the end, across the
    // records; the records' precipitation is never dropped
    let mut p = project();
    p.water.bc.irrigation.as_mut().unwrap().mode = IrrigationMode::FullDuration;
    let mut sim = Simulation::new(p).expect("project");
    assert_eq!(sim.run(|_| true), StepStatus::Finished, "{:?}", sim.res.messages);
    let r = &sim.res;
    assert!(!r.irrigation.is_empty(), "no irrigation was triggered");
    let irrigating = |t: f64| r.irrigation.iter().any(|ev| t > ev.t_start + 1e-6 && t <= ev.t_end + 1e-6);
    for tl in &r.tlevel {
        let expected = if irrigating(tl.t) { 0.6 - 0.1 - 3.0 } else { 0.6 - 0.1 };
        assert!(near(tl.r_top, expected), "rTop {} at t = {} (expected {expected})", tl.r_top, tl.t);
    }
    // the full amount is applied: rate * duration per event that ended before tMax
    let applied: f64 = r
        .tlevel
        .iter()
        .filter(|tl| irrigating(tl.t))
        .map(|tl| 3.0 * tl.dt)
        .sum();
    let nominal: f64 = r.irrigation.iter().map(|ev| 3.0 * (ev.t_end.min(12.0) - ev.t_start)).sum();
    assert!((applied - nominal).abs() < 1e-6, "applied {applied}, nominal {nominal}");
    // irrigation water counts as precipitation in the infiltration bookkeeping
    let mut p = project();
    p.water.bc.irrigation = None;
    let mut dry = Simulation::new(p).expect("project");
    dry.run(|_| true);
    assert!(r.tlevel.last().unwrap().cum_infil > dry.res.tlevel.last().unwrap().cum_infil + 0.5 * nominal);
}
