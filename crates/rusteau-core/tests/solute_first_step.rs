//! First-step booking of the top solute flux (SOLUTE.FOR `TLevel = 1` rule).
//!
//! SOLUTE.FOR skips the old-level top flux at `TLevel = 1` and books the full new-level flux `vN * cTop` instead of
//! `epsi * vN * cTop`, for every top boundary.  The HYDRUS 4.17 outputs follow it: Test9 / TEST9a (first `cvTop` row = full
//! new-level flux) and ROOTUPTK with the atmospheric top (constant solute balance error `CncBalT1` of -0.0191 from the first
//! step on).  An earlier version used the generic `alf * vO + epsi * vN` booking for the atmospheric top, fitted to the
//! ROOTUPTK output of an older HYDRUS build.
use rusteau_core::*;

/// Dry profile (|vO| << |vN| in the first step) with a prescribed top flux of 2 and a solute concentration of 10.
fn project(atmospheric: bool) -> Project {
    let mut p = examples::solute_pulse();
    for nd in p.profile.nodes.iter_mut() {
        nd.h = -300.0;
    }
    p.time.t_max = 0.05;
    p.time.print_times = vec![0.05];
    if atmospheric {
        p.water.bc = WaterBc { atmospheric: true, top_time_variable: true, kod_top: -1, free_drainage: true, kod_bot: -1, ..Default::default() };
        p.atmosphere.records = vec![
            AtmRecord { t: 1.0, prec: 2.0, evap: 0.0, transp: 0.0, h_crit_a: 10000.0, c_top: vec![10.0], c_bot: vec![0.0], ..Default::default() },
        ];
    }
    p
}

/// (dt, |vTop|, cum_top) of the first time step
fn first_step(p: Project) -> (f64, f64, f64) {
    let mut sim = Simulation::new(p).expect("project");
    assert_eq!(sim.run(|_| true), StepStatus::Finished, "{:?}", sim.res.messages);
    let l = &sim.res.tlevel[0];
    assert_eq!(l.tlevel, 1);
    (l.dt, l.v_top.abs(), l.solutes[0].cum_top)
}

#[test]
fn constant_top_books_the_full_new_level_flux_in_the_first_step() {
    let (dt, v_top, cum) = first_step(project(false));
    println!("constant flux top: dt = {dt}, |vTop| = {v_top}, cum_top = {cum}");
    assert!((v_top - 2.0).abs() < 1e-9);
    assert!((cum - 10.0 * v_top * dt).abs() < 1e-9 * cum.abs().max(1.0), "cum_top {cum} != cTop*|vN|*dt");
}

#[test]
fn atmospheric_top_books_the_full_new_level_flux_in_the_first_step() {
    let (dt, v_top, cum) = first_step(project(true));
    println!("atmospheric top: dt = {dt}, |vTop| = {v_top}, cum_top = {cum}");
    assert!((v_top - 2.0).abs() < 1e-6, "|vTop| {v_top}");
    // the same rule as for the constant top: full new-level flux, no old-level part
    let full = 10.0 * v_top * dt;
    assert!((cum - full).abs() < 1e-6 * full.max(1.0), "cum_top {cum} vs full {full}");
}
