//! Mobile/immobile (iModel 6) water balance output: "W-volume" is the mobile water only, for the total and for every
//! sub-region (HYDRUS 4.17, Test9 / TEST9a; the 2009 SubReg added the immobile water to the sub-region volumes),
//! "W-volumeI" the immobile water; In-flow and the balance include both.
use hydrus_core::*;

fn project() -> Project {
    let mut p = examples::infiltration_evaporation();
    p.water.model = SoilModel::DualPorosityW;
    p.water.materials = vec![SoilMaterial {
        name: "Mobile-immobile".into(),
        qr: 0.0, qs: 0.2, alpha: 0.04, n: 1.8, ks: 30.0, l: 0.5,
        extra: [0.0, 0.15, 1.0e-3, 0.0, 0.0], // thr_im, ths_im, omega
        ..Default::default()
    }];
    p.processes.root_water_uptake = false;
    p.profile.nodes.iter_mut().for_each(|nd| { nd.mat = 1; nd.beta = 0.0; nd.h = -60.0; });
    p.time.t_max = 10.0;
    p.time.print_times = vec![5.0, 10.0];
    p
}

#[test]
fn w_volume_is_mobile_water_only_and_the_balance_includes_the_immobile_water() {
    let mut sim = Simulation::new(project()).expect("project");
    let st = sim.run(|_| true);
    assert_eq!(st, StepStatus::Finished, "{:?}", sim.res.messages);
    let b0 = &sim.res.balance[0];
    assert!(b0.total.volume_im > 0.0, "immobile water expected");
    // the sub-region volumes are mobile water only, like the total
    let sub_sum: f64 = b0.sub.iter().map(|s| s.volume).sum();
    assert!((sub_sum - b0.total.volume).abs() < 1e-9 * sub_sum.max(1.0),
        "sub-region volumes are mobile water only: {sub_sum} vs total {} (immobile {})", b0.total.volume, b0.total.volume_im);
    // thr = 0, ths = 0.2, ths_im = 0.15: the immobile water is a substantial part of the profile water
    assert!(b0.total.volume_im > 0.3 * b0.total.volume);
    // the balance error stays small, i.e. WatBalT is evaluated with mobile + immobile water
    let worst = sim.res.balance.iter().skip(1).map(|b| b.wat_bal_r).fold(0.0, f64::max);
    println!("mobile-immobile: W-volume {:.5}, W-volumeI {:.5}, worst balance error {worst:.4} %", b0.total.volume, b0.total.volume_im);
    assert!(worst < 3.0, "water balance error {worst} %");
}
