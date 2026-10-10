//! Built-in example projects.

use crate::model::*;

fn uniform_profile(depth: f64, n: usize, h: f64, mat_of: impl Fn(f64) -> usize) -> Profile {
    let dz = depth / (n as f64 - 1.0);
    let nodes = (0..n)
        .map(|i| {
            let z = i as f64 * dz;
            let m = mat_of(z);
            let mut nd = ProfileNode::new(z, h, m);
            nd.layer = m;
            nd
        })
        .collect();
    Profile { nodes, observation_nodes: vec![], fixed_points: vec![FixedPoint::new(0.0), FixedPoint::new(depth)] }
}

/// Infiltration and evaporation in a two-layer soil with root water uptake.
pub fn infiltration_evaporation() -> Project {
    let mut p = Project::default();
    p.title = "Infiltration and evaporation in a layered soil with root water uptake".into();
    p.processes.root_water_uptake = true;
    p.water.max_iter = 20;
    p.water.materials = vec![
        SoilMaterial { name: "Loam".into(), qr: 0.078, qs: 0.43, alpha: 0.036, n: 1.56, ks: 24.96, l: 0.5, ..Default::default() },
        SoilMaterial { name: "Sand".into(), qr: 0.045, qs: 0.43, alpha: 0.145, n: 2.68, ks: 712.8, l: 0.5, ..Default::default() },
    ];
    p.water.bc = WaterBc { atmospheric: true, top_time_variable: true, surface_layer: false, kod_top: -1, free_drainage: true, kod_bot: -1, ..Default::default() };
    p.profile = uniform_profile(100.0, 101, -200.0, |z| if z < 50.0 { 1 } else { 2 });
    for nd in p.profile.nodes.iter_mut() {
        nd.beta = if nd.depth < 30.0 { 1.0 } else { 0.0 };
        if nd.mat == 2 {
            nd.h = -80.0;
        }
    }
    p.profile.observation_nodes = vec![21, 51, 101];
    p.time = TimeSettings { dt: 0.001, dt_min: 1e-5, dt_max: 5.0, t_max: 30.0, print_times: vec![5.0, 10.0, 15.0, 20.0, 25.0, 30.0], ..Default::default() };
    let rec = |t, prec, evap, tr| AtmRecord { t, prec, evap, transp: tr, h_crit_a: 10000.0, ..Default::default() };
    p.atmosphere.records = vec![rec(2.0, 3.0, 0.3, 0.4), rec(4.0, 0.0, 0.5, 0.5), rec(8.0, 0.0, 0.5, 0.5), rec(12.0, 8.0, 0.3, 0.4), rec(20.0, 0.0, 0.6, 0.6), rec(30.0, 0.0, 0.6, 0.6)];
    p
}

/// Tracer pulse transported through a uniform loam column under steady infiltration,
/// with linear sorption and first-order decay.
pub fn solute_pulse() -> Project {
    let mut p = Project::default();
    p.title = "Tracer pulse with sorption and decay under steady infiltration".into();
    p.processes.solute = true;
    p.water.materials = vec![SoilMaterial { name: "Loam".into(), qr: 0.078, qs: 0.43, alpha: 0.036, n: 1.56, ks: 24.96, l: 0.5, ..Default::default() }];
    p.water.bc = WaterBc { kod_top: -1, r_top: -2.0, free_drainage: true, kod_bot: -1, ..Default::default() };
    p.profile = uniform_profile(100.0, 201, -40.0, |_| 1);
    p.profile.observation_nodes = vec![41, 101, 201];
    p.time = TimeSettings { dt: 0.001, dt_min: 1e-6, dt_max: 0.5, t_max: 60.0, print_times: vec![10.0, 20.0, 30.0, 40.0, 50.0, 60.0], ..Default::default() };
    p.solute = SoluteSettings {
        epsi: 0.5,
        artificial_dispersion: true,
        tortuosity: true,
        max_iter: 20,
        tol_abs: 0.001,
        tol_rel: 0.01,
        t_pulse: 10.0,
        k_top: -1,
        k_bot: 0,
        materials: vec![SoluteMaterial { bulk_density: 1.3, disp_l: 2.0, frac: 1.0, th_immobile: 0.0, th_im_matrix: 0.0 }],
        species: vec![Species {
            name: "Tracer".into(),
            diff_w: 0.5,
            diff_g: 0.0,
            per_material: vec![SpeciesMaterial { ks: 0.2, beta: 1.0, mu_w: 0.01, mu_s: 0.01, ..Default::default() }],
            c_top: 10.0,
            c_bot: 0.0,
        }],
        ..Default::default()
    };
    // a wet, steady profile
    for nd in p.profile.nodes.iter_mut() {
        nd.h = -40.0;
    }
    p
}

/// One-step outflow experiment (synthetic data) set up as an inverse problem: a 10 cm soil core drains against a
/// suction of 100 cm applied at its bottom; α, n and Ks are estimated from the cumulative outflow and one retention
/// point. The data were produced with θr 0.08, θs 0.40, α 0.025, n 1.7, Ks 3 cm/h, l 0.5 and rounded; the initial
/// estimates are off by 20-70 %.
pub fn inverse_outflow() -> Project {
    use crate::inverse::*;
    let mut p = Project::default();
    p.title = "One-step outflow: parameter estimation from synthetic data".into();
    p.units.time = TimeUnit::Hours;
    p.water.max_iter = 20;
    p.water.tol_th = 0.0001;
    p.water.tol_h = 0.1;
    p.water.materials = vec![SoilMaterial { name: "Soil".into(), qr: 0.08, qs: 0.40, alpha: 0.025, n: 1.7, ks: 3.0, l: 0.5, ..Default::default() }];
    p.water.bc = WaterBc { kod_top: -1, r_top: 0.0, kod_bot: 1, ..Default::default() };
    // hydrostatic start with -5 cm at the bottom; the bottom node holds the suction of -100 cm
    let mut prof = uniform_profile(10.0, 41, -5.0, |_| 1);
    for nd in prof.nodes.iter_mut() {
        nd.h = -5.0 - (10.0 - nd.depth);
    }
    prof.nodes.last_mut().unwrap().h = -100.0;
    prof.observation_nodes = vec![21];
    p.profile = prof;
    p.time = TimeSettings { dt: 0.001, dt_min: 1e-6, dt_max: 1.0, t_max: 50.0, print_times: vec![1.0, 5.0, 20.0, 50.0], ..Default::default() };
    p.processes.inverse = true;
    let mut s = InverseSettings::from_project(&p);
    s.title1 = "One-step outflow".into();
    s.title2 = "Synthetic data".into();
    s.max_iter = 30;
    s.weighting = 0;
    for q in s.params.iter_mut().filter(|q| q.mat == 0) {
        match q.index {
            3 => (q.value, q.fit, q.min, q.max) = (0.035, true, 0.001, 0.5),
            4 => (q.value, q.fit, q.min, q.max) = (1.4, true, 1.05, 5.0),
            5 => (q.value, q.fit, q.min, q.max) = (5.0, true, 0.01, 100.0),
            _ => {}
        }
    }
    let data = [(0.05, -0.0530), (0.1, -0.0980), (0.25, -0.2019), (0.5, -0.3240), (1.0, -0.4871), (2.0, -0.6832), (4.0, -0.9007), (8.0, -1.1309), (16.0, -1.3588), (32.0, -1.5759), (50.0, -1.7060)];
    s.observations = data.iter().map(|&(t, v)| Observation { x: t, value: v, kind: 0, pos: 2, weight: 1.0 }).collect();
    s.observations.push(Observation { x: -100.0, value: 0.2357, kind: 5, pos: 1, weight: 1.0 });
    p.inverse = Some(s);
    p
}

/// Soil temperatures under a daily temperature wave (synthetic data) set up as an inverse problem: heat transport in a
/// 1 m loam profile wetted by an infiltration of 2 cm/d, the surface temperature oscillates by ±8 °C around 15 °C. The
/// conductivity coefficient b1 and the heat capacity of the solid phase Cn are estimated from temperatures at 5, 10
/// and 20 cm (at constant water content only their ratio could be estimated). The data were produced with b1 = 1.5673e16 and Cn = 1.4333e14 (HYDRUS units for cm and days, the
/// loam values of the HYDRUS examples) and rounded to 0.001 °C; the initial estimates are off by +25 % and -20 %.
pub fn inverse_heat() -> Project {
    use crate::inverse::*;
    let mut p = Project::default();
    p.title = "Soil temperature: estimation of heat transport parameters from synthetic data".into();
    p.processes.heat = true;
    p.water.bc = WaterBc { kod_top: -1, r_top: -2.0, free_drainage: true, kod_bot: -1, ..Default::default() };
    p.water.materials = vec![SoilMaterial { name: "Loam".into(), qr: 0.078, qs: 0.43, alpha: 0.036, n: 1.56, ks: 24.96, l: 0.5, ..Default::default() }];
    p.profile = uniform_profile(100.0, 101, -100.0, |_| 1);
    for nd in p.profile.nodes.iter_mut() {
        nd.temp = 15.0;
    }
    p.profile.observation_nodes = vec![6, 11, 21];
    p.heat = HeatSettings {
        materials: vec![HeatMaterial { qn: 0.6, qo: 0.0, disp: 5.0, b1: 1.5673e16, b2: 2.5347e16, b3: 9.8939e16, cn: 1.4333e14, co: 1.8737e14, cw: 3.1204e14 }],
        amplitude: 8.0,
        period: 1.0,
        campbell: false,
        k_top: 1,
        t_top: 15.0,
        k_bot: 0,
        t_bot: 15.0,
    };
    p.time = TimeSettings { dt: 0.001, dt_min: 1e-5, dt_max: 0.02, t_max: 5.0, print_times: vec![1.0, 2.0, 3.0, 4.0, 5.0], ..Default::default() };
    p.processes.inverse = true;
    let mut s = InverseSettings::from_project(&p);
    s.title1 = "Soil temperature".into();
    s.title2 = "Synthetic data".into();
    s.fit_water = false;
    s.fit_heat = true;
    s.max_iter = 20;
    s.weighting = 2;
    for q in s.params.iter_mut().filter(|q| q.group == ParamGroup::Heat && q.mat == 0) {
        match q.index {
            4 => (q.value, q.fit, q.min, q.max) = (1.96e16, true, 1e15, 1e17),
            7 => (q.value, q.fit, q.min, q.max) = (1.15e14, true, 1e13, 1e15),
            _ => {}
        }
    }
    s.observations = HEAT_DATA.iter().map(|&(t, pos, v)| Observation { x: t, value: v, kind: 4, pos, weight: 1.0 }).collect();
    p.inverse = Some(s);
    p
}

/// Temperatures (time, observation node, °C) of [`inverse_heat`].
const HEAT_DATA: [(f64, i32, f64); 60] = [
    (0.25, 1, 11.765), (0.25, 2, 12.034), (0.25, 3, 13.697), (0.5, 1, 19.304), (0.5, 2, 16.808), (0.5, 3, 14.514),
    (0.75, 1, 18.458), (0.75, 2, 18.341), (0.75, 3, 16.606), (1.0, 1, 10.841), (1.0, 2, 13.391), (1.0, 3, 15.619),
    (1.25, 1, 11.595), (1.25, 2, 11.754), (1.25, 3, 13.446), (1.5, 1, 19.137), (1.5, 2, 16.584), (1.5, 3, 14.363),
    (1.75, 1, 18.383), (1.75, 2, 18.200), (1.75, 3, 16.486), (2.0, 1, 10.865), (2.0, 2, 13.410), (2.0, 3, 15.604),
    (2.25, 1, 11.621), (2.25, 2, 11.808), (2.25, 3, 13.520), (2.5, 1, 19.130), (2.5, 2, 16.584), (2.5, 3, 14.396),
    (2.75, 1, 18.374), (2.75, 2, 18.181), (2.75, 3, 16.463), (3.0, 1, 10.870), (3.0, 2, 13.413), (3.0, 3, 15.589),
    (3.25, 1, 11.625), (3.25, 2, 11.817), (3.25, 3, 13.531), (3.5, 1, 19.126), (3.5, 2, 16.582), (3.5, 3, 14.403),
    (3.75, 1, 18.371), (3.75, 2, 18.176), (3.75, 3, 16.457), (4.0, 1, 10.873), (4.0, 2, 13.415), (4.0, 3, 15.587),
    (4.25, 1, 11.627), (4.25, 2, 11.821), (4.25, 3, 13.536), (4.5, 1, 19.125), (4.5, 2, 16.581), (4.5, 3, 14.405),
    (4.75, 1, 18.370), (4.75, 2, 18.174), (4.75, 3, 16.455), (5.0, 1, 10.874), (5.0, 2, 13.416), (5.0, 3, 15.587),
];

pub fn all() -> Vec<(&'static str, fn() -> Project)> {
    vec![
        ("Infiltration & evaporation with roots", infiltration_evaporation),
        ("Solute pulse with sorption and decay", solute_pulse),
        ("Parameter estimation: one-step outflow", inverse_outflow),
        ("Parameter estimation: soil temperature", inverse_heat),
    ]
}
