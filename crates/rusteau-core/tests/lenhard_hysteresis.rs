//! Lenhard air-entrapment hysteresis (HYSTER.FOR, iHyst = 3).
use rusteau_core::*;

fn project() -> Project {
    let mut p = examples::infiltration_evaporation();
    p.water.model = SoilModel::VanGenuchten;
    p.water.hysteresis = Hysteresis::Lenhard;
    p.water.init_kappa = -1;
    p.water.materials = vec![SoilMaterial {
        name: "Sand".into(),
        qr: 0.05, qs: 0.40, alpha: 0.02, n: 2.0, ks: 50.0, l: 0.5,
        qm: 0.40, qs_w: 0.36, alpha_w: 0.04, ks_w: 50.0,
        ..Default::default()
    }];
    p.processes.root_water_uptake = false;
    p.profile.nodes.iter_mut().for_each(|nd| { nd.mat = 1; nd.beta = 0.0; nd.h = -30.0; });
    p
}

/// Set the same pressure head in all nodes, evaluate the converged state (iKod = 3) and make it the old state.
fn visit(sim: &mut Simulation, h: f64) -> (f64, f64, f64, i32) {
    for i in 0..sim.n {
        sim.h_new[i] = h;
    }
    sim.lenhard_hyst(0, 3, rusteau_core::lenhard::ThetaTarget::New);
    let out = (sim.th_new[0], sim.con[0], sim.cap[0], sim.kappa[0]);
    for i in 0..sim.n {
        sim.h_old[i] = h;
    }
    out
}

fn sweep(sim: &mut Simulation, from: f64, to: f64, steps: usize) -> Vec<(f64, f64, f64, i32)> {
    (1..=steps).map(|k| visit(sim, from + (to - from) * k as f64 / steps as f64)).collect()
}

#[test]
fn scanning_curves_show_hysteresis() {
    let mut sim = Simulation::new(project()).expect("project");
    // main drying curve from -30 to -300 cm
    let dry = sweep(&mut sim, -30.0, -300.0, 90);
    assert!(dry.iter().all(|r| r.3 == -1), "drying path expected");
    let th_dry_150 = sweep(&mut sim, -300.0, -300.0, 1)[0].0; // end point
    let _ = th_dry_150;
    // retention on the main drying curve at -100 cm (visit again from the same state would change the state, so
    // evaluate with a fresh simulation)
    let th_main_dry = {
        let mut s2 = Simulation::new(project()).unwrap();
        sweep(&mut s2, -30.0, -100.0, 70).last().unwrap().0
    };
    // wetting scanning curve back to -100 cm
    let wet = sweep(&mut sim, -300.0, -100.0, 100);
    assert!(wet.iter().all(|r| r.3 == 1), "wetting path expected after the reversal");
    let th_wet_100 = wet.last().unwrap().0;
    println!("theta(-100): main drying {th_main_dry:.4}, wetting scanning {th_wet_100:.4}");
    assert!(th_wet_100 < th_main_dry - 1e-4, "wetting scanning curve must lie below the drying curve");
    // wetting is monotonic and the conductivity stays in [0, Ks]
    for w in wet.windows(2) {
        assert!(w[1].0 >= w[0].0 - 1e-12, "theta must not decrease while wetting");
    }
    assert!(wet.iter().all(|r| r.1 >= 0.0 && r.1 <= 50.0 + 1e-9 && r.2 >= 0.0));
    // second reversal: drying scanning curve starts from the wetting scanning point
    let dry2 = sweep(&mut sim, -100.0, -200.0, 50);
    assert!(dry2.iter().all(|r| r.3 == -1));
    assert!(dry2[0].0 <= th_wet_100 + 1e-9 && dry2[0].0 > dry2.last().unwrap().0);
}

#[test]
fn air_entrapment_limits_the_wetted_water_content() {
    let mut sim = Simulation::new(project()).unwrap();
    sweep(&mut sim, -30.0, -400.0, 100);
    // rewet to full saturation at the water table (h = 0)
    let wet = sweep(&mut sim, -400.0, 0.0, 200);
    let th_end = wet.last().unwrap().0;
    println!("theta(h=0) after wetting: {th_end:.4}  (saturated 0.40, thsW 0.36)");
    assert!(th_end < 0.40 - 1e-3, "entrapped air must keep theta below the main-drainage saturation");
    // The maximum trapped-air saturation is SARWI = (ths - thsW)/(ths - thr) (HYSTER.FOR; HYDRUS 4.17 Test10 agrees), so a
    // completely dry reversal ends at thr + (1 - SARWI)(ths - thr) = thsW = 0.36; here the minimum apparent saturation was
    // 0.12, so theta ends just above it.  (The older formula (ths - thsW)/ths gave 0.365.)
    assert!(th_end >= 0.36 - 1e-6 && th_end <= 0.36 + 0.002, "theta(h=0) = {th_end}");
}

#[test]
fn initial_branch_flag_is_taken_literally() {
    // IKappa = +1 starts on the main wetting branch, any other value on the main drainage branch
    let mut p = project();
    p.water.init_kappa = 1;
    let sim = Simulation::new(p).unwrap();
    assert_eq!(sim.lenhard_i_hyst, 2);
    let mut p = project();
    p.water.init_kappa = 0;
    let sim = Simulation::new(p).unwrap();
    assert_eq!(sim.lenhard_i_hyst, 1);
}

#[test]
fn infiltration_evaporation_cycles_with_lenhard_hysteresis() {
    let mut p = project();
    p.time.t_max = 12.0;
    p.time.print_times = vec![3.0, 6.0, 9.0, 12.0];
    p.atmosphere.records = vec![
        AtmRecord { t: 3.0, prec: 3.0, evap: 0.0, transp: 0.0, h_crit_a: 10000.0, ..Default::default() },
        AtmRecord { t: 6.0, prec: 0.0, evap: 0.8, transp: 0.0, h_crit_a: 10000.0, ..Default::default() },
        AtmRecord { t: 9.0, prec: 3.0, evap: 0.0, transp: 0.0, h_crit_a: 10000.0, ..Default::default() },
        AtmRecord { t: 12.0, prec: 0.0, evap: 0.8, transp: 0.0, h_crit_a: 10000.0, ..Default::default() },
    ];
    let mut sim = Simulation::new(p).unwrap();
    let st = sim.run(|_| true);
    assert_eq!(st, StepStatus::Finished, "{:?}", sim.res.messages);
    let worst = sim.res.balance.iter().skip(1).map(|b| b.wat_bal_r).fold(0.0, f64::max);
    println!("hysteresis cycles: water balance error {worst:.4} %, path failures {}", sim.lenhard_path_failures);
    assert!(worst < 3.0, "water balance error {worst} %");
    assert_eq!(sim.lenhard_path_failures, 0);
    assert!(sim.th_new.iter().all(|t| t.is_finite() && *t > 0.0));
}
