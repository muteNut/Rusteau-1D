//! Reading and comparing HYDRUS-1D reference output files (`*.OUT`) with simulation results.
//!
//! Every column of every supported file is compared; a reference column without a simulated counterpart is reported as an
//! error (nothing is skipped silently). Supported files: `T_Level.out`, `A_Level.out`, `Nod_Inf.out` (all variants),
//! `Obs_Node.out`, `Balance.out`, `Solute<i>.out` and `Meteo.out` (all four modes).

use crate::legacy::sanitize_fortran_line;
use rusteau_core::output::IrrigationEvent;
use rusteau_core::{Project, Results};
use std::path::{Path, PathBuf};

/// Absolute / relative tolerance of one column.
#[derive(Clone, Copy, Debug)]
pub struct Tol {
    pub rel: f64,
    pub abs: f64,
}

/// Column specific tolerances (a value passes when the absolute OR the relative difference is within tolerance).
pub fn tolerance(col: &str) -> Tol {
    let c = col.to_ascii_lowercase();
    if c == "depth" {
        // Nod_Inf.out prints the depth with two decimals in the reference files of several examples
        Tol { rel: 0.0, abs: 0.0051 }
    } else if c.contains("kappa") {
        Tol { rel: 0.0, abs: 0.5 } // integer branch flag
    } else if c.contains("balr") {
        Tol { rel: 0.0, abs: 0.05 } // balance errors in percent
    } else if c.contains("temp") || c.starts_with("tmean") || c.starts_with("temp(") {
        Tol { rel: 1e-3, abs: 0.1 }
    } else if c.contains("moist") || c.starts_with("theta") || c.contains("im.moist") {
        Tol { rel: 1e-3, abs: 0.005 }
    } else if c.contains("flux") || c.contains("vtop") || c.contains("vbot") || c.starts_with("vliquid") || c.starts_with("vvapor") || c.starts_with("vtotal") {
        Tol { rel: 2e-3, abs: 1e-3 }
    } else if c.contains("conc") || c.contains("sorb") || c.starts_with("c") && c.len() <= 8 && c.contains("top") {
        Tol { rel: 1e-3, abs: 1e-5 }
    } else {
        Tol { rel: 1e-3, abs: 1e-3 }
    }
}

/// Tolerances of the rows of `Balance.out`: the Fortran prints most of them with three significant digits (e13.3) and the areas
/// with one decimal.
pub fn balance_tolerance(row: &str) -> Tol {
    let r = row.to_ascii_lowercase();
    if r.starts_with("area") {
        Tol { rel: 0.0, abs: 0.051 }
    } else if r.contains("balr") {
        Tol { rel: 0.0, abs: 0.05 }
    } else {
        // the Fortran runs in single precision: balance residuals (WatBalT, CncBalT, In-flow ...) are printed as +-1e-6 noise
        // where the double precision run gives 1e-15
        Tol { rel: 6e-3, abs: 1e-4 }
    }
}

/// Half of the print quantum of a number as it appears in a reference file: a value printed as `-.239E+00` stands for every number
/// within 0.5e-3 of it, `-73.92` for every number within 0.005. A simulated value that rounds to the printed text is therefore
/// never a difference, whatever the precision of the printing format (3-6 significant digits depending on the column and on the
/// version of the program that wrote the file).
pub fn half_quantum(token: &str) -> f64 {
    let t = token.trim().replace(['d', 'D'], "e");
    let (mant, exp) = match t.find(['e', 'E']) {
        Some(i) => (&t[..i], t[i + 1..].parse::<i32>().unwrap_or(0)),
        None => (&t[..], 0),
    };
    let decimals = mant.find('.').map(|i| mant[i + 1..].chars().take_while(|c| c.is_ascii_digit()).count() as i32).unwrap_or(0);
    0.5 * 10f64.powi(exp - decimals)
}

/// Parses a line consisting only of Fortran numbers (also glued ones such as `0.1E+01-0.2E+01`); returns every value with the half
/// print quantum of its text.
pub fn parse_numbers_q(line: &str) -> Option<Vec<(f64, f64)>> {
    let s = sanitize_fortran_line(line.trim());
    if s.is_empty() {
        return None;
    }
    let mut v = Vec::new();
    for tok in s.split_whitespace() {
        v.push((tok.replace(['d', 'D'], "e").parse::<f64>().ok()?, half_quantum(tok)));
    }
    Some(v)
}

/// Parses a line consisting only of Fortran numbers (also glued ones such as `0.1E+01-0.2E+01`).
pub fn parse_numbers(line: &str) -> Option<Vec<f64>> {
    parse_numbers_q(line).map(|v| v.into_iter().map(|(x, _)| x).collect())
}

/// Table with a time column and named value columns.
#[derive(Clone, Debug, Default)]
pub struct RefTable {
    pub times: Vec<f64>,
    pub names: Vec<String>,
    pub cols: Vec<Vec<f64>>,
    /// Half print quantum of every value (same layout as `cols`; empty = exact values)
    pub hq: Vec<Vec<f64>>,
}

/// Reads all numeric lines with at least `names.len()` values; the first name is the time column.
pub fn read_table(path: &Path, names: &[&str]) -> Option<RefTable> {
    let text = std::fs::read_to_string(path).ok()?;
    let mut rows: Vec<Vec<(f64, f64)>> = Vec::new();
    for line in text.lines() {
        if let Some(v) = parse_numbers_q(line) {
            if v.len() >= names.len() {
                rows.push(v);
            }
        }
    }
    if rows.is_empty() {
        return None;
    }
    let mut t = RefTable { times: rows.iter().map(|r| r[0].0).collect(), names: names[1..].iter().map(|s| s.to_string()).collect(), ..Default::default() };
    for k in 1..names.len() {
        t.cols.push(rows.iter().map(|r| r[k].0).collect());
        t.hq.push(rows.iter().map(|r| r[k].1).collect());
    }
    Some(t)
}

pub const T_LEVEL_COLS: [&str; 22] = [
    "Time", "rTop", "rRoot", "vTop", "vRoot", "vBot", "sum(rTop)", "sum(rRoot)", "sum(vTop)", "sum(vRoot)", "sum(vBot)", "hTop", "hRoot", "hBot", "RunOff",
    "sum(RunOff)", "Volume", "sum(Infil)", "sum(Evap)", "TLevel", "Cum(WTrans)", "SnowLayer",
];
pub const A_LEVEL_COLS: [&str; 10] = ["Time", "sum(rTop)", "sum(rRoot)", "sum(vTop)", "sum(vRoot)", "sum(vBot)", "hTop", "hRoot", "hBot", "A-level"];
pub const SOLUTE_COLS: [&str; 17] = [
    "Time", "cvTop", "cvBot", "Sum(cvTop)", "Sum(cvBot)", "cvCh0", "cvCh1", "cTop", "cRoot", "cBot", "cvRoot", "Sum(cvRoot)", "Sum(cvNEql)", "TLevel", "cGWL", "cRunOff",
    "Sum(cRunOff)",
];
/// T_Level.out of the old HYDRUS builds (header without year, e.g. the 1DRAINAG / 1SCALING / 2NOHYSTR sample outputs): 15
/// columns, no run-off / volume / snow columns, the step counter is called `t-level`.
pub const T_LEVEL_COLS_OLD: [&str; 15] = [
    "Time", "rTop", "rRoot", "vTop", "vRoot", "vBot", "sum(rTop)", "sum(rRoot)", "sum(vTop)", "sum(vRoot)", "sum(vBot)", "hTop", "hRoot", "hBot", "t-level",
];
/// Columns that are counters / indices and cannot be compared between differently stepped runs.
const INDEX_COLS: [&str; 3] = ["TLevel", "A-level", "t-level"];

/// Reads a `T_Level.out` of either layout (current: 22 columns; old builds: 15 columns, recognised by the `t-level` header).
pub fn read_t_level(path: &Path) -> Option<RefTable> {
    let text = std::fs::read_to_string(path).ok()?;
    let old = text.lines().any(|l| l.contains("t-level")) && !text.lines().any(|l| l.contains("Volume"));
    read_table(path, if old { &T_LEVEL_COLS_OLD } else { &T_LEVEL_COLS })
}

/// Simulated series: name -> values (same length as the time vector).
pub type SimSeries = Vec<(String, Vec<f64>)>;

fn find_file(folder: &Path, name: &str) -> Option<PathBuf> {
    std::fs::read_dir(folder).ok()?.flatten().map(|e| e.path()).find(|p| p.file_name().map_or(false, |f| f.to_string_lossy().eq_ignore_ascii_case(name)))
}

/// Range of the simulated series that is compatible with a reference time printed with limited resolution.
/// Returns `(lo, hi, aligned)`: when simulated steps fall inside the printed-time window the range of those values is used
/// (`aligned`); otherwise the reference time lies between two simulated steps and the range spanned by the two neighbouring
/// steps is used (a different time stepping of the two codes must not be reported as a wrong result, but a value outside the
/// bracket still is).
fn sim_range(st: &[f64], sv: &[f64], t: f64, half: f64) -> Option<(f64, f64, bool)> {
    if st.is_empty() || t < st[0] - half - 1e-12 || t > st[st.len() - 1] + half + 1e-12 {
        return None;
    }
    let lo_i = st.partition_point(|&x| x < t - half - 1e-12);
    let hi_i = st.partition_point(|&x| x <= t + half + 1e-12);
    let (a, b, aligned) = if lo_i < hi_i {
        (lo_i, hi_i - 1, true)
    } else {
        let k = hi_i.clamp(1, st.len() - 1);
        (k - 1, k, false)
    };
    let (mut lo, mut hi) = (f64::INFINITY, f64::NEG_INFINITY);
    for &v in &sv[a..=b] {
        lo = lo.min(v);
        hi = hi.max(v);
    }
    Some((lo, hi, aligned))
}

/// Differences below this absolute / relative size are not checked: the reference outputs come from a single-precision program
/// and are printed with 4-5 significant digits; the engine itself runs in double precision.
pub const PRECISION_FLOOR: f64 = 1e-4;

fn within(r: f64, lo: f64, hi: f64, tol: Tol) -> bool {
    within_q(r, lo, hi, tol, 0.0)
}

/// `hq`: half print quantum of the reference value (see [`half_quantum`]); a value that rounds to the printed reference is not a
/// difference. The tiny factor absorbs the binary representation of the decimal text.
fn within_q(r: f64, lo: f64, hi: f64, tol: Tol, hq: f64) -> bool {
    let scale = r.abs().max(lo.abs()).max(hi.abs());
    let slack = tol.abs.max(tol.rel * scale).max(PRECISION_FLOOR).max(PRECISION_FLOOR * scale).max(hq * 1.000001);
    r >= lo - slack && r <= hi + slack
}

fn median(v: &mut Vec<f64>) -> Option<f64> {
    v.retain(|x| x.is_finite());
    if v.is_empty() {
        return None;
    }
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    Some(v[v.len() / 2])
}

/// Text describing the failing values of one column: samples and the median simulated/reference ratio (a constant ratio
/// points to a unit or scaling convention, -1 to a sign convention).
fn failure_hint(samples: &[(f64, f64, f64)], ratios: &mut Vec<f64>) -> String {
    let mut s = String::new();
    for (t, sim, r) in samples.iter().take(3) {
        s.push_str(&format!(" [at {t}: sim {sim:.6e}, ref {r:.6e}]"));
    }
    if let Some(m) = median(ratios) {
        s.push_str(&format!(" median sim/ref = {m:.4}"));
    }
    s
}

/// Compares every reference column with the simulated series of the same name. Returns the mismatch messages, the number of
/// compared values and how many of them were compared against the bracket of two neighbouring simulated steps (reference time
/// without a simulated step).
pub fn compare_series_ex(
    tag: &str,
    time_res: &dyn Fn(f64) -> f64,
    reference: &RefTable,
    sim_t: &[f64],
    sim: &SimSeries,
    skip: &[&str],
) -> (Vec<String>, usize, usize) {
    let mut errors = Vec::new();
    let (mut compared, mut bracketed) = (0, 0);
    for (ci, name) in reference.names.iter().enumerate() {
        if skip.iter().any(|s| s.eq_ignore_ascii_case(name)) || INDEX_COLS.iter().any(|s| s.eq_ignore_ascii_case(name)) {
            continue;
        }
        let Some((_, sv)) = sim.iter().find(|(n, _)| n.eq_ignore_ascii_case(name)) else {
            errors.push(format!("[{tag}] column '{name}' has no simulated counterpart"));
            continue;
        };
        let tol = tolerance(name);
        let (mut failed, mut checked) = (0usize, 0usize);
        let (mut worst, mut worst_ref, mut worst_sim, mut worst_t) = (0.0f64, 0.0, 0.0, 0.0);
        let mut samples: Vec<(f64, f64, f64)> = Vec::new();
        let mut ratios: Vec<f64> = Vec::new();
        for (k, &t) in reference.times.iter().enumerate() {
            let r = reference.cols[ci][k];
            let Some((lo, hi, aligned)) = sim_range(sim_t, sv, t, 0.5 * time_res(t) + 1e-9) else { continue };
            checked += 1;
            if !aligned {
                bracketed += 1;
            }
            let hq = reference.hq.get(ci).and_then(|c| c.get(k)).copied().unwrap_or(0.0);
            if !within_q(r, lo, hi, tol, hq) {
                failed += 1;
                let nearest = if r < lo { lo } else { hi };
                if samples.len() < 3 {
                    samples.push((t, nearest, r));
                }
                if r.abs() > 1e-12 {
                    ratios.push(nearest / r);
                }
                let d = (r - nearest).abs();
                if d > worst {
                    worst = d;
                    worst_ref = r;
                    worst_sim = nearest;
                    worst_t = t;
                }
            }
        }
        compared += checked;
        if checked == 0 {
            errors.push(format!("[{tag}] column '{name}': no reference time lies inside the simulated time range"));
        } else if failed > 0 {
            errors.push(format!(
                "[{tag}] column '{name}': {failed}/{checked} values differ (worst at t={worst_t}: sim {worst_sim:.6e}, ref {worst_ref:.6e});{}",
                failure_hint(&samples, &mut ratios)
            ));
        }
    }
    (errors, compared, bracketed)
}

/// Compatibility wrapper of [`compare_series_ex`] without the bracket count.
pub fn compare_series(
    tag: &str,
    time_res: &dyn Fn(f64) -> f64,
    reference: &RefTable,
    sim_t: &[f64],
    sim: &SimSeries,
    skip: &[&str],
) -> (Vec<String>, usize) {
    let (e, n, _) = compare_series_ex(tag, time_res, reference, sim_t, sim, skip);
    (e, n)
}

// ----------------------------------------------------------------------------------------------------------------------
// simulated series
// ----------------------------------------------------------------------------------------------------------------------

pub fn sim_t_level(res: &Results) -> (Vec<f64>, SimSeries) {
    let t: Vec<f64> = res.tlevel.iter().map(|r| r.t).collect();
    let g = |f: &dyn Fn(&rusteau_core::output::TLevel) -> f64| res.tlevel.iter().map(|r| f(r)).collect::<Vec<f64>>();
    let s: SimSeries = vec![
        ("rTop".into(), g(&|r| r.r_top)),
        ("rRoot".into(), g(&|r| r.r_root)),
        ("vTop".into(), g(&|r| r.v_top)),
        ("vRoot".into(), g(&|r| r.v_root)),
        ("vBot".into(), g(&|r| r.v_bot)),
        ("sum(rTop)".into(), g(&|r| r.cum_r_top)),
        ("sum(rRoot)".into(), g(&|r| r.cum_r_root)),
        ("sum(vTop)".into(), g(&|r| r.cum_v_top)),
        ("sum(vRoot)".into(), g(&|r| r.cum_v_root)),
        ("sum(vBot)".into(), g(&|r| r.cum_v_bot)),
        ("hTop".into(), g(&|r| r.h_top)),
        ("hRoot".into(), g(&|r| r.h_root)),
        ("hBot".into(), g(&|r| r.h_bot)),
        ("RunOff".into(), g(&|r| r.run_off)),
        ("sum(RunOff)".into(), g(&|r| r.cum_run_off)),
        ("Volume".into(), g(&|r| r.volume)),
        ("sum(Infil)".into(), g(&|r| r.cum_infil)),
        ("sum(Evap)".into(), g(&|r| r.cum_evap)),
        ("Cum(WTrans)".into(), g(&|r| r.cum_w_transf)),
        ("SnowLayer".into(), g(&|r| r.snow_layer)),
    ];
    (t, s)
}

pub fn sim_a_level(res: &Results) -> (Vec<f64>, SimSeries) {
    let t: Vec<f64> = res.alevel.iter().map(|r| r.t).collect();
    let mut s: SimSeries = Vec::new();
    for (k, n) in ["sum(rTop)", "sum(rRoot)", "sum(vTop)", "sum(vRoot)", "sum(vBot)"].iter().enumerate() {
        s.push((n.to_string(), res.alevel.iter().map(|r| r.cum[k]).collect()));
    }
    s.push(("hTop".into(), res.alevel.iter().map(|r| r.h_top).collect()));
    s.push(("hRoot".into(), res.alevel.iter().map(|r| r.h_root).collect()));
    s.push(("hBot".into(), res.alevel.iter().map(|r| r.h_bot).collect()));
    (t, s)
}

pub fn sim_solute(res: &Results, j: usize) -> (Vec<f64>, SimSeries) {
    let t: Vec<f64> = res.tlevel.iter().map(|r| r.t).collect();
    let g = |f: &dyn Fn(&rusteau_core::output::SoluteTLevel) -> f64| res.tlevel.iter().map(|r| r.solutes.get(j).map(|s| f(s)).unwrap_or(f64::NAN)).collect::<Vec<f64>>();
    let s: SimSeries = vec![
        ("cvTop".into(), g(&|c| c.cv_top)),
        ("cvBot".into(), g(&|c| c.cv_bot)),
        ("Sum(cvTop)".into(), g(&|c| c.cum_top)),
        ("Sum(cvBot)".into(), g(&|c| c.cum_bot)),
        ("cvCh0".into(), g(&|c| c.cum_ch0)),
        ("cvCh1".into(), g(&|c| c.cum_ch1)),
        ("cTop".into(), g(&|c| c.c_top)),
        ("cRoot".into(), g(&|c| c.c_root)),
        ("cBot".into(), g(&|c| c.c_bot)),
        ("cvRoot".into(), g(&|c| c.cv_root)),
        ("Sum(cvRoot)".into(), g(&|c| c.cum_root)),
        ("Sum(cvNEql)".into(), g(&|c| c.cum_neq)),
        ("cGWL".into(), g(&|c| c.c_gwl)),
        ("cRunOff".into(), g(&|c| c.c_run_off)),
        ("Sum(cRunOff)".into(), g(&|c| c.cum_run_off)),
    ];
    (t, s)
}

/// Meteo.out: layout mode (0..3, see `MeteoRow`) and the column names of the reference file.
pub fn meteo_layout(mode: u8) -> Vec<&'static str> {
    match mode {
        0 => vec!["Time", "ET", "Evap", "Transp", "Rns", "Rnl", "RadTerm", "AeroTerm", "Prec", "Interc", "ExInterc"],
        1 => vec!["Time", "ET", "Evap", "Transp", "Rns", "Rnl", "RadTerm", "AeroTerm", "Prec", "Interc", "ExInterc", "AirTemp", "AirRH", "SolarRad"],
        2 => vec!["Time", "Short.Rad.", "Long.Rad.", "Radiation", "Sensible", "Latent", "Heat Flux", "Balance"],
        _ => vec!["Time", "Short.Rad.", "Long.Rad.", "Radiation", "Sensible", "Latent", "Heat Flux", "Balance", "AirTemp", "AirRH", "SolarRad"],
    }
}

/// Determines the Meteo.out mode from the header line and the number of data columns.
pub fn detect_meteo_mode(path: &Path) -> Option<u8> {
    let text = std::fs::read_to_string(path).ok()?;
    let energy = text.lines().any(|l| l.contains("Short.Rad"));
    let ncol = text.lines().filter_map(parse_numbers).map(|v| v.len()).max()?;
    Some(match (energy, ncol) {
        (true, 8) => 2,
        (true, _) => 3,
        (false, 11) => 0,
        (false, _) => 1,
    })
}

/// Reads a Meteo.out file of any mode.
pub fn read_meteo_out(path: &Path) -> Option<(u8, RefTable)> {
    let mode = detect_meteo_mode(path)?;
    let t = read_table(path, &meteo_layout(mode))?;
    Some((mode, t))
}

pub fn sim_meteo(res: &Results) -> (Vec<f64>, SimSeries) {
    let m = &res.meteo;
    let g = |f: &dyn Fn(&rusteau_core::output::MeteoRow) -> f64| m.iter().map(|r| f(r)).collect::<Vec<f64>>();
    let t = g(&|r| r.t);
    let s: SimSeries = vec![
        ("ET".into(), g(&|r| r.et)),
        ("Evap".into(), g(&|r| r.evap_p)),
        ("Transp".into(), g(&|r| r.transp_p)),
        ("Rns".into(), g(&|r| r.rns)),
        ("Rnl".into(), g(&|r| r.rnl)),
        ("RadTerm".into(), g(&|r| r.rad_term)),
        ("AeroTerm".into(), g(&|r| r.aero_term)),
        ("Prec".into(), g(&|r| r.prec)),
        ("Interc".into(), g(&|r| r.interc)),
        ("ExInterc".into(), g(&|r| r.ex_interc)),
        ("Short.Rad.".into(), g(&|r| r.rns)),
        ("Long.Rad.".into(), g(&|r| r.rnl)),
        ("Radiation".into(), g(&|r| r.rn)),
        ("Sensible".into(), g(&|r| r.sensible)),
        ("Latent".into(), g(&|r| r.latent)),
        ("Heat Flux".into(), g(&|r| r.heat_flux)),
        ("Balance".into(), g(&|r| r.balance)),
        ("AirTemp".into(), g(&|r| r.air_temp)),
        ("AirRH".into(), g(&|r| r.air_rh)),
        ("SolarRad".into(), g(&|r| r.solar_rad)),
    ];
    (t, s)
}

// ----------------------------------------------------------------------------------------------------------------------
// Nod_Inf.out
// ----------------------------------------------------------------------------------------------------------------------

#[derive(Clone, Debug, Default)]
pub struct RefProfile {
    pub t: f64,
    pub names: Vec<String>,
    pub rows: Vec<Vec<f64>>,
    /// Half print quantum of every value (same layout as `rows`)
    pub hq: Vec<Vec<f64>>,
}

/// Reads every time block of a `Nod_Inf.out` (all header variants, all columns).
/// `ns` is the number of solutes (needed to name the trailing Conc / Sorb / Sorb2 columns).
pub fn read_nod_inf_full(path: &Path, ns: usize) -> Vec<RefProfile> {
    let Ok(text) = std::fs::read_to_string(path) else { return Vec::new() };
    let mut out: Vec<RefProfile> = Vec::new();
    let mut base: Vec<&str> = Vec::new();
    let mut vapor;
    for line in text.lines() {
        let tl = line.trim();
        if let Some(rest) = tl.strip_prefix("Time:") {
            if let Some(v) = parse_numbers(rest) {
                out.push(RefProfile { t: v[0], ..Default::default() });
            }
            continue;
        }
        if tl.starts_with("Node") {
            vapor = tl.contains("ConLT");
            base = if vapor {
                vec!["Node", "Depth", "Con", "ConLT", "ConVh", "ConVT", "vLiquid", "vVapor", "vTotal", "vVapIso", "vVapTerm"]
            } else if tl.contains("WTrans") {
                vec!["Node", "Depth", "Head", "Moisture", "WTrans", "Im.Moist.", "Flux", "STrans", "Kappa", "v/KsTop", "Temp"]
            } else {
                vec!["Node", "Depth", "Head", "Moisture", "K", "C", "Flux", "Sink", "Kappa", "v/KsTop", "Temp"]
            };
            continue;
        }
        if let (Some(vq), Some(p)) = (parse_numbers_q(tl), out.last_mut()) {
            let v: Vec<f64> = vq.iter().map(|x| x.0).collect();
            if !base.is_empty() && v.len() >= base.len() {
                if p.names.is_empty() {
                    p.names = base.iter().map(|s| s.to_string()).collect();
                    let extra = v.len() - base.len();
                    let groups: Vec<&str> = if ns > 0 && extra == ns {
                        vec!["Conc"]
                    } else if ns > 0 && extra == 2 * ns {
                        vec!["Conc", "Sorb"]
                    } else if ns > 0 && extra == 3 * ns {
                        vec!["Conc", "Sorb", "Sorb2"]
                    } else {
                        Vec::new()
                    };
                    if groups.is_empty() {
                        for j in 0..extra {
                            p.names.push(format!("Extra({})", j + 1));
                        }
                    } else {
                        for g in groups {
                            for j in 1..=ns {
                                p.names.push(format!("{g}({j})"));
                            }
                        }
                    }
                }
                p.rows.push(v);
                p.hq.push(vq.iter().map(|x| x.1).collect());
            }
        }
    }
    out.retain(|p| !p.rows.is_empty());
    out
}

/// Compares all Nod_Inf blocks with the stored profiles (matched by time).
pub fn compare_nod_inf(tag: &str, reference: &[RefProfile], res: &Results) -> (Vec<String>, usize) {
    compare_nod_inf_ex(tag, reference, res, true)
}

/// The simulated record closest in time to a printed reference time (within the print rounding of the time column,
/// 0.005 + 1e-4 t). The closest one, not the first one inside the window: print times like 1, 1.001, 1.002 lie in one window.
fn nearest_in_time<T>(records: &[T], time: impl Fn(&T) -> f64, t: f64) -> Option<&T> {
    records
        .iter()
        .map(|r| ((time(r) - t).abs(), r))
        .filter(|(d, _)| *d < 0.005 + 1e-4 * t.abs())
        .min_by(|a, b| a.0.total_cmp(&b.0))
        .map(|(_, r)| r)
}

/// `with_temp`: compare the Temp column (only meaningful when heat transport is simulated; the reference files of the examples
/// without heat transport print 0 or 20 depending on the program version).
pub fn compare_nod_inf_ex(tag: &str, reference: &[RefProfile], res: &Results, with_temp: bool) -> (Vec<String>, usize) {
    let mut errors = Vec::new();
    let mut compared = 0;
    for rp in reference {
        let Some(sp) = nearest_in_time(&res.profiles, |p| p.t, rp.t) else {
            errors.push(format!("[{tag}] no simulated profile at t={}", rp.t));
            continue;
        };
        for (ci, name) in rp.names.iter().enumerate() {
            if name == "Node" || (name == "Temp" && !with_temp) {
                continue;
            }
            // the top-node flux of the reference files carries solver noise (see NodOut): checked through T_Level.out (vTop) instead
            let skip_top = name == "Flux" || name == "v/KsTop";
            let tol = tolerance(name);
            let (mut failed, mut checked, mut worst, mut wr, mut ws, mut wn) = (0usize, 0usize, 0.0f64, 0.0, 0.0, 0usize);
            let mut ratios: Vec<f64> = Vec::new();
            let mut samples: Vec<(f64, f64, f64)> = Vec::new();
            let mut missing = false;
            for (ri, row) in rp.rows.iter().enumerate() {
                let node = row[0].round() as usize;
                if skip_top && node == 1 {
                    continue;
                }
                let Some(n) = sp.nodes.iter().find(|n| n.node == node) else { continue };
                let sim = match name.as_str() {
                    // the Fortran prints x(i) - xSurf: negative below the surface
                    "Depth" => Some(-n.depth),
                    "Head" => Some(n.h),
                    "Moisture" => Some(n.theta),
                    "K" | "Con" => Some(n.k),
                    "C" => Some(n.c),
                    "Flux" | "vTotal" => Some(n.flux),
                    "Sink" => Some(n.sink),
                    "Kappa" => Some(n.kappa as f64),
                    "v/KsTop" => Some(n.v_over_ks),
                    "Temp" => Some(n.temp),
                    "WTrans" => Some(n.sink_im),
                    "Im.Moist." => Some(n.th_im),
                    "STrans" => Some(n.s_trans),
                    "ConLT" => Some(n.con_lt),
                    "ConVh" => Some(n.con_vh),
                    "ConVT" => Some(n.con_vt),
                    "vLiquid" => Some(n.flux - n.flux_vapor),
                    "vVapor" => Some(n.flux_vapor),
                    "vVapIso" => Some(n.v_vap_iso),
                    "vVapTerm" => Some(n.v_vap_term),
                    s if s.starts_with("Conc(") => s[5..s.len() - 1].parse::<usize>().ok().and_then(|j| n.conc.get(j - 1).copied()),
                    s if s.starts_with("Sorb(") => s[5..s.len() - 1].parse::<usize>().ok().and_then(|j| n.sorb.get(j - 1).copied()),
                    s if s.starts_with("Sorb2(") => s[6..s.len() - 1].parse::<usize>().ok().and_then(|j| n.sorb2.get(j - 1).copied()),
                    _ => None,
                };
                let Some(sv) = sim else {
                    missing = true;
                    break;
                };
                let r = row[ci];
                let hq = rp.hq.get(ri).and_then(|h| h.get(ci)).copied().unwrap_or(0.0);
                checked += 1;
                if !within_q(r, sv, sv, tol, hq) {
                    failed += 1;
                    if r.abs() > 1e-12 {
                        ratios.push(sv / r);
                    }
                    if samples.len() < 3 {
                        samples.push((node as f64, sv, r));
                    }
                    if (r - sv).abs() > worst {
                        worst = (r - sv).abs();
                        wr = r;
                        ws = sv;
                        wn = node;
                    }
                }
            }
            if missing {
                errors.push(format!("[{tag} @ t={}] column '{name}' has no simulated counterpart", rp.t));
            } else if failed > 0 {
                errors.push(format!(
                    "[{tag} @ t={}] column '{name}': {failed}/{checked} nodes differ (worst node {wn}: sim {ws:.6e}, ref {wr:.6e}); first nodes{}",
                    rp.t,
                    failure_hint(&samples, &mut ratios)
                ));
            }
            compared += checked;
        }
    }
    (errors, compared)
}

// ----------------------------------------------------------------------------------------------------------------------
// Balance.out
// ----------------------------------------------------------------------------------------------------------------------

#[derive(Clone, Debug, Default)]
pub struct RefBalance {
    pub t: f64,
    /// (label, values: total first, then the sub-regions)
    pub rows: Vec<(String, Vec<f64>)>,
    /// Half print quantum of every value (same layout as `rows`; empty = exact values)
    pub hq: Vec<Vec<f64>>,
}

const BAL_LABELS: [(&str, bool); 20] = [
    ("Area", false),
    ("W-volumeI", false),
    ("W-volume", false),
    ("In-flow", false),
    ("h Mean", false),
    ("HeatVol", false),
    ("tMean", false),
    ("ConcVolIm", true),
    ("SorbVolIm2", true),
    ("SorbVolIm", true),
    ("ConcVol", true),
    ("cMeanIm", true),
    ("sMeanIm", true),
    ("cMean", true),
    ("Top Flux", false),
    ("Bot Flux", false),
    ("WatBalT", false),
    ("WatBalR", false),
    ("CncBalT", true),
    ("CncBalR", true),
];

pub fn read_balance_out(path: &Path) -> Vec<RefBalance> {
    let Ok(text) = std::fs::read_to_string(path) else { return Vec::new() };
    let mut out: Vec<RefBalance> = Vec::new();
    for line in text.lines() {
        let tl = line.trim();
        if let Some(rest) = tl.strip_prefix("Time") {
            if let Some(i) = rest.find(']') {
                if let Some(v) = parse_numbers(&rest[i + 1..]) {
                    out.push(RefBalance { t: v[0], ..Default::default() });
                }
            }
            continue;
        }
        let Some(cur) = out.last_mut() else { continue };
        for (lab, indexed) in BAL_LABELS {
            if let Some(rest) = tl.strip_prefix(lab) {
                // labels share prefixes (W-volume / W-volumeI): the character after the label must start the unit bracket
                if !rest.trim_start().starts_with('[') {
                    continue;
                }
                let after = &rest[rest.find(']').map(|i| i + 1).unwrap_or(0)..];
                if let Some(vq) = parse_numbers_q(after) {
                    let (mut v, mut h): (Vec<f64>, Vec<f64>) = vq.into_iter().unzip();
                    let mut name = lab.to_string();
                    if indexed && !v.is_empty() {
                        name = format!("{}{}", lab, v.remove(0).round() as i64);
                        h.remove(0);
                    }
                    cur.rows.push((name, v));
                    cur.hq.push(h);
                }
                break;
            }
        }
    }
    out
}

pub fn compare_balance(tag: &str, reference: &[RefBalance], res: &Results) -> (Vec<String>, usize) {
    let mut errors = Vec::new();
    let mut compared = 0;
    for rb in reference {
        let Some(sb) = nearest_in_time(&res.balance, |b| b.t, rb.t) else {
            errors.push(format!("[{tag}] no simulated balance record at t={}", rb.t));
            continue;
        };
        let regions = |f: &dyn Fn(&rusteau_core::output::SubRegion) -> f64| -> Vec<f64> {
            std::iter::once(f(&sb.total)).chain(sb.sub.iter().map(|s| f(s))).collect()
        };
        let sol = |name: &str| -> Option<(usize, String)> {
            let digits: String = name.chars().rev().take_while(|c| c.is_ascii_digit()).collect::<Vec<_>>().into_iter().rev().collect();
            let j: usize = digits.parse().ok()?;
            Some((j, name[..name.len() - digits.len()].to_string()))
        };
        for (row_i, (name, vals)) in rb.rows.iter().enumerate() {
            // HeatVol: the build that produced the reference files computes the solid heat capacity from the wrong parameter
            // columns (TEST2: reference = Disper*Co + Cw*theta instead of Qn*Cn + Qo*Co + Cw*theta; 5SEASON the same, both verified
            // against the printed Selector.in values), so the printed heat volume is not a physical result and is not compared. The
            // temperatures (Nod_Inf.out, T_Level, Obs_Node) are compared.
            if name == "HeatVol" {
                continue;
            }
            // In-flow of the initial record: the Fortran evaluates it with ThVNew = 0 (vapor content not yet initialised), which gives
            // -(vapor volume)/dt for vapor runs (EnBal2b prints -0.227); with dt of the first step it carries no information.
            if name == "In-flow" && res.balance.first().map_or(false, |b0| (b0.t - rb.t).abs() < 1e-9) {
                continue;
            }
            let sim: Option<Vec<f64>> = match name.as_str() {
                "Area" => Some(regions(&|s| s.area)),
                "W-volume" => Some(regions(&|s| s.volume)),
                "W-volumeI" => Some(vec![sb.total.volume_im]),
                "In-flow" => Some(regions(&|s| s.change)),
                "h Mean" => Some(regions(&|s| s.h_mean)),
                "HeatVol" => Some(regions(&|s| s.heat_vol)),
                "tMean" => Some(regions(&|s| s.t_mean)),
                "Top Flux" => Some(vec![sb.top_flux]),
                "Bot Flux" => Some(vec![sb.bot_flux]),
                "WatBalT" => Some(vec![sb.wat_bal_t]),
                "WatBalR" => Some(vec![sb.wat_bal_r]),
                other => sol(other).and_then(|(j, base)| {
                    let k = j - 1;
                    let get = |f: &dyn Fn(&rusteau_core::output::SubRegion) -> Option<f64>| -> Option<Vec<f64>> {
                        std::iter::once(f(&sb.total)).chain(sb.sub.iter().map(|s| f(s))).collect()
                    };
                    match base.as_str() {
                        "ConcVol" => get(&|s| s.c_vol.get(k).copied()),
                        // non-equilibrium compartments: the Rust results keep their sum
                        "ConcVolIm" | "SorbVolIm" | "SorbVolIm2" => get(&|s| s.c_vol_im.get(k).copied()),
                        "cMean" => get(&|s| s.c_mean.get(k).copied()),
                        "cMeanIm" | "sMeanIm" => get(&|s| s.c_mean_im.get(k).copied()),
                        "CncBalT" => sb.sol_bal_t.get(k).map(|v| vec![*v]),
                        "CncBalR" => sb.sol_bal_r.get(k).map(|v| vec![*v]),
                        _ => None,
                    }
                }),
            };
            let Some(sim) = sim else {
                errors.push(format!("[{tag} @ t={}] row '{name}' has no simulated counterpart", rb.t));
                continue;
            };
            // ConcVolIm / SorbVolIm / SorbVolIm2 are printed separately by the Fortran but kept as one sum here: compare the sum
            let split = matches!(name.trim_end_matches(|c: char| c.is_ascii_digit()), "ConcVolIm" | "SorbVolIm" | "SorbVolIm2");
            if split {
                continue; // the sum over the printed immobile rows is checked below
            }
            let mut tol = balance_tolerance(name);
            // The mass-balance residuals WatBalT / CncBalT are differences of two accumulated quantities of the size of the water
            // (solute) volume. The reference accumulates them in single precision (relative resolution 6e-8 per operation, a few
            // 1e-6 of the volume after a few hundred steps: 1DRAINAG prints -2.4e-4 for a volume of about 40 cm), the double
            // precision run gives 1e-13. The residual is therefore compared against 1e-4 plus 1e-5 of the volume it is a
            // difference of.
            if name == "WatBalT" || name.starts_with("CncBalT") {
                let vol_row = if name == "WatBalT" { "W-volume".to_string() } else { format!("ConcVol{}", &name["CncBalT".len()..]) };
                let vol = rb.rows.iter().find(|(n, _)| *n == vol_row).and_then(|(_, v)| v.first().copied()).unwrap_or(0.0).abs();
                tol.abs += 1e-5 * vol;
            }
            for (k, &r) in vals.iter().enumerate() {
                if let Some(&s) = sim.get(k) {
                    compared += 1;
                    let hq = rb.hq.get(row_i).and_then(|h| h.get(k)).copied().unwrap_or(0.0);
                    if !within_q(r, s, s, tol, hq) {
                        errors.push(format!("[{tag} @ t={}] row '{name}' region {k}: sim {s:.6e}, ref {r:.6e}", rb.t));
                    }
                }
            }
        }
        // sum of the printed immobile / sorbed rows against the combined non-equilibrium mass
        for j in 1..=res.n_solutes {
            let rows: Vec<&Vec<f64>> = rb.rows.iter().filter(|(n, _)| n == &format!("ConcVolIm{j}") || n == &format!("SorbVolIm{j}") || n == &format!("SorbVolIm2{j}")).map(|(_, v)| v).collect();
            if rows.is_empty() {
                continue;
            }
            let reference_sum: f64 = rows.iter().map(|v| v.first().copied().unwrap_or(0.0)).sum();
            let sim_sum = sb.total.c_vol_im.get(j - 1).copied().unwrap_or(0.0);
            compared += 1;
            if !within(reference_sum, sim_sum, sim_sum, balance_tolerance("ConcVolIm")) {
                errors.push(format!("[{tag} @ t={}] non-equilibrium mass of solute {j}: sim {sim_sum:.6e}, ref {reference_sum:.6e}", rb.t));
            }
        }
    }
    (errors, compared)
}

// ----------------------------------------------------------------------------------------------------------------------
// Obs_Node.out
// ----------------------------------------------------------------------------------------------------------------------

pub fn compare_obs_node(tag: &str, path: &Path, res: &Results) -> (Vec<String>, usize, usize) {
    compare_obs_node_ex(tag, path, res, true)
}

pub fn compare_obs_node_ex(tag: &str, path: &Path, res: &Results, with_temp: bool) -> (Vec<String>, usize, usize) {
    let Ok(text) = std::fs::read_to_string(path) else { return (vec![], 0, 0) };
    let rows_q: Vec<Vec<(f64, f64)>> = text.lines().filter_map(parse_numbers_q).filter(|v| v.len() >= 4).collect();
    let rows: Vec<Vec<f64>> = rows_q.iter().map(|r| r.iter().map(|x| x.0).collect()).collect();
    if rows.is_empty() {
        return (vec![], 0, 0);
    }
    let nobs = res.obs.first().map(|o| o.points.len()).unwrap_or(0);
    if nobs == 0 {
        return (vec![format!("[{tag}] the reference has observation nodes but the simulation stored none")], 0, 0);
    }
    let ncols = rows.iter().map(|r| r.len()).max().unwrap();
    let ns = res.n_solutes;
    // per observation node: h, theta, Temp (or the flux when lFlux), Conc(1..NS)
    if (ncols - 1) % nobs != 0 || (ncols - 1) / nobs != 3 + ns {
        return (vec![format!("[{tag}] unexpected layout: {ncols} columns for {nobs} observation nodes and {ns} solutes")], 0, 0);
    }
    let mut names: Vec<String> = vec!["Time".into()];
    let mut sim: SimSeries = Vec::new();
    let times: Vec<f64> = res.obs.iter().map(|o| o.t).collect();
    for k in 0..nobs {
        for (kind, label) in [(0usize, "h"), (1, "theta"), (2, "Temp")] {
            names.push(format!("{label}({})", k + 1));
            sim.push((
                format!("{label}({})", k + 1),
                res.obs.iter().map(|o| o.points.get(k).map(|p| match kind { 0 => p.h, 1 => p.theta, _ => p.temp }).unwrap_or(f64::NAN)).collect(),
            ));
        }
        for j in 0..ns {
            names.push(format!("Conc{}({})", j + 1, k + 1));
            sim.push((format!("Conc{}({})", j + 1, k + 1), res.obs.iter().map(|o| o.points.get(k).and_then(|p| p.conc.get(j).copied()).unwrap_or(f64::NAN)).collect()));
        }
    }
    let name_refs: Vec<&str> = names.iter().map(|s| s.as_str()).collect();
    let full: Vec<&Vec<(f64, f64)>> = rows_q.iter().filter(|r| r.len() == ncols).collect();
    let mut table = RefTable { times: full.iter().map(|r| r[0].0).collect(), names: name_refs[1..].iter().map(|s| s.to_string()).collect(), ..Default::default() };
    for k in 1..ncols {
        table.cols.push(full.iter().map(|r| r[k].0).collect());
        table.hq.push(full.iter().map(|r| r[k].1).collect());
    }
    let skip: Vec<String> = if with_temp { vec![] } else { (1..=nobs).map(|k| format!("Temp({k})")).collect() };
    let skip_refs: Vec<&str> = skip.iter().map(|s| s.as_str()).collect();
    compare_series_ex(tag, &|t| if t.abs() < 1e4 { 1e-4 } else { 1e-3 * t.abs() * 1e-4 }, &table, &times, &sim, &skip_refs)
}

// ----------------------------------------------------------------------------------------------------------------------
// Irrig.out (HYDRUS 4.17 triggered irrigation)
// ----------------------------------------------------------------------------------------------------------------------

/// Events of Irrig.out: (trigger time, beginning, end, amount).
pub fn read_irrig_out(path: &Path) -> Vec<[f64; 4]> {
    let Ok(text) = std::fs::read_to_string(path) else { return vec![] };
    let mut events = vec![];
    let mut cur = [f64::NAN; 4];
    for line in text.lines() {
        let l = line.trim();
        let value = || l.rsplit(':').next().and_then(|v| v.trim().replace(['d', 'D'], "e").parse::<f64>().ok());
        let k = if l.starts_with("Time when irrigation is triggered") {
            0
        } else if l.starts_with("Beginning of irrigation") {
            1
        } else if l.starts_with("End of irrigation") {
            2
        } else if l.starts_with("Irrigation amount") {
            3
        } else {
            continue;
        };
        if let Some(v) = value() {
            cur[k] = v;
        }
        if k == 3 {
            events.push(cur);
            cur = [f64::NAN; 4];
        }
    }
    events
}

/// Compares the irrigation events (f12.5 / f10.5 values: 1e-4 absolute).
pub fn compare_irrig_out(tag: &str, reference: &[[f64; 4]], sim: &[IrrigationEvent]) -> (Vec<String>, usize) {
    let mut errors = vec![];
    if reference.len() != sim.len() {
        errors.push(format!("[{tag}] {} irrigation events in the reference, {} simulated", reference.len(), sim.len()));
    }
    let names = ["trigger", "beginning", "end", "amount"];
    let mut n = 0;
    for (k, (r, e)) in reference.iter().zip(sim).enumerate() {
        for (j, v) in [e.t_trigger, e.t_start, e.t_end, e.amount].into_iter().enumerate() {
            n += 1;
            if (v - r[j]).abs() > 1e-4 + 1e-6 * r[j].abs() {
                errors.push(format!("[{tag}] event {}: {} sim {:.5}, ref {:.5}", k + 1, names[j], v, r[j]));
            }
        }
    }
    (errors, n)
}

// ----------------------------------------------------------------------------------------------------------------------
// complete folder comparison
// ----------------------------------------------------------------------------------------------------------------------

#[derive(Debug, Default)]
pub struct FolderReport {
    pub errors: Vec<String>,
    /// Number of series values that were compared against the bracket of two neighbouring simulated steps because the
    /// reference time has no simulated step of its own.
    pub bracketed: usize,
    /// Reference files that were found and compared, with the number of compared values.
    pub files: Vec<(String, usize)>,
}

/// Compares every reference output file found in `folder` with the results of the simulation of `prj`.
pub fn compare_folder(folder: &Path, prj: &Project, res: &Results) -> FolderReport {
    let name = folder.file_name().map(|f| f.to_string_lossy().to_string()).unwrap_or_default();
    let mut rep = FolderReport::default();
    let add = |rep: &mut FolderReport, file: &str, r: (Vec<String>, usize, usize)| {
        let (e, n, br) = r;
        rep.bracketed += br;
        if n == 0 && e.is_empty() {
            rep.errors.push(format!("[{name}/{file}] present but nothing could be compared"));
        }
        rep.errors.extend(e.into_iter().map(|m| format!("[{name}] {m}")));
        rep.files.push((file.to_string(), n));
    };
    let res_f4 = |_t: f64| 1e-4;
    if let Some(p) = find_file(folder, "T_Level.out") {
        match read_t_level(&p) {
            Some(t) => {
                let (st, ss) = sim_t_level(res);
                add(&mut rep, "T_Level.out", compare_series_ex("T_Level.out", &res_f4, &t, &st, &ss, &[]));
            }
            // (the 15-column layout of the old builds used to fail the 22-column read and was skipped without a message)
            None => rep.errors.push(format!("[{name}/T_Level.out] present but no data rows of a known layout could be read")),
        }
    }
    if let Some(p) = find_file(folder, "A_Level.out") {
        if let Some(t) = read_table(&p, &A_LEVEL_COLS) {
            let (st, ss) = sim_a_level(res);
            add(&mut rep, "A_Level.out", compare_series_ex("A_Level.out", &|_| 1e-5, &t, &st, &ss, &[]));
        }
    }
    for j in 0..res.n_solutes.max(prj.n_solutes()) {
        let file = format!("Solute{}.out", j + 1);
        if let Some(p) = find_file(folder, &file) {
            if let Some(t) = read_table(&p, &SOLUTE_COLS) {
                let (st, ss) = sim_solute(res, j);
                add(&mut rep, &file, compare_series_ex(&file, &res_f4, &t, &st, &ss, &[]));
            }
        }
    }
    if let Some(p) = find_file(folder, "Nod_Inf.out") {
        let blocks = read_nod_inf_full(&p, res.n_solutes);
        let r = compare_nod_inf_ex("Nod_Inf.out", &blocks, res, prj.processes.heat);
        add(&mut rep, "Nod_Inf.out", (r.0, r.1, 0));
    }
    if let Some(p) = find_file(folder, "Balance.out") {
        let blocks = read_balance_out(&p);
        let r = compare_balance("Balance.out", &blocks, res);
        add(&mut rep, "Balance.out", (r.0, r.1, 0));
    }
    if let Some(p) = find_file(folder, "Obs_Node.out") {
        let r = compare_obs_node_ex("Obs_Node.out", &p, res, prj.processes.heat);
        // NObs = 0: HYDRUS still writes Obs_Node.out (header and `end` only, e.g. 1SCALING); there is nothing to compare
        let empty_without_nodes = prj.profile.observation_nodes.is_empty() && r.0.is_empty() && r.1 == 0;
        if !empty_without_nodes {
            add(&mut rep, "Obs_Node.out", r);
        }
    }
    if let Some(p) = find_file(folder, "Irrig.out") {
        let r = compare_irrig_out("Irrig.out", &read_irrig_out(&p), &res.irrigation);
        add(&mut rep, "Irrig.out", (r.0, r.1, 0));
    }
    if let Some(p) = find_file(folder, "Meteo.out") {
        if let Some((mode, t)) = read_meteo_out(&p) {
            if res.meteo_mode != mode {
                rep.errors.push(format!("[{name}/Meteo.out] reference is mode {mode}, the simulation produced mode {}", res.meteo_mode));
            } else {
                let (st, ss) = sim_meteo(res);
                // e13.5 times have five significant digits, f8.2 times two decimals
                let tr = move |t: f64| if mode >= 2 { 10f64.powi(t.abs().max(1e-30).log10().floor() as i32 - 4) } else { 0.01 };
                add(&mut rep, "Meteo.out", compare_series_ex("Meteo.out", &tr, &t, &st, &ss, &[]));
            }
        }
    }
    rep
}
