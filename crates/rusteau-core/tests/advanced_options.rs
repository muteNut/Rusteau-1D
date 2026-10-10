//! Options of the HYDRUS code without a place in the HYDRUS-1D interface (INPUT.FOR "New Options - Not supported by
//! GUI", the file `Options.in`): lExtrap, iEnhanc, TauW (non-equilibrium water flow), lCentrif (centrifuge) and
//! lDensity (density-dependent flow).
use rusteau_core::*;

fn run(p: Project) -> Simulation {
    let mut sim = Simulation::new(p).expect("project");
    while sim.step() == StepStatus::Running {}
    assert!(!sim.res.failed, "{:?}", sim.res.messages);
    sim
}

/// Largest relative water balance error of the run [%].
fn balance_error(sim: &Simulation) -> f64 {
    sim.res
        .balance
        .iter()
        .skip(1)
        .map(|b| b.wat_bal_r)
        .fold(0.0, f64::max)
}

#[test]
fn defaults_are_hydrus_defaults() {
    let o = AdvancedOptions::default();
    assert!(
        o.extrapolate_heads
            && o.vapor_enhancement
            && o.tau_w == 0.0
            && o.centrifuge.is_none()
            && o.density.is_none()
    );
    // default options are not written to the project file
    let p = examples::infiltration_evaporation();
    let json = serde_json::to_string(&p).unwrap();
    assert!(!json.contains("\"options\""));
    let back: Project = serde_json::from_str(&json).unwrap();
    assert_eq!(back.options, o);
}

#[test]
fn head_extrapolation_off() {
    let p = examples::infiltration_evaporation();
    let a = run(p.clone());
    let mut q = p;
    q.options.extrapolate_heads = false;
    let b = run(q);
    let (va, vb) = (
        a.res.tlevel.last().unwrap().volume,
        b.res.tlevel.last().unwrap().volume,
    );
    println!("volume at the end: extrapolated {va:.5}, not extrapolated {vb:.5}");
    assert!((va - vb).abs() < 0.01 * va);
    assert!(
        a.res.tlevel.len() != b.res.tlevel.len() || a.th_new != b.th_new,
        "the option must change the iteration"
    );
}

#[test]
fn vapor_enhancement_factor() {
    // thermal vapor conductivity of a moist loam at 20 °C with and without the enhancement factor
    let (mut lt, mut vt, mut vh) = (vec![0.0], vec![0.0], vec![0.0]);
    let mut kvt = [0.0; 2];
    for (k, ie) in [1, 0].into_iter().enumerate() {
        vapor::con_vapor(
            1,
            &[0],
            &[-500.0],
            &[20.0],
            &[1e-3],
            &[0.25],
            &[0.43],
            &mut lt,
            &mut vt,
            &mut vh,
            100.0,
            1.0 / 86400.0,
            true,
            ie,
            -1e10,
        );
        kvt[k] = vt[0];
    }
    println!(
        "ConVT with / without enhancement: {:e} / {:e}",
        kvt[0], kvt[1]
    );
    assert!(kvt[0] > 2.0 * kvt[1] && kvt[1] > 0.0);
}

/// Infiltration of 1 cm/d into a dry loam column with free drainage.
fn loam_infiltration(tau_w: f64) -> Project {
    let mut p = examples::solute_pulse();
    p.processes.solute = false;
    p.water.bc.r_top = -1.0;
    for nd in p.profile.nodes.iter_mut() {
        nd.h = -200.0;
    }
    p.time.t_max = 20.0;
    p.time.print_times = vec![5.0, 10.0, 20.0];
    p.options.tau_w = tau_w;
    p
}

#[test]
fn non_equilibrium_water_flow() {
    let eq = run(loam_infiltration(0.0));
    // a time constant far below the time steps is equilibrium flow
    let fast = run(loam_infiltration(1e-12));
    assert_eq!(eq.th_new, fast.th_new);
    // a time constant of one day: the water content lags behind the pressure head
    let mut sim = Simulation::new(loam_infiltration(1.0)).unwrap();
    let mut lag: f64 = 0.0;
    while sim.step() == StepStatus::Running {
        for i in 0..sim.n {
            lag = lag.max((sim.th_new[i] - sim.th_eq[i]).abs());
        }
    }
    assert!(!sim.res.failed, "t = {}: {:?}", sim.t, sim.res.messages);
    let err = balance_error(&sim);
    // water-content profiles at t = 5 d
    let th = |s: &Simulation| {
        s.res
            .profiles
            .iter()
            .find(|p| (p.t - 5.0).abs() < 1e-6)
            .unwrap()
            .nodes
            .iter()
            .map(|n| n.theta)
            .collect::<Vec<f64>>()
    };
    let d = th(&eq)
        .iter()
        .zip(th(&sim))
        .map(|(a, b)| (a - b).abs())
        .fold(0.0f64, f64::max);
    println!("τw = 1 d: largest θ − θeq(h) {lag:.4}, largest difference to equilibrium flow at t = 5 d {d:.4}, balance error {err:.4} %");
    assert!(
        lag > 0.05,
        "the water content must lag behind the equilibrium water content"
    );
    assert!(d > 0.01);
    assert!(err < 0.1, "water balance error {err} %");
}

/// Soil column in a centrifuge: a saturated sand drains through a water-saturated bottom (h = 0) towards the
/// equilibrium h(d) = −ω²/(2g) ((R + L)² − (R + d)²).
fn centrifuge_column(rpm: f64, radius: f64) -> Project {
    let mut p = Project { title: "Centrifuge".into(), ..Default::default() };
    p.units.time = TimeUnit::Hours;
    p.water.materials = vec![SoilMaterial {
        name: "Sand".into(),
        qr: 0.045,
        qs: 0.43,
        alpha: 0.145,
        n: 2.68,
        ks: 29.7,
        l: 0.5,
        ..Default::default()
    }];
    p.water.max_iter = 20;
    p.water.bc = WaterBc {
        kod_top: -1,
        r_top: 0.0,
        kod_bot: 1,
        free_drainage: false,
        ..Default::default()
    };
    p.regrid_uniform(10.0, 101);
    for nd in p.profile.nodes.iter_mut() {
        nd.mat = 1;
        nd.h = 0.0;
    }
    p.time = TimeSettings {
        dt: 1e-4,
        dt_min: 1e-7,
        dt_max: 0.5,
        t_max: 200.0,
        print_times: vec![200.0],
        ..Default::default()
    };
    p.options.centrifuge = Some(Centrifuge { rpm, radius });
    p
}

#[test]
fn centrifuge_equilibrium() {
    let (rpm, r, l) = (100.0, 20.0, 10.0);
    let sim = run(centrifuge_column(rpm, r));
    let w = rpm * 2.0 * std::f64::consts::PI / 60.0;
    let c = w * w / 980.665; // 1/cm
    let n = sim.n;
    let mut worst: f64 = 0.0;
    for i in 0..n {
        let d = -sim.x[i];
        let h = -c / 2.0 * ((r + l).powi(2) - (r + d).powi(2));
        worst = worst.max((sim.h_new[i] - h).abs());
    }
    let h_top = -c * (r * l + l * l / 2.0);
    println!("centrifuge {rpm} rpm, R = {r} cm: h(top) {:.2} cm (analytic {h_top:.2}), largest deviation {worst:.3} cm", sim.h_new[n - 1]);
    assert!(
        worst < 0.01 * h_top.abs(),
        "equilibrium profile deviates by {worst} cm"
    );
    let b = sim.res.balance.last().unwrap();
    let drained = sim.res.balance[0].total.volume - b.total.volume;
    println!("drained {drained:.4} cm, balance error {:.4} cm ({:.2} %)", b.wat_bal_t, b.wat_bal_r);
    assert!(balance_error(&sim) < 0.1, "balance error {} %", balance_error(&sim));
    assert!(sim.res.tlevel.last().unwrap().v_bot.abs() < 1e-6, "no outflow at equilibrium");
    // the same column under gravity drains much less
    let mut g = centrifuge_column(rpm, r);
    g.options.centrifuge = None;
    let gs = run(g);
    let b = gs.res.balance.last().unwrap();
    println!("gravity: drained {:.4} cm, balance error {:.4} cm ({:.2} %)", gs.res.balance[0].total.volume - b.total.volume, b.wat_bal_t, b.wat_bal_r);
    assert!(
        gs.h_new[n - 1] > -l - 0.5 && gs.h_new[n - 1] < -l + 0.5,
        "gravity: h(top) = {}",
        gs.h_new[n - 1]
    );
}

#[test]
fn centrifuge_speed_from_the_bottom_boundary() {
    // time-variable bottom condition: the hBot column is the rotation speed, the bottom head 0
    let mut p = centrifuge_column(100.0, 20.0);
    p.water.bc.bot_time_variable = true;
    p.water.bc.kod_bot = 1;
    let rec = |t: f64, rpm: f64| AtmRecord {
        t,
        h_bot: rpm,
        h_crit_a: 1e5,
        ..Default::default()
    };
    p.atmosphere.records = vec![rec(100.0, 50.0), rec(200.0, 100.0)];
    let sim = run(p);
    let steady = run(centrifuge_column(100.0, 20.0));
    let n = sim.n;
    // first 100 h at 50 rpm, then 100 rpm: ends close to the equilibrium of 100 rpm, starting from a wetter state
    assert!(
        (sim.h_new[n - 1] - steady.h_new[n - 1]).abs() < 0.05 * steady.h_new[n - 1].abs(),
        "{} vs {}",
        sim.h_new[n - 1],
        steady.h_new[n - 1]
    );
    assert!(sim.h_new[0].abs() < 1e-9, "bottom head {}", sim.h_new[0]);
    let mid = sim.res.tlevel.iter().rfind(|l| l.t <= 100.0).unwrap();
    let end = sim.res.tlevel.last().unwrap();
    assert!(
        mid.volume > end.volume + 0.05,
        "{} {}",
        mid.volume,
        end.volume
    );
}

#[test]
fn density_dependent_flow() {
    let mut p = examples::solute_pulse();
    p.time.t_max = 20.0;
    p.time.print_times = vec![10.0, 20.0];
    let base = run(p.clone());
    // neutral polynomials: exactly the run without the option
    let mut q = p.clone();
    q.options.density = Some(DensityDependence {
        density: [1.0, 0.0, 0.0, 0.0],
        viscosity: [1.0, 0.0, 0.0, 0.0],
    });
    let same = run(q);
    assert_eq!(base.th_new, same.th_new);
    assert_eq!(
        base.res.tlevel.last().unwrap().cum_v_bot,
        same.res.tlevel.last().unwrap().cum_v_bot
    );
    // a dense solution (ρ = 1 + 0.03 c, c up to 10): larger conductivity and gravity term, the pulse moves deeper
    let d = DensityDependence {
        density: [1.0, 0.03, 0.0, 0.0],
        viscosity: [1.0, 0.0, 0.0, 0.0],
    };
    let mut q = p;
    q.options.density = Some(d);
    let dense = run(q);
    let centre = |s: &Simulation| {
        let c = &s.sol.as_ref().unwrap().conc[0];
        let (mut m, mut md) = (0.0, 0.0);
        for ((ci, th), x) in c.iter().zip(&s.th_new).zip(&s.x) {
            m += ci * th;
            md += ci * th * -x;
        }
        md / m
    };
    let (zb, zd) = (centre(&base), centre(&dense));
    // HYDRUS solves ρ ∂θ/∂t = ∂/∂z[K (∂h/∂z + ρ)] − S (Reset of WATFLOW.FOR), while Balance.out sums ρθ: the balance
    // misses ∫θ (ρ − ρ0) dz, the solution mass gained by the change of the density
    let c = &dense.sol.as_ref().unwrap().conc[0];
    let mut gained = 0.0;
    for i in 0..dense.n - 1 {
        let dx = dense.x[i + 1] - dense.x[i];
        gained += dx
            * (dense.th_new[i] * (d.rho(c[i]) - 1.0)
                + dense.th_new[i + 1] * (d.rho(c[i + 1]) - 1.0))
            / 2.0;
    }
    let wbt = dense.res.balance.last().unwrap().wat_bal_t;
    println!("solute centre of mass at t = 20 d: {zb:.2} cm, dense solution {zd:.2} cm; balance {wbt:.4} cm, mass gained by the density {gained:.4} cm");
    assert!(
        zd > zb + 0.3,
        "the dense solution must move deeper ({zd} vs {zb})"
    );
    assert!(
        (wbt - gained).abs() < 0.1 * gained,
        "the balance error must be the density change ({wbt} vs {gained})"
    );
}

#[test]
fn validation() {
    let mut p = examples::infiltration_evaporation();
    p.options.tau_w = -1.0;
    assert!(p.validate().iter().any(|e| e.contains("τw")));
    p.options.tau_w = 1.0;
    p.water.model = SoilModel::DualPermeability;
    assert!(p.validate().iter().any(|e| e.contains("dual-permeability")));
    p.water.model = SoilModel::VanGenuchten;
    p.options.centrifuge = Some(Centrifuge {
        rpm: 0.0,
        radius: 1.0,
    });
    assert!(p.validate().iter().any(|e| e.contains("Centrifuge")));
}
