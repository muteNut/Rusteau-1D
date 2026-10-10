//! Legacy save: a project written as a HYDRUS-1D 4.x project (`<name>.h1d` + folder) and read back must equal the
//! saved project and give the same results.
//!
//! * `builtin_examples_round_trip` uses the examples built into Rusteau (no external data).
//! * `hydrus_examples_round_trip` reads every example of the HYDRUS-1D `Examples/Direct` and `Examples/Inverse`
//!   folders (`RUSTEAU_EXAMPLES_DIR`, `RUSTEAU_INVERSE_DIR`; skipped without them), saves it and reads it again.

use rusteau_core::{examples, Project, Simulation, StepStatus};
use rusteau_io::*;
use std::path::{Path, PathBuf};

fn tmp(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("rusteau_legacy_save_{}_{}", std::process::id(), name));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// The settings HYDRUS-1D does not store are reset to what the reader gives.
fn normalize_project(p: &Project) -> Project {
    let mut p = p.clone();
    p.water.use_tables = true;
    let ns = p.n_solutes();
    for n in p.profile.nodes.iter_mut() {
        n.conc.resize(ns, 0.0);
        n.sorb.resize(ns, 0.0);
    }
    if p.processes.root_water_uptake {
        p.root.c_root_max.resize(ns, 0.0);
        let n_mat = p.water.materials.len();
        if let rusteau_core::RootStress::Feddes { p_optm, .. } = &mut p.root.stress {
            let last = p_optm.last().copied().unwrap_or(-25.0);
            p_optm.resize(n_mat, last);
        }
    }
    // Hysteresis.in: water contents and branches (no conductivities), only with Kool & Parker hysteresis
    if !p.water.hysteresis.is_kool_parker() {
        p.profile.hysteresis = None;
    }
    if let Some(st) = p.profile.hysteresis.as_mut() {
        st.iter_mut().for_each(|s| s.k = None);
    }
    p.time.flux_out = false;
    // Options.in: not read by HYDRUS-1D, not saved
    p.options = Default::default();
    if p.time.print_times.is_empty() {
        p.time.print_times = vec![p.time.t_max];
    }
    if let Some(i) = p.water.bc.irrigation.as_mut() {
        i.mode = Default::default();
    }
    // names are not stored
    for (k, m) in p.water.materials.iter_mut().enumerate() {
        m.name = format!("Material {}", k + 1);
    }
    for (k, s) in p.solute.species.iter_mut().enumerate() {
        s.name = format!("Solute {}", k + 1);
    }
    // inverse: Rusteau iteration settings are not stored
    if let Some(s) = p.inverse.as_mut() {
        s.extra_starts = rusteau_core::inverse::DEFAULT_EXTRA_STARTS;
        s.hydrus_stop = false;
        let keep: Vec<bool> = s.params.iter().map(|q| s.group_active(q.group)).collect();
        let mut k = keep.iter();
        s.params.retain(|_| *k.next().unwrap());
    }
    p
}

fn normalize(p: &Project) -> serde_json::Value {
    let mut v = serde_json::to_value(normalize_project(p)).unwrap();
    // fields that only exist for other settings
    if let Some(o) = v.pointer_mut("/solute") {
        if let Some(m) = o.as_object_mut() {
            m.remove("i_moist_dep");
            m.remove("i_conc_type");
        }
    }
    v
}

fn diff(a: &serde_json::Value, b: &serde_json::Value, path: &str, out: &mut Vec<String>) {
    use serde_json::Value::*;
    match (a, b) {
        (Object(x), Object(y)) => {
            for (k, va) in x {
                match y.get(k) {
                    Some(vb) => diff(va, vb, &format!("{path}/{k}"), out),
                    None => out.push(format!("{path}/{k}: missing")),
                }
            }
        }
        (Array(x), Array(y)) => {
            if x.len() != y.len() {
                out.push(format!("{path}: length {} vs {}", x.len(), y.len()));
                return;
            }
            for (i, (va, vb)) in x.iter().zip(y).enumerate() {
                diff(va, vb, &format!("{path}/{i}"), out);
            }
        }
        (Number(x), Number(y)) => {
            let (x, y) = (x.as_f64().unwrap_or(0.0), y.as_f64().unwrap_or(0.0));
            if (x - y).abs() > 1e-12 * x.abs().max(y.abs()) {
                out.push(format!("{path}: {x} vs {y}"));
            }
        }
        _ => {
            if a != b {
                out.push(format!("{path}: {a} vs {b}"));
            }
        }
    }
}

fn run(prj: &Project) -> Vec<f64> {
    let mut sim = Simulation::new(prj.clone()).expect("project");
    let mut n = 0;
    while sim.step() == StepStatus::Running {
        n += 1;
        if n > 2_000_000 {
            break;
        }
    }
    sim.res.tlevel.iter().flat_map(|l| [l.t, l.cum_v_top, l.cum_v_bot, l.volume]).collect()
}

/// Saves `prj` as `<dir>/<name>.h1d` + folder, reads it back and compares. Returns the problems.
fn round_trip(prj: &Project, dir: &Path, name: &str, simulate: bool) -> Vec<String> {
    let h1d = dir.join(format!("{name}.h1d"));
    let (files, _warnings) = save_h1d_project(prj, &h1d).expect("legacy save");
    assert!(h1d.is_file() && files.iter().all(|f| f.is_file()));
    let folder = legacy_project_dir(&h1d).expect("project folder next to the .h1d");
    let back = match read_legacy_project(&folder) {
        Ok(p) => p,
        Err(e) => return vec![format!("read back: {e}")],
    };
    let mut out = vec![];
    diff(&normalize(prj), &normalize(&back), "", &mut out);
    if out.is_empty() && simulate {
        let (a, b) = (run(&normalize_project(prj)), run(&back));
        if a != b {
            out.push("the results differ".into());
        }
    }
    // the files written again from the project read back are identical (idempotent writer)
    let dir2 = dir.join("again");
    let _ = std::fs::create_dir_all(&dir2);
    let (files2, _) = save_h1d_project(&back, &dir2.join(format!("{name}.h1d"))).unwrap();
    for (f1, f2) in files.iter().zip(&files2) {
        if std::fs::read(f1).ok() != std::fs::read(f2).ok() {
            out.push(format!("{} differs when written again", f1.file_name().unwrap().to_string_lossy()));
        }
    }
    out
}

#[test]
fn builtin_examples_round_trip() {
    let dir = tmp("builtin");
    let mut problems = vec![];
    for (k, (name, make)) in examples::all().into_iter().enumerate() {
        let prj = make();
        for p in round_trip(&prj, &dir.join(format!("ex{k}")), "project", true) {
            problems.push(format!("{name}: {p}"));
        }
    }
    let _ = std::fs::remove_dir_all(&dir);
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

/// Active uptake of the second of two solutes: SELECTOR.IN of HYDRUS 4.17 with iActSolU (ROOTUPTK_ACT2).
#[test]
fn active_uptake_of_the_second_solute_round_trip() {
    use rusteau_core::*;
    let dir = tmp("act2");
    let mut p = examples::infiltration_evaporation();
    p.processes.solute = true;
    let ns = 2;
    let mut sol = SoluteSettings { species: vec![], materials: vec![SoluteMaterial::default(); 2], max_iter: 1, ..Default::default() };
    for j in 0..ns {
        sol.species.push(Species { name: format!("Solute {}", j + 1), per_material: vec![SpeciesMaterial { beta: 1.0, ..Default::default() }; 2], ..Default::default() });
    }
    sol.k_top = -1;
    p.solute = sol;
    for r in p.atmosphere.records.iter_mut() {
        r.c_top = vec![1.0, 0.0];
        r.c_bot = vec![0.0, 0.1];
    }
    for nd in p.profile.nodes.iter_mut() {
        nd.conc = vec![1.0; ns];
        nd.sorb = vec![0.0; ns];
    }
    p.root.c_root_max = vec![0.1; ns];
    p.root.l_act_rsu = true;
    p.root.active_solute = 1;
    p.root.r_km = vec![0.5];
    p.root.c_min = vec![0.0];
    p.root.omega_act = vec![1.0];
    assert!(legacy_save_warnings(&p).iter().all(|w| !w.contains("ctive")), "{:?}", legacy_save_warnings(&p));
    let problems = round_trip(&p, &dir, "act2", true);
    assert!(problems.is_empty(), "{}", problems.join("\n"));
    let sel = std::fs::read_to_string(dir.join("act2").join("SELECTOR.IN")).unwrap();
    let mut lines = sel.lines().skip_while(|l| !l.contains("iActSolU"));
    lines.next();
    let v: Vec<&str> = lines.next().unwrap().split_whitespace().collect();
    assert_eq!(v.len(), 6);
    assert_eq!(v[5], "2");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn hysteresis_state_round_trip() {
    let dir = tmp("hyst");
    let mut prj = examples::all()[0].1();
    prj.water.hysteresis = rusteau_core::Hysteresis::RetentionAndConductivity;
    for m in prj.water.materials.iter_mut() {
        m.qm = m.qs;
        m.qs_w = m.qs - 0.02;
        m.alpha_w = 2.0 * m.alpha;
        m.ks_w = 0.8 * m.ks;
    }
    let n = prj.profile.nodes.len();
    // distinct values, so that the node order is checked
    let st: Vec<rusteau_core::HysteresisNode> =
        (0..n).map(|j| rusteau_core::HysteresisNode { theta: 0.1 + 0.002 * j as f64, kappa: if j % 3 == 0 { 1 } else { -1 }, k: Some(1.0) }).collect();
    prj.profile.hysteresis = Some(st.clone());
    let problems = round_trip(&prj, &dir, "hyst", true);
    assert!(problems.is_empty(), "{}", problems.join("\n"));
    // the file lists the bottom node first (HysterIn assigns the lines in the internal order)
    let text = std::fs::read_to_string(dir.join("hyst").join("Hysteresis.in")).unwrap();
    let first: Vec<f64> = text.lines().nth(2).unwrap().split_whitespace().map(|t| t.parse().unwrap()).collect();
    assert_eq!(first, vec![1.0, st[n - 1].theta, st[n - 1].kappa as f64]);
    assert!(text.contains("\r\n"));
    // without hysteresis no file; a stale one is removed
    prj.water.hysteresis = rusteau_core::Hysteresis::None;
    let (files, _) = save_h1d_project(&prj, &dir.join("hyst.h1d")).unwrap();
    assert!(!dir.join("hyst").join("Hysteresis.in").exists());
    assert!(files.iter().all(|f| !f.to_string_lossy().contains("Hysteresis")));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn h1d_file_layout() {
    let dir = tmp("layout");
    let prj = examples::all()[0].1();
    let (files, _) = save_h1d_project(&prj, &dir.join("My Project.h1d")).unwrap();
    assert!(dir.join("My Project.h1d").is_file());
    assert!(dir.join("My Project").join("SELECTOR.IN").is_file());
    assert!(dir.join("My Project").join("PROFILE.DAT").is_file());
    assert!(dir.join("My Project").join("HYDRUS1D.DAT").is_file());
    assert_eq!(files[0], dir.join("My Project.h1d"));
    // a folder path gives the same layout
    let (f2, _) = save_h1d_project(&prj, &dir.join("Other")).unwrap();
    assert_eq!(f2[0], dir.join("Other.h1d"));
    // Windows line ends
    let sel = std::fs::read_to_string(dir.join("My Project").join("SELECTOR.IN")).unwrap();
    assert!(sel.starts_with("Pcp_File_Version=4\r\n"));
    let _ = std::fs::remove_dir_all(&dir);
}

fn example_dirs(var: &str, default: &str) -> Vec<PathBuf> {
    let base = std::env::var(var).map(PathBuf::from).unwrap_or_else(|_| PathBuf::from(default));
    let Ok(rd) = std::fs::read_dir(&base) else { return vec![] };
    let mut v: Vec<PathBuf> = rd
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_dir() && std::fs::read_dir(p).map(|r| r.flatten().any(|e| e.file_name().to_string_lossy().eq_ignore_ascii_case("selector.in"))).unwrap_or(false))
        .collect();
    v.sort();
    v
}

#[test]
fn hydrus_examples_round_trip() {
    let mut dirs = example_dirs("RUSTEAU_EXAMPLES_DIR", r"C:\Users\Public\Documents\PC-Progress\Hydrus-1D 4.xx\Examples\Direct");
    dirs.extend(example_dirs("RUSTEAU_INVERSE_DIR", r"C:\Users\Public\Documents\PC-Progress\Hydrus-1D 4.xx\Examples\Inverse"));
    if dirs.is_empty() {
        println!("skipped: no HYDRUS-1D examples (set RUSTEAU_EXAMPLES_DIR / RUSTEAU_INVERSE_DIR)");
        return;
    }
    let simulate = std::env::var("RUSTEAU_LEGACY_SAVE_RUN").is_ok();
    let dir = tmp("hydrus");
    let mut problems = vec![];
    for (k, d) in dirs.iter().enumerate() {
        let name = d.file_name().unwrap().to_string_lossy().to_string();
        let prj = match read_legacy_project(d) {
            Ok(p) => p,
            Err(e) => {
                println!("{name}: not readable ({e}), skipped");
                continue;
            }
        };
        for p in round_trip(&prj, &dir.join(format!("{k}")), &name, simulate) {
            problems.push(format!("{name}: {p}"));
        }
        println!("{name}: checked");
    }
    let _ = std::fs::remove_dir_all(&dir);
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}
