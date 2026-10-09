//! End-to-end tests of `rusteau` (native project -> run -> results -> comparison with reference files).

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_rusteau"))
}

fn workdir(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("rusteau_cli_{}_{}", std::process::id(), name));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn make_project(dir: &Path) -> PathBuf {
    let mut p = rusteau_core::examples::solute_pulse();
    p.time.t_max = 6.0;
    p.time.print_times = vec![2.0, 4.0, 6.0];
    p.solute.t_pulse = 3.0;
    let f = dir.join("pulse.h1dr");
    rusteau_io::save_project(&p, &f).unwrap();
    f
}

fn text(o: &Output) -> String {
    format!("{}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr))
}

/// Fortran-style T_Level.out written from the T_LEVEL.csv of a first run (all 22 columns).
fn reference_from_csv(csv: &Path, out: &Path, spoil: bool) {
    let text = std::fs::read_to_string(csv).unwrap();
    let mut lines = text.lines();
    let head: Vec<&str> = lines.next().unwrap().split(',').collect();
    let col = |n: &str| head.iter().position(|h| *h == n).unwrap_or_else(|| panic!("column {n}"));
    let names = [
        "time", "rTop", "rRoot", "vTop", "vRoot", "vBot", "sum_rTop", "sum_rRoot", "sum_vTop", "sum_vRoot", "sum_vBot", "hTop", "hRoot", "hBot", "runoff", "sum_runoff", "volume",
        "sum_infil", "sum_evap", "tlevel", "cum_w_transf", "snow_layer",
    ];
    let idx: Vec<usize> = names.iter().map(|n| col(n)).collect();
    let mut s = String::from("\n   Time   rTop ...\n   [T]\n\n");
    for (k, l) in lines.enumerate() {
        let f: Vec<f64> = l.split(',').map(|x| x.parse().unwrap_or(0.0)).collect();
        let mut v: Vec<f64> = idx.iter().map(|&i| f[i]).collect();
        if spoil && k % 3 == 0 {
            v[3] = v[3] * 1.2 - 0.05; // vTop
        }
        s.push_str(&format!("{:13.4}", v[0]));
        for x in &v[1..19] {
            s.push_str(&format!("{:13.5E}", x));
        }
        s.push_str(&format!("{:7}{:13.5E}{:11.3}\n", v[19] as i64, v[20], v[21]));
    }
    std::fs::write(out.join("T_Level.out"), s).unwrap();
}

#[test]
fn version_help_and_usage_errors() {
    let o = bin().arg("--version").output().unwrap();
    assert!(o.status.success() && text(&o).starts_with("rusteau "));
    let o = bin().arg("--help").output().unwrap();
    assert!(o.status.success() && text(&o).contains("--compare"));
    assert_eq!(bin().output().unwrap().status.code(), Some(2), "no project given");
    assert_eq!(bin().args(["x.h1dr", "--bogus"]).output().unwrap().status.code(), Some(2));
    assert_eq!(bin().args(["x.h1dr", "y.h1dr"]).output().unwrap().status.code(), Some(2));
    assert_eq!(bin().arg("/does/not/exist.h1dr").output().unwrap().status.code(), Some(1));
}

#[test]
fn run_writes_results_summary_and_json() {
    let d = workdir("run");
    let prj = make_project(&d);
    let out = d.join("out");
    let json = d.join("res.json");
    let o = bin().arg(&prj).args(["-q", "-s", "-o"]).arg(&out).arg("--json").arg(&json).output().unwrap();
    assert!(o.status.success(), "{}", text(&o));
    let t = text(&o);
    assert!(t.contains("status            : finished") && t.contains("solute 1 balance"), "{t}");
    for f in ["T_LEVEL.csv", "A_LEVEL.csv", "NOD_INF.csv", "OBS_NODE.csv", "BALANCE.csv"] {
        assert!(out.join(f).exists(), "{f} missing");
    }
    let res = rusteau_io::load_results(&json).unwrap();
    assert!(res.finished && !res.tlevel.is_empty());
}

#[test]
fn compare_agrees_with_matching_reference_and_reports_differences() {
    let d = workdir("cmp");
    let prj = make_project(&d);
    let out = d.join("out");
    assert!(bin().arg(&prj).args(["-q", "-o"]).arg(&out).output().unwrap().status.success());
    let good = d.join("good");
    let bad = d.join("bad");
    std::fs::create_dir_all(&good).unwrap();
    std::fs::create_dir_all(&bad).unwrap();
    reference_from_csv(&out.join("T_LEVEL.csv"), &good, false);
    reference_from_csv(&out.join("T_LEVEL.csv"), &bad, true);

    let o = bin().arg(&prj).args(["-q", "-o"]).arg(&out).arg("--compare").arg(&good).output().unwrap();
    let t = text(&o);
    assert_eq!(o.status.code(), Some(0), "{t}");
    assert!(t.contains("compared T_Level.out") && t.contains("reference values agree"), "{t}");

    let o = bin().arg(&prj).args(["-q", "-o"]).arg(&out).arg("-c").arg(&bad).output().unwrap();
    let t = text(&o);
    assert_eq!(o.status.code(), Some(4), "{t}");
    assert!(t.contains("MISMATCH") && t.contains("'vTop'"), "{t}");

    // directory without reference files, and a directory that does not exist
    let empty = d.join("empty");
    std::fs::create_dir_all(&empty).unwrap();
    let o = bin().arg(&prj).args(["-q", "-o"]).arg(&out).arg("-c").arg(&empty).output().unwrap();
    assert_eq!(o.status.code(), Some(1), "{}", text(&o));
    let o = bin().arg(&prj).args(["-q", "-o"]).arg(&out).arg("-c").arg(d.join("nope")).output().unwrap();
    assert_eq!(o.status.code(), Some(1), "{}", text(&o));
}

#[test]
fn compare_without_directory_uses_the_project_folder() {
    let d = workdir("cmp_default");
    let prj = make_project(&d);
    let out = d.join("out");
    assert!(bin().arg(&prj).args(["-q", "-o"]).arg(&out).output().unwrap().status.success());
    reference_from_csv(&out.join("T_LEVEL.csv"), &d, false); // T_Level.out next to the project file
    let o = bin().arg(&prj).args(["-q", "-o"]).arg(&out).arg("--compare").output().unwrap();
    assert_eq!(o.status.code(), Some(0), "{}", text(&o));
}
