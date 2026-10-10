//! rusteau: runs a HYDRUS-1D project from the command line and writes the results.
//!
//! Exit codes: 0 ok, 1 read/initialisation/write error, 2 usage error, 3 simulation failed (no convergence),
//! 4 `--compare` found differences to the reference output files (inverse problems: a fit worse than the reference
//! Fit.out by more than 5 %).

use rusteau_core::{Simulation, StepStatus};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Instant;

const USAGE: &str = "\
rusteau <project-dir | name.h1d | project.h1dr> [options]

Runs a HYDRUS-1D project (legacy directory with SELECTOR.IN, or the HYDRUS-1D 4.x project file name.h1d next to
that directory, or native .h1dr JSON) and writes the results
(T_LEVEL.csv, A_LEVEL.csv, NOD_INF.csv, OBS_NODE.csv, BALANCE.csv and, when produced, T_LEVEL1.csv, METEO.csv, Meteo.out).
An inverse project (lInverse with Fit.in) is optimized first; Fit.out and the results of the final run are written.

Options:
  -o, --out DIR        output directory (default: <project-dir>/rust_out, or <file>.results)
  -c, --compare [DIR]  compare ALL columns of the reference output files (T_Level.out, A_Level.out, Nod_Inf.out, Obs_Node.out,
                       Balance.out, Solute<i>.out, Meteo.out) found in DIR (default: the project directory) with the results;
                       exit code 4 when values differ. Inverse problems: the fit is compared with the reference Fit.out
      --direct         run an inverse project as a direct problem with the initial estimates
      --hydrus-stop    inverse problems: iterate exactly as HYDRUS 4.17 does (one starting point, adaptive time steps in
                       every run)
      --save-h1d FILE  write the project as a HYDRUS-1D 4.x project (FILE.h1d and the folder FILE with the input files)
                       and exit without running it
      --starts N       inverse problems: number of additional starting points (default: the project setting, 8 when
                       not set; 0 = only the initial estimates)
  -q, --quiet          no progress display
  -s, --summary        print a summary of the run (steps, balances, written files)
      --json FILE      additionally save the complete results as JSON (readable by the GUI)
  -V, --version        print the version
  -h, --help           this text";

fn usage(code: i32) -> ! {
    if code == 0 {
        println!("{USAGE}");
    } else {
        eprintln!("{USAGE}");
    }
    std::process::exit(code)
}

struct Options {
    input: PathBuf,
    out: Option<PathBuf>,
    quiet: bool,
    summary: bool,
    compare: Option<Option<PathBuf>>,
    json: Option<PathBuf>,
    direct: bool,
    hydrus_stop: bool,
    starts: Option<usize>,
    save_h1d: Option<PathBuf>,
}

fn parse_args() -> Options {
    let mut args = std::env::args().skip(1).peekable();
    let (mut input, mut out, mut json) = (None::<PathBuf>, None::<PathBuf>, None::<PathBuf>);
    let (mut quiet, mut summary, mut compare, mut direct) = (false, false, None::<Option<PathBuf>>, false);
    let mut hydrus_stop = false;
    let mut starts = None;
    let mut save_h1d = None;
    while let Some(a) = args.next() {
        match a.as_str() {
            "-o" | "--out" => out = Some(args.next().map(PathBuf::from).unwrap_or_else(|| usage(2))),
            "--json" => json = Some(args.next().map(PathBuf::from).unwrap_or_else(|| usage(2))),
            "-c" | "--compare" => {
                // optional directory argument: the next token unless it is an option or the project itself
                let dir = match args.peek() {
                    Some(n) if !n.starts_with('-') && input.is_some() => args.next().map(PathBuf::from),
                    _ => None,
                };
                compare = Some(dir);
            }
            "-q" | "--quiet" => quiet = true,
            "--direct" => direct = true,
            "--hydrus-stop" => hydrus_stop = true,
            "--save-h1d" => save_h1d = Some(args.next().map(PathBuf::from).unwrap_or_else(|| usage(2))),
            "--starts" => starts = Some(args.next().and_then(|x| x.parse().ok()).unwrap_or_else(|| usage(2))),
            "-s" | "--summary" => summary = true,
            "-V" | "--version" => {
                println!("rusteau {}", env!("CARGO_PKG_VERSION"));
                std::process::exit(0)
            }
            "-h" | "--help" => usage(0),
            s if s.starts_with('-') => {
                eprintln!("unknown option '{s}'");
                usage(2)
            }
            _ => {
                if input.is_some() {
                    eprintln!("only one project may be given");
                    usage(2)
                }
                input = Some(PathBuf::from(a))
            }
        }
    }
    Options { input: input.unwrap_or_else(|| usage(2)), out, quiet, summary, compare, json, direct, hydrus_stop, starts, save_h1d }
}

fn fail(msg: impl std::fmt::Display, code: i32) -> ! {
    eprintln!("{msg}");
    std::process::exit(code)
}

fn main() {
    let opt = parse_args();
    if !opt.input.exists() {
        fail(format!("'{}' does not exist", opt.input.display()), 1);
    }
    // A HYDRUS-1D 4.x project file `<name>.h1d` (or a file inside a project folder) stands for the project folder.
    let resolved;
    let input = if !opt.input.is_dir() && rusteau_io::is_legacy_project_path(&opt.input) {
        resolved = rusteau_io::legacy_project_dir(&opt.input).unwrap_or_else(|e| fail(format!("{}: {}", opt.input.display(), e), 1));
        &resolved
    } else {
        &opt.input
    };
    let prj = if input.is_dir() {
        rusteau_io::read_legacy_project(input)
    } else {
        rusteau_io::load_project(input)
    };
    let mut prj = prj.unwrap_or_else(|e| fail(format!("{}: {}", input.display(), e), 1));
    if let Some(h1d) = &opt.save_h1d {
        match rusteau_io::save_h1d_project(&prj, h1d) {
            Ok((files, warnings)) => {
                for f in &files {
                    eprintln!("written {}", f.display());
                }
                for w in &warnings {
                    eprintln!("note: {w}");
                }
                return;
            }
            Err(e) => fail(e, 1),
        }
    }
    let outdir = opt.out.clone().unwrap_or_else(|| if input.is_dir() { input.join("rust_out") } else { input.with_extension("results") });
    if prj.processes.inverse {
        if opt.direct {
            // direct problem with the initial estimates of Fit.in
            if let Some(s) = prj.inverse.clone() {
                s.apply(&mut prj, &s.initial());
            }
            prj.processes.inverse = false;
        } else {
            run_inverse(&opt, input, &prj, &outdir);
            return;
        }
    }
    let mut sim = Simulation::new(prj.clone()).unwrap_or_else(|e| fail(e, 1));

    let started = Instant::now();
    let (mut last_pct, mut steps) = (-100.0f64, 0usize);
    let quiet = opt.quiet;
    let status = sim.run(|s| {
        steps += 1;
        if !quiet {
            let span = (s.t_max - s.t_init).max(1e-12);
            let pct = 100.0 * (s.t - s.t_init) / span;
            if pct - last_pct >= 1.0 || s.t >= s.t_max {
                eprint!("\r{:5.1}%  t = {:<14.6}  dt = {:.3e}   ", pct.clamp(0.0, 100.0), s.t, s.dt);
                let _ = std::io::stderr().flush();
                last_pct = pct;
            }
        }
        true
    });
    if !quiet {
        eprintln!();
    }
    let elapsed = started.elapsed().as_secs_f64();
    for m in &sim.res.messages {
        eprintln!("{m}");
    }

    // ---- write results
    if let Err(e) = rusteau_io::writer::write_all(&sim.res, &outdir) {
        fail(e, 1);
    }
    if let Some(j) = &opt.json {
        if let Err(e) = rusteau_io::save_results(&sim.res, j) {
            fail(e, 1);
        }
    }
    eprintln!("Results written to {}", outdir.display());

    if opt.summary {
        print_summary(&sim, steps, elapsed, &outdir, status);
    }

    // ---- comparison with the reference output files
    let mut exit = if status == StepStatus::Failed { 3 } else { 0 };
    if let Some(dir) = &opt.compare {
        let folder = dir.clone().unwrap_or_else(|| if input.is_dir() { input.clone() } else { input.parent().map(Path::to_path_buf).unwrap_or_else(|| PathBuf::from(".")) });
        if !folder.is_dir() {
            fail(format!("--compare: '{}' is not a directory", folder.display()), 1);
        }
        let rep = rusteau_io::compare_folder(&folder, &prj, &sim.res);
        if rep.files.is_empty() && rep.errors.is_empty() {
            eprintln!("--compare: no reference output files found in {}", folder.display());
            exit = exit.max(1);
        } else {
            let values: usize = rep.files.iter().map(|f| f.1).sum();
            for (f, n) in &rep.files {
                eprintln!("  compared {f:<16} {n:>9} values");
            }
            if rep.errors.is_empty() {
                eprintln!("Comparison with {}: all {values} reference values agree ({} of the series values at times without a simulated step, checked against the neighbouring steps).", folder.display(), rep.bracketed);
            } else {
                for e in &rep.errors {
                    eprintln!("MISMATCH {e}");
                }
                eprintln!("Comparison with {}: {} mismatching columns/rows (of {values} compared values).", folder.display(), rep.errors.len());
                if exit == 0 {
                    exit = 4;
                }
            }
        }
    }
    if exit != 0 {
        std::process::exit(exit)
    }
}

fn print_summary(sim: &Simulation, steps: usize, elapsed: f64, outdir: &Path, status: StepStatus) {
    let r = &sim.res;
    println!("---------------------------------------------------------------");
    println!(" status            : {}", if status == StepStatus::Failed { "FAILED" } else { "finished" });
    println!(" time              : {:.6} (of {:.6})", sim.t, sim.t_max);
    println!(" time steps        : {} ({} stored levels), {:.2} s", steps, r.tlevel.len(), elapsed);
    if let Some(l) = r.tlevel.last() {
        println!(" cumulative fluxes : sum(vTop) {:.5e}, sum(vBot) {:.5e}, sum(rRoot) {:.5e}, volume {:.5e}", l.cum_v_top, l.cum_v_bot, l.cum_r_root, l.volume);
    }
    let wat = r.balance.iter().skip(1).map(|b| b.wat_bal_r).fold(0.0f64, f64::max);
    println!(" water balance     : max error {wat:.4} %");
    for j in 0..r.n_solutes {
        let e = r.balance.iter().skip(1).map(|b| b.sol_bal_r.get(j).copied().unwrap_or(0.0)).fold(0.0f64, f64::max);
        println!(" solute {} balance  : max error {e:.4} %", j + 1);
    }
    if !r.meteo.is_empty() {
        println!(" Meteo.out         : {} rows (mode {})", r.meteo.len(), r.meteo_mode);
    }
    if !r.tlevel1.is_empty() {
        println!(" T_Level1          : {} rows", r.tlevel1.len());
    }
    if let Ok(rd) = std::fs::read_dir(outdir) {
        let mut names: Vec<String> = rd.flatten().map(|e| e.file_name().to_string_lossy().to_string()).collect();
        names.sort();
        println!(" files             : {}", names.join(", "));
    }
    println!("---------------------------------------------------------------");
}

/// Inverse problem: Marquardt-Levenberg optimization, Fit.out, results of the final run.
fn run_inverse(opt: &Options, input: &Path, prj: &rusteau_core::Project, outdir: &Path) {
    use rusteau_core::inverse::*;
    let mut s = prj.inverse.clone().unwrap_or_else(|| fail("the project has no inverse settings (Fit.in)", 1));
    s.hydrus_stop |= opt.hydrus_stop;
    if let Some(n) = opt.starts {
        s.extra_starts = n;
    }
    let quiet = opt.quiet;
    let names: Vec<String> = s.fitted().iter().map(|&i| s.param_name(&s.params[i])).collect();
    if !quiet {
        eprintln!("Parameter estimation: {} parameters ({}), {} observations, MIT {}", names.len(), names.join(", "), s.observations.len(), s.max_iter);
        if !s.hydrus_stop && s.extra_starts > 0 {
            eprintln!("Starting points: the initial estimates and {} more (in parallel; iterations shown for the first)", s.extra_starts);
        }
        eprintln!(" Iteration     SSQ        {}", names.iter().map(|n| format!("{n:<12}")).collect::<String>());
    }
    let res = optimize(prj, &s, &MarqOptions::default(), None, |p| {
        if quiet {
            return;
        }
        if p.finished {
            if p.ssq.is_finite() {
                eprintln!("   start {:>2} finished: SSQ {:.4e}  {}", p.start, p.ssq, p.params.iter().map(|v| format!("{:<12.4e}", v)).collect::<String>());
            } else {
                eprintln!("   start {:>2} failed", p.start);
            }
        } else {
            eprintln!("{:>7}   {:.4e}  {}", p.iteration, p.ssq, p.params.iter().map(|v| format!("{:<12.4e}", v)).collect::<String>());
        }
    })
    .unwrap_or_else(|e| fail(e, 3));
    if !quiet && res.starts.len() > 1 {
        eprintln!("Best result from start {} (SSQ {:.4e})", res.best_start, res.ssq);
    }
    if !quiet {
        match res.stop {
            StopReason::NoReduction => eprintln!("No further reduction in SSQ obtained, optimization stopped"),
            StopReason::MaxIterations => eprintln!("Maximum number of iterations reached"),
            StopReason::Converged => eprintln!("Converged (no parameter changed by more than 1 %)"),
            StopReason::Cancelled => eprintln!("Cancelled"),
        }
    }
    if let Some(r) = &res.final_results {
        if let Err(e) = rusteau_io::writer::write_all(r, outdir) {
            fail(e, 1);
        }
        if let Some(j) = &opt.json {
            if let Err(e) = rusteau_io::save_results(r, j) {
                fail(e, 1);
            }
        }
    }
    if let Err(e) = rusteau_io::write_fit_out(outdir, prj, &s, &res) {
        fail(e, 1);
    }
    eprintln!("Fit.out and the results of the final run written to {}", outdir.display());
    if opt.summary || !quiet {
        println!("---------------------------------------------------------------");
        println!(" Variable        Value        S.E.Coeff.     Lower 95%     Upper 95%");
        for p in &res.params {
            println!("   {:<8} {:>13.5e} {:>13.5e} {:>13.5e} {:>13.5e}", p.name, p.value, p.se, p.lower_95, p.upper_95);
        }
        let m = &res.measures;
        println!(" SSQ {:.5e}, R² {:.5}, RMSE {:.4e}, {} iterations, {} runs, {:.1} s", res.ssq, m.r_squared, m.rmse, res.iterations.len() - 1, res.runs, res.elapsed_secs);
        println!("---------------------------------------------------------------");
    }
    if let Some(dir) = &opt.compare {
        let folder = dir.clone().unwrap_or_else(|| if input.is_dir() { input.to_path_buf() } else { input.parent().map(Path::to_path_buf).unwrap_or_else(|| PathBuf::from(".")) });
        let fit_out = std::fs::read_dir(&folder).ok().and_then(|rd| rd.flatten().map(|e| e.path()).find(|p| p.file_name().is_some_and(|f| f.to_string_lossy().eq_ignore_ascii_case("fit.out"))));
        let Some(fit_out) = fit_out else { fail(format!("--compare: no Fit.out in {}", folder.display()), 1) };
        let r = rusteau_io::parse_fit_out(&std::fs::read_to_string(&fit_out).unwrap_or_default());
        let ssq_ref = r.iterations.last().map(|x| x.0).unwrap_or(f64::NAN);
        eprintln!("Comparison with {}:", fit_out.display());
        eprintln!("  SSQ {:.4e} (reference {:.4e}), R² {:.5} (reference {})", res.ssq, ssq_ref, res.measures.r_squared, r.r_squared.map(|x| format!("{x:.5}")).unwrap_or_default());
        for (j, p) in res.params.iter().enumerate() {
            if let Some(q) = r.params.get(j) {
                eprintln!("  {:<8} {:>13.5e}  reference {:>13.5e} ± {:.3e}  ({:+.2} S.E.)", p.name, p.value, q.1, q.2, (p.value - q.1) / q.2.abs().max(1e-300));
            }
        }
        if res.ssq > 1.05 * ssq_ref {
            eprintln!("The fit is worse than the reference by more than 5 %.");
            std::process::exit(4);
        }
    }
}
