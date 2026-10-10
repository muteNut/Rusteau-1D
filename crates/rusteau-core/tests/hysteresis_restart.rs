//! Initial hysteresis state (HYDRUS `Hysteresis.in`, HysterIn of INPUT.FOR): a run with Kool & Parker hysteresis
//! continued from the water contents and branches at the end of a previous run.
use rusteau_core::*;

fn project(hyst: Hysteresis) -> Project {
    let mut p = examples::infiltration_evaporation();
    p.water.hysteresis = hyst;
    p.water.init_kappa = -1;
    for m in p.water.materials.iter_mut() {
        m.qm = m.qs;
        m.qs_w = m.qs - 0.03;
        m.alpha_w = 2.0 * m.alpha;
        m.ks_w = if hyst == Hysteresis::RetentionAndConductivity {
            0.7 * m.ks
        } else {
            m.ks
        };
    }
    p
}

fn run(p: Project) -> Simulation {
    let mut sim = Simulation::new(p).expect("project");
    while sim.step() == StepStatus::Running {}
    assert!(!sim.res.failed);
    sim
}

/// The project continued at `t1` from the state of `sim` (time shifted to start at 0).
fn restart(p: &Project, sim: &Simulation, t1: f64) -> Project {
    let mut q = p.clone();
    let n = sim.n;
    for (j, nd) in q.profile.nodes.iter_mut().enumerate() {
        nd.h = sim.h_new[n - 1 - j];
    }
    q.profile.hysteresis = sim.hysteresis_state();
    q.time.t_max -= t1;
    q.time.print_times = p
        .time
        .print_times
        .iter()
        .filter(|&&t| t > t1 + 1e-9)
        .map(|t| t - t1)
        .collect();
    q.atmosphere.records = p
        .atmosphere
        .records
        .iter()
        .filter(|r| r.t > t1 + 1e-9)
        .map(|r| AtmRecord {
            t: r.t - t1,
            ..r.clone()
        })
        .collect();
    q
}

fn first_part(p: &Project, t1: f64) -> Project {
    let mut a = p.clone();
    a.time.t_max = t1;
    a.time.print_times.retain(|&t| t <= t1 + 1e-9);
    a
}

#[test]
fn initial_state_is_the_given_point() {
    for hyst in [Hysteresis::Retention, Hysteresis::RetentionAndConductivity] {
        let p = project(hyst);
        let t1 = 10.0; // during the rain of day 8..12: wetting near the top, drying below
        let sim = run(first_part(&p, t1));
        let st = sim.hysteresis_state().unwrap();
        let wet = st.iter().filter(|s| s.kappa == 1).count();
        println!("{hyst:?}: {wet} of {} nodes wetting at t = {t1}", st.len());
        // with the conductivities of a Rusteau restart, and without them as from a HYDRUS Hysteresis.in
        for with_k in [true, false] {
            let mut q = restart(&p, &sim, t1);
            if !with_k {
                q.profile
                    .hysteresis
                    .iter_mut()
                    .flatten()
                    .for_each(|s| s.k = None);
            }
            let s2 = Simulation::new(q).unwrap();
            let n = s2.n;
            let mut worst: f64 = 0.0;
            for i in 0..n {
                worst = worst.max((s2.th_old[i] - st[n - 1 - i].theta).abs());
                assert!(
                    s2.con[i] > 0.0 && s2.con[i].is_finite(),
                    "{hyst:?}: K = {} at node {i}",
                    s2.con[i]
                );
                // every node starts on the scanning curve of the opposite direction
                assert_eq!(s2.kappa[i], -st[n - 1 - i].kappa);
            }
            println!(
                "{hyst:?} (K stored: {with_k}): largest |θ(t=0) - θ(Hysteresis.in)| = {worst:.2e}"
            );
            assert!(worst < 1e-6, "{hyst:?}: the scanning curves must pass through the given water contents ({worst:e})");
        }
    }
}

#[test]
fn restart_continues_the_run() {
    for hyst in [Hysteresis::Retention, Hysteresis::RetentionAndConductivity] {
        let p = project(hyst);
        let t1 = 10.0;
        let whole = run(p.clone());
        let part = run(first_part(&p, t1));
        let cont = run(restart(&p, &part, t1));
        // without the state the second part starts on the main drying curves
        let mut q = restart(&p, &part, t1);
        q.profile.hysteresis = None;
        let fresh = run(q);
        let n = whole.n;
        let diff = |s: &Simulation| {
            (0..n)
                .map(|i| (s.th_new[i] - whole.th_new[i]).abs())
                .fold(0.0f64, f64::max)
        };
        let (d_cont, d_fresh) = (diff(&cont), diff(&fresh));
        println!("{hyst:?}: max |Δθ| at t = 30: restart {d_cont:.2e}, restart without the state {d_fresh:.2e}");
        assert!(d_cont < 0.01, "{hyst:?}: restart differs by {d_cont}");
        assert!(
            d_cont < 0.5 * d_fresh,
            "{hyst:?}: the hysteresis state must matter ({d_cont} vs {d_fresh})"
        );
    }
}

#[test]
fn state_is_ignored_without_kool_parker_hysteresis() {
    let p = project(Hysteresis::Retention);
    let sim = run(first_part(&p, 12.5));
    let mut q = restart(&p, &sim, 12.5);
    q.water.hysteresis = Hysteresis::None;
    let a = run(q.clone());
    q.profile.hysteresis = None;
    let b = run(q);
    assert_eq!(a.th_new, b.th_new);
}

#[test]
fn state_must_match_the_profile() {
    let mut p = project(Hysteresis::Retention);
    p.profile.hysteresis = Some(vec![
        HysteresisNode {
            theta: 0.2,
            kappa: -1,
            k: None
        };
        p.profile.nodes.len() - 1
    ]);
    assert!(p.validate().iter().any(|e| e.contains("Hysteresis.in")));
    // a new discretization drops the state
    let n = p.profile.nodes.len();
    p.profile.hysteresis = Some(vec![
        HysteresisNode {
            theta: 0.2,
            kappa: -1,
            k: None
        };
        n
    ]);
    p.regrid_uniform(100.0, 51);
    assert!(p.profile.hysteresis.is_none());
}
