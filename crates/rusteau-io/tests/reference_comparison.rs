//! Verification of the engine against the HYDRUS-1D reference outputs.
//!
//! * `test_all_direct_examples_comparison` runs every example of the HYDRUS-1D `Examples/Direct` directory (override the
//!   location with the environment variable `RUSTEAU_EXAMPLES_DIR`) and compares ALL columns of ALL reference files found
//!   there (`T_Level.out`, `A_Level.out`, `Nod_Inf.out`, `Obs_Node.out`, `Balance.out`, `Solute<i>.out`, `Meteo.out`).
//!   The differences to HYDRUS 4.17 that are understood (README, "Known differences") are accepted within an envelope
//!   (table `KNOWN`); anything else, or a known difference that grows, fails the test.
//! * The remaining tests need no external data: they check the readers, the column mapping and the tolerance logic with
//!   the real Fortran `Meteo.out` sample (`tests/data/Meteo_sample.out`) and with reference files generated from a
//!   simulation.

use rusteau_core::output::MeteoRow;
use rusteau_core::{examples, Project, Results, Simulation, StepStatus};
use rusteau_io::*;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

fn run(prj: Project) -> Simulation {
    let mut sim = Simulation::new(prj).expect("project");
    while sim.step() == StepStatus::Running {}
    sim
}

// ---------------------------------------------------------------------------------------------------------------------
// known differences to HYDRUS 4.17
// ---------------------------------------------------------------------------------------------------------------------

/// Why a difference to the HYDRUS 4.17 output is accepted (README, "Known differences").
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Cause {
    /// HYDRUS keeps h, theta, K and the temperatures in REAL*4; near the TolTh/TolH border this changes the number of
    /// iterations and with it the time steps (TEST1 with 10x tighter tolerances: 12 -> 5 problems).
    RealPrecision,
    /// No-flux top: HYDRUS 4.17 prints vTop up to 0.49 cm/h where rTop = 0 and its water balance error grows to 0.031 cm
    /// (rerun with tighter tolerances: 47622 steps); Rust keeps vTop = 0 and a balance error of 1e-13.
    NoFluxTopNoise,
    /// Onset of ponding with TolH = 1 cm: the run is chaotic after t ~ 0.86 h; with tighter tolerances both programs stop
    /// on non-convergence at the same transition (0.329 / 0.331 h).
    PondingChaos,
    /// HYDRUS 4.17 starts the drying scanning curve of the top node after ponding from a lower K at the reversal point
    /// than the 2009 source (5 - 14 % depending on the first drying step and the grid); the rule is unknown, Rust keeps
    /// the source.
    HysteresisTopNode,
    /// Dry surface at hCritA: vTop alternates every time step between evaporation and 0 in both programs; Latent is
    /// proportional to vTop. With dtMax = 0.002 both programs agree exactly (EnBal2b_DT).
    LatentOscillation,
    /// Triggered irrigation: vTop ~ +-1e-6 is booked as infiltration or evaporation by its sign.
    IrrigationSign,
    /// First time steps after the end of ponding: same time steps and iteration counts as HYDRUS 4.17, the heads near
    /// the surface differ by less than the iteration tolerance (with 100x tighter TolTh / TolH the Rust result itself
    /// moves 3 - 10 times more). The compiled 2009 source agrees with Rust.
    PondingEndTolerance,
}
use Cause::*;

/// One accepted problem: example folder, reference file, column (or Balance row), the largest relative deviation of the
/// worst value of a problem line |sim - ref| / max(|sim|, |ref|), the largest number of differing values / nodes in one
/// problem line and the largest number of problem lines (one per Nod_Inf / Balance time).  The envelopes are the
/// results of 9 Oct 2026 with 25 % (counts) and 50 % (deviations) head room.
struct Known {
    cause: Cause,
    example: &'static str,
    file: &'static str,
    column: &'static str,
    max_rel: f64,
    max_count: usize,
    max_lines: usize,
}

const fn k(cause: Cause, example: &'static str, file: &'static str, column: &'static str, max_rel: f64, max_count: usize, max_lines: usize) -> Known {
    Known { cause, example, file, column, max_rel, max_count, max_lines }
}

/// Examples of the HYDRUS-1D distribution (Examples/Direct) and the HYDRUS 4.17 check runs kept with them
/// (EnBal2b_A / EnBal2b_B: observation nodes at the surface, 5SEASON_IRR: triggered irrigation, 1SCALING_BC /
/// 2NOHYSTR_AE / 2NOHYSTR_DUR / 2NOHYSTR_LN: Brooks-Corey, air-entry, Durner and Kosugi models, 2HYSTER_RET: hysteresis
/// in the retention curve only, ROOTUPTK_ACT / ROOTUPTK_ACT2 / ROOTUPTK_ACT3: active solute uptake with one solute, of
/// the second of two solutes without and with a potential uptake rate).  Examples that are not listed here (1SCALING, 2NOHYSTR, 3LAIJURI, 3SELIM, 4HEAT,
/// 5SEASON, DRAINAGE, EnBal2b_B, TEST3, TEST4, TEST5, TEST9a, Test9, Test10, UPINFIL, 1SCALING_BC, 2NOHYSTR_AE,
/// 2NOHYSTR_DUR, and any other folder) must match without any problem.
const KNOWN: &[Known] = &[
    // RealPrecision
    k(RealPrecision, "ROOTUPTK", "Solute1.out", "cRoot", 0.0054, 6, 3),
    k(RealPrecision, "ROOTUPTK", "Nod_Inf.out", "Conc(1)", 0.0068, 5, 8),
    k(RealPrecision, "ROOTUPTK", "Obs_Node.out", "Conc1(5)", 0.004, 14, 3),
    k(RealPrecision, "ROOTUPTK_ACT", "Solute1.out", "cRoot", 0.003, 3, 2),
    k(RealPrecision, "ROOTUPTK_ACT", "Nod_Inf.out", "Conc(1)", 0.0018, 2, 2),
    k(RealPrecision, "ROOTUPTK_ACT2", "Solute1.out", "cRoot", 0.003, 3, 2),
    k(RealPrecision, "ROOTUPTK_ACT2", "Solute2.out", "cRoot", 0.0019, 2, 2),
    k(RealPrecision, "ROOTUPTK_ACT2", "Nod_Inf.out", "Conc(1)", 0.0019, 2, 2),
    k(RealPrecision, "ROOTUPTK_ACT3", "Solute1.out", "cRoot", 0.0032, 3, 2),
    k(RealPrecision, "ROOTUPTK_ACT3", "Solute2.out", "cRoot", 0.0028, 2, 2),
    k(RealPrecision, "ROOTUPTK_ACT3", "Nod_Inf.out", "Conc(1)", 0.0019, 2, 2),
    k(RealPrecision, "TEST1", "Nod_Inf.out", "Head", 0.012, 7, 8),
    k(RealPrecision, "TEST1", "Nod_Inf.out", "v/KsTop", 0.0071, 11, 8),
    k(RealPrecision, "TEST1", "Obs_Node.out", "h(1)", 0.0018, 4, 3),
    k(RealPrecision, "TEST1", "Obs_Node.out", "h(5)", 0.011, 10, 3),
    k(RealPrecision, "TEST11", "T_Level.out", "hBot", 0.0038, 159, 3),
    k(RealPrecision, "TEST11", "Nod_Inf.out", "Head", 0.0029, 4, 5),
    k(RealPrecision, "TEST11", "Obs_Node.out", "h(6)", 0.0038, 157, 3),
    k(RealPrecision, "TEST2", "Balance.out", "Bot Flux", 0.032, 4, 4),
    k(RealPrecision, "VOLATILE", "Solute1.out", "cTop", 0.0021, 5, 3),
    k(RealPrecision, "VOLATILE", "Balance.out", "Bot Flux", 0.04, 4, 3),
    k(RealPrecision, "VOLATILE", "Obs_Node.out", "h(2)", 0.0085, 5, 3),
    // NoFluxTopNoise
    k(NoFluxTopNoise, "1DRAINAG", "T_Level.out", "vTop", 1.6, 130, 3),
    k(NoFluxTopNoise, "1DRAINAG", "T_Level.out", "vBot", 0.047, 144, 3),
    k(NoFluxTopNoise, "1DRAINAG", "T_Level.out", "sum(vBot)", 0.013, 22, 3),
    k(NoFluxTopNoise, "1DRAINAG", "T_Level.out", "hTop", 0.044, 79, 3),
    k(NoFluxTopNoise, "1DRAINAG", "Nod_Inf.out", "Head", 0.0017, 9, 3),
    k(NoFluxTopNoise, "1DRAINAG", "Nod_Inf.out", "K", 0.0017, 50, 3),
    k(NoFluxTopNoise, "1DRAINAG", "Nod_Inf.out", "Flux", 0.0033, 14, 3),
    k(NoFluxTopNoise, "1DRAINAG", "Obs_Node.out", "h(1)", 0.043, 77, 3),
    k(NoFluxTopNoise, "1DRAINAG", "Obs_Node.out", "h(2)", 1.7, 40, 3),
    k(NoFluxTopNoise, "1DRAINAG", "Obs_Node.out", "h(3)", 1.4, 39, 3),
    k(NoFluxTopNoise, "1DRAINAG", "Obs_Node.out", "h(4)", 1.6, 22, 3),
    k(NoFluxTopNoise, "1DRAINAG", "Obs_Node.out", "h(5)", 1.3, 70, 3),
    // PondingChaos
    k(PondingChaos, "1INFILTR", "T_Level.out", "vTop", 0.15, 1291, 3),
    k(PondingChaos, "1INFILTR", "Nod_Inf.out", "Head", 1.5, 141, 6),
    k(PondingChaos, "1INFILTR", "Nod_Inf.out", "K", 0.18, 111, 6),
    k(PondingChaos, "1INFILTR", "Nod_Inf.out", "Flux", 0.26, 134, 6),
    k(PondingChaos, "1INFILTR", "Nod_Inf.out", "v/KsTop", 0.26, 122, 6),
    k(PondingChaos, "1INFILTR", "Balance.out", "In-flow", 0.053, 4, 9),
    k(PondingChaos, "1INFILTR", "Balance.out", "Top Flux", 0.053, 4, 5),
    k(PondingChaos, "1INFILTR", "Balance.out", "h Mean", 0.059, 4, 6),
    k(PondingChaos, "1INFILTR", "Obs_Node.out", "h(2)", 1.4, 1321, 3),
    k(PondingChaos, "1INFILTR", "Obs_Node.out", "h(3)", 1.5, 966, 3),
    k(PondingChaos, "1INFILTR", "Obs_Node.out", "h(4)", 1.5, 676, 3),
    k(PondingChaos, "1INFILTR", "Obs_Node.out", "h(5)", 0.034, 396, 3),
    // HysteresisTopNode
    k(HysteresisTopNode, "2HYSTER", "T_Level.out", "hTop", 0.0057, 35, 3),
    k(HysteresisTopNode, "2HYSTER", "A_Level.out", "hTop", 0.0093, 4, 3),
    k(HysteresisTopNode, "2HYSTER", "Nod_Inf.out", "K", 0.075, 4, 13),
    k(HysteresisTopNode, "2HYSTER", "Nod_Inf.out", "Head", 0.0036, 4, 3),
    k(HysteresisTopNode, "2HYSTER", "Obs_Node.out", "h(1)", 0.0056, 34, 3),
    // LatentOscillation
    k(LatentOscillation, "EnBal2b", "Meteo.out", "Latent", 0.085, 24, 3),
    k(LatentOscillation, "EnBal2b_A", "Meteo.out", "Latent", 0.085, 24, 3),
    // IrrigationSign
    k(IrrigationSign, "5SEASON_IRR", "T_Level.out", "sum(Infil)", 0.0072, 464, 3),
    k(IrrigationSign, "5SEASON_IRR", "T_Level.out", "sum(Evap)", 0.3, 464, 3),
    // PondingEndTolerance (check runs 2HYSTER_RET: hysteresis in the retention curve only; 2NOHYSTR_LN: Kosugi model)
    k(PondingEndTolerance, "2HYSTER_RET", "T_Level.out", "hTop", 0.0033, 2, 3),
    k(PondingEndTolerance, "2HYSTER_RET", "A_Level.out", "hTop", 0.0033, 2, 3),
    k(PondingEndTolerance, "2HYSTER_RET", "Obs_Node.out", "h(1)", 0.003, 3, 3),
    k(PondingEndTolerance, "2HYSTER_RET", "Obs_Node.out", "h(2)", 0.039, 8, 3),
    k(PondingEndTolerance, "2NOHYSTR_LN", "T_Level.out", "hTop", 0.0025, 2, 3),
    k(PondingEndTolerance, "2NOHYSTR_LN", "Obs_Node.out", "h(1)", 0.002, 3, 3),
    k(PondingEndTolerance, "2NOHYSTR_LN", "Obs_Node.out", "h(2)", 0.0064, 2, 3),
];

/// A problem line of the comparison, reduced to what the envelope checks.
struct Problem {
    file: String,
    column: String,
    count: usize,
    rel: f64,
}

/// Number at the start of `s` (up to the next ',', ')', ';' or blank).
fn number_at(s: &str) -> Option<f64> {
    s.split(|c: char| c == ',' || c == ')' || c == ';' || c.is_whitespace()).next()?.parse().ok()
}

/// Reads "[EX] [File.out @ t=..] column 'C': k/n values differ (worst ..: sim A, ref B) ..." and
/// "[EX] [Balance.out @ t=..] row 'R' region 0: sim A, ref B".  Lines of another form (parse or run errors, missing
/// records) give None and are never accepted.
fn parse_problem(line: &str) -> Option<Problem> {
    let rest = line.split_once("] [")?.1;
    let file = rest.split(']').next()?.split_whitespace().next()?.to_string();
    let col_start = rest.find("column '").map(|i| i + 8).or_else(|| rest.find("row '").map(|i| i + 5))?;
    let column = rest[col_start..].split('\'').next()?.to_string();
    let count = match rest.find(" values differ").or_else(|| rest.find(" nodes differ")) {
        Some(i) => rest[..i].rsplit(' ').next()?.split('/').next()?.parse().ok()?,
        None => 1,
    };
    let s = rest.find("sim ")?;
    let sim = number_at(&rest[s + 4..])?;
    let r = rest[s..].find("ref ")? + s;
    let rf = number_at(&rest[r + 4..])?;
    let rel = (sim - rf).abs() / sim.abs().max(rf.abs()).max(1e-300);
    Some(Problem { file, column, count, rel })
}

fn known_entry(example: &str, p: &Problem) -> Option<&'static Known> {
    KNOWN.iter().find(|k| k.example.eq_ignore_ascii_case(example) && k.file.eq_ignore_ascii_case(&p.file) && k.column == p.column)
}

/// Splits the problems of one example into known differences (within their envelope) and unexpected ones.
fn classify(example: &str, errors: Vec<String>) -> (Vec<String>, Vec<String>) {
    let mut known = Vec::new();
    let mut unexpected = Vec::new();
    let mut lines: std::collections::BTreeMap<(String, String), usize> = std::collections::BTreeMap::new();
    let mut kept: Vec<(String, Option<&'static Known>, Option<(String, String)>)> = Vec::new();
    for e in errors {
        match parse_problem(&e) {
            Some(p) => match known_entry(example, &p) {
                Some(kn) if p.count <= kn.max_count && p.rel <= kn.max_rel => {
                    let key = (p.file.clone(), p.column.clone());
                    *lines.entry(key.clone()).or_insert(0) += 1;
                    kept.push((e, Some(kn), Some(key)));
                }
                Some(kn) => unexpected.push(format!(
                    "{e}   <-- larger than the known {:?} difference (count {} > {} or deviation {:.3e} > {:.3e})",
                    kn.cause, p.count, kn.max_count, p.rel, kn.max_rel
                )),
                None => unexpected.push(e),
            },
            None => unexpected.push(e),
        }
    }
    for (e, kn, key) in kept {
        let (kn, key) = (kn.unwrap(), key.unwrap());
        let n = lines[&key];
        if n > kn.max_lines {
            unexpected.push(format!("{e}   <-- {n} problem lines for this column, known {:?} difference allows {}", kn.cause, kn.max_lines));
        } else {
            known.push(format!("{e}   [known: {:?}]", kn.cause));
        }
    }
    (known, unexpected)
}

// ---------------------------------------------------------------------------------------------------------------------
// benchmark against the Fortran examples
// ---------------------------------------------------------------------------------------------------------------------

fn examples_dir() -> PathBuf {
    std::env::var("RUSTEAU_EXAMPLES_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(r"C:\Users\Public\Documents\PC-Progress\Hydrus-1D 4.xx\Examples\Direct"))
}

/// The folder holds a HYDRUS-1D project (SELECTOR.IN in any letter case).
fn has_selector_in(dir: &Path) -> bool {
    std::fs::read_dir(dir).map_or(false, |rd| rd.flatten().any(|e| e.file_name().to_string_lossy().eq_ignore_ascii_case("selector.in")))
}

/// The HYDRUS reference run stopped because the solution did not converge (Error.msg in the example folder).
fn reference_stopped(dir: &Path) -> bool {
    std::fs::read_dir(dir).map_or(false, |rd| {
        rd.flatten().filter(|e| e.file_name().to_string_lossy().eq_ignore_ascii_case("error.msg")).any(|e| {
            std::fs::read_to_string(e.path()).map_or(false, |t| t.to_ascii_lowercase().replace(['\r', '\n'], "").contains("not converged"))
        })
    })
}

/// Key of a problem line for the statistics over all examples: "<file> column '<name>'".
fn problem_key(line: &str) -> String {
    let file = line.split(']').filter(|p| p.contains(".out") || p.contains(".OUT")).next().and_then(|p| p.rsplit('[').next()).unwrap_or("?").to_string();
    let col = line.split("column '").nth(1).or_else(|| line.split("row '").nth(1)).and_then(|r| r.split('\'').next()).unwrap_or("(other)");
    format!("{file} '{col}'")
}

/// Environment variables: `RUSTEAU_EXAMPLES_DIR` (examples directory), `RUSTEAU_COMPARE_ONLY` (comma separated example
/// names), `RUSTEAU_COMPARE_VERBOSE=1` (print every problem), `RUSTEAU_COMPARE_STRICT=1` (known differences fail too),
/// `RUSTEAU_COMPARE_REPORT` (path of the full report, default `reference_comparison_report.txt` in the working directory).
#[test]
fn test_all_direct_examples_comparison() {
    let base = examples_dir();
    assert!(base.exists(), "FATAL: benchmark directory not found at {:?} (set RUSTEAU_EXAMPLES_DIR). Verification cannot proceed.", base);
    let only: Option<Vec<String>> = std::env::var("RUSTEAU_COMPARE_ONLY").ok().map(|v| v.split(',').map(|s| s.trim().to_ascii_lowercase()).collect());
    let verbose = std::env::var("RUSTEAU_COMPARE_VERBOSE").map_or(false, |v| v == "1");
    let strict = std::env::var("RUSTEAU_COMPARE_STRICT").map_or(false, |v| v == "1");

    struct Outcome {
        name: String,
        errors: Vec<String>,
        known: Vec<String>,
        files: Vec<(String, usize)>,
        bracketed: usize,
        en_bal: bool,
        seconds: f64,
        steps: usize,
        messages: Vec<String>,
    }
    let mut outcomes: Vec<Outcome> = Vec::new();
    let mut entries: Vec<PathBuf> = std::fs::read_dir(&base).expect("examples directory").flatten().map(|e| e.path()).filter(|p| p.is_dir() && has_selector_in(p)).collect();
    entries.sort();
    for path in entries {
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        if let Some(o) = &only {
            if !o.contains(&name.to_ascii_lowercase()) {
                continue;
            }
        }
        let t0 = std::time::Instant::now();
        let mut out = Outcome { name: name.clone(), errors: vec![], known: vec![], files: vec![], bracketed: 0, en_bal: false, seconds: 0.0, steps: 0, messages: vec![] };
        let mut problems = Vec::new();
        let mut stopped_too = None;
        match read_legacy_project(&path) {
            Err(e) => problems.push(format!("[{name}] project parse error: {e}")),
            Ok(prj) if prj.processes.inverse => {
                // the outputs of an inverse example belong to the final run with the fitted parameters
                println!("Skipping {name:<14} ... inverse problem (compared with its Fit.out by the inverse_test)");
                continue;
            }
            Ok(prj) => {
                out.en_bal = prj.atmosphere.meteo.as_ref().map_or(false, |m| m.l_en_bal);
                match Simulation::new(prj.clone()) {
                    Err(e) => problems.push(format!("[{name}] init error: {e}")),
                    Ok(mut sim) => {
                        while sim.step() == StepStatus::Running {}
                        out.messages = sim.res.messages.clone();
                        out.steps = sim.res.tlevel.len();
                        if sim.res.failed {
                            let msg = format!("[{name}] the simulation did not finish: {:?}", sim.res.messages.last());
                            if reference_stopped(&path) && !strict {
                                stopped_too = Some(msg);
                            } else {
                                problems.push(msg);
                            }
                        }
                        let rep = compare_folder(&path, &prj, &sim.res);
                        out.files = rep.files;
                        out.bracketed = rep.bracketed;
                        problems.extend(rep.errors);
                        if out.files.is_empty() && problems.is_empty() {
                            problems.push(format!("[{name}] no reference output files found"));
                        }
                    }
                }
            }
        }
        if strict {
            out.errors = problems;
        } else {
            (out.known, out.errors) = classify(&name, problems);
        }
        if let Some(m) = stopped_too {
            out.known.insert(0, format!("{m}   [known: the HYDRUS reference run stopped on non-convergence as well (Error.msg)]"));
        }
        out.seconds = t0.elapsed().as_secs_f64();
        let n: usize = out.files.iter().map(|f| f.1).sum();
        let verdict = match (out.errors.is_empty(), out.known.is_empty()) {
            (false, _) => format!("FAILED ({} problems{})", out.errors.len(), if out.known.is_empty() { String::new() } else { format!(", {} known", out.known.len()) }),
            (true, true) => "PASSED".to_string(),
            (true, false) => format!("PASSED ({} known differences)", out.known.len()),
        };
        println!(
            "Testing {:<14} ... {verdict} ({} files, {} values, {} at non-coinciding times, {} stored levels, {:.1} s)",
            out.name,
            out.files.len(),
            n,
            out.bracketed,
            out.steps,
            out.seconds,
        );
        outcomes.push(out);
    }
    assert!(!outcomes.is_empty(), "no example selected / found in {:?}", base);

    // ---- details first (the assertions below stop the test)
    let report_path = std::env::var("RUSTEAU_COMPARE_REPORT").unwrap_or_else(|_| "reference_comparison_report.txt".into());
    let mut report = String::new();
    println!("\n================ DETAILS ================");
    for o in outcomes.iter().filter(|o| !o.errors.is_empty()) {
        println!("\n--- {} ({} problems) ---", o.name, o.errors.len());
        for m in o.messages.iter().take(3) {
            println!("   run message: {m}");
        }
        for (i, e) in o.errors.iter().enumerate() {
            if verbose || i < 12 {
                println!("   {e}");
            }
        }
        if !verbose && o.errors.len() > 12 {
            println!("   ... {} more (RUSTEAU_COMPARE_VERBOSE=1 or the report file)", o.errors.len() - 12);
        }
    }
    for o in outcomes.iter().filter(|o| !o.errors.is_empty() || !o.known.is_empty()) {
        let _ = writeln!(report, "--- {} ({} problems, {} known differences) ---", o.name, o.errors.len(), o.known.len());
        for e in o.errors.iter().chain(&o.known) {
            let _ = writeln!(report, "{e}");
        }
    }
    // known differences that did not show up (fixed, or the example is not in the folder)
    let ran: Vec<&str> = outcomes.iter().map(|o| o.name.as_str()).collect();
    for kn in KNOWN.iter().filter(|kn| ran.iter().any(|r| r.eq_ignore_ascii_case(kn.example))) {
        let o = outcomes.iter().find(|o| o.name.eq_ignore_ascii_case(kn.example)).unwrap();
        let seen = o.known.iter().chain(&o.errors).any(|e| parse_problem(e).is_some_and(|p| p.file.eq_ignore_ascii_case(kn.file) && p.column == kn.column));
        if !seen {
            let _ = writeln!(report, "note: known difference {} {} '{}' no longer occurs", kn.example, kn.file, kn.column);
        }
    }
    let _ = std::fs::write(&report_path, &report);

    // ---- which columns fail in how many examples (systematic problems show up here)
    let mut stats: std::collections::BTreeMap<String, (usize, usize)> = std::collections::BTreeMap::new();
    for o in &outcomes {
        let mut seen = std::collections::BTreeSet::new();
        for e in &o.errors {
            let k = problem_key(e);
            let ent = stats.entry(k.clone()).or_insert((0, 0));
            ent.1 += 1;
            if seen.insert(k) {
                ent.0 += 1;
            }
        }
    }
    let mut ranked: Vec<_> = stats.into_iter().collect();
    ranked.sort_by(|a, b| b.1 .0.cmp(&a.1 .0).then(b.1 .1.cmp(&a.1 .1)));
    if !ranked.is_empty() {
        println!("\n================ FAILING COLUMNS (examples affected / problems) ================");
        for (k, (ex, n)) in ranked.iter().take(40) {
            println!("  {ex:>3} / {n:>4}   {k}");
        }
    }

    let total = outcomes.len();
    let passed = outcomes.iter().filter(|o| o.errors.is_empty()).count();
    let exact = outcomes.iter().filter(|o| o.errors.is_empty() && o.known.is_empty()).count();
    let values: usize = outcomes.iter().map(|o| o.files.iter().map(|f| f.1).sum::<usize>()).sum();
    let known: usize = outcomes.iter().map(|o| o.known.len()).sum();
    let en_run = outcomes.iter().filter(|o| o.en_bal).count();
    let en_ok = outcomes.iter().filter(|o| o.en_bal && o.errors.is_empty()).count();
    println!("\n==========================================");
    println!("Benchmark Results: {passed}/{total} passed ({exact} without any difference, {known} known differences accepted), {values} reference values compared");
    println!("Energy balance projects executed: {en_run}, verified: {en_ok}");
    println!("Full report: {report_path}");
    println!("==========================================");

    let problems: usize = outcomes.iter().map(|o| o.errors.len()).sum();
    assert!(problems == 0, "{problems} problems in {} of {total} examples (details above and in {report_path})", total - passed);
    if en_run > 0 {
        assert!(en_ok > 0, "STRICTNESS FAILURE: no project with lEnBal = true was verified.");
    }
}

/// Replay of the HYDRUS time steps: examples whose difference to 4.17 comes only from a different sequence of time steps
/// (iteration counts near the TolTh / TolH limit in single precision) are run on the time levels of the 4.17
/// `T_Level.out` (`Simulation::fixed_levels`) and must then agree with all reference files within print precision.
/// TEST1: 12 problems with its own time steps, none on those of 4.17.
#[test]
fn time_step_replay_of_hydrus_runs() {
    const REPLAY_EXACT: &[&str] = &["TEST1"];
    let base = examples_dir();
    let mut fails = vec![];
    let mut checked = 0;
    for name in REPLAY_EXACT {
        let dir = base.join(name);
        if !has_selector_in(&dir) {
            continue;
        }
        let Some(tl) = std::fs::read_dir(&dir).ok().and_then(|rd| rd.flatten().map(|e| e.path()).find(|p| p.file_name().is_some_and(|f| f.to_string_lossy().eq_ignore_ascii_case("t_level.out")))) else {
            continue;
        };
        let prj = read_legacy_project(&dir).expect("project");
        let t = read_t_level(&tl).expect("T_Level.out");
        let mut levels: Vec<f64> = t.times.iter().copied().filter(|&x| x > prj.time.t_init + 1e-12).collect();
        levels.dedup();
        let mut sim = Simulation::new(prj.clone()).expect("project");
        sim.fixed_levels = Some(std::sync::Arc::new(levels.clone()));
        while sim.step() == StepStatus::Running {}
        let rep = compare_folder(&dir, &prj, &sim.res);
        println!("{name}: {} levels of 4.17, {} simulated, {} problems", levels.len(), sim.res.tlevel.len(), rep.errors.len());
        if sim.res.tlevel.len() != levels.len() {
            fails.push(format!("{name}: {} time levels on the 4.17 levels ({} expected)", sim.res.tlevel.len(), levels.len()));
        }
        fails.extend(rep.errors.iter().map(|e| format!("{name}: {e}")));
        checked += 1;
    }
    if checked == 0 {
        println!("skipped: no replay examples in {}", base.display());
    }
    assert!(fails.is_empty(), "{}", fails.join("\n"));
}

#[test]
fn problem_lines_are_parsed_for_the_known_difference_envelope() {
    let p = parse_problem("[2HYSTER] [Nod_Inf.out @ t=2] column 'K': 1/61 nodes differ (worst node 1: sim 4.149711e-1, ref 3.944000e-1); first nodes [at 1: sim 4.149711e-1, ref 3.944000e-1] median sim/ref = 1.0522").unwrap();
    assert_eq!((p.file.as_str(), p.column.as_str(), p.count), ("Nod_Inf.out", "K", 1));
    assert!((p.rel - (0.4149711 - 0.3944) / 0.4149711).abs() < 1e-12);
    let p = parse_problem("[TEST2] [Balance.out @ t=120] row 'Bot Flux' region 0: sim -1.291967e-2, ref -1.314500e-2").unwrap();
    assert_eq!((p.file.as_str(), p.column.as_str(), p.count), ("Balance.out", "Bot Flux", 1));
    let p = parse_problem("[EnBal2b] [Meteo.out] column 'Latent': 17/1623 values differ (worst at t=329.49: sim 4.691988e-2, ref 4.972300e-2); [at 329.39: sim 1.027585e0, ref 1.028800e0]").unwrap();
    assert_eq!((p.file.as_str(), p.column.as_str(), p.count), ("Meteo.out", "Latent", 17));
    assert!(parse_problem("[X] the simulation did not finish: None").is_none());

    // within the envelope -> known; a larger deviation, an unknown column or another example -> unexpected
    let line = |sim: f64| format!("[2HYSTER] [Nod_Inf.out @ t=2] column 'K': 1/61 nodes differ (worst node 1: sim {sim:e}, ref 3.944000e-1)");
    let (known, unexpected) = classify("2HYSTER", vec![line(0.41497)]);
    assert_eq!((known.len(), unexpected.len()), (1, 0));
    let (known, unexpected) = classify("2HYSTER", vec![line(0.6)]);
    assert_eq!((known.len(), unexpected.len()), (0, 1));
    let (known, unexpected) = classify("2NOHYSTR", vec![line(0.41497)]);
    assert_eq!((known.len(), unexpected.len()), (0, 1));
    let (_, unexpected) = classify("2HYSTER", vec!["[2HYSTER] [Nod_Inf.out @ t=2] column 'Head': 1/61 nodes differ (worst node 1: sim -38.3, ref -40.0)".into()]);
    assert_eq!(unexpected.len(), 1);
    // too many problem lines for one column
    let (known, unexpected) = classify("TEST2", (0..6).map(|i| format!("[TEST2] [Balance.out @ t={i}] row 'Bot Flux' region 0: sim -1.291967e-2, ref -1.314500e-2")).collect());
    assert_eq!((known.len(), unexpected.len()), (0, 6));
}

// ---------------------------------------------------------------------------------------------------------------------
// Meteo.out sample (real Fortran output)
// ---------------------------------------------------------------------------------------------------------------------

/// `tests/data/Meteo_sample.out`, a `Meteo.out` written by the Fortran code (energy balance, 1217 time steps). It is not
/// part of the source package; the tests that need it are skipped without it.
fn meteo_sample() -> Option<String> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data/Meteo_sample.out");
    let s = std::fs::read_to_string(&path).ok();
    if s.is_none() {
        eprintln!("skipped: {} not found", path.display());
    }
    s
}

fn sample_results(sample: &str) -> Results {
    let mut r = Results::default();
    r.meteo_mode = 2;
    for v in sample.lines().filter_map(parse_numbers).filter(|v| v.len() == 8) {
        r.meteo.push(MeteoRow { t: v[0], rns: v[1], rnl: v[2], rn: v[3], sensible: v[4], latent: v[5], heat_flux: v[6], balance: v[7], ..Default::default() });
    }
    r
}

fn write_tmp(name: &str, text: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("rusteau_cmp_{}_{}", std::process::id(), name));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("Meteo.out"), text).unwrap();
    dir
}

fn meteo_tr(t: f64) -> f64 {
    10f64.powi(t.abs().max(1e-30).log10().floor() as i32 - 4)
}

#[test]
fn meteo_sample_is_read_with_all_columns() {
    let Some(sample) = meteo_sample() else {
        return;
    };
    let dir = write_tmp("read", &sample);
    let (mode, table) = read_meteo_out(&dir.join("Meteo.out")).expect("Meteo.out");
    assert_eq!(mode, 2, "energy-balance layout without daily data");
    assert_eq!(table.names, vec!["Short.Rad.", "Long.Rad.", "Radiation", "Sensible", "Latent", "Heat Flux", "Balance"]);
    assert_eq!(table.times.len(), 1217);
    assert!((table.times[0] - 328.00).abs() < 1e-9 && (table.cols[1][0] + 7.2580).abs() < 1e-9);
    assert!((table.cols[5][0] - 18.064).abs() < 1e-9, "Heat Flux of the first row");
}

#[test]
fn meteo_sample_compares_clean_against_itself_and_detects_changes() {
    let Some(sample) = meteo_sample() else {
        return;
    };
    let dir = write_tmp("self", &sample);
    let (_, table) = read_meteo_out(&dir.join("Meteo.out")).unwrap();
    let res = sample_results(&sample);
    let (st, ss) = sim_meteo(&res);
    let (errors, n) = compare_series("Meteo.out", &meteo_tr, &table, &st, &ss, &[]);
    assert!(errors.is_empty(), "{errors:?}");
    assert_eq!(n, 7 * 1217, "every column of every row must be compared");

    // time stamps are printed with five digits (0.01 d at day 328): a simulation that is sampled slightly differently still passes
    let mut jittered = res.clone();
    for (i, m) in jittered.meteo.iter_mut().enumerate() {
        m.t += if i % 2 == 0 { 0.003 } else { -0.003 };
    }
    let (jt, js) = sim_meteo(&jittered);
    let mut order: Vec<usize> = (0..jt.len()).collect();
    order.sort_by(|a, b| jt[*a].partial_cmp(&jt[*b]).unwrap());
    let jt_sorted: Vec<f64> = order.iter().map(|&i| jt[i]).collect();
    let js_sorted: Vec<(String, Vec<f64>)> = js.iter().map(|(n, v)| (n.clone(), order.iter().map(|&i| v[i]).collect())).collect();
    let (errors, _) = compare_series("Meteo.out", &meteo_tr, &table, &jt_sorted, &js_sorted, &[]);
    assert!(errors.is_empty(), "{errors:?}");

    // a change of a single column is reported for exactly that column
    let mut bad = res.clone();
    for m in bad.meteo.iter_mut().skip(400).take(50) {
        m.latent *= 1.05;
    }
    let (bt, bs) = sim_meteo(&bad);
    let (errors, _) = compare_series("Meteo.out", &meteo_tr, &table, &bt, &bs, &[]);
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(errors[0].contains("'Latent'"), "{}", errors[0]);
}

#[test]
fn reference_column_without_counterpart_is_an_error() {
    let Some(sample) = meteo_sample() else {
        return;
    };
    let dir = write_tmp("missing", &sample);
    let (_, table) = read_meteo_out(&dir.join("Meteo.out")).unwrap();
    let res = sample_results(&sample);
    let (st, mut ss) = sim_meteo(&res);
    ss.retain(|(n, _)| n != "Sensible");
    let (errors, _) = compare_series("Meteo.out", &meteo_tr, &table, &st, &ss, &[]);
    assert!(errors.iter().any(|e| e.contains("'Sensible'") && e.contains("no simulated counterpart")), "{errors:?}");
}

#[test]
fn meteo_mode_is_detected_from_the_header_and_column_count() {
    let pm = "\n   Time       ET        Evap    Transp     Rns        Rnl    RadTerm  AeroTerm     Prec     Interc   ExInterc\n    [d]     [mm/d]    [mm/d]    [mm/d] [MJ/m2/d]  [MJ/m2/d]   [mm/d]    [mm/d]    [mm/d]    [mm/d]    [mm/d]\n\n  328.00     2.100     1.000     1.100     5.000     3.000     1.000     1.100     0.000     0.000     0.000\nend\n";
    let dir = write_tmp("pm", pm);
    assert_eq!(detect_meteo_mode(&dir.join("Meteo.out")), Some(0));
    let (_, t) = read_meteo_out(&dir.join("Meteo.out")).unwrap();
    assert_eq!(t.names.len(), 10);
    assert!((t.cols[0][0] - 2.1).abs() < 1e-12);
}

// ---------------------------------------------------------------------------------------------------------------------
// generated reference files: parsers, column mapping and mismatch detection for every supported file
// ---------------------------------------------------------------------------------------------------------------------

fn e(v: f64) -> String {
    format!("{:13.5e}", v)
}

fn write_reference_files(dir: &Path, res: &Results) {
    std::fs::create_dir_all(dir).unwrap();
    // T_Level.out
    let mut s = String::from("\n       Time          rTop   ...\n        [T]         [L/T]\n\n");
    for r in &res.tlevel {
        let _ = write!(s, "{:13.4}", r.t);
        for v in [
            r.r_top, r.r_root, r.v_top, r.v_root, r.v_bot, r.cum_r_top, r.cum_r_root, r.cum_v_top, r.cum_v_root, r.cum_v_bot, r.h_top, r.h_root, r.h_bot, r.run_off, r.cum_run_off,
            r.volume, r.cum_infil, r.cum_evap,
        ] {
            s.push_str(&e(v));
        }
        let _ = write!(s, "{:7}", r.tlevel);
        s.push_str(&e(r.cum_w_transf));
        let _ = writeln!(s, "{:11.3}", r.snow_layer);
    }
    std::fs::write(dir.join("T_Level.out"), s).unwrap();
    // A_Level.out
    let mut s = String::from("\n   Time         sum(rTop)  ...\n    [T]           [L]\n\n");
    for (i, a) in res.alevel.iter().enumerate() {
        let _ = write!(s, "{:12.5}", a.t);
        for v in a.cum {
            s.push_str(&format!("{:14.6e}", v));
        }
        let _ = writeln!(s, "{:11.3}{:11.3}{:11.3}{:8}", a.h_top, a.h_root, a.h_bot, i + 1);
    }
    std::fs::write(dir.join("A_Level.out"), s).unwrap();
    // Solute<i>.out
    for j in 0..res.n_solutes {
        let mut s = String::from(" All solute fluxes and cumulative solute fluxes are positive into the region\n\n       Time         cvTop ...\n\n");
        for (k, r) in res.tlevel.iter().enumerate() {
            let c = &r.solutes[j];
            let _ = write!(s, "{:13.4}", r.t);
            for v in [c.cv_top, c.cv_bot, c.cum_top, c.cum_bot, c.cum_ch0, c.cum_ch1, c.c_top, c.c_root, c.c_bot, c.cv_root, c.cum_root, c.cum_neq] {
                s.push_str(&e(v));
            }
            let _ = write!(s, "{:7}", k + 1);
            s.push_str(&e(c.c_gwl));
            s.push_str(&e(c.c_run_off));
            let _ = writeln!(s, "{}", e(c.cum_run_off));
        }
        std::fs::write(dir.join(format!("Solute{}.out", j + 1)), s).unwrap();
    }
    // Nod_Inf.out (variant with Kappa / v/KsTop / Temp and solute columns)
    let mut s = String::new();
    for p in &res.profiles {
        let _ = write!(s, "\n Time: {:14.4}\n\n", p.t);
        s.push_str(" Node      Depth      Head Moisture       K          C         Flux        Sink         Kappa   v/KsTop   Temp   Conc(1..NS) Sorb(1...NS)\n");
        s.push_str("           [L]        [L]    [-]        [L/T]      [1/L]      [L/T]        [1/T]         [-]      [-]      [C]      [M/L*3]\n\n");
        for n in p.nodes.iter() {
            let _ = write!(s, "{:4} {:10.4} {:11.3} {:6.4}", n.node, -n.depth, n.h, n.theta); // Fortran: x(i) - xSurf < 0
            for v in [n.k, n.c, n.flux, n.sink] {
                s.push_str(&format!("{:12.4e}", v));
            }
            let _ = write!(s, "{:8} {:11.3e}{:8.2}", n.kappa, n.v_over_ks, n.temp);
            for v in n.conc.iter().chain(n.sorb.iter()) {
                s.push_str(&format!("{:12.4e}", v));
            }
            s.push('\n');
        }
    }
    std::fs::write(dir.join("Nod_Inf.out"), s).unwrap();
    // Balance.out
    let mut s = String::new();
    for b in &res.balance {
        let _ = write!(s, "\n----------------------------------------------------------\n Time       [T]{:14.4}\n----------------------------------------------------------\n", b.t);
        let row = |lab: &str, f: &dyn Fn(&rusteau_core::output::SubRegion) -> f64| -> String {
            let mut l = format!(" {lab}");
            l.push_str(&e(f(&b.total)));
            for sub in &b.sub {
                l.push_str(&e(f(sub)));
            }
            l.push('\n');
            l
        };
        s.push_str(&row("Area     [L]      ", &|r| r.area));
        s.push_str(&row("W-volume [L]      ", &|r| r.volume));
        s.push_str(&row("In-flow  [L/T]    ", &|r| r.change));
        s.push_str(&row("h Mean   [L]      ", &|r| r.h_mean));
        for j in 0..res.n_solutes {
            s.push_str(&row(&format!("ConcVol  [M/L2] {} ", j + 1), &|r| r.c_vol.get(j).copied().unwrap_or(0.0)));
            s.push_str(&row(&format!("cMean    [M/L3] {} ", j + 1), &|r| r.c_mean.get(j).copied().unwrap_or(0.0)));
        }
        let _ = writeln!(s, " Top Flux [L/T]    {}\n Bot Flux [L/T]    {}", e(b.top_flux), e(b.bot_flux));
        let _ = writeln!(s, " WatBalT  [L]      {}\n WatBalR  [%]      {:13.3}", e(b.wat_bal_t), b.wat_bal_r);
        for j in 0..res.n_solutes {
            let _ = writeln!(s, " CncBalT  [M]    {} {}\n CncBalR  [%]    {} {:13.3}", j + 1, e(b.sol_bal_t[j]), j + 1, b.sol_bal_r[j]);
        }
    }
    std::fs::write(dir.join("Balance.out"), s).unwrap();
}

fn small_solute_run() -> (Project, Results) {
    // atmospheric boundary condition (A_Level records) with a solute pulse in the rain water
    use rusteau_core::{AtmRecord, SoluteMaterial, SpeciesMaterial};
    let mut p = examples::infiltration_evaporation();
    p.processes.root_water_uptake = false;
    let m0 = p.water.materials[0].clone();
    p.water.materials = vec![m0];
    p.processes.solute = true;
    p.solute = examples::solute_pulse().solute.clone();
    p.solute.k_top = -1;
    p.solute.t_pulse = 1.0e6;
    p.solute.materials = vec![SoluteMaterial { bulk_density: 1.3, disp_l: 2.0, frac: 1.0, th_immobile: 0.0, th_im_matrix: 0.0 }];
    p.solute.species[0].per_material = vec![SpeciesMaterial { ks: 0.2, beta: 1.0, mu_w: 0.01, ..Default::default() }];
    for nd in p.profile.nodes.iter_mut() {
        nd.mat = 1;
        nd.beta = 0.0;
        nd.h = -100.0;
        nd.conc = vec![0.0];
        nd.sorb = vec![0.0];
    }
    p.time.t_max = 6.0;
    p.time.print_times = vec![2.0, 4.0, 6.0];
    p.atmosphere.records = (1..=6)
        .map(|i| AtmRecord { t: i as f64, prec: if i <= 3 { 1.5 } else { 0.0 }, evap: if i > 3 { 0.2 } else { 0.0 }, h_crit_a: 10000.0, c_top: vec![1.0], c_bot: vec![0.0], ..Default::default() })
        .collect();
    let sim = run(p.clone());
    assert!(!sim.res.alevel.is_empty() && !sim.res.balance.is_empty() && !sim.res.profiles.is_empty());
    (p, sim.res)
}

#[test]
fn generated_reference_files_compare_clean_for_every_file_and_column() {
    let (prj, res) = small_solute_run();
    let dir = std::env::temp_dir().join(format!("rusteau_cmp_{}_pipeline", std::process::id()));
    write_reference_files(&dir, &res);
    let rep = compare_folder(&dir, &prj, &res);
    assert!(rep.errors.is_empty(), "{:#?}", rep.errors);
    let names: Vec<&str> = rep.files.iter().map(|f| f.0.as_str()).collect();
    for want in ["T_Level.out", "A_Level.out", "Solute1.out", "Nod_Inf.out", "Balance.out"] {
        assert!(names.contains(&want), "{want} not compared: {names:?}");
    }
    for (f, n) in &rep.files {
        assert!(*n > 0, "{f}: no values compared");
    }
    // T_Level: 20 value columns (TLevel is an index) for every row; Solute: 14 columns
    let tl = rep.files.iter().find(|f| f.0 == "T_Level.out").unwrap().1;
    assert_eq!(tl, 20 * res.tlevel.len());
}

#[test]
fn generated_reference_files_detect_a_changed_value_in_each_file() {
    let (prj, res) = small_solute_run();
    let dir = std::env::temp_dir().join(format!("rusteau_cmp_{}_perturb", std::process::id()));
    write_reference_files(&dir, &res);
    let mut hit = Vec::new();
    for file in ["T_Level.out", "A_Level.out", "Solute1.out", "Nod_Inf.out", "Balance.out"] {
        let path = dir.join(file);
        let original = std::fs::read_to_string(&path).unwrap();
        // multiply the first number of the last numeric line by 1.5 (a clearly visible change)
        let mut lines: Vec<String> = original.lines().map(|l| l.to_string()).collect();
        let idx = lines.iter().rposition(|l| parse_numbers(l).map_or(false, |v| v.len() > 3 && v[1].abs() > 1e-9)).or_else(|| lines.iter().rposition(|l| l.contains("Top Flux")));
        let Some(i) = idx else { continue };
        let toks: Vec<String> = lines[i].split_whitespace().map(|t| t.to_string()).collect();
        // change the second numeric token (the first value after the time / node column) when present
        let mut new_line = String::new();
        let mut changed = false;
        for (k, t) in toks.iter().enumerate() {
            if let Ok(v) = t.parse::<f64>() {
                if k >= 1 && !changed && v.abs() > 1e-9 {
                    new_line.push_str(&format!(" {:.6e}", v * 1.5));
                    changed = true;
                    continue;
                }
            }
            new_line.push_str(&format!(" {t}"));
        }
        lines[i] = new_line;
        std::fs::write(&path, lines.join("\n") + "\n").unwrap();
        let rep = compare_folder(&dir, &prj, &res);
        let found = rep.errors.iter().any(|m| m.contains(file));
        std::fs::write(&path, original).unwrap();
        if found {
            hit.push(file);
        }
    }
    println!("changes detected in: {hit:?}");
    assert!(hit.contains(&"T_Level.out") && hit.contains(&"A_Level.out") && hit.contains(&"Nod_Inf.out"), "{hit:?}");
}

#[test]
fn tolerances_are_column_specific() {
    assert!(tolerance("Kappa").abs >= 0.5 && tolerance("Kappa").rel == 0.0);
    assert!(tolerance("WatBalR").abs == 0.05);
    assert!(tolerance("Moisture").abs == 0.005);
    assert!(tolerance("Temp").abs == 0.1);
    assert!(tolerance("vTop").rel == 2e-3);
    assert!(tolerance("hTop").rel == 1e-3);
    assert!(tolerance("Depth").abs > 0.005, "reference depths are printed with two decimals");
    assert!(balance_tolerance("Area").abs > 0.05 && balance_tolerance("ConcVol1").rel >= 6e-3 && balance_tolerance("WatBalT").abs >= 1e-4 && balance_tolerance("WatBalR").abs == 0.05);
}

#[test]
fn reference_time_without_a_simulated_step_is_checked_against_the_neighbouring_steps() {
    // simulated steps at 0, 1, 2 (values 0, 10, 20); the reference prints t = 1.5 with 15 (between the neighbours) and 25 (outside)
    let reference = |v: f64| RefTable { times: vec![1.5], names: vec!["vTop".into()], cols: vec![vec![v]], ..Default::default() };
    let st = [0.0, 1.0, 2.0];
    let sim: SimSeries = vec![("vTop".into(), vec![0.0, 10.0, 20.0])];
    let (e, n, br) = compare_series_ex("x", &|_| 1e-4, &reference(15.0), &st, &sim, &[]);
    assert!(e.is_empty() && n == 1 && br == 1, "{e:?} {n} {br}");
    let (e, _, _) = compare_series_ex("x", &|_| 1e-4, &reference(25.0), &st, &sim, &[]);
    assert_eq!(e.len(), 1);
    assert!(e[0].contains("median sim/ref"), "{}", e[0]);
    // a reference time on a simulated step is compared strictly
    let on_step = RefTable { times: vec![1.0], names: vec!["vTop".into()], cols: vec![vec![12.0]], ..Default::default() };
    let (e, n, br) = compare_series_ex("x", &|_| 1e-4, &on_step, &st, &sim, &[]);
    assert!(e.len() == 1 && n == 1 && br == 0, "{e:?}");
}

#[test]
fn nod_inf_depth_is_negative_below_the_surface_and_extra_columns_are_named_by_solute_count() {
    let (_prj, res) = small_solute_run();
    let dir = std::env::temp_dir().join(format!("rusteau_cmp_{}_depth", std::process::id()));
    write_reference_files(&dir, &res);
    let blocks = read_nod_inf_full(&dir.join("Nod_Inf.out"), res.n_solutes);
    assert!(!blocks.is_empty());
    let b = &blocks[0];
    assert_eq!(&b.names[1..3], &["Depth", "Head"]);
    assert!(b.names.contains(&"Conc(1)".to_string()) && b.names.contains(&"Sorb(1)".to_string()));
    assert!(b.rows[0][1].abs() < 1e-9, "node 1 is the top node (depth 0)");
    assert!(b.rows[b.rows.len() - 1][1] < -99.0, "the bottom node lies 100 cm below the surface: {}", b.rows[b.rows.len() - 1][1]);
    let (e, n) = compare_nod_inf("Nod_Inf.out", &blocks, &res);
    assert!(e.is_empty() && n > 0, "{e:?}");
    // with three groups of columns (dual non-equilibrium) the third one is Sorb2
    let text = "\n Time:         1.0000\n\n Node      Depth      Head Moisture       K          C         Flux        Sink         Kappa   v/KsTop   Temp\n\n   1     0.0000      -1.000 0.2000  1.0E-01  1.0E-02  1.0E-01  0.0E+00      -1  1.0E-01   20.00  1.0E+00  2.0E+00  3.0E+00\n";
    let p = dir.join("Nod_Inf_neq.out");
    std::fs::write(&p, text).unwrap();
    let blocks = read_nod_inf_full(&p, 1);
    assert_eq!(&blocks[0].names[11..], &["Conc(1)", "Sorb(1)", "Sorb2(1)"]);
}

#[test]
fn single_precision_noise_is_not_checked_and_temperature_needs_heat_transport() {
    // differences below the 1e-4 floor pass even for columns with a tighter tolerance
    let reference = RefTable { times: vec![1.0], names: vec!["Kappa".into()], cols: vec![vec![1.00005]], ..Default::default() };
    let sim: SimSeries = vec![("Kappa".into(), vec![1.0])];
    let (e, _, _) = compare_series_ex("x", &|_| 1e-4, &reference, &[1.0], &sim, &[]);
    assert!(e.is_empty(), "{e:?}");
    // the Temp column of Nod_Inf.out is skipped when heat transport is off
    let (_prj, res) = small_solute_run();
    let dir = std::env::temp_dir().join(format!("rusteau_cmp_{}_temp", std::process::id()));
    write_reference_files(&dir, &res);
    let text = std::fs::read_to_string(dir.join("Nod_Inf.out")).unwrap();
    // set the printed temperature of every node to 0 (as older HYDRUS versions do without heat transport)
    let spoiled: String = text
        .lines()
        .map(|l| match parse_numbers(l) {
            Some(v) if v.len() > 10 && l.trim_start().chars().next().map_or(false, |c| c.is_ascii_digit()) => {
                let mut tok: Vec<String> = l.split_whitespace().map(|s| s.to_string()).collect();
                tok[10] = "0.00".into();
                tok.join(" ")
            }
            _ => l.to_string(),
        })
        .collect::<Vec<_>>()
        .join("\n");
    std::fs::write(dir.join("Nod_Inf.out"), spoiled).unwrap();
    let blocks = read_nod_inf_full(&dir.join("Nod_Inf.out"), res.n_solutes);
    let (e, _) = compare_nod_inf_ex("Nod_Inf.out", &blocks, &res, true);
    assert!(e.iter().any(|m| m.contains("'Temp'")), "with heat transport the temperature is compared");
    let (e, n) = compare_nod_inf_ex("Nod_Inf.out", &blocks, &res, false);
    assert!(e.is_empty() && n > 0, "{e:?}");
}

#[test]
fn initial_balance_and_observation_records_carry_the_initial_time() {
    let mut p = examples::solute_pulse();
    p.time.t_init = 5.0;
    p.time.t_max = 8.0;
    p.time.print_times = vec![6.0, 8.0];
    p.time.dt = 0.5;
    p.time.dt_max = 1.0;
    p.profile.observation_nodes = vec![1, 21];
    let sim = run(p);
    assert_eq!(sim.res.balance[0].t, 5.0, "balance record of the initial state");
    assert_eq!(sim.res.obs[0].t, 5.0, "observation record of the initial state");
    assert!(sim.res.obs[1].t > 5.0);
}

#[test]
fn heat_volume_of_the_reference_build_is_not_compared() {
    let (_prj, res) = small_solute_run();
    let rb = RefBalance { t: res.balance[0].t, rows: vec![("HeatVol".into(), vec![1.0e30, 1.0e30]), ("Area".into(), vec![res.balance[0].total.area])], ..Default::default() };
    let (e, n) = compare_balance("Balance.out", &[rb], &res);
    assert!(e.is_empty() && n >= 1, "{e:?}");
}

#[test]
fn a_value_that_rounds_to_the_printed_reference_is_not_a_difference() {
    // the print quantum follows from the text of the reference file
    assert!((half_quantum(".171E+02") - 0.05).abs() < 1e-12, "3 digits behind the point, exponent 2");
    assert!((half_quantum("-3.60") - 0.005).abs() < 1e-12);
    assert!((half_quantum("0.1910") - 0.00005).abs() < 1e-12);
    let (v, q) = parse_numbers_q(" 1 -.239E+00 .171E+02").map(|x| (x[1].0, x[2].1)).unwrap();
    assert!((v + 0.239).abs() < 1e-12 && (q - 0.05).abs() < 1e-12);

    // reference column `K` printed with 3 significant digits (TEST5: Ks = 17.12 is printed as .171E+02)
    let reference = |val: f64, text: &str| RefTable { times: vec![1.0], names: vec!["K".into()], cols: vec![vec![val]], hq: vec![vec![half_quantum(text)]] };
    let check = |sim: f64, r: &RefTable| compare_series_ex("x", &|_| 1e-4, r, &[1.0], &vec![("K".into(), vec![sim])], &[]).0;
    // sim 17.12 rounds to the printed 17.1 -> no difference although it is 0.12 % away (tolerance 1e-3)
    let e = check(17.12, &reference(17.1, ".171E+02"));
    assert!(e.is_empty(), "{e:?}");
    // 2.9947 prints as 2.99, not as the reference 3.00 -> a real (small) difference
    let e = check(2.9947, &reference(3.0, ".300E+01"));
    assert_eq!(e.len(), 1, "{e:?}");
    // a value two print units away is a difference
    let e = check(17.2, &reference(17.1, ".171E+02"));
    assert_eq!(e.len(), 1, "{e:?}");
    // without a print quantum (exact reference values) the old tolerance decides
    let exact = RefTable { times: vec![1.0], names: vec!["K".into()], cols: vec![vec![17.1]], hq: vec![] };
    assert_eq!(check(17.12, &exact).len(), 1);
}

#[test]
fn t_level_of_the_old_builds_is_read_with_its_own_layout() {
    // 15-column T_Level.out of the old HYDRUS builds (header without year, `t-level` counter; e.g. the 1SCALING sample output).
    // It used to fail the 22-column read and was then skipped without a message.
    let text = " ******* Program HYDRUS\n Date:   1.10.    Time:  14: 1:28\n\n       Time        rTop      rRoot      vTop       vRoot      vBot     sum(rTop) sum(rRoot)  sum(vTop) sum(vRoot)  sum(vBot)    hTop       hRoot      hBot      t-level\n        [T]       [L/T]      [L/T]\n\n        .01000  -.167E-01   .000E+00  -.167E-01   .000E+00  -.150E-05  -.167E-03   .000E+00  -.167E-03   .000E+00  -.150E-07  -.498E+03   .000E+00  -.500E+03       1\n        .02300  -.167E-01   .000E+00  -.167E-01   .000E+00  -.150E-05  -.383E-03   .000E+00  -.383E-03   .000E+00  -.345E-07  -.495E+03   .000E+00  -.500E+03       2\nend\n";
    let dir = write_tmp("old_t_level", "");
    let path = dir.join("T_LEVEL.OUT");
    std::fs::write(&path, text).unwrap();
    let t = read_t_level(&path).expect("old layout is read");
    assert_eq!(t.times.len(), 2);
    assert_eq!(t.names.len(), 14);
    assert_eq!(t.names.last().map(String::as_str), Some("t-level"));
    assert!((t.cols[10][1] + 495.0).abs() < 1e-9, "hTop of the second row");
    // the counter is not compared; every other column has a simulated counterpart
    let res = sample_results("");
    let sim_t = vec![0.01, 0.023];
    let (_, ss) = sim_t_level(&res);
    let sim: SimSeries = ss.into_iter().map(|(n, _)| (n, vec![0.0, 0.0])).collect();
    let (errors, compared, _) = compare_series_ex("T_Level.out", &|_| 1e-4, &t, &sim_t, &sim, &[]);
    assert!(errors.iter().all(|e| !e.contains("no simulated counterpart") && !e.contains("t-level")), "{errors:?}");
    assert_eq!(compared, 13 * 2);
    // the current 22-column layout is still read with the full column set
    let generated = dir.join("T_Level.out.new");
    let mut s = String::from("\n       Time  rTop ... Volume ... TLevel\n\n");
    s.push_str(&format!("{:13.4}", 1.0));
    for _ in 0..18 {
        s.push_str(" 0.10000E+01");
    }
    s.push_str("      1 0.00000E+00      0.000\n");
    std::fs::write(&generated, s).unwrap();
    assert_eq!(read_t_level(&generated).expect("current layout").names.len(), 21);
}
