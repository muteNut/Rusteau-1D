//! Fit.in (Block J, inverse solution) reader and writer and the Fit.out report in the layout of HYDRUS-1D.

use rusteau_core::inverse::*;
use rusteau_core::{HydrusError, Project};
use std::fmt::Write as _;
use std::path::Path;

type R<T> = Result<T, HydrusError>;

fn perr(msg: impl Into<String>) -> HydrusError {
    HydrusError::Parse(format!("Fit.in: {}", msg.into()))
}

fn num(t: &str) -> R<f64> {
    t.replace(['d', 'D'], "e").parse::<f64>().map_err(|_| perr(format!("'{t}' is not a number")))
}

fn logical(t: &str) -> R<bool> {
    match t.trim_matches('.').to_ascii_lowercase().as_str() {
        "t" | "true" | "1" | "y" => Ok(true),
        "f" | "false" | "0" | "n" => Ok(false),
        _ => Err(perr(format!("'{t}' is not a logical value"))),
    }
}

struct Lines<'a> {
    lines: Vec<&'a str>,
    pos: usize,
}

impl<'a> Lines<'a> {
    fn next(&mut self, what: &str) -> R<&'a str> {
        let l = self.lines.get(self.pos).copied().ok_or_else(|| perr(format!("unexpected end of file ({what})")))?;
        self.pos += 1;
        Ok(l)
    }
    fn tokens(&mut self, what: &str, min: usize) -> R<Vec<&'a str>> {
        let t: Vec<&str> = self.next(what)?.split_whitespace().collect();
        if t.len() < min {
            return Err(perr(format!("{what}: {} values expected, {} found", min, t.len())));
        }
        Ok(t)
    }
}

/// Reads Fit.in.
pub fn read_fit_in(path: &Path) -> R<InverseSettings> {
    let text = std::fs::read_to_string(path).map_err(|e| HydrusError::Io(format!("{}: {}", path.display(), e)))?;
    parse_fit_in(&text)
}

/// Parses the content of Fit.in (with or without the `Pcp_File_Version=4` line).
pub fn parse_fit_in(text: &str) -> R<InverseSettings> {
    let mut l = Lines { lines: text.lines().map(|x| x.trim_end_matches('\r')).collect(), pos: 0 };
    let mut s = InverseSettings::default();
    let first = l.next("version")?;
    if !first.to_ascii_lowercase().contains("pcp_file_version") {
        l.pos -= 1;
    }
    l.next("NCase")?;
    s.title1 = l.next("title")?.trim().to_string();
    s.title2 = l.next("title")?.trim().to_string();
    l.next("comment")?;
    let t = l.tokens("NOBB MIT iWeight", 3)?;
    let nobb = num(t[0])? as usize;
    s.max_iter = num(t[1])? as usize;
    s.weighting = num(t[2])? as i32;
    l.next("comment")?;
    let t = l.tokens("lWatF lChemF NMat", 3)?;
    s.fit_water = logical(t[0])?;
    s.fit_solute = logical(t[1])?;
    s.n_mat = (num(t[2])? as usize).max(1);
    s.fit_heat = t.get(3).map(|x| logical(x)).transpose()?.unwrap_or(false);
    if s.fit_water {
        l.next("comment")?;
        let t = l.tokens("Model Hysteresis", 2)?;
        s.model = num(t[0])? as i32;
        s.hysteresis = num(t[1])? as i32;
        if s.hysteresis > 0 {
            l.next("comment")?;
            let t = l.tokens("iQSame lAw2Ad lKSame IKappa", 4)?;
            s.q_same = num(t[0])? as i32;
            s.alpha_w_2d = logical(t[1])?;
            s.ks_same = logical(t[2])?;
            s.kappa = num(t[3])? as i32;
        }
    }
    if s.fit_solute {
        l.next("comment")?;
        let t = l.tokens("NS", 1)?;
        s.n_solutes = num(t[0])? as usize;
        s.conc_type = t.get(1).map(|x| num(x)).transpose()?.map(|x| x as i32).unwrap_or(0);
    }
    let block = |l: &mut Lines, group: ParamGroup, s: &mut InverseSettings, what: &str| -> R<usize> {
        let mut ncol = 0;
        for m in 0..s.n_mat {
            let head: Vec<&str> = l.next(what)?.split_whitespace().collect();
            ncol = head.len();
            let init = l.tokens(what, ncol)?;
            let idx = l.tokens(what, ncol)?;
            let lo = l.tokens(what, ncol)?;
            let hi = l.tokens(what, ncol)?;
            for k in 0..ncol {
                s.params.push(FitParameter {
                    group,
                    index: k + 1,
                    mat: m,
                    value: num(init[k])?,
                    fit: num(idx[k])? != 0.0,
                    min: num(lo[k])?,
                    max: num(hi[k])?,
                });
            }
        }
        Ok(ncol)
    };
    if s.fit_water {
        s.n_water_par = block(&mut l, ParamGroup::Water, &mut s, "soil hydraulic parameters")?;
    }
    if s.fit_solute {
        block(&mut l, ParamGroup::Solute, &mut s, "solute transport parameters")?;
    }
    if s.fit_heat {
        // HYDRUS 4.17: Qn Qo Disper. B1 B2 B3 Cn Co Cw per material, after the soil hydraulic records
        block(&mut l, ParamGroup::Heat, &mut s, "heat transport parameters")?;
    }
    l.next("comment")?;
    while s.observations.len() < nobb {
        let Ok(line) = l.next("observations") else { break };
        let t: Vec<&str> = line.split_whitespace().collect();
        if t.first().is_some_and(|x| x.to_ascii_lowercase().starts_with("end")) {
            break;
        }
        if t.len() < 5 {
            continue;
        }
        s.observations.push(Observation { x: num(t[0])?, value: num(t[1])?, kind: num(t[2])? as i32, pos: num(t[3])? as i32, weight: num(t[4])? });
    }
    if s.observations.len() < nobb {
        return Err(perr(format!("{} observations announced, {} found", nobb, s.observations.len())));
    }
    Ok(s)
}

fn lg(b: bool) -> &'static str {
    if b {
        "t"
    } else {
        "f"
    }
}

/// Fit.in text in the format of HYDRUS-1D 4.x.
pub fn fit_in_text(s: &InverseSettings) -> String {
    let mut o = String::new();
    let _ = writeln!(o, "Pcp_File_Version=4");
    let _ = writeln!(o, "      1");
    let _ = writeln!(o, "{}", s.title1);
    let _ = writeln!(o, "{}", s.title2);
    let _ = writeln!(o, "   NOBB     MIT iWeight");
    let _ = writeln!(o, "{:>7} {:>7} {:>7}", s.observations.len(), s.max_iter, s.weighting);
    let _ = writeln!(o, "  lWatF  lChemF    NMat   lTempF");
    let _ = writeln!(o, "{:>7} {:>7} {:>7} {:>7}", lg(s.fit_water), lg(s.fit_solute), s.n_mat, lg(s.fit_heat));
    if s.fit_water {
        let _ = writeln!(o, "  Model   Hysteresis");
        let _ = writeln!(o, "{:>7} {:>7}", s.model, s.hysteresis);
        if s.hysteresis > 0 {
            let _ = writeln!(o, "  iQSame  lAw2Ad  lKSame  IKappa");
            let _ = writeln!(o, "{:>7} {:>7} {:>7} {:>7}", s.q_same, lg(s.alpha_w_2d), lg(s.ks_same), s.kappa);
        }
    }
    if s.fit_solute {
        let _ = writeln!(o, "     NS");
        if s.conc_type != 0 {
            let _ = writeln!(o, "{:>7} {:>7}", s.n_solutes.max(1), s.conc_type);
        } else {
            let _ = writeln!(o, "{:>7}", s.n_solutes.max(1));
        }
    }
    let g = |v: f64| {
        let t = format!("{}", v);
        if t.len() > 10 {
            format!("{:e}", v)
        } else {
            t
        }
    };
    let block = |o: &mut String, group: ParamGroup, labels: Vec<&str>| {
        for m in 0..s.n_mat {
            let ps: Vec<&FitParameter> = s.params.iter().filter(|p| p.group == group && p.mat == m).collect();
            // fields of 10 characters separated by a blank (values such as 2.5347e16 fill a field)
            let row = |f: &dyn Fn(&FitParameter) -> String| ps.iter().map(|p| format!(" {:>10}", f(p))).collect::<String>();
            let _ = writeln!(o, "{}", row(&|p| labels.get(p.index - 1).copied().unwrap_or("Par").to_string()));
            let _ = writeln!(o, "{}", row(&|p| g(p.value)));
            let _ = writeln!(o, "{}", row(&|p| u8::from(p.fit).to_string()));
            let _ = writeln!(o, "{}", row(&|p| g(p.min)));
            let _ = writeln!(o, "{}", row(&|p| g(p.max)));
        }
    };
    if s.fit_water {
        let mut labels: Vec<&str> = WATER_LABELS.to_vec();
        if s.model == 1 {
            labels.truncate(6);
            labels.extend(WATER_MVG_LABELS);
        }
        block(&mut o, ParamGroup::Water, labels);
    }
    if s.fit_solute {
        block(&mut o, ParamGroup::Solute, SOLUTE_LABELS.to_vec());
    }
    if s.fit_heat {
        block(&mut o, ParamGroup::Heat, HEAT_LABELS.to_vec());
    }
    let _ = writeln!(o, "      HO(N)         FOS   ITYPE(N) POS     WTS");
    for ob in &s.observations {
        let _ = writeln!(o, "{:>12} {:>12} {:>5} {:>5} {:>8}", g(ob.x), g(ob.value), ob.kind, ob.pos, g(ob.weight));
    }
    let _ = writeln!(o, "end*** END OF INPUT FILE 'FIT.IN' **********************************");
    o
}

/// Writes Fit.in.
pub fn write_fit_in(path: &Path, s: &InverseSettings) -> R<()> {
    std::fs::write(path, fit_in_text(s)).map_err(|e| HydrusError::Io(format!("{}: {}", path.display(), e)))
}

/// Fortran `Ew.d` field (`0.1234E+01`), width `w`, `d` digits.
fn fe(v: f64, w: usize, d: usize, exp_char: char) -> String {
    let (sign, a) = if v < 0.0 { ("-", -v) } else { ("", v) };
    let (mut mant, mut exp) = (0.0, 0i32);
    if a > 0.0 && a.is_finite() {
        exp = a.log10().floor() as i32 + 1;
        mant = a / 10f64.powi(exp);
        let scale = 10f64.powi(d as i32);
        if (mant * scale).round() >= scale {
            mant /= 10.0;
            exp += 1;
        }
    }
    let digits = (mant * 10f64.powi(d as i32)).round() as u64;
    let body = format!("{}0.{:0width$}{}{}{:02}", sign, digits, exp_char, if exp < 0 { '-' } else { '+' }, exp.abs(), width = d);
    format!("{:>w$}", body, w = w)
}

/// Fit.out in the layout of HYDRUS-1D 4.17.
pub fn fit_out_text(prj: &Project, s: &InverseSettings, r: &InverseResult) -> String {
    let mut o = String::new();
    let star = " ************************************************************************";
    let _ = writeln!(o, "{star}");
    let _ = writeln!(o, " *{:70}*", "");
    let _ = writeln!(o, " *{:70}*", "");
    let _ = writeln!(o, " *{:<70}*", s.title1);
    let _ = writeln!(o, " *{:<70}*", s.title2);
    let _ = writeln!(o, " *{:70}*", "");
    let _ = writeln!(o, "{star}");
    let _ = writeln!(o, "\n");
    let _ = writeln!(o, " Program parameters");
    let _ = writeln!(o, " ==================");
    let _ = writeln!(o, " Max. iterations.....................(MIT)............... {}", s.max_iter);
    let _ = writeln!(o, " No. of observations.................(NOBb).............  {}", s.observations.len());
    let _ = writeln!(o, "\n");
    if s.fit_water {
        for m in 0..s.n_mat {
            let v = |k: usize| s.params.iter().find(|p| p.group == ParamGroup::Water && p.mat == m && p.index == k).map(|p| p.value).unwrap_or(0.0);
            let _ = writeln!(o, " Soil hydraulic properties");
            let _ = writeln!(o, " =========================");
            let _ = writeln!(o, " Residual moisture content...........(WCR)............ {:9.3}", v(1));
            let _ = writeln!(o, " Saturated moisture content..........(WCS)............ {:9.3}", v(2));
            let _ = writeln!(o, " First coefficient...................(ALPHA).......... {:9.3}", v(3));
            let _ = writeln!(o, " Second coefficient..................(N).............. {:9.3}", v(4));
            let _ = writeln!(o, " Saturated conductivity..............(CONDS).......... {}", fe(v(5), 9, 4, 'E').trim_start_matches('0'));
            let _ = writeln!(o, " Pore connectivity factor............(L).............. {:9.3}", v(6));
            let _ = writeln!(o, "\n");
        }
    }
    if s.fit_solute {
        for m in 0..s.n_mat {
            let _ = writeln!(o, " Solute transport parameters");
            let _ = writeln!(o, " ===========================");
            for p in s.params.iter().filter(|p| p.group == ParamGroup::Solute && p.mat == m) {
                let label = SOLUTE_DESCRIPTIONS.get(p.index - 1).copied().unwrap_or("Parameter");
                let tag = format!("({})", SOLUTE_TAGS.get(p.index - 1).copied().unwrap_or("PAR"));
                let dots = 36usize.saturating_sub(label.len());
                let _ = writeln!(o, " {}{}{:.<17}{}", label, ".".repeat(dots), tag, fe(p.value, 10, 3, 'E'));
            }
            let _ = writeln!(o, "\n");
        }
    }
    if s.fit_heat {
        for m in 0..s.n_mat {
            let _ = writeln!(o, " Heat transport properties");
            let _ = writeln!(o, " =========================");
            for p in s.params.iter().filter(|p| p.group == ParamGroup::Heat && p.mat == m) {
                let label = HEAT_DESCRIPTIONS.get(p.index - 1).copied().unwrap_or("Parameter");
                let tag = format!("({})", HEAT_NAMES.get(p.index - 1).copied().unwrap_or("PAR"));
                let dots = 36usize.saturating_sub(label.len());
                let value = if p.index <= 3 { format!("{:11.3}", p.value) } else { fe(p.value, 11, 4, 'E') };
                let _ = writeln!(o, " {}{}{:.<17}{}", label, ".".repeat(dots), tag, value);
            }
            let _ = writeln!(o, "\n");
        }
    }
    let _ = writeln!(o, " Observed Quantity ");
    let _ = writeln!(o, " ==================");
    let _ = writeln!(o, "    Obs     Time    Quantity     Type  Position   Weight");
    let order = report_order(s);
    for (k, &i) in order.iter().enumerate() {
        let ob = &s.observations[i];
        let _ = writeln!(o, "{:>6}{}{}{:>8}{:>6}{:>14.6}", k + 1, f_or_e(ob.x), f_or_e(ob.value), ob.kind, ob.pos, r.weights[i]);
    }
    let _ = writeln!(o, "\n\n");
    let _ = writeln!(o, " Parameter estimation with the  1. initial estimate");
    let _ = writeln!(o, " ==================================================");
    let _ = writeln!(o);
    let mut head = String::from(" Iteration     SSQ     ");
    for nm in &r.names {
        let _ = write!(head, "    {:<8}", nm);
    }
    let _ = writeln!(o, "{}", head.trim_end());
    for (k, it) in r.iterations.iter().enumerate() {
        let mut line = format!("{:>7}{}", k, fe(it.ssq, 13, 4, 'D'));
        for v in &it.params {
            line.push_str(&fe(*v, 12, 4, 'E'));
        }
        let _ = writeln!(o, "{}", line);
    }
    let _ = writeln!(o, "\n");
    match r.stop {
        StopReason::NoReduction => {
            let _ = writeln!(o, " No further reduction in SSQ obtained, optimization stopped\n\n");
        }
        StopReason::Cancelled => {
            let _ = writeln!(o, " Optimization cancelled by the user\n\n");
        }
        _ => {}
    }
    let _ = writeln!(o, " Mass balance error in FE solution during final run was {:7.4} %", r.mass_balance_error);
    let _ = writeln!(o, "\n");
    let _ = writeln!(o, " Correlation matrix");
    let _ = writeln!(o, " ==================");
    let np = r.names.len();
    let mut head = String::from("    ");
    for j in 0..np {
        let _ = write!(head, "{:>6}     ", j + 1);
    }
    let _ = writeln!(o, "{}", head.trim_end());
    for i in 0..np {
        let mut line = format!("{:>4}", i + 1);
        for j in 0..=i {
            let _ = write!(line, "{:>9.4}  ", r.correlation[i][j]);
        }
        let _ = writeln!(o, "{}", line.trim_end());
    }
    let _ = writeln!(o);
    let _ = writeln!(o, " Non-linear least-squares analysis: final results");
    let _ = writeln!(o, " ================================================");
    let _ = writeln!(o, "                                           95% Confidence limits");
    let _ = writeln!(o, " Variable        Value        S.E.Coeff.     Lower         Upper");
    for p in &r.params {
        let _ = writeln!(o, "   {:<6}{}{}{}{}", p.name, fe(p.value, 14, 5, 'E'), fe(p.se, 15, 5, 'E'), fe(p.lower_95, 13, 5, 'E'), fe(p.upper_95, 13, 5, 'E'));
    }
    let _ = writeln!(o, "\n");
    let _ = writeln!(o, " Contributions to the final objective function\n");
    for (k, c) in &r.contributions {
        let _ = writeln!(o, " Measurement Set{:>4}:{}", k, fe(*c, 13, 4, 'E'));
    }
    let m = &r.measures;
    let _ = writeln!(o, "\n RSQUARE for regression of predicted vs observed ={:7.5}\n", m.r_squared);
    let _ = writeln!(o, " Mean Weighted Error, ME               : {}", fe(m.me, 11, 4, 'E'));
    let _ = writeln!(o, " Mean Weighted Absolute Error, MAE     : {}", fe(m.mae, 11, 4, 'E'));
    let _ = writeln!(o, " Root Mean Square Weighted Error, RMSE : {}", fe(m.rmse, 11, 4, 'E'));
    let _ = writeln!(o, "\n");
    let _ = writeln!(o, " Akaike Information Criterion, AIC     : {}", fe(m.aic, 11, 4, 'E'));
    let _ = writeln!(o, " Bayesian Information Criterion, BIC   : {}", fe(m.bic, 11, 4, 'E'));
    let _ = writeln!(o, "\n\n");
    let _ = writeln!(o, "------------Observed & Fitted Quantity -----------------------------");
    let _ = writeln!(o, "                                                   Resi-");
    let _ = writeln!(o, "    No     Time           Obs        Fitted        dual   Type  Position");
    // grouped by type and position, as HYDRUS prints it
    let mut groups: Vec<(i32, i32)> = s.observations.iter().map(|ob| (ob.kind, ob.pos)).collect();
    groups.sort_unstable();
    groups.dedup();
    for (kind, pos) in groups {
        for (k, &i) in order.iter().enumerate() {
            let ob = &s.observations[i];
            if ob.kind != kind || ob.pos != pos {
                continue;
            }
            let f = r.fitted[i];
            let _ = writeln!(o, "{:>5}{:>11.3}{:>14.5}{:>13.5}{:>13.5}{:>6}{:>6}", k + 1, ob.x, ob.value, f, ob.value - f, ob.kind, ob.pos);
        }
    }
    let _ = writeln!(o, "\n");
    if s.fit_water {
        if let Some(p) = r.project.as_ref() {
            property_tables(&mut o, p);
        }
    }
    if r.starts.len() > 1 || r.stages > 1 {
        // Rusteau iteration (not in HYDRUS): stages on frozen time levels and further starting points
        let _ = writeln!(o, "\n Rusteau iteration: {} stage(s) on frozen time levels; {} starting point(s), the result of start {} is reported", r.stages, r.starts.len(), r.best_start);
        if r.starts.len() > 1 {
            let _ = writeln!(o, "\n Start   Final SSQ   Iter  Initial estimates");
            for (k, st) in r.starts.iter().enumerate() {
                let init: String = st.initial.iter().map(|v| fe(*v, 12, 4, 'E')).collect();
                match &st.error {
                    Some(e) => {
                        let _ = writeln!(o, "{k:>6}   failed      {:>4} {init}   ({})", st.iterations, e.lines().next().unwrap_or(""));
                    }
                    None => {
                        let _ = writeln!(o, "{k:>6} {} {:>4} {init}", fe(st.ssq, 11, 4, 'D'), st.iterations);
                    }
                }
            }
        }
    }
    let _ = writeln!(o, " \n Inverse solution calculation time [sec]  {}", r.elapsed_secs);
    let _ = prj;
    o
}

/// HYDRUS prints values below 0.01 in E format in the list of observations.
fn f_or_e(v: f64) -> String {
    if v != 0.0 && v.abs() < 0.01 {
        fe(v, 11, 3, 'E')
    } else {
        format!("{:>11.3}", v)
    }
}

/// Tables of the fitted hydraulic properties (θ, K, D = K/C against log |h|) of every material, as at the end of Fit.out.
fn property_tables(o: &mut String, p: &Project) {
    use rusteau_core::material::{fc, fk, fq, par_of};
    let xc = p.units.x_conv();
    for m in &p.water.materials {
        let par = par_of(m, p.water.model, xc);
        let ks = fk(p.water.model, 0.0, &par);
        let _ = writeln!(o, "  Pressure    Log P      WC       Rel K     Log RK   Abs K    Log KA     Diffus     Log D");
        let _ = writeln!(o, "{}{:8}{:10.4}{}{:8}{}", fe(0.0, 10, 3, 'E'), "", fq(p.water.model, 0.0, &par), fe(1.0, 13, 3, 'E'), "", fe(ks, 13, 3, 'E'));
        for k in 0..75 {
            let lp = 0.15 + 0.075 * k as f64;
            let h = 10f64.powf(lp);
            let th = fq(p.water.model, -h, &par);
            let kk = fk(p.water.model, -h, &par);
            let c = fc(p.water.model, -h, &par).max(1e-300);
            let rk = kk / ks.max(1e-300);
            let d = kk / c;
            let lg = |x: f64| if x > 0.0 { x.log10() } else { -99.999 };
            let _ = writeln!(o, "{}{:8.3}{:10.4}{}{:8.3}{}{:8.3}{}{:8.3}", fe(h, 10, 3, 'E'), lp, th, fe(rk, 13, 3, 'E'), lg(rk), fe(kk, 13, 3, 'E'), lg(kk), fe(d, 13, 3, 'E'), lg(d));
        }
        let _ = writeln!(o, "\n");
    }
}

const SOLUTE_DESCRIPTIONS: [&str; 20] = [
    "Bulk density",
    "Longitudinal dispersivity",
    "Type-1 adsorption sites fraction",
    "Immobile water content",
    "Diffusion in water",
    "Diffusion in gas",
    "First adsorption coefficient",
    "Second adsorption coefficient",
    "Third adsorption coefficient",
    "Henry law constant",
    "First-order reaction (liquid)",
    "First-order reaction (solid)",
    "first-order reaction (gas)",
    "Chain first-order reaction (liquid)",
    "Chain first-order reaction (solid)",
    "Chain first-order reaction (gas)",
    "Zero-order reaction (liquid)",
    "Zero-order reaction (solid)",
    "Zero-order reaction (gas)",
    "Mass-transfer coefficient",
];
const HEAT_DESCRIPTIONS: [&str; 9] = [
    "Solid fraction",
    "Organic matter fraction",
    "Heat dispersivity",
    "Parameter B1",
    "Parameter B2",
    "Parameter B3",
    "Heat capacity - solid",
    "Heat capacity - organic matter",
    "Heat capacity - water",
];
const SOLUTE_TAGS: [&str; 20] = [
    "RO", "DISP", "FRAC", "THIMOB", "DIFW", "DIFG", "KD", "NU", "BETA", "HENRY", "SNKL1", "SNKS1", "SNKG1", "SNKL1`", "SNKS1`", "SNKG1`", "SNKL0",
    "SNKS0", "SNKG0", "ALPHA",
];

/// Order of the observations in the report: timed observations by time (stable), then the others as in Fit.in.
pub fn report_order(s: &InverseSettings) -> Vec<usize> {
    let mut timed: Vec<usize> = (0..s.observations.len()).filter(|&i| s.observations[i].is_timed()).collect();
    timed.sort_by(|&a, &b| s.observations[a].x.total_cmp(&s.observations[b].x));
    timed.extend((0..s.observations.len()).filter(|&i| !s.observations[i].is_timed()));
    timed
}

/// Writes Fit.out.
pub fn write_fit_out(dir: &Path, prj: &Project, s: &InverseSettings, r: &InverseResult) -> R<()> {
    std::fs::create_dir_all(dir).map_err(|e| HydrusError::Io(e.to_string()))?;
    let p = dir.join("Fit.out");
    std::fs::write(&p, fit_out_text(prj, s, r)).map_err(|e| HydrusError::Io(format!("{}: {}", p.display(), e)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fit_in_round_trip() {
        let text = "Pcp_File_Version=4\n      1\nT1\nT2\n   NOBB     MIT iWeight\n      3       6       2\n  lWatF  lChemF    NMat   lTempF\n      t       f       1       f\n  Model   Hysteresis\n      0       2\n  iQSame  lAw2Ad  lKSame  IKappa\n      2      f       t       0\n    thr     ths    Alfa       n         Ks       l      thm     thsW   AlfaW       KsW\n 0.0001    0.43  0.0318  2.6567     0.0145  0.6596    0.43  0.3843     0.0666    0.01 \n      1       1       1       1          1       1       0       0          1       0 \n 1e-005     0.3       0     1.1          0       0     0.3     0.3          0       0 \n  0.019       0       0       0          0       0       0     0.5          0       0 \n      HO(N)         FOS   ITYPE(N) POS     WTS\n           5        0.282     2     2        1 \n          10        0.246     2     2        1 \n      -15000        0.157     5     1        1 \nend\n";
        let s = parse_fit_in(text).unwrap();
        assert_eq!((s.max_iter, s.weighting, s.n_mat, s.hysteresis, s.q_same, s.ks_same), (6, 2, 1, 2, 2, true));
        assert_eq!(s.params.len(), 10);
        assert_eq!(s.fitted(), vec![0, 1, 2, 3, 4, 5, 8]);
        assert_eq!(s.param_name(&s.params[8]), "AlphaW");
        assert_eq!(s.params[0].bounds(), (Some(1e-5), Some(0.019)));
        assert_eq!(s.params[1].bounds(), (Some(0.3), None));
        assert_eq!(s.params[2].bounds(), (None, None));
        assert_eq!(s.observations.len(), 3);
        assert_eq!(s.observations[2].kind, 5);
        let again = parse_fit_in(&fit_in_text(&s)).unwrap();
        assert_eq!(again, s);
    }

    #[test]
    fn fit_in_with_heat_records() {
        // HYDRUS 4.17 (TEST2 with soil hydraulic and heat parameters, two materials)
        let mut text = String::from("Pcp_File_Version=4\n      1\nExample 2\n  \n   NOBB     MIT iWeight\n      2      10       2\n  lWatF  lChemF    NMat   lTempF\n      t       f       2       t\n  Model   Hysteresis\n      0       0\n");
        for m in 0..2 {
            text += "    thr     ths    Alfa       n         Ks       l\n 0.0001   0.399  0.0174  1.3757      29.75     0.5 \n";
            text += if m == 0 { "      0       0       0       0          1       0 \n" } else { "      0       0       0       0          0       0 \n" };
            text += "      0       0       0       0          0       0 \n      0       0       0       0          0       0 \n";
        }
        for m in 0..2 {
            text += "       Qn       Qo         Disper.         B1       B2       B3       Cn       Co       Cw\n";
            text += "     0.601          0          5   1.9e+016 2.5347e+016 9.8939e+016  1.15e+014 1.8737e+014 3.1204e+014 \n";
            text += if m == 0 { "         0          0          0          1          0          0          1          0          0 \n" } else { "         0          0          0          0          0          0          0          0          0 \n" };
            text += "         0          0          0     1e+015          0          0     1e+013          0          0 \n         0          0          0     1e+017          0          0     1e+015          0          0 \n";
        }
        text += "      HO(N)         FOS   ITYPE(N) POS     WTS\n         100        7.692     4     1        1 \n         100        7.613     4     2        1 \nend*** END OF INPUT FILE 'FIT.IN' ****\n";
        let s = parse_fit_in(&text).unwrap();
        assert!(s.fit_water && s.fit_heat && !s.fit_solute && s.observes_temperature());
        assert_eq!(s.params.iter().filter(|p| p.group == ParamGroup::Heat).count(), 18);
        let names: Vec<String> = s.fitted().iter().map(|&i| s.param_name(&s.params[i])).collect();
        assert_eq!(names, ["CONDS", "B1", "Cn"]);
        let b1 = &s.params[s.fitted()[1]];
        assert_eq!((b1.value, b1.min, b1.max, b1.mat), (1.9e16, 1e15, 1e17, 0));
        let again = parse_fit_in(&fit_in_text(&s)).unwrap();
        assert_eq!(again, s);
    }

    #[test]
    fn fortran_e_format() {
        assert_eq!(fe(0.05318, 13, 4, 'D'), "   0.5318D-01");
        assert_eq!(fe(-0.5, 12, 4, 'E'), " -0.5000E+00");
        assert_eq!(fe(0.14011, 13, 5, 'E'), "  0.14011E+00");
    }
}

/// Content of a HYDRUS Fit.out used for comparisons.
#[derive(Clone, Debug, Default)]
pub struct FitOutRef {
    /// Iteration table: names, then (SSQ, parameters) per row
    pub names: Vec<String>,
    pub iterations: Vec<(f64, Vec<f64>)>,
    pub no_reduction: bool,
    /// Final results: (name, value, S.E., lower, upper)
    pub params: Vec<(String, f64, f64, f64, f64)>,
    /// Lower triangle of the correlation matrix
    pub correlation: Vec<Vec<f64>>,
    /// "Observed Quantity" list: (no, time, value, type, position, weight)
    pub observed: Vec<(usize, f64, f64, i32, i32, f64)>,
    /// "Observed & Fitted" rows: (no, time, obs, fitted, residual, type, position)
    pub fitted: Vec<(usize, f64, f64, f64, f64, i32, i32)>,
    pub contributions: Vec<(i32, f64)>,
    pub r_squared: Option<f64>,
    pub me: Option<f64>,
    pub mae: Option<f64>,
    pub rmse: Option<f64>,
    pub aic: Option<f64>,
    pub bic: Option<f64>,
    pub mass_balance_error: Option<f64>,
}

/// Reads a Fit.out of HYDRUS-1D.
pub fn parse_fit_out(text: &str) -> FitOutRef {
    let f = |t: &str| t.replace(['d', 'D'], "e").parse::<f64>().ok();
    let after = |l: &str, key: &str| l.split(key).nth(1).and_then(|x| f(x.split_whitespace().next().unwrap_or("")));
    let mut r = FitOutRef::default();
    #[derive(PartialEq)]
    enum M {
        None,
        Obs,
        Iter,
        Corr,
        Final,
        Fitted,
    }
    let mut mode = M::None;
    for line in text.lines() {
        let l = line.trim_end_matches('\r');
        let t: Vec<&str> = l.split_whitespace().collect();
        if l.contains("Observed Quantity") && !l.contains("Fitted") {
            mode = M::Obs;
            continue;
        }
        if l.contains("Iteration") && l.contains("SSQ") {
            r.names = t[2..].iter().map(|x| x.to_string()).collect();
            mode = M::Iter;
            continue;
        }
        if l.contains("Correlation matrix") {
            mode = M::Corr;
            continue;
        }
        if l.contains("Variable") && l.contains("Value") {
            mode = M::Final;
            continue;
        }
        if l.contains("Observed & Fitted") {
            mode = M::Fitted;
            continue;
        }
        if l.contains("Pressure") && l.contains("Log P") {
            mode = M::None;
        }
        if l.contains("No further reduction") {
            r.no_reduction = true;
        }
        if l.contains("Mass balance error") {
            r.mass_balance_error = after(l, "was");
        }
        if let Some(rest) = l.trim().strip_prefix("Measurement Set") {
            let mut p = rest.split(':');
            if let (Some(a), Some(b)) = (p.next(), p.next()) {
                if let (Ok(k), Some(v)) = (a.trim().parse::<i32>(), f(b.trim())) {
                    r.contributions.push((k, v));
                }
            }
            continue;
        }
        if l.contains("RSQUARE") {
            r.r_squared = after(l, "=");
        }
        for (key, slot) in [(", ME ", &mut r.me), (", MAE ", &mut r.mae), (", RMSE ", &mut r.rmse), (", AIC ", &mut r.aic), (", BIC ", &mut r.bic)] {
            if l.contains(key) {
                *slot = after(l, ":");
            }
        }
        match mode {
            M::Obs if t.len() == 6 && t[0].parse::<usize>().is_ok() => {
                if let (Some(a), Some(b), Some(w)) = (f(t[1]), f(t[2]), f(t[5])) {
                    r.observed.push((t[0].parse().unwrap(), a, b, t[3].parse().unwrap_or(0), t[4].parse().unwrap_or(0), w));
                }
            }
            M::Iter if t.len() >= 2 && t[0].parse::<usize>().is_ok() => {
                let v: Vec<f64> = t[1..].iter().filter_map(|x| f(x)).collect();
                if v.len() == t.len() - 1 {
                    r.iterations.push((v[0], v[1..].to_vec()));
                }
            }
            M::Corr if !t.is_empty() && t[0].parse::<usize>().is_ok() && t.len() >= 2 && l.contains('.') => {
                r.correlation.push(t[1..].iter().filter_map(|x| f(x)).collect());
            }
            M::Final if t.len() == 5 => {
                if let (Some(a), Some(b), Some(c), Some(d)) = (f(t[1]), f(t[2]), f(t[3]), f(t[4])) {
                    r.params.push((t[0].to_string(), a, b, c, d));
                }
            }
            M::Fitted if t.len() == 7 && t[0].parse::<usize>().is_ok() => {
                if let (Some(a), Some(b), Some(c), Some(d)) = (f(t[1]), f(t[2]), f(t[3]), f(t[4])) {
                    let no: usize = t[0].parse().unwrap();
                    // HYDRUS prints the observations of position 0 twice
                    if !r.fitted.iter().any(|x| x.0 == no) {
                        r.fitted.push((no, a, b, c, d, t[5].parse().unwrap_or(0), t[6].parse().unwrap_or(0)));
                    }
                }
            }
            _ => {}
        }
    }
    r
}
