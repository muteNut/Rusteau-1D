//! rusteau: runs a HYDRUS-1D project from the command line and writes the results.
//!
//! Exit codes: 0 ok, 1 read/initialisation/write error, 2 usage error, 3 simulation failed (no convergence),
//! 4 `--compare` found differences to the reference output files.

use rusteau_core::{Simulation, StepStatus};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Instant;

const USAGE: &str = "\
rusteau <project-dir | name.h1d | project.h1dr> [options]

Runs a HYDRUS-1D project (legacy directory with SELECTOR.IN, or the HYDRUS-1D 4.x project file name.h1d next to
that directory, or native .h1dr JSON) and writes the results
(T_LEVEL.csv, A_LEVEL.csv, NOD_INF.csv, OBS_NODE.csv, BALANCE.csv and, when produced, T_LEVEL1.csv, METEO.csv, Meteo.out).

Options:
  -o, --out DIR        output directory (default: <project-dir>/rust_out, or <file>.results)
  -c, --compare [DIR]  compare ALL columns of the reference output files (T_Level.out, A_Level.out, Nod_Inf.out, Obs_Node.out,
                       Balance.out, Solute<i>.out, Meteo.out) found in DIR (default: the project directory) with the results;
                       exit code 4 when values differ
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
}

fn parse_args() -> Options {
    let mut args = std::env::args().skip(1).peekable();
    let (mut input, mut out, mut json) = (None::<PathBuf>, None::<PathBuf>, None::<PathBuf>);
    let (mut quiet, mut summary, mut compare) = (false, false, None::<Option<PathBuf>>);
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
    Options { input: input.unwrap_or_else(|| usage(2)), out, quiet, summary, compare, json }
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
    let prj = prj.unwrap_or_else(|e| fail(format!("{}: {}", input.display(), e), 1));
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
    let outdir = opt.out.clone().unwrap_or_else(|| if input.is_dir() { input.join("rust_out") } else { input.with_extension("results") });
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
