//! Dual-permeability (Gerke & van Genuchten) model: water balance, exchange and solute transport.
use hydrus_core::*;

fn project() -> Project {
    let mut p = examples::infiltration_evaporation();
    p.processes.root_water_uptake = false;
    p.water.model = SoilModel::DualPermeability;
    // primary columns: fracture domain; `extra`: matrix domain thr, ths, alpha, n, Ks
    p.water.materials = vec![SoilMaterial {
        name: "Structured loam".into(),
        qr: 0.0, qs: 0.15, alpha: 0.1, n: 2.0, ks: 200.0, l: 0.5,
        extra: [0.08, 0.40, 0.02, 1.5, 10.0],
        ..Default::default()
    }];
    p.profile.nodes.iter_mut().for_each(|nd| { nd.mat = 1; nd.beta = 0.0; nd.h = -100.0; });
    p.water.dual_perm = DualPermSettings { w: 0.15, flux_frac_fracture: -1.0, matrix_dual_por: false, matrix_dual_por_head: false, materials: vec![DualPermMaterial { a: 5.0, ..Default::default() }] };
    p.time.t_max = 10.0;
    p.time.print_times = vec![2.0, 5.0, 10.0];
    p.atmosphere.records = vec![
        AtmRecord { t: 2.0, prec: 4.0, evap: 0.0, transp: 0.0, h_crit_a: 10000.0, ..Default::default() },
        AtmRecord { t: 10.0, prec: 0.0, evap: 0.2, transp: 0.0, h_crit_a: 10000.0, ..Default::default() },
    ];
    p
}

fn run(p: Project) -> Simulation {
    let mut sim = Simulation::new(p).expect("project");
    assert_eq!(sim.run(|_| true), StepStatus::Finished, "{:?}", sim.res.messages);
    sim
}

#[test]
fn dual_permeability_water_balance_closes() {
    let sim = run(project());
    let worst = sim.res.balance.iter().skip(1).map(|b| b.wat_bal_r).fold(0.0, f64::max);
    println!("water balance error {worst:.4} %");
    assert!(worst < 2.0, "water balance error {worst} %");
    assert!(sim.res.tlevel.last().unwrap().cum_v_top.is_finite());
}

#[test]
fn exchange_coefficient_controls_equilibration() {
    // a large interface path length (small alpha_w) keeps the domains apart for longer
    let gap = |a: f64| {
        let mut p = project();
        p.water.dual_perm.materials[0].a = a;
        let sim = run(p);
        let n = sim.n;
        let mut s = 0.0;
        for i in 0..n { s += (sim.h_new[i] - sim.h_matrix_new[i]).abs(); }
        s / n as f64
    };
    let (tight, loose) = (gap(1.0), gap(50.0));
    println!("mean |h_f - h_m|: a=1 -> {tight:.3}, a=50 -> {loose:.3}");
    assert!(loose > tight, "weaker coupling must leave a larger head difference");
}

/// (fracture, matrix) pressure head at the surface after the infiltration / evaporation run for a given fraction of the
/// boundary flux that enters the fractures (negative: proportional to the volume fractions).
fn top_heads(f: f64) -> (f64, f64) {
    let mut p = project();
    p.water.dual_perm.flux_frac_fracture = f;
    let sim = run(p);
    let n = sim.n;
    (sim.h_new[n - 1], sim.h_matrix_new[n - 1])
}

#[test]
fn boundary_flux_partition_changes_fracture_response() {
    // Sending all boundary flux through the small fracture domain concentrates the evaporation demand there (flux per
    // fracture area = f / w): the fracture surface dries out much more than when the flux goes into the matrix.
    let (frac_all, mat_all) = top_heads(1.0);
    let (frac_none, mat_none) = top_heads(0.0);
    let (frac_prop, mat_prop) = top_heads(-1.0);
    println!("f=1: ({frac_all:.1}, {mat_all:.1})  f=0: ({frac_none:.1}, {mat_none:.1})  proportional: ({frac_prop:.1}, {mat_prop:.1})");
    for h in [frac_all, mat_all, frac_none, mat_none, frac_prop, mat_prop] {
        assert!(h.is_finite());
    }
    assert!(frac_all < frac_none - 100.0, "fracture surface must be much drier with f = 1 ({frac_all}) than with f = 0 ({frac_none})");
    assert!(mat_none < mat_all + 1e-6 || (mat_none - mat_all).abs() < 50.0, "the matrix hardly feels the fracture's dry surface");
}

#[test]
fn matrix_top_boundary_switches_independently_of_the_fracture() {
    // f = 1: the fracture has to deliver the whole evaporation, reaches its dry limit and switches to a head condition.
    // That must not force the matrix (which receives no evaporation demand) onto the same head. This case used to end with
    // "The numerical solution has not converged" because the matrix was forced to the fracture's surface head.
    let (frac, mat) = top_heads(1.0);
    assert!(frac < -1000.0, "fracture surface head {frac}");
    assert!(mat > -1000.0, "matrix surface head {mat} must stay moist");
}

fn solute_project(c_top: f64, alpha_s: f64, bc_flux: bool) -> Project {
    let mut p = project();
    p.processes.solute = true;
    let base = examples::solute_pulse();
    p.solute = base.solute.clone();
    // (with the atmospheric top the record concentrations stay in force after tPulse, see
    // `atmospheric_top_keeps_the_record_concentration_after_tpulse`)
    p.solute.t_pulse = 100.0;
    p.solute.k_top = if bc_flux { -1 } else { 1 };
    p.solute.materials = vec![SoluteMaterial { bulk_density: 1.3, disp_l: 2.0, frac: 1.0, th_immobile: 0.0, th_im_matrix: 0.0 }];
    p.solute.species[0].per_material = vec![SpeciesMaterial { ks: 0.2, beta: 1.0, alpha_s, mu_w: 0.01, ..Default::default() }];
    p.solute.species[0].c_top = c_top;
    for nd in p.profile.nodes.iter_mut() { nd.conc = vec![0.0]; nd.sorb = vec![0.0]; }
    // with an atmospheric top boundary the solute concentrations come from the atmospheric records
    for r in p.atmosphere.records.iter_mut() {
        r.c_top = vec![c_top];
        r.c_bot = vec![0.0];
    }
    p
}

fn solute_balance(sim: &Simulation) -> f64 {
    sim.res.balance.iter().skip(1).map(|b| b.sol_bal_r.first().copied().unwrap_or(0.0)).fold(0.0, f64::max)
}

#[test]
fn dual_permeability_solute_mass_balance() {
    for (alpha_s, flux_bc) in [(0.02, false), (0.5, false), (0.02, true), (0.5, true)] {
        let sim = run(solute_project(1.0, alpha_s, flux_bc));
        let worst = solute_balance(&sim);
        println!("alpha_s={alpha_s} flux_bc={flux_bc}: solute balance error {worst:.4} %");
        assert!(worst < 3.0, "solute balance error {worst} % (alpha_s={alpha_s}, flux_bc={flux_bc})");
    }
}

#[test]
fn matrix_solute_is_transported_and_exchanges() {
    // the matrix domain must receive solute both by exchange and by its own advective inflow
    let sim = run(solute_project(1.0, 0.1, false));
    let n = sim.n;
    let c_m: Vec<f64> = (0..n).map(|i| sim.conc_matrix(0, i)).collect();
    assert!(c_m.iter().all(|c| c.is_finite() && *c >= 0.0));
    assert!(c_m[n - 1] > 0.1, "matrix top concentration {}", c_m[n - 1]);
    // strong exchange keeps both domains close, weak exchange lets them differ
    let diff = |a: f64| {
        let s = run(solute_project(1.0, a, false));
        (0..s.n).map(|i| (s.conc(0, i) - s.conc_matrix(0, i)).abs()).sum::<f64>() / s.n as f64
    };
    let (strong, weak) = (diff(5.0), diff(0.001));
    println!("mean |c_f - c_m|: alpha_s=5 -> {strong:.4}, alpha_s=0.001 -> {weak:.4}");
    assert!(strong < weak);
}

/// SOLUTE.FOR ends the solute pulse at `tPulse` only for `.not.AtmBC`: with the atmospheric (time-variable) top boundary the
/// concentrations of the atmospheric records stay in force (HYDRUS 4.17, ROOTUPTK: cvTop = 0.4 after tPulse = 100 of 200 days).
/// An earlier version ended the pulse also for AtmBC, fitted to the ROOTUPTK output of an older build.
#[test]
fn atmospheric_top_keeps_the_record_concentration_after_tpulse() {
    let top = |t_pulse: f64| {
        let mut p = solute_project(1.0, 0.1, false);
        p.solute.t_pulse = t_pulse;
        let sim = run(p);
        let n = sim.n;
        (sim.conc(0, n - 1), sim.conc_matrix(0, n - 1))
    };
    let (f_cont, m_cont) = top(100.0);
    assert!(f_cont > 0.5 && m_cont > 0.1, "continuous input: fracture {f_cont}, matrix {m_cont}");
    let (f_end, m_end) = top(4.0);
    assert!((f_end - f_cont).abs() < 1e-9 && (m_end - m_cont).abs() < 1e-9, "tPulse = 4 of 10 must not change the atmospheric input: fracture {f_end} vs {f_cont}, matrix {m_end} vs {m_cont}");
    // records with cTop = 0 after t = 4 end the input
    let mut p = solute_project(1.0, 0.1, false);
    // (records: t = 2 rain with cTop = 1, t = 4 and t = 10 evaporation; the t = 4 record keeps cTop = 1, the last one has 0)
    p.atmosphere.records.insert(1, AtmRecord { t: 4.0, prec: 0.0, evap: 0.2, transp: 0.0, h_crit_a: 10000.0, c_top: vec![1.0], c_bot: vec![0.0], ..Default::default() });
    p.atmosphere.records[2].c_top = vec![0.0];
    let sim = run(p);
    let n = sim.n;
    assert!(sim.conc(0, n - 1) < f_cont, "zero record concentration after t = 4: top {} vs {f_cont}", sim.conc(0, n - 1));
}

#[test]
fn dual_permeability_with_two_site_sorption_in_both_domains() {
    // equilibrium fraction f = 0.5, kinetic rate omega, fracture-matrix transfer alpha_s
    let mut p = solute_project(1.0, 0.1, false);
    p.solute.materials[0].frac = 0.5;
    p.solute.species[0].per_material[0].omega = 0.2;
    p.solute.species[0].per_material[0].mu_s = 0.005;
    let sim = run(p);
    let worst = solute_balance(&sim);
    println!("two-site dual permeability: balance error {worst:.4} %");
    assert!(worst < 3.0, "balance error {worst} %");
    let n = sim.n;
    // kinetic sites take up solute in both domains
    assert!(sim.sorb(0, n - 1) > 0.0 && sim.sorb_matrix(0, n - 1) > 0.0);
}

#[test]
fn dual_permeability_decay_chain_produces_daughter_in_matrix() {
    let mut p = solute_project(1.0, 0.1, false);
    let mut daughter = p.solute.species[0].clone();
    daughter.name = "daughter".into();
    daughter.c_top = 0.0;
    p.solute.species[0].per_material[0].gam_w = 0.05; // parent decay that feeds the daughter
    p.solute.species[0].per_material[0].mu_w = 0.0;
    daughter.per_material[0].gam_w = 0.0;
    daughter.per_material[0].mu_w = 0.0;
    p.solute.species.push(daughter);
    for nd in p.profile.nodes.iter_mut() {
        nd.conc = vec![0.0, 0.0];
        nd.sorb = vec![0.0, 0.0];
    }
    for r in p.atmosphere.records.iter_mut() {
        r.c_top = vec![1.0, 0.0];
        r.c_bot = vec![0.0, 0.0];
    }
    let sim = run(p);
    let n = sim.n;
    let (cm_parent, cm_daughter) = (sim.conc_matrix(0, 3 * n / 4), sim.conc_matrix(1, 3 * n / 4));
    println!("matrix top: parent {cm_parent}, daughter {cm_daughter}");
    assert!(cm_daughter > 1e-4, "no daughter produced in the matrix");
    for js in 0..2 {
        let worst = sim.res.balance.iter().skip(1).map(|b| b.sol_bal_r.get(js).copied().unwrap_or(0.0)).fold(0.0, f64::max);
        println!("species {js}: balance error {worst:.4} %");
        assert!(worst < 3.0, "species {js}: balance error {worst} %");
    }
}

// ---- matrix-domain processes: mobile-immobile water, gas phase, colloid model --------------------------------------

#[test]
fn matrix_mobile_immobile_water() {
    // immobile water in the matrix; sites (1-f) are in contact with it
    let mut p = solute_project(1.0, 0.1, false);
    p.solute.materials[0].frac = 0.6;
    p.solute.materials[0].th_im_matrix = 0.08;
    p.solute.species[0].per_material[0].omega = 0.3;
    let sim = run(p);
    let worst = solute_balance(&sim);
    println!("matrix mobile-immobile: balance error {worst:.4} %");
    assert!(worst < 3.0, "balance error {worst} %");
    let n = sim.n;
    // the immobile concentration (reported as sorb_matrix) trails the mobile one behind the front and is positive
    let i = 3 * n / 4;
    let (cm, cim) = (sim.conc_matrix(0, i), sim.sorb_matrix(0, i));
    println!("matrix node {i}: mobile {cm:.4}, immobile {cim:.4}");
    assert!(cim > 0.0 && cim <= cm * 1.02 + 1e-9, "immobile {cim} vs mobile {cm}");
}

#[test]
fn matrix_immobile_water_delays_breakthrough() {
    let at = |th_im: f64| {
        let mut p = solute_project(1.0, 0.1, false);
        p.solute.materials[0].th_im_matrix = th_im;
        p.solute.species[0].per_material[0].omega = 0.02; // slow exchange
        let sim = run(p);
        sim.conc_matrix(0, sim.n / 2)
    };
    let (none, some) = (at(0.0), at(0.15));
    println!("matrix mid concentration: no immobile water {none:.4}, with {some:.4}");
    // with slow exchange the solute only uses the (smaller) mobile water: pore-water velocity rises and the front arrives earlier
    assert!(some > none, "slow exchange with immobile water must speed up the mobile front");
    // fast exchange equilibrates mobile and immobile water again and retards the front relative to slow exchange
    let fast = {
        let mut p = solute_project(1.0, 0.1, false);
        p.solute.materials[0].th_im_matrix = 0.15;
        p.solute.species[0].per_material[0].omega = 50.0;
        let sim = run(p);
        sim.conc_matrix(0, sim.n / 2)
    };
    println!("fast exchange: {fast:.4}");
    assert!(fast < some, "fast exchange must retard the front");
}

#[test]
fn matrix_gas_phase_and_volatilisation() {
    let balance_and_mass = |henry: f64, volat: bool| {
        let mut p = solute_project(1.0, 0.1, false);
        p.solute.species[0].per_material[0].henry = henry;
        p.solute.species[0].diff_g = 500.0;
        p.solute.species[0].per_material[0].mu_g = 0.0;
        if volat {
            p.solute.k_top = -2;
            p.solute.d_surf = 1.0;
            p.solute.c_atm = 0.0;
        }
        let sim = run(p);
        let last = sim.res.balance.last().unwrap();
        (solute_balance(&sim), last.total.c_vol[0])
    };
    let (b0, m0) = balance_and_mass(0.0, false);
    let (b1, m1) = balance_and_mass(0.5, false);
    let (b2, m2) = balance_and_mass(0.5, true);
    println!("gas: H=0 {b0:.3} % mass {m0:.3};  H=0.5 {b1:.3} % mass {m1:.3};  volatilisation {b2:.3} % mass {m2:.3}");
    assert!(b1 < 3.0 && b2 < 3.0, "balance errors {b1} / {b2}");
    assert!(m1 > m0, "gas phase must store additional mass");
    assert!(m2 < m1, "volatilisation must remove mass");
}

#[test]
fn matrix_colloid_model() {
    let mut p = solute_project(1.0, 0.1, false);
    p.solute.l_bact = true;
    {
        let m = &mut p.solute.species[0].per_material[0];
        m.ks = 0.0;
        m.mu_w = 0.0;
        m.r_ka1 = 0.05;
        m.r_kd1 = 0.02;
        m.s_max1 = 0.0;
        m.r_ka2 = 0.02;
        m.r_kd2 = 0.01;
        m.s_max2 = 0.0;
        m.mu_s = 0.001;
    }
    let sim = run(p);
    let worst = solute_balance(&sim);
    println!("matrix colloids: balance error {worst:.4} %");
    assert!(worst < 3.0, "balance error {worst} %");
    let n = sim.n;
    assert!(sim.sorb_matrix(0, 3 * n / 4) > 0.0, "attached colloids expected in the matrix");
}

#[test]
fn matrix_colloid_blocking_limits_attachment() {
    let attached = |s_max: f64| {
        let mut p = solute_project(1.0, 0.1, false);
        p.solute.l_bact = true;
        let m = &mut p.solute.species[0].per_material[0];
        m.ks = 0.0;
        m.mu_w = 0.0;
        m.r_ka1 = 0.5;
        m.r_kd1 = 0.0;
        m.i_psi1 = if s_max > 0.0 { 1 } else { 0 };
        m.s_max1 = s_max;
        let sim = run(p);
        sim.sorb_matrix(0, sim.n - 2)
    };
    let (free, blocked) = (attached(0.0), attached(0.05));
    println!("attached at the top: unblocked {free:.4}, Langmuir blocking {blocked:.4}");
    assert!(blocked < free && blocked <= 0.0501, "blocking must cap the attached concentration");
}

// ---- completion of the matrix: dual non-equilibrium, dual porosity, colloid exclusion and colloids with immobile water ----

#[test]
fn matrix_dual_non_equilibrium() {
    let mut p = solute_project(1.0, 0.1, false);
    p.solute.l_dual_neq = true;
    p.solute.materials[0].frac = 0.6;
    p.solute.materials[0].th_im_matrix = 0.08;
    let m = &mut p.solute.species[0].per_material[0];
    m.omega = 0.3; // mobile-immobile transfer
    m.mu_g = 0.5; // f_em
    m.gam_g = 0.2; // omega_S
    m.mu_s = 0.004;
    let sim = run(p);
    let worst = solute_balance(&sim);
    println!("matrix dual non-equilibrium: balance error {worst:.4} %");
    assert!(worst < 3.0, "balance error {worst} %");
    let n = sim.n;
    assert!(sim.sorb_matrix(0, 3 * n / 4) > 0.0);
}

#[test]
fn matrix_dual_non_equilibrium_kinetic_sites_matter() {
    let top = |omega_s: f64| {
        let mut p = solute_project(1.0, 0.1, false);
        p.solute.l_dual_neq = true;
        p.solute.materials[0].frac = 1.0;
        p.solute.materials[0].th_im_matrix = 0.05;
        let m = &mut p.solute.species[0].per_material[0];
        m.omega = 5.0;
        m.mu_g = 0.2; // only 20 % of the mobile sites are in equilibrium
        m.gam_g = omega_s;
        m.ks = 1.0;
        let sim = run(p);
        sim.conc_matrix(0, sim.n / 2)
    };
    let (slow, fast) = (top(0.0005), top(50.0));
    println!("mid concentration: slow kinetic sites {slow:.4}, fast {fast:.4}");
    assert!(slow > fast, "frozen kinetic sites take up less solute than equilibrated ones: {slow} vs {fast}");
}

fn matrix_dual_por_project() -> Project {
    let mut p = solute_project(1.0, 0.1, false);
    p.water.dual_perm.matrix_dual_por = true;
    p.water.dual_perm.materials[0].thr_im = 0.0;
    p.water.dual_perm.materials[0].ths_im = 0.08;
    p.water.dual_perm.materials[0].omega_w = 0.05;
    p
}

#[test]
fn matrix_dual_porosity_water_balance_and_transfer() {
    let mut p = project();
    p.water.dual_perm.matrix_dual_por = true;
    p.water.dual_perm.materials[0].ths_im = 0.08;
    p.water.dual_perm.materials[0].omega_w = 0.05;
    let sim = run(p);
    let worst = sim.res.balance.iter().skip(1).map(|b| b.wat_bal_r).fold(0.0, f64::max);
    println!("matrix dual porosity: water balance error {worst:.4} %, immobile top {:.4}", sim.th_im_mat_new[sim.n - 1]);
    assert!(worst < 2.0, "water balance error {worst} %");
    assert!(sim.th_im_mat_new.iter().all(|t| *t >= 0.0 && *t <= 0.0801));
}

#[test]
fn matrix_dual_porosity_solute_transport() {
    let sim = run(matrix_dual_por_project());
    let worst = solute_balance(&sim);
    println!("matrix dual porosity: solute balance error {worst:.4} %");
    assert!(worst < 3.0, "solute balance error {worst} %");
    let n = sim.n;
    let i = 3 * n / 4;
    let (cm, cim) = (sim.conc_matrix(0, i), sim.sorb_matrix(0, i));
    println!("node {i}: mobile {cm:.4}, immobile {cim:.4}");
    assert!(cm.is_finite() && cim.is_finite() && cim >= 0.0);
}

#[test]
fn matrix_colloids_with_immobile_water() {
    let mut p = solute_project(1.0, 0.1, false);
    p.solute.l_bact = true;
    p.solute.materials[0].th_im_matrix = 0.08;
    {
        let m = &mut p.solute.species[0].per_material[0];
        m.ks = 0.0;
        m.mu_w = 0.001;
        m.r_ka1 = 0.05;
        m.r_kd1 = 0.02;
        m.r_ka2 = 0.02;
        m.r_kd2 = 0.01;
        m.omega_im = 0.2; // exchange with the immobile region
    }
    let sim = run(p);
    let worst = solute_balance(&sim);
    println!("matrix colloids + immobile water: balance error {worst:.4} %");
    assert!(worst < 3.0, "balance error {worst} %");
    let n = sim.n;
    assert!(sim.sorb_matrix(0, 3 * n / 4) > 0.0, "immobile colloid concentration expected");
}

#[test]
fn matrix_colloid_exclusion_with_steady_flow() {
    let mut p = solute_project(1.0, 0.0, false);
    p.processes.water_flow = false; // steady flow: Exclusion applies
    p.solute.l_bact = true;
    p.solute.materials[0].th_im_matrix = 0.06; // colloid exclusion water content th_c of the matrix
    {
        let m = &mut p.solute.species[0].per_material[0];
        m.ks = 0.0;
        m.mu_w = 0.0;
        m.r_ka1 = 0.02;
        m.r_kd1 = 0.01;
        m.r_ka2 = 0.0;
        m.r_kd2 = 0.0;
    }
    let sim = run(p);
    let worst = solute_balance(&sim);
    println!("matrix colloid exclusion: balance error {worst:.4} %");
    assert!(worst < 5.0, "balance error {worst} %");
    let st = sim.sol.as_ref().unwrap();
    let (thc, vc) = st.excl_m.expect("exclusion must be active in the matrix");
    println!("matrix exclusion: accessible theta {thc:.4}, colloid velocity {vc:.4e}");
    assert!(thc > 0.0 && thc < 0.5);
    assert!(sim.conc_matrix(0, sim.n - 2).is_finite());
}

#[test]
fn matrix_dual_porosity_pressure_head_driven() {
    let mut p = project();
    p.water.dual_perm.matrix_dual_por = true;
    p.water.dual_perm.matrix_dual_por_head = true;
    {
        let m = &mut p.water.dual_perm.materials[0];
        m.thr_im = 0.0;
        m.ths_im = 0.08;
        m.alpha_im = 0.05;
        m.n_im = 1.6;
        m.ks_im = 2.0;
    }
    let sim = run(p);
    let worst = sim.res.balance.iter().skip(1).map(|b| b.wat_bal_r).fold(0.0, f64::max);
    println!("pressure-head driven matrix dual porosity: water balance error {worst:.4} %");
    assert!(worst < 2.0, "water balance error {worst} %");
    assert!(sim.th_im_mat_new.iter().all(|t| *t >= 0.0 && *t <= 0.0801));
    // the immobile region is initialised in equilibrium with the initial head (-100 cm): theta_im < theta_s,im
    assert!(sim.th_im_mat_old[0] < 0.08);
    // water must have moved between mobile and immobile matrix water during the infiltration event
    assert!(sim.res.tlevel.iter().any(|t| t.cum_w_transf.abs() > 1e-9));
}

#[test]
fn matrix_dual_porosity_pressure_head_solute_balance() {
    let mut p = matrix_dual_por_project();
    p.water.dual_perm.matrix_dual_por_head = true;
    {
        let m = &mut p.water.dual_perm.materials[0];
        m.alpha_im = 0.05;
        m.n_im = 1.6;
        m.ks_im = 2.0;
    }
    let sim = run(p);
    let worst = solute_balance(&sim);
    println!("pressure-head driven matrix dual porosity: solute balance error {worst:.4} %");
    assert!(worst < 3.0, "solute balance error {worst} %");
}
