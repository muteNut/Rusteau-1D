//! Active root solute uptake (Michaelis-Menten, SINK.FOR / SetSSnk) and the choice of the solute with active uptake
//! (HYDRUS 4.17 "Solute with Active Uptake"; the 2009 code allows a single solute only).

use rusteau_core::*;

/// The root example with two identical solutes in the profile and active uptake of solute `active`.
fn project(ns: usize, active: Option<usize>) -> Project {
    let mut p = examples::infiltration_evaporation();
    p.time.t_max = 12.0;
    p.time.print_times = vec![6.0, 12.0];
    p.processes.solute = true;
    let mut sol = SoluteSettings { species: vec![], materials: vec![SoluteMaterial::default(); 2], max_iter: 1, ..Default::default() };
    for j in 0..ns {
        sol.species.push(Species {
            name: format!("S{}", j + 1),
            per_material: vec![SpeciesMaterial { beta: 1.0, ..Default::default() }; 2],
            ..Default::default()
        });
    }
    sol.k_top = -1;
    p.solute = sol;
    for r in p.atmosphere.records.iter_mut() {
        r.c_top = vec![0.0; ns];
        r.c_bot = vec![0.0; ns];
    }
    for nd in p.profile.nodes.iter_mut() {
        nd.conc = vec![1.0; ns];
        nd.sorb = vec![0.0; ns];
    }
    p.root.c_root_max = vec![0.02; ns];
    if let Some(k) = active {
        p.root.l_act_rsu = true;
        p.root.active_solute = k;
        p.root.s_pot = 0.3;
        p.root.r_km = vec![0.5];
        p.root.c_min = vec![0.0];
        p.root.omega_act = vec![1.0];
        // SPot from the cBot column of the solute with active uptake
        for r in p.atmosphere.records.iter_mut() {
            r.c_bot[k] = 0.3;
        }
    }
    p
}

/// Cumulative root uptake of every solute at the end.
fn uptake(p: Project) -> Vec<f64> {
    let mut sim = Simulation::new(p).unwrap();
    while sim.step() == StepStatus::Running {}
    assert!(!sim.res.failed);
    sim.res.tlevel.last().unwrap().solutes.iter().map(|s| s.cum_root.abs()).collect()
}

#[test]
fn active_uptake_goes_to_the_selected_solute() {
    let passive = uptake(project(2, None));
    let a0 = uptake(project(2, Some(0)));
    let a1 = uptake(project(2, Some(1)));
    // identical solutes: equal passive uptake
    assert!((passive[0] - passive[1]).abs() <= 1e-12 * passive[0].max(1e-30), "{passive:?}");
    // the active uptake adds to the selected solute only, and the choice is symmetric
    assert!(a0[0] > passive[0] * 1.05 && (a0[1] - passive[1]).abs() <= 1e-9 * passive[1].max(1e-30), "{a0:?} vs {passive:?}");
    assert!(a1[1] > passive[1] * 1.05 && (a1[0] - passive[0]).abs() <= 1e-9 * passive[0].max(1e-30), "{a1:?} vs {passive:?}");
    assert!((a0[0] - a1[1]).abs() <= 1e-9 * a0[0], "{a0:?} {a1:?}");
}

#[test]
fn single_solute_active_uptake_is_unchanged() {
    // one solute: the selection is the solute itself (the 2009 case)
    let p = project(1, Some(0));
    let a = uptake(p.clone());
    let mut q = p;
    q.root.active_solute = 5; // out of range: clamped to the only solute
    assert_eq!(a, uptake(q));
    assert!(a[0] > uptake(project(1, None))[0] * 1.05);
}
