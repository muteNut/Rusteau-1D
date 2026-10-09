//! Non-equilibrium solute transport: the immobile / kinetic phase must relax to the mobile
//! concentration (mobile-immobile) or to the kinetic-site equilibrium (two-site model).
use rusteau_core::*;

fn project(th_im: f64, frac: f64, ks: f64, omega: f64) -> Project {
    let mut p = examples::solute_pulse();
    p.time.t_max = 40.0;
    p.time.print_times = vec![20.0, 40.0];
    p.solute.t_pulse = 1.0e6; // keep the top concentration constant
    p.solute.materials = vec![SoluteMaterial { bulk_density: 1.3, disp_l: 2.0, frac, th_immobile: th_im, th_im_matrix: 0.0 }];
    p.solute.species[0].per_material = vec![SpeciesMaterial { ks, beta: 1.0, omega, ..Default::default() }];
    p.solute.species[0].c_top = 1.0;
    for nd in p.profile.nodes.iter_mut() {
        nd.conc = vec![1.0];
        nd.sorb = vec![0.0];
    }
    p
}

fn run(p: Project) -> Simulation {
    let mut sim = Simulation::new(p).expect("project");
    assert_eq!(sim.run(|_| true), StepStatus::Finished, "{:?}", sim.res.messages);
    sim
}

#[test]
fn mobile_immobile_sorb_relaxes_to_mobile_concentration() {
    let sim = run(project(0.1, 1.0, 0.0, 0.5));
    let mid = sim.n / 2;
    let (c, s) = (sim.conc(0, mid), sim.sorb(0, mid));
    assert!((c - 1.0).abs() < 0.05, "mobile conc {c}");
    assert!((s - c).abs() < 0.03 * c, "immobile conc {s} should approach the mobile conc {c}");
}

#[test]
fn two_site_sorb_relaxes_to_kinetic_site_equilibrium() {
    // Sorb = (1-Frac) * Ks * c at equilibrium
    let (frac, ks) = (0.5, 0.4);
    let sim = run(project(0.0, frac, ks, 0.5));
    let mid = sim.n / 2;
    let (c, s) = (sim.conc(0, mid), sim.sorb(0, mid));
    let expect = (1.0 - frac) * ks * c;
    assert!((s - expect).abs() < 0.05 * expect, "sorb {s} expected {expect}");
}

fn pulse_project(th_im: f64, frac: f64, ks: f64, beta: f64, nu: f64) -> Project {
    let mut p = project(th_im, frac, ks, 0.3);
    p.solute.t_pulse = 10.0;
    p.time.t_max = 40.0;
    p.time.print_times = vec![5.0, 10.0, 20.0, 30.0, 40.0];
    p.solute.species[0].per_material = vec![SpeciesMaterial { ks, beta, nu, omega: 0.3, mu_w: 0.01, mu_s: 0.01, ..Default::default() }];
    for nd in p.profile.nodes.iter_mut() {
        nd.conc = vec![0.0];
        nd.sorb = vec![0.0];
    }
    p
}

fn max_bal_err(sim: &Simulation) -> f64 {
    sim.res.balance.iter().skip(1).map(|b| b.sol_bal_r.first().copied().unwrap_or(0.0)).fold(0.0, f64::max)
}

#[test]
fn solute_mass_balance_closes_for_all_sorption_models() {
    let cases = [
        ("equilibrium linear", pulse_project(0.0, 1.0, 0.3, 1.0, 0.0)),
        ("two-site", pulse_project(0.0, 0.5, 0.4, 1.0, 0.0)),
        ("mobile-immobile", pulse_project(0.1, 1.0, 0.0, 1.0, 0.0)),
        ("mobile-immobile + sorption", pulse_project(0.1, 0.6, 0.3, 1.0, 0.0)),
        ("freundlich", pulse_project(0.0, 1.0, 0.3, 0.8, 0.0)),
        ("two-site freundlich", pulse_project(0.0, 0.5, 0.3, 0.8, 0.0)),
    ];
    for (name, p) in cases {
        let sim = run(p);
        let e = max_bal_err(&sim);
        println!("{name}: max solute balance error {e:.4} %");
        assert!(e < 2.0, "{name}: solute mass balance error {e} %");
    }
}

#[test]
fn dual_non_equilibrium_mass_balance_and_limits() {
    // lDualNEq: mobile-immobile model plus kinetic sites in the mobile zone
    // (f_em = ChPar(13) -> mu_g, OmegaS = ChPar(16) -> gam_g)
    let mut p = pulse_project(0.1, 0.6, 0.3, 1.0, 0.0);
    p.solute.l_dual_neq = true;
    p.solute.species[0].per_material[0].mu_g = 0.5;
    p.solute.species[0].per_material[0].gam_g = 0.2;
    let sim = run(p);
    let e = max_bal_err(&sim);
    println!("dual non-equilibrium: max balance error {e:.4} %");
    assert!(e < 2.0, "dual non-equilibrium balance error {e} %");
    let mid = sim.n / 2;
    assert!(sim.conc(0, mid).is_finite() && sim.sorb(0, mid) >= 0.0);
}

#[test]
fn filtration_model_mass_balance() {
    // colloid/bacteria transport: two attachment sites, first-order detachment
    let mut p = pulse_project(0.0, 1.0, 0.0, 1.0, 0.0);
    p.solute.l_bact = true;
    {
        let m = &mut p.solute.species[0].per_material[0];
        m.s_max1 = 0.0;
        m.s_max2 = 0.0;
        m.r_ka1 = 0.05;
        m.r_kd1 = 0.02;
        m.r_ka2 = 0.02;
        m.r_kd2 = 0.01;
        m.mu_w = 0.0;
        m.mu_s = 0.0;
    }
    let sim = run(p);
    let e = max_bal_err(&sim);
    println!("filtration model: max balance error {e:.4} %");
    assert!(e < 3.0, "filtration model balance error {e} %");
    let mid = sim.n / 2;
    assert!(sim.sorb(0, mid) > 0.0, "attachment must take up solute");
}

#[test]
fn mass_initial_condition_conserves_the_given_mass() {
    // Initial condition given as total mass per soil volume: theta*c + rho*Ks*c + (ths-theta)*KH*c = mass
    let mut p = pulse_project(0.0, 1.0, 0.3, 1.0, 0.0);
    p.solute.mass_init = true;
    p.solute.species[0].per_material[0].henry = 0.01;
    p.solute.t_pulse = 0.0;
    p.solute.species[0].c_top = 0.0;
    for nd in p.profile.nodes.iter_mut() {
        nd.conc = vec![1.0];
    }
    let mut sim = Simulation::new(p).expect("project");
    let _ = sim.step();
    let i = sim.n / 2;
    // after the first (tiny) step the concentration is the mass divided by the capacity of the soil
    let (th, ro, ks, kh) = (sim.th_new[i], 1.3, 0.3, 0.01);
    let ths = sim.ths[sim.mat[i]];
    let c0 = 1.0 / (th + ro * ks + (ths - th) * kh);
    assert!((sim.conc(0, i) - c0).abs() < 0.02 * c0, "c = {}, expected about {}", sim.conc(0, i), c0);
}

#[test]
fn solute_t_level_has_groundwater_and_runoff_columns() {
    let sim = run(pulse_project(0.0, 1.0, 0.3, 1.0, 0.0));
    let last = sim.res.tlevel.last().unwrap();
    assert!(last.solutes[0].c_gwl.is_finite() && last.solutes[0].c_run_off.is_finite() && last.solutes[0].cum_run_off.is_finite());
}

#[test]
fn dual_non_equilibrium_zero_order_production_is_conserved() {
    // mobile-immobile model plus kinetic sites with f < 1 and f_em < 1: the zero-order production on the solid phase
    // must be split without creating or losing mass (Frac f_em equilibrium + Frac (1-f_em) kinetic + (1-Frac) immobile)
    let mut p = pulse_project(0.1, 0.5, 0.3, 1.0, 0.0);
    p.solute.l_dual_neq = true;
    p.solute.species[0].c_top = 0.0;
    let m = &mut p.solute.species[0].per_material[0];
    m.mu_g = 0.5; // f_em
    m.gam_g = 0.2; // omega_S
    m.mu0_s = 0.02; // zero-order production on the solid phase
    m.mu_w = 0.0;
    m.mu_s = 0.0;
    let sim = run(p);
    let worst = max_bal_err(&sim);
    println!("dual non-equilibrium zero-order production: balance error {worst:.4} %");
    assert!(worst < 2.0, "balance error {worst} %");
    assert!(sim.res.balance.last().unwrap().total.c_vol_im[0] > 0.0, "kinetic and immobile sites must hold produced mass");
}

#[test]
fn decay_chain_daughter_uses_the_old_concentration_of_its_parent() {
    // Uniform parent concentration, first-order chain decay k = 0.2 /d, no transport to speak of in 0.5 d at mid-depth:
    //   parent(t) = c0 exp(-k t),  daughter(t) = c0 (1 - exp(-k t)).
    // The Crank-Nicolson production of the daughter at the old time level must use the OLD concentration of the PARENT; using
    // the daughter's own old concentration (about zero) halves the production.
    let mut p = pulse_project(0.0, 1.0, 0.0, 1.0, 0.0);
    let mut daughter = p.solute.species[0].clone();
    daughter.name = "daughter".into();
    daughter.c_top = 0.0;
    p.solute.species[0].per_material[0].gam_w = 0.2;
    p.solute.species[0].per_material[0].mu_w = 0.0;
    daughter.per_material[0].gam_w = 0.0;
    daughter.per_material[0].mu_w = 0.0;
    p.solute.species.push(daughter);
    for nd in p.profile.nodes.iter_mut() {
        nd.conc = vec![1.0, 0.0];
        nd.sorb = vec![0.0, 0.0];
    }
    p.solute.species[0].c_top = 1.0;
    p.time.t_max = 0.5;
    p.time.print_times = vec![0.5];
    let sim = run(p);
    let i = sim.n / 2;
    let (cp, cd) = (sim.conc(0, i), sim.conc(1, i));
    let (ep, ed) = ((-0.1f64).exp(), 1.0 - (-0.1f64).exp());
    println!("parent {cp:.5} (exact {ep:.5}), daughter {cd:.5} (exact {ed:.5})");
    assert!((cp - ep).abs() < 0.01, "parent {cp} vs {ep}");
    assert!((cd - ed).abs() < 0.03 * ed, "daughter {cd} vs {ed}: the production of the old time level is wrong");
}

#[test]
fn hydraulic_property_tables_reproduce_the_analytic_functions_closely() {
    use rusteau_core::material::{fc, fk, fq};
    let mut a = examples::infiltration_evaporation();
    a.processes.root_water_uptake = false;
    a.water.use_tables = false;
    let mut b = a.clone();
    b.water.use_tables = true;
    b.water.h_tab1 = 0.001;
    b.water.h_tab_n = 100.0; // typical HYDRUS-1D values: table between -0.001 and -100 cm
    let (sa, sb) = (Simulation::new(a).unwrap(), Simulation::new(b).unwrap());
    assert!(sa.h_tab.is_empty() && !sa.l_table);
    assert!(sb.l_table && sb.h_tab.len() == 100);
    assert!((sb.h_tab[0] + 0.001).abs() < 1e-12 && (sb.h_tab[99] + 100.0).abs() < 1e-9);
    // table values equal the functions at the nodes, and the interpolated values deviate only slightly in between
    let par = sb.par_d[0];
    for k in [0usize, 10, 50, 99] {
        assert!((sb.con_tab[0][k] - fk(sb.model, sb.h_tab[k], &par)).abs() <= 1e-9 * sb.con_tab[0][k].abs() + 1e-30);
        assert!((sb.cap_tab[0][k] - fc(sb.model, sb.h_tab[k], &par)).abs() <= 1e-9 * sb.cap_tab[0][k].abs() + 1e-30);
        assert!((sb.the_tab[0][k] - fq(sb.model, sb.h_tab[k], &par)).abs() <= 1e-9 * sb.the_tab[0][k].abs() + 1e-30);
    }
    // simulations with and without tables agree to well below one percent but are not identical
    let (ra, rb) = {
        let mut p = examples::infiltration_evaporation();
        p.processes.root_water_uptake = false;
        p.time.t_max = 4.0;
        p.time.print_times = vec![4.0];
        let mut q = p.clone();
        q.water.use_tables = true;
        q.water.h_tab1 = 0.001;
        q.water.h_tab_n = 100.0;
        (run(p), run(q))
    };
    let n = ra.n;
    let d: f64 = (0..n).map(|i| (ra.h_new[i] - rb.h_new[i]).abs()).fold(0.0, f64::max);
    let scale: f64 = (0..n).map(|i| ra.h_new[i].abs()).fold(0.0, f64::max);
    println!("max head difference with tables: {d:.4} of {scale:.1}");
    assert!(d < 0.02 * scale.max(1.0), "tables change the solution too much: {d}");
    assert!(d > 0.0, "tables must be active");
}

#[test]
fn printed_top_node_flux_equals_the_boundary_flux() {
    // NodOut includes the storage term of the top half-element: for a flux boundary the printed flux of the top node is the boundary
    // flux (HYDRUS 2HYSTER prints -9 for rTop = -9).
    let mut p = examples::infiltration_evaporation();
    p.processes.root_water_uptake = false;
    p.water.bc = WaterBc { kod_top: -1, r_top: -0.5, free_drainage: true, kod_bot: -1, ..Default::default() };
    p.atmosphere.records.clear();
    for nd in p.profile.nodes.iter_mut() {
        nd.mat = 1;
        nd.beta = 0.0;
        nd.h = -60.0;
    }
    p.time.t_max = 1.0;
    p.time.print_times = vec![1.0];
    let sim = run(p);
    let last = sim.res.profiles.last().unwrap();
    let top = last.nodes.iter().find(|n| n.node == 1).unwrap();
    println!("top node flux {:e}", top.flux);
    assert!((top.flux + 0.5).abs() < 5e-3, "top-node flux {} should be the boundary flux -0.5", top.flux);
}
