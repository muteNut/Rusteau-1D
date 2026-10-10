//! Inverse solution compared with the Fit.out files of the HYDRUS-1D 4.17 inverse examples (DRAINAGE, EVAPOR, FITHYST,
//! FITTEST2-5, HORINFDR, HORIZINF, MULTSTEP, ONESTEP, UPINFIL, recalculated with 4.17).
//!
//! The folder is taken from `RUSTEAU_INVERSE_DIR` (default: the HYDRUS-1D examples folder of a Windows installation);
//! `RUSTEAU_INVERSE_CASE` selects one example. Without the folder the tests are skipped. Run them with `--release`
//! (the full optimizations from nine starting points take a few minutes, much longer in a debug build).

use rusteau_core::inverse::*;
use rusteau_io::*;
use std::path::{Path, PathBuf};

fn find(dir: &Path, name: &str) -> Option<PathBuf> {
    std::fs::read_dir(dir).ok()?.flatten().map(|e| e.path()).find(|p| p.file_name().is_some_and(|f| f.to_string_lossy().eq_ignore_ascii_case(name)))
}

/// Example folders with Selector.in, Fit.in and Fit.out.
fn examples() -> Option<Vec<(String, PathBuf)>> {
    let base = std::env::var("RUSTEAU_INVERSE_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(r"C:\Users\Public\Documents\PC-Progress\Hydrus-1D 4.xx\Examples\Inverse"));
    if !base.is_dir() {
        println!("skipped: inverse examples not found at {} (set RUSTEAU_INVERSE_DIR)", base.display());
        return None;
    }
    let only = std::env::var("RUSTEAU_INVERSE_CASE").ok();
    let mut v: Vec<(String, PathBuf)> = std::fs::read_dir(&base)
        .ok()?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_dir() && find(p, "selector.in").is_some() && find(p, "fit.in").is_some() && find(p, "fit.out").is_some())
        .map(|p| (p.file_name().unwrap().to_string_lossy().to_string(), p))
        .filter(|(n, _)| only.as_ref().is_none_or(|o| n.eq_ignore_ascii_case(o)))
        .collect();
    v.sort();
    Some(v)
}

struct Case {
    prj: rusteau_core::Project,
    s: InverseSettings,
    r: FitOutRef,
}

fn load(dir: &Path) -> Case {
    let prj = read_legacy_project(dir).expect("project");
    let s = prj.inverse.clone().expect("Fit.in read with the project");
    let r = parse_fit_out(&std::fs::read_to_string(find(dir, "fit.out").unwrap()).unwrap());
    assert!(!r.iterations.is_empty() && !r.params.is_empty(), "Fit.out of {} not understood", dir.display());
    Case { prj, s, r }
}

/// Examples (and their check-run variants, e.g. UPINFIL_W1) whose estimation path with the iteration of HYDRUS depends
/// on the numerical noise of the objective (adaptive time steps): their objective has several local minima along
/// narrow valleys, and the minimum the HYDRUS iteration reaches depends on the last bits of the arithmetic. Initial
/// values changed by 1e-12 (relative), or another platform's math library, lead to other minima: UPINFIL between 4e-5
/// and 4e-3 (4.17 1.9e-3), HORIZINF 0.112 to 0.149 (4.17 0.1163), HORINFDR 0.065 to 0.082 (4.17 0.0706); 4.17 itself
/// ends at 2.7e-4 in UPINFIL and at 0.1071 in HORIZINF when θs is changed in the 6th digit (check runs UPINFIL_P,
/// HORIZINF_P). The Rusteau iteration (frozen time levels, several starting points) is not path-dependent there.
const PATH_DEPENDENT: &[&str] = &["HORINFDR", "HORIZINF", "UPINFIL"];

fn path_dependent(name: &str) -> bool {
    PATH_DEPENDENT.iter().any(|k| name.to_ascii_uppercase().starts_with(k))
}

fn ssq(s: &InverseSettings, w: &[f64], f: &[f64]) -> f64 {
    s.observations.iter().zip(w).zip(f).map(|((o, wi), fi)| (wi * (o.value - fi)).powi(2)).sum()
}

/// Weights of the measurement sets, the observation order of Fit.out and the objective function with the initial and
/// the final parameters of 4.17 (the forward model and the extraction of every observation type): within 1 %. In the
/// steep valleys of the path-dependent examples the 5 printed digits of the final parameters and the timing of a sharp
/// wetting front (h up to 2 cm apart at single observations) move the objective by a few %, there 10 % are allowed.
#[test]
fn weights_and_objective_at_the_hydrus_parameters() {
    let Some(list) = examples() else { return };
    let mut fails = vec![];
    for (name, dir) in &list {
        let c = load(dir);
        let w = c.s.effective_weights();
        let order = report_order(&c.s);
        for (k, ob) in c.r.observed.iter().enumerate() {
            let i = order[k];
            if (ob.1 - c.s.observations[i].x).abs() > 1e-3 * ob.1.abs().max(1.0) {
                fails.push(format!("[{name}] observation {} is out of order", k + 1));
            }
            if ob.5 >= 1e-3 && (w[i] - ob.5).abs() > 2e-3 * ob.5 {
                fails.push(format!("[{name}] weight of observation {}: {} (4.17: {})", k + 1, w[i], ob.5));
            }
        }
        let fin: Vec<f64> = c.r.params.iter().map(|p| p.1).collect();
        // the initial estimates of Fit.in (Fit.out prints them with 4 digits only)
        for (label, b, want) in [("first", c.s.initial(), c.r.iterations[0].0), ("final", fin, c.r.iterations.last().unwrap().0)] {
            match forward(&c.prj, &c.s, &b, false, None) {
                Ok(run) => {
                    let got = ssq(&c.s, &w, &run.values);
                    let rel = (got - want).abs() / want;
                    println!("{name:<9} {label:<5} SSQ {got:.5e}  4.17 {want:.4e}  ({:+.2} %)", 100.0 * (got - want) / want);
                    let tol = if label == "final" && path_dependent(name) { 0.1 } else { 0.01 };
                    if rel > tol {
                        fails.push(format!("[{name}] SSQ with the {label} parameters {got:.5e}, 4.17 {want:.4e}"));
                    }
                }
                Err(e) => fails.push(format!("[{name}] forward run failed: {e}")),
            }
        }
    }
    assert!(fails.is_empty(), "{}", fails.join("\n"));
}

/// The first Marquardt iteration from the initial estimates gives the parameters of iteration 1 of 4.17 (Marquardt
/// parameter 0.02 / 10, 1 % forward differences, scaling to a unit diagonal, constraints). The examples left out have
/// derivatives dominated by numerical noise (ONESTEP, UPINFIL), collinear parameters (FITTEST3) or a first step with
/// a different Marquardt parameter (FITTEST2, HORINFDR), where the path cannot be reproduced to the printed digits.
#[test]
fn first_marquardt_iteration_reproduces_hydrus() {
    let Some(list) = examples() else { return };
    let cases = ["DRAINAGE", "EVAPOR", "FITHYST", "FITTEST4", "FITTEST5", "HORIZINF", "MULTSTEP"];
    let mut fails = vec![];
    for (name, dir) in list.iter().filter(|(n, _)| cases.iter().any(|c| n.eq_ignore_ascii_case(c))) {
        let c = load(dir);
        let mut s = c.s.clone();
        s.max_iter = 1;
        s.hydrus_stop = true;
        let res = optimize(&c.prj, &s, &MarqOptions::default(), None, |_| {}).expect("optimization");
        let Some(it) = res.iterations.get(1) else {
            fails.push(format!("[{name}] no iteration accepted"));
            continue;
        };
        let (ref_ssq, ref_p) = &c.r.iterations[1];
        println!("{name:<9} iteration 1: SSQ {:.4e} (4.17 {:.4e}) {:?} (4.17 {:?})", it.ssq, ref_ssq, it.params.iter().map(|v| format!("{v:.4e}")).collect::<Vec<_>>(), ref_p);
        for (j, (a, b)) in it.params.iter().zip(ref_p).enumerate() {
            if (a - b).abs() > 0.01 * b.abs() {
                fails.push(format!("[{name}] iteration 1, {}: {a:.5e}, 4.17 {b:.4e}", res.names[j]));
            }
        }
        if (it.ssq - ref_ssq).abs() > 0.03 * ref_ssq {
            fails.push(format!("[{name}] iteration 1 SSQ {:.4e}, 4.17 {ref_ssq:.4e}", it.ssq));
        }
    }
    assert!(fails.is_empty(), "{}", fails.join("\n"));
}

/// Full parameter estimation with the Rusteau iteration (frozen time levels, the initial estimates and eight further
/// starting points): the objective must not end above the one of 4.17 by more than 5 %, in every example, also in the
/// path-dependent ones.
#[test]
fn parameter_estimation_compared_with_hydrus() {
    let Some(list) = examples() else { return };
    let mut fails = vec![];
    for (name, dir) in &list {
        let c = load(dir);
        let mut s = c.s.clone();
        s.hydrus_stop = false;
        s.extra_starts = DEFAULT_EXTRA_STARTS;
        let res = match optimize(&c.prj, &s, &MarqOptions::default(), None, |_| {}) {
            Ok(r) => r,
            Err(e) => {
                fails.push(format!("[{name}] optimization failed: {e}"));
                continue;
            }
        };
        let ref_ssq = c.r.iterations.last().unwrap().0;
        println!(
            "{name:<9} {:?}, {} iterations (4.17: {}), best of {} starts: {}, {} runs, SSQ {:.4e} (4.17 {:.4e}, {:+.1} %), R² {:.5} (4.17 {:?})",
            res.stop,
            res.iterations.len() - 1,
            c.r.iterations.len() - 1,
            res.starts.len(),
            res.best_start,
            res.runs,
            res.ssq,
            ref_ssq,
            100.0 * (res.ssq - ref_ssq) / ref_ssq,
            res.measures.r_squared,
            c.r.r_squared
        );
        for (p, q) in res.params.iter().zip(&c.r.params) {
            println!("    {:<7} {:>12.5e}  4.17 {:>12.5e} ± {:.3e}  ({:+.2} S.E.)", p.name, p.value, q.1, q.2, (p.value - q.1) / q.2.abs().max(1e-300));
        }
        if res.ssq > 1.05 * ref_ssq {
            fails.push(format!("[{name}] SSQ {:.4e} above the 4.17 value {ref_ssq:.4e} by more than 5 %", res.ssq));
        }
        if res.params.len() != c.r.params.len() {
            fails.push(format!("[{name}] {} fitted parameters, 4.17 {}", res.params.len(), c.r.params.len()));
        }
    }
    assert!(fails.is_empty(), "{}", fails.join("\n"));
}

/// The iteration of HYDRUS 4.17 (adaptive time steps in every run, one starting point): the examples with stable paths
/// end within 5 % of the 4.17 objective. The path-dependent examples, and the Rusteau iteration from the initial
/// estimates alone, are reported only.
#[test]
fn hydrus_iteration() {
    let Some(list) = examples() else { return };
    let mut fails = vec![];
    for (name, dir) in &list {
        let c = load(dir);
        let mut s = c.s.clone();
        s.hydrus_stop = true;
        let a = match optimize(&c.prj, &s, &MarqOptions::default(), None, |_| {}) {
            Ok(r) => r,
            Err(e) => {
                fails.push(format!("[{name}] HYDRUS iteration failed: {e}"));
                continue;
            }
        };
        let ref_ssq = c.r.iterations.last().unwrap().0;
        let stable = ["DRAINAGE", "EVAPOR", "FITTEST3", "FITTEST4", "FITTEST5", "MULTSTEP"].iter().any(|k| name.eq_ignore_ascii_case(k));
        let mut s1 = c.s.clone();
        s1.hydrus_stop = false;
        s1.extra_starts = 0;
        let b = optimize(&c.prj, &s1, &MarqOptions::default(), None, |_| {}).ok();
        println!(
            "{name:<9} HYDRUS iteration: {:?} {} iterations SSQ {:.4e} ({:+.1} %); Rusteau, one start: SSQ {}; 4.17 {ref_ssq:.4e}{}",
            a.stop,
            a.iterations.len() - 1,
            a.ssq,
            100.0 * (a.ssq - ref_ssq) / ref_ssq,
            b.as_ref().map(|b| format!("{:.4e}", b.ssq)).unwrap_or_else(|| "failed".into()),
            if path_dependent(name) { " (path-dependent)" } else { "" }
        );
        if stable && a.ssq > 1.05 * ref_ssq {
            fails.push(format!("[{name}] HYDRUS iteration: SSQ {:.4e}, 4.17 {ref_ssq:.4e}", a.ssq));
        }
    }
    assert!(fails.is_empty(), "{}", fails.join("\n"));
}
