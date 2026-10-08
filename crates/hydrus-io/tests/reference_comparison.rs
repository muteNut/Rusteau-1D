//! Verification of the engine against the HYDRUS-1D reference outputs.
//!
//! * `test_all_direct_examples_comparison` runs every example of the HYDRUS-1D `Examples/Direct` directory (override the
//!   location with the environment variable `HYDRUS_EXAMPLES_DIR`) and compares ALL columns of ALL reference files found
//!   there (`T_Level.out`, `A_Level.out`, `Nod_Inf.out`, `Obs_Node.out`, `Balance.out`, `Solute<i>.out`, `Meteo.out`).
//! * The remaining tests need no external data: they check the readers, the column mapping and the tolerance logic with
//!   the real Fortran `Meteo.out` sample (`tests/data/Meteo_sample.out`) and with reference files generated from a
//!   simulation.

use hydrus_core::output::MeteoRow;
use hydrus_core::{examples, Project, Results, Simulation, StepStatus};
use hydrus_io::*;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

fn run(prj: Project) -> Simulation {
    let mut sim = Simulation::new(prj).expect("project");
    while sim.step() == StepStatus::Running {}
    sim
}

// ---------------------------------------------------------------------------------------------------------------------
// benchmark against the Fortran examples
// ---------------------------------------------------------------------------------------------------------------------

fn examples_dir() -> PathBuf {
    std::env::var("HYDRUS_EXAMPLES_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(r"C:\Users\Public\Documents\PC-Progress\Hydrus-1D 4.xx\Examples\Direct"))
}

/// Key of a problem line for the statistics over all examples: "<file> column '<name>'".
fn problem_key(line: &str) -> String {
    let file = line.split(']').filter(|p| p.contains(".out") || p.contains(".OUT")).next().and_then(|p| p.rsplit('[').next()).unwrap_or("?").to_string();
    let col = line.split("column '").nth(1).or_else(|| line.split("row '").nth(1)).and_then(|r| r.split('\'').next()).unwrap_or("(other)");
    format!("{file} '{col}'")
}

/// Environment variables: `HYDRUS_EXAMPLES_DIR` (examples directory), `HYDRUS_COMPARE_ONLY` (comma separated example names),
/// `HYDRUS_COMPARE_VERBOSE=1` (print every problem), `HYDRUS_COMPARE_REPORT` (path of the full report, default
/// `reference_comparison_report.txt` in the working directory).
#[test]
fn test_all_direct_examples_comparison() {
    let base = examples_dir();
    assert!(base.exists(), "FATAL: benchmark directory not found at {:?} (set HYDRUS_EXAMPLES_DIR). Verification cannot proceed.", base);
    let only: Option<Vec<String>> = std::env::var("HYDRUS_COMPARE_ONLY").ok().map(|v| v.split(',').map(|s| s.trim().to_ascii_lowercase()).collect());
    let verbose = std::env::var("HYDRUS_COMPARE_VERBOSE").map_or(false, |v| v == "1");

    struct Outcome {
        name: String,
        errors: Vec<String>,
        files: Vec<(String, usize)>,
        bracketed: usize,
        en_bal: bool,
        seconds: f64,
        steps: usize,
        messages: Vec<String>,
    }
    let mut outcomes: Vec<Outcome> = Vec::new();
    let mut entries: Vec<PathBuf> = std::fs::read_dir(&base).expect("examples directory").flatten().map(|e| e.path()).filter(|p| p.is_dir() && p.join("SELECTOR.IN").exists()).collect();
    entries.sort();
    for path in entries {
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        if let Some(o) = &only {
            if !o.contains(&name.to_ascii_lowercase()) {
                continue;
            }
        }
        let t0 = std::time::Instant::now();
        let mut out = Outcome { name: name.clone(), errors: vec![], files: vec![], bracketed: 0, en_bal: false, seconds: 0.0, steps: 0, messages: vec![] };
        match read_legacy_project(&path) {
            Err(e) => out.errors.push(format!("[{name}] project parse error: {e}")),
            Ok(prj) => {
                out.en_bal = prj.atmosphere.meteo.as_ref().map_or(false, |m| m.l_en_bal);
                match Simulation::new(prj.clone()) {
                    Err(e) => out.errors.push(format!("[{name}] init error: {e}")),
                    Ok(mut sim) => {
                        while sim.step() == StepStatus::Running {}
                        out.messages = sim.res.messages.clone();
                        out.steps = sim.res.tlevel.len();
                        if sim.res.failed {
                            out.errors.push(format!("[{name}] the simulation did not finish: {:?}", sim.res.messages.last()));
                        }
                        let rep = compare_folder(&path, &prj, &sim.res);
                        out.files = rep.files;
                        out.bracketed = rep.bracketed;
                        out.errors.extend(rep.errors);
                        if out.files.is_empty() && out.errors.is_empty() {
                            out.errors.push(format!("[{name}] no reference output files found"));
                        }
                    }
                }
            }
        }
        out.seconds = t0.elapsed().as_secs_f64();
        let n: usize = out.files.iter().map(|f| f.1).sum();
        println!(
            "Testing {:<12} ... {} ({} files, {} values, {} at non-coinciding times, {} stored levels, {:.1} s{})",
            out.name,
            if out.errors.is_empty() { "PASSED" } else { "FAILED" },
            out.files.len(),
            n,
            out.bracketed,
            out.steps,
            out.seconds,
            if out.errors.is_empty() { String::new() } else { format!(", {} problems", out.errors.len()) }
        );
        outcomes.push(out);
    }
    assert!(!outcomes.is_empty(), "no example selected / found in {:?}", base);

    // ---- details first (the assertions below stop the test)
    let report_path = std::env::var("HYDRUS_COMPARE_REPORT").unwrap_or_else(|_| "reference_comparison_report.txt".into());
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
            println!("   ... {} more (HYDRUS_COMPARE_VERBOSE=1 or the report file)", o.errors.len() - 12);
        }
        let _ = writeln!(report, "--- {} ---", o.name);
        for e in &o.errors {
            let _ = writeln!(report, "{e}");
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
    println!("\n================ FAILING COLUMNS (examples affected / problems) ================");
    for (k, (ex, n)) in ranked.iter().take(40) {
        println!("  {ex:>3} / {n:>4}   {k}");
    }

    let total = outcomes.len();
    let passed = outcomes.iter().filter(|o| o.errors.is_empty()).count();
    let values: usize = outcomes.iter().map(|o| o.files.iter().map(|f| f.1).sum::<usize>()).sum();
    let en_run = outcomes.iter().filter(|o| o.en_bal).count();
    let en_ok = outcomes.iter().filter(|o| o.en_bal && o.errors.is_empty()).count();
    println!("\n==========================================");
    println!("Benchmark Results: {passed}/{total} passed, {values} reference values compared");
    println!("Energy balance projects executed: {en_run}, verified: {en_ok}");
    println!("Full report: {report_path}");
    println!("==========================================");

    let problems: usize = outcomes.iter().map(|o| o.errors.len()).sum();
    assert!(problems == 0, "{problems} problems in {} of {total} examples (details above and in {report_path})", total - passed);
    assert!(en_ok > 0, "STRICTNESS FAILURE: no project with lEnBal = true was executed and verified.");
}

// ---------------------------------------------------------------------------------------------------------------------
// Meteo.out sample (real Fortran output)
// ---------------------------------------------------------------------------------------------------------------------

const METEO_SAMPLE: &str = include_str!("data/Meteo_sample.out");

fn sample_results() -> Results {
    let mut r = Results::default();
    r.meteo_mode = 2;
    for v in METEO_SAMPLE.lines().filter_map(parse_numbers).filter(|v| v.len() == 8) {
        r.meteo.push(MeteoRow { t: v[0], rns: v[1], rnl: v[2], rn: v[3], sensible: v[4], latent: v[5], heat_flux: v[6], balance: v[7], ..Default::default() });
    }
    r
}

fn write_tmp(name: &str, text: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("hydrus_cmp_{}_{}", std::process::id(), name));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("Meteo.out"), text).unwrap();
    dir
}

fn meteo_tr(t: f64) -> f64 {
    10f64.powi(t.abs().max(1e-30).log10().floor() as i32 - 4)
}

#[test]
fn meteo_sample_is_read_with_all_columns() {
    let dir = write_tmp("read", METEO_SAMPLE);
    let (mode, table) = read_meteo_out(&dir.join("Meteo.out")).expect("Meteo.out");
    assert_eq!(mode, 2, "energy-balance layout without daily data");
    assert_eq!(table.names, vec!["Short.Rad.", "Long.Rad.", "Radiation", "Sensible", "Latent", "Heat Flux", "Balance"]);
    assert_eq!(table.times.len(), 1217);
    assert!((table.times[0] - 328.00).abs() < 1e-9 && (table.cols[1][0] + 7.2580).abs() < 1e-9);
    assert!((table.cols[5][0] - 18.064).abs() < 1e-9, "Heat Flux of the first row");
}

#[test]
fn meteo_sample_compares_clean_against_itself_and_detects_changes() {
    let dir = write_tmp("self", METEO_SAMPLE);
    let (_, table) = read_meteo_out(&dir.join("Meteo.out")).unwrap();
    let res = sample_results();
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
    let dir = write_tmp("missing", METEO_SAMPLE);
    let (_, table) = read_meteo_out(&dir.join("Meteo.out")).unwrap();
    let res = sample_results();
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
        let row = |lab: &str, f: &dyn Fn(&hydrus_core::output::SubRegion) -> f64| -> String {
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
    use hydrus_core::{AtmRecord, SoluteMaterial, SpeciesMaterial};
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
    let dir = std::env::temp_dir().join(format!("hydrus_cmp_{}_pipeline", std::process::id()));
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
    let dir = std::env::temp_dir().join(format!("hydrus_cmp_{}_perturb", std::process::id()));
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
    let dir = std::env::temp_dir().join(format!("hydrus_cmp_{}_depth", std::process::id()));
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
    let dir = std::env::temp_dir().join(format!("hydrus_cmp_{}_temp", std::process::id()));
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
    let res = sample_results();
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
