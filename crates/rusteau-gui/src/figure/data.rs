//! The data of the figures: every source (results, input, measured data, data files) is a table of named columns
//! with units; a layer takes its x and y values from two columns of the tables of its items (observation nodes,
//! print times, materials, …) and runs (the current results and the runs kept for comparison).

use crate::measured::{Measured, Target};
use crate::util::conc_unit;
use rusteau_core::figure::{Figure, Layer, Source};
use rusteau_core::inverse::{InverseResult, InverseSettings};
use rusteau_core::{Project, Results};

/// A run kept for comparison: its results and the project they belong to.
#[derive(Clone)]
pub struct SavedRun {
    pub name: String,
    pub project: Project,
    pub results: Results,
}

/// Name, unit and key of a column.
#[derive(Clone, Debug, PartialEq)]
pub struct ColInfo {
    pub key: String,
    /// name with its symbol, e.g. "Water content θ"
    pub name: String,
    /// unit, "–" for dimensionless, "" when unknown
    pub unit: String,
}

impl ColInfo {
    /// Axis title: the name with the unit in brackets.
    pub fn title(&self) -> String {
        if self.unit.is_empty() {
            self.name.clone()
        } else {
            format!("{} ({})", self.name, self.unit)
        }
    }
}

/// Columns of equal length.
#[derive(Clone, Debug, Default)]
pub struct Table {
    pub cols: Vec<(ColInfo, Vec<f64>)>,
}

impl Table {
    fn push(&mut self, key: &str, name: &str, unit: &str, values: Vec<f64>) {
        self.cols.push((ColInfo { key: key.into(), name: name.into(), unit: unit.into() }, values));
    }
    pub fn get(&self, key: &str) -> Option<&(ColInfo, Vec<f64>)> {
        self.cols.iter().find(|c| c.0.key == key)
    }
    pub fn infos(&self) -> Vec<ColInfo> {
        self.cols.iter().map(|c| c.0.clone()).collect()
    }
}

/// One series of a layer: the points and the legend text.
#[derive(Clone, Debug)]
pub struct Series {
    pub label: String,
    pub x: Vec<f64>,
    pub y: Vec<f64>,
    pub x_info: ColInfo,
    pub y_info: ColInfo,
    /// position of the item and the run in the selection of the layer (colours and line styles of comparisons)
    pub item: usize,
    pub run: usize,
}

/// Everything the figures can show.
pub struct DataCtx<'a> {
    pub prj: &'a Project,
    pub results: Option<&'a Results>,
    pub runs: &'a [SavedRun],
    pub measured: &'a [Measured],
    pub fit: Option<(&'a InverseSettings, &'a InverseResult)>,
}

/// Group of a source in the source menu.
pub fn source_group(s: &Source) -> &'static str {
    match s {
        Source::TimeLevel | Source::ObsNodes | Source::Profiles | Source::Balance | Source::ALevel | Source::TLevel1 | Source::Meteo => "Results",
        Source::Hydraulic | Source::AtmInput | Source::ProfileInput => "Input",
        Source::Measured | Source::Fit | Source::DataFile(_) => "Measured and other data",
    }
}

/// Name of a source in the menus.
pub fn source_name(s: &Source, fig: &Figure) -> String {
    match s {
        Source::TimeLevel => "Boundary fluxes and heads (T_Level)".into(),
        Source::ObsNodes => "Observation nodes".into(),
        Source::Profiles => "Profiles at the print times".into(),
        Source::Balance => "Mass balance".into(),
        Source::ALevel => "Atmospheric records: cumulative values (A_Level)".into(),
        Source::TLevel1 => "Extended fluxes (T_Level1)".into(),
        Source::Meteo => "Meteorological output".into(),
        Source::Hydraulic => "Soil hydraulic properties".into(),
        Source::AtmInput => "Time-variable boundary data".into(),
        Source::ProfileInput => "Soil profile: initial condition".into(),
        Source::Measured => "Measured data".into(),
        Source::Fit => "Parameter estimation: observed and fitted".into(),
        Source::DataFile(i) => format!("Data file: {}", fig.data.get(*i).map(|d| d.name.as_str()).unwrap_or("?")),
    }
}

/// All sources offered for a figure.
pub fn all_sources(fig: &Figure) -> Vec<Source> {
    let mut v = vec![
        Source::TimeLevel,
        Source::ObsNodes,
        Source::Profiles,
        Source::Balance,
        Source::ALevel,
        Source::TLevel1,
        Source::Meteo,
        Source::Hydraulic,
        Source::AtmInput,
        Source::ProfileInput,
        Source::Measured,
        Source::Fit,
    ];
    v.extend((0..fig.data.len()).map(Source::DataFile));
    v
}

/// Whether the source has items to choose (nodes, print times, materials, sets).
pub fn has_items(s: &Source) -> bool {
    matches!(s, Source::ObsNodes | Source::Profiles | Source::Hydraulic | Source::Measured | Source::Fit)
}

/// Whether the source depends on the run (kept runs can be compared).
pub fn per_run(s: &Source) -> bool {
    !matches!(s, Source::Measured | Source::Fit | Source::DataFile(_))
}

fn mass(p: &Project) -> String {
    let m = p.units.mass.trim();
    if m.is_empty() || m == "-" {
        "M".into()
    } else {
        m.into()
    }
}

/// Fit sets: (type, position) in the order of the parameter-estimation results page.
fn fit_sets(s: &InverseSettings) -> Vec<(i32, i32)> {
    let mut k: Vec<(i32, i32)> = s.observations.iter().map(|o| (o.kind, o.pos)).collect();
    k.sort_unstable();
    k.dedup();
    k
}

impl DataCtx<'_> {
    /// Project and results of run `name` ("" is the current run).
    pub fn run(&self, name: &str) -> Option<(&Project, Option<&Results>)> {
        if name.is_empty() {
            return Some((self.prj, self.results));
        }
        self.runs.iter().find(|r| r.name == name).map(|r| (&r.project, Some(&r.results)))
    }

    /// Labels of the items of a source (from the current run).
    pub fn items(&self, src: &Source) -> Vec<String> {
        let p = self.prj;
        let lu = p.units.length_str();
        let tu = p.units.time_str();
        match src {
            Source::ObsNodes => {
                let nodes: Vec<usize> = match self.results.and_then(|r| r.obs.first()) {
                    Some(o) => o.points.iter().map(|q| q.node).collect(),
                    None => p.profile.observation_nodes.clone(),
                };
                nodes
                    .iter()
                    .map(|n| {
                        let d = p.profile.nodes.get(n.wrapping_sub(1)).map(|x| x.depth).unwrap_or(f64::NAN);
                        format!("node {n} ({} {lu})", crate::util::sig(d))
                    })
                    .collect()
            }
            Source::Profiles => {
                let times: Vec<f64> = match self.results {
                    Some(r) if !r.profiles.is_empty() => r.profiles.iter().map(|q| q.t).collect(),
                    _ => p.time.print_times.clone(),
                };
                times.iter().map(|t| format!("t = {} {tu}", crate::util::fmt_t(*t))).collect()
            }
            Source::Hydraulic => p.water.materials.iter().enumerate().map(|(k, m)| format!("{} {}", k + 1, m.name)).collect(),
            Source::Measured => self.measured.iter().map(|m| m.name.clone()).collect(),
            Source::Fit => match self.fit {
                Some((s, _)) => {
                    let heat = s.observes_temperature();
                    fit_sets(s).iter().map(|(k, pos)| format!("{}, position {pos}", crate::inverse_ui::type_name(*k, heat))).collect()
                }
                None => vec![],
            },
            _ => vec![],
        }
    }

    /// Columns of a source (for the menus of the x and y variables).
    pub fn columns(&self, src: &Source, fig: &Figure) -> Vec<ColInfo> {
        // the table of the first item of the current run, or of an empty run when there are no results
        if let Some(t) = self.table(src, 0, "", fig) {
            return t.infos();
        }
        let empty = Results { n_solutes: self.prj.n_solutes(), ..Default::default() };
        let ctx = DataCtx { prj: self.prj, results: Some(&empty), runs: &[], measured: &[], fit: None };
        ctx.table_of(src, 0, self.prj, Some(&empty), fig).map(|t| t.infos()).unwrap_or_default()
    }

    /// The table of item `item` of a source in run `run`.
    pub fn table(&self, src: &Source, item: usize, run: &str, fig: &Figure) -> Option<Table> {
        let (p, r) = self.run(run)?;
        self.table_of(src, item, p, r, fig)
    }

    fn table_of(&self, src: &Source, item: usize, p: &Project, r: Option<&Results>, fig: &Figure) -> Option<Table> {
        let lu = p.units.length_str();
        let tu = p.units.time_str();
        let fu = format!("{lu}/{tu}");
        let mu = mass(p);
        let cu = conc_unit(p);
        let sfu = format!("{mu}/{lu}²/{tu}");
        let scu = format!("{mu}/{lu}²");
        let mut t = Table::default();
        match src {
            Source::TimeLevel => {
                let r = r?;
                let v = |f: &dyn Fn(&rusteau_core::output::TLevel) -> f64| r.tlevel.iter().map(f).collect::<Vec<f64>>();
                t.push("t", "Time", tu, v(&|x| x.t));
                t.push("dt", "Time step", tu, v(&|x| x.dt));
                t.push("iter", "Iterations", "–", v(&|x| x.iter_w as f64));
                t.push("rTop", "Potential surface flux", &fu, v(&|x| x.r_top));
                t.push("vTop", "Actual surface flux", &fu, v(&|x| x.v_top));
                t.push("vBot", "Bottom flux", &fu, v(&|x| x.v_bot));
                t.push("rRoot", "Potential transpiration", &fu, v(&|x| x.r_root));
                t.push("vRoot", "Actual transpiration", &fu, v(&|x| x.v_root));
                t.push("sum_rTop", "Cumulative potential surface flux", lu, v(&|x| x.cum_r_top));
                t.push("sum_vTop", "Cumulative surface flux", lu, v(&|x| x.cum_v_top));
                t.push("sum_vBot", "Cumulative bottom flux", lu, v(&|x| x.cum_v_bot));
                t.push("sum_rRoot", "Cumulative potential transpiration", lu, v(&|x| x.cum_r_root));
                t.push("sum_vRoot", "Cumulative actual transpiration", lu, v(&|x| x.cum_v_root));
                t.push("sum_infil", "Cumulative infiltration", lu, v(&|x| x.cum_infil));
                t.push("sum_evap", "Cumulative evaporation", lu, v(&|x| x.cum_evap));
                t.push("runoff", "Surface runoff", &fu, v(&|x| x.run_off));
                t.push("sum_runoff", "Cumulative runoff", lu, v(&|x| x.cum_run_off));
                t.push("precip", "Precipitation", &fu, v(&|x| x.precip));
                t.push("hTop", "Pressure head at the surface", lu, v(&|x| x.h_top));
                t.push("hRoot", "Mean pressure head of the root zone", lu, v(&|x| x.h_root));
                t.push("hBot", "Pressure head at the bottom", lu, v(&|x| x.h_bot));
                t.push("volume", "Water storage", lu, v(&|x| x.volume));
                t.push("snow", "Snow layer", lu, v(&|x| x.snow_layer));
                t.push("tTop", "Temperature at the surface", "°C", v(&|x| x.temp_top));
                t.push("tBot", "Temperature at the bottom", "°C", v(&|x| x.temp_bot));
                t.push("peclet", "Maximum Peclet number", "–", v(&|x| x.peclet));
                t.push("courant", "Maximum Courant number", "–", v(&|x| x.courant));
                for j in 0..r.n_solutes {
                    let s = |f: &dyn Fn(&rusteau_core::output::SoluteTLevel) -> f64| {
                        r.tlevel.iter().map(|x| x.solutes.get(j).map(f).unwrap_or(f64::NAN)).collect::<Vec<f64>>()
                    };
                    let n = j + 1;
                    t.push(&format!("cvTop{n}"), &format!("Solute {n}: flux at the surface"), &sfu, s(&|x| x.cv_top));
                    t.push(&format!("cvBot{n}"), &format!("Solute {n}: flux at the bottom"), &sfu, s(&|x| x.cv_bot));
                    t.push(&format!("sum_cvTop{n}"), &format!("Solute {n}: cumulative flux at the surface"), &scu, s(&|x| x.cum_top));
                    t.push(&format!("sum_cvBot{n}"), &format!("Solute {n}: cumulative flux at the bottom"), &scu, s(&|x| x.cum_bot));
                    t.push(&format!("cTop{n}"), &format!("Solute {n}: concentration at the surface"), &cu, s(&|x| x.c_top));
                    t.push(&format!("cBot{n}"), &format!("Solute {n}: concentration at the bottom"), &cu, s(&|x| x.c_bot));
                    t.push(&format!("cvRoot{n}"), &format!("Solute {n}: root uptake"), &sfu, s(&|x| x.cv_root));
                    t.push(&format!("sum_cvRoot{n}"), &format!("Solute {n}: cumulative root uptake"), &scu, s(&|x| x.cum_root));
                }
            }
            Source::ObsNodes => {
                let r = r?;
                let first = r.obs.first()?;
                let k = item.min(first.points.len().checked_sub(1)?);
                let v = |f: &dyn Fn(&rusteau_core::output::ObsPoint) -> f64| r.obs.iter().map(|o| o.points.get(k).map(f).unwrap_or(f64::NAN)).collect::<Vec<f64>>();
                t.push("t", "Time", tu, r.obs.iter().map(|o| o.t).collect());
                t.push("h", "Pressure head h", lu, v(&|x| x.h));
                t.push("theta", "Water content θ", "–", v(&|x| x.theta));
                t.push("temp", "Temperature", "°C", v(&|x| x.temp));
                t.push("flux", "Water flux", &fu, v(&|x| x.flux));
                for j in 0..r.n_solutes {
                    t.push(&format!("c{}", j + 1), &format!("Concentration {}", j + 1), &cu, v(&|x| x.conc.get(j).copied().unwrap_or(f64::NAN)));
                }
                if first.points.get(k).is_some_and(|q| q.h_matrix.is_some()) {
                    t.push("h_m", "Pressure head h (matrix / immobile)", lu, v(&|x| x.h_matrix.unwrap_or(f64::NAN)));
                    t.push("theta_m", "Water content θ (matrix / immobile)", "–", v(&|x| x.th_matrix.unwrap_or(f64::NAN)));
                }
            }
            Source::Profiles => {
                let r = r?;
                let pr = r.profiles.get(item.min(r.profiles.len().checked_sub(1)?))?;
                let v = |f: &dyn Fn(&rusteau_core::output::NodeOut) -> f64| pr.nodes.iter().map(f).collect::<Vec<f64>>();
                t.push("depth", "Depth", lu, v(&|x| x.depth));
                t.push("h", "Pressure head h", lu, v(&|x| x.h));
                t.push("theta", "Water content θ", "–", v(&|x| x.theta));
                t.push("K", "Hydraulic conductivity K", &fu, v(&|x| x.k));
                t.push("C", "Water capacity C", &format!("1/{lu}"), v(&|x| x.c));
                t.push("flux", "Water flux", &fu, v(&|x| x.flux));
                t.push("sink", "Root water uptake", &format!("1/{tu}"), v(&|x| x.sink));
                t.push("vKs", "Flux / Ks of the top node", "–", v(&|x| x.v_over_ks));
                t.push("temp", "Temperature", "°C", v(&|x| x.temp));
                for j in 0..r.n_solutes {
                    t.push(&format!("c{}", j + 1), &format!("Concentration {}", j + 1), &cu, v(&|x| x.conc.get(j).copied().unwrap_or(f64::NAN)));
                    t.push(&format!("s{}", j + 1), &format!("Sorbed / immobile concentration {}", j + 1), "", v(&|x| x.sorb.get(j).copied().unwrap_or(f64::NAN)));
                }
                if pr.nodes.first().is_some_and(|q| q.h_matrix.is_some()) {
                    t.push("h_m", "Pressure head h (matrix / immobile)", lu, v(&|x| x.h_matrix.unwrap_or(f64::NAN)));
                    t.push("theta_m", "Water content θ (matrix / immobile)", "–", v(&|x| x.th_matrix.unwrap_or(f64::NAN)));
                }
                t.push("node", "Node", "–", v(&|x| x.node as f64));
            }
            Source::Balance => {
                let r = r?;
                let v = |f: &dyn Fn(&rusteau_core::output::BalanceOut) -> f64| r.balance.iter().map(f).collect::<Vec<f64>>();
                t.push("t", "Time", tu, v(&|x| x.t));
                t.push("volume", "Water volume", lu, v(&|x| x.total.volume));
                t.push("WatBalT", "Absolute water balance error", lu, v(&|x| x.wat_bal_t));
                t.push("WatBalR", "Relative water balance error", "%", v(&|x| x.wat_bal_r));
                t.push("top_flux", "Flux at the surface", &fu, v(&|x| x.top_flux));
                t.push("bot_flux", "Flux at the bottom", &fu, v(&|x| x.bot_flux));
                for j in 0..r.n_solutes {
                    let n = j + 1;
                    t.push(&format!("mass{n}"), &format!("Solute {n}: mass in the profile"), &scu, v(&|x| x.total.c_vol.get(j).copied().unwrap_or(f64::NAN)));
                    t.push(&format!("CncBalR{n}"), &format!("Solute {n}: relative balance error"), "%", v(&|x| x.sol_bal_r.get(j).copied().unwrap_or(f64::NAN)));
                }
                let ns = r.balance.first().map(|b| b.sub.len()).unwrap_or(0);
                if ns > 1 {
                    for k in 0..ns {
                        t.push(&format!("vol_sub{}", k + 1), &format!("Water volume of sub-region {}", k + 1), lu, v(&|x| x.sub.get(k).map(|s| s.volume).unwrap_or(f64::NAN)));
                    }
                }
            }
            Source::ALevel => {
                let r = r?;
                let v = |f: &dyn Fn(&rusteau_core::output::ALevel) -> f64| r.alevel.iter().map(f).collect::<Vec<f64>>();
                t.push("t", "Time", tu, v(&|x| x.t));
                for (k, (key, name)) in [
                    ("sum_rTop", "Cumulative potential surface flux"),
                    ("sum_rRoot", "Cumulative potential transpiration"),
                    ("sum_vTop", "Cumulative surface flux"),
                    ("sum_vRoot", "Cumulative actual transpiration"),
                    ("sum_vBot", "Cumulative bottom flux"),
                ]
                .into_iter()
                .enumerate()
                {
                    t.push(key, name, lu, v(&|x| x.cum[k]));
                }
                t.push("hTop", "Pressure head at the surface", lu, v(&|x| x.h_top));
                t.push("hRoot", "Mean pressure head of the root zone", lu, v(&|x| x.h_root));
                t.push("hBot", "Pressure head at the bottom", lu, v(&|x| x.h_bot));
            }
            Source::TLevel1 => {
                let r = r?;
                if r.tlevel1.is_empty() {
                    return None;
                }
                let v = |f: &dyn Fn(&rusteau_core::output::TLevel1) -> f64| r.tlevel1.iter().map(f).collect::<Vec<f64>>();
                t.push("t", "Time", tu, v(&|x| x.t));
                t.push("prec", "Precipitation", &fu, v(&|x| x.prec));
                t.push("evapP", "Potential evaporation", &fu, v(&|x| x.r_soil));
                t.push("evapA", "Actual evaporation", &fu, v(&|x| x.r_evap));
                t.push("infilA", "Actual infiltration", &fu, v(&|x| x.r_infil));
                t.push("transpP", "Potential transpiration", &fu, v(&|x| x.r_root));
                t.push("transpA", "Actual transpiration", &fu, v(&|x| x.v_root));
                t.push("flux_bot", "Bottom flux", &fu, v(&|x| x.v_bot));
                t.push("runoff", "Runoff", &fu, v(&|x| x.run_off));
                t.push("sum_prec", "Cumulative precipitation", lu, v(&|x| x.cum_prec));
                t.push("sum_evapP", "Cumulative potential evaporation", lu, v(&|x| x.cum_evap_p));
                t.push("sum_evapA", "Cumulative actual evaporation", lu, v(&|x| x.cum_evap));
                t.push("sum_infilA", "Cumulative infiltration", lu, v(&|x| x.cum_infil));
                t.push("sum_transpP", "Cumulative potential transpiration", lu, v(&|x| x.cum_r_root));
                t.push("sum_transpA", "Cumulative actual transpiration", lu, v(&|x| x.cum_v_root));
                t.push("sum_flux_bot", "Cumulative bottom flux", lu, v(&|x| x.cum_v_bot));
                t.push("storage", "Water storage", lu, v(&|x| x.volume));
            }
            Source::Meteo => {
                let r = r?;
                if r.meteo.is_empty() {
                    return None;
                }
                let v = |f: &dyn Fn(&rusteau_core::output::MeteoRow) -> f64| r.meteo.iter().map(f).collect::<Vec<f64>>();
                t.push("t", "Time", tu, v(&|x| x.t));
                if r.meteo_mode <= 1 {
                    t.push("et", "Potential evapotranspiration", "mm/d", v(&|x| x.et));
                    t.push("evapP", "Potential evaporation", "mm/d", v(&|x| x.evap_p));
                    t.push("transpP", "Potential transpiration", "mm/d", v(&|x| x.transp_p));
                    t.push("rad_term", "Radiation term", "mm/d", v(&|x| x.rad_term));
                    t.push("aero_term", "Aerodynamic term", "mm/d", v(&|x| x.aero_term));
                    t.push("prec", "Precipitation", "mm/d", v(&|x| x.prec));
                    t.push("interc", "Interception", "mm/d", v(&|x| x.interc));
                }
                t.push("rns", "Net short-wave radiation", "MJ/m²/d", v(&|x| x.rns));
                t.push("rnl", "Net long-wave radiation", "MJ/m²/d", v(&|x| x.rnl));
                if r.meteo_mode >= 2 {
                    t.push("rn", "Net radiation", "MJ/m²/d", v(&|x| x.rn));
                    t.push("sensible", "Sensible heat flux", "MJ/m²/d", v(&|x| x.sensible));
                    t.push("latent", "Latent heat flux", "MJ/m²/d", v(&|x| x.latent));
                    t.push("heat_flux", "Soil heat flux", "MJ/m²/d", v(&|x| x.heat_flux));
                }
                if r.meteo_mode == 1 || r.meteo_mode == 3 {
                    t.push("air_temp", "Air temperature", "°C", v(&|x| x.air_temp));
                    t.push("air_rh", "Relative humidity", "%", v(&|x| x.air_rh));
                }
            }
            Source::Hydraulic => {
                let nm = p.water.materials.len();
                if nm == 0 {
                    return None;
                }
                let (c, w) = crate::hydro::curves(p, item.min(nm - 1));
                let xc = p.units.x_conv();
                t.push("h", "Pressure head |h|", lu, c.h.clone());
                t.push("pF", "pF = log₁₀|h (cm)|", "–", c.h.iter().map(|h| (h / xc * 100.0).log10()).collect());
                t.push("theta", "Water content θ", "–", c.th.clone());
                t.push("K", "Hydraulic conductivity K", &fu, c.k.clone());
                t.push("C", "Water capacity C", &format!("1/{lu}"), c.c.clone());
                if let Some(w) = w {
                    t.push("theta_w", "Water content θ (main wetting curve)", "–", w.th);
                    t.push("K_w", "Hydraulic conductivity K (main wetting curve)", &fu, w.k);
                }
            }
            Source::AtmInput => {
                let recs = &p.atmosphere.records;
                if recs.is_empty() {
                    return None;
                }
                // a record holds from the end of the previous one (or the initial time) to its own time: the first row
                // repeats the first record at the initial time, so that the geom "Steps" draws the intervals
                let mut rows: Vec<&rusteau_core::AtmRecord> = vec![&recs[0]];
                rows.extend(recs.iter());
                let mut times = vec![p.time.t_init];
                times.extend(recs.iter().map(|x| x.t));
                let v = |f: &dyn Fn(&rusteau_core::AtmRecord) -> f64| rows.iter().map(|x| f(x)).collect::<Vec<f64>>();
                t.push("t", "Time", tu, times);
                t.push("prec", "Precipitation", &fu, v(&|x| x.prec));
                t.push("evap", if p.atmosphere.lai_partitioning { "Potential evapotranspiration" } else { "Potential evaporation" }, &fu, v(&|x| x.evap));
                t.push("transp", if p.atmosphere.lai_partitioning { "Leaf area index" } else { "Potential transpiration" }, if p.atmosphere.lai_partitioning { "–" } else { &fu }, v(&|x| x.transp));
                t.push("hCritA", "Minimum surface pressure head hCritA", lu, v(&|x| x.h_crit_a));
                t.push("rBot", "Bottom flux", &fu, v(&|x| x.r_bot));
                t.push("hBot", "Bottom pressure head", lu, v(&|x| x.h_bot));
                t.push("hTop", "Surface pressure head", lu, v(&|x| x.h_top));
                if p.processes.heat {
                    t.push("tTop", "Temperature at the surface", "°C", v(&|x| x.t_top));
                    t.push("tBot", "Temperature at the bottom", "°C", v(&|x| x.t_bot));
                }
                for j in 0..p.n_solutes() {
                    t.push(&format!("cTop{}", j + 1), &format!("Solute {}: concentration at the surface", j + 1), &cu, v(&|x| x.c_top.get(j).copied().unwrap_or(f64::NAN)));
                    t.push(&format!("cBot{}", j + 1), &format!("Solute {}: concentration at the bottom", j + 1), &cu, v(&|x| x.c_bot.get(j).copied().unwrap_or(f64::NAN)));
                }
                if p.atmosphere.has_root_depth {
                    t.push("xRoot", "Rooting depth", lu, v(&|x| x.x_root));
                }
            }
            Source::ProfileInput => {
                let nodes = &p.profile.nodes;
                if nodes.is_empty() {
                    return None;
                }
                let v = |f: &dyn Fn(&rusteau_core::ProfileNode) -> f64| nodes.iter().map(f).collect::<Vec<f64>>();
                t.push("depth", "Depth", lu, v(&|x| x.depth));
                if p.water.init_in_water_content {
                    t.push("theta", "Initial water content θ", "–", v(&|x| x.h));
                } else {
                    t.push("h", "Initial pressure head h", lu, v(&|x| x.h));
                }
                t.push("mat", "Material", "–", v(&|x| x.mat as f64));
                t.push("layer", "Sub-region", "–", v(&|x| x.layer as f64));
                t.push("beta", "Root distribution", "–", v(&|x| x.beta));
                t.push("ah", "Scaling factor αh", "–", v(&|x| x.ah));
                t.push("ak", "Scaling factor αK", "–", v(&|x| x.ak));
                t.push("ath", "Scaling factor αθ", "–", v(&|x| x.ath));
                t.push("temp", "Initial temperature", "°C", v(&|x| x.temp));
                for j in 0..p.n_solutes() {
                    t.push(&format!("c{}", j + 1), &format!("Initial concentration {}", j + 1), &cu, v(&|x| x.conc.get(j).copied().unwrap_or(f64::NAN)));
                }
            }
            Source::Measured => {
                let m = self.measured.get(item)?;
                let (xn, xu, yn, yu) = measured_names(m, p);
                t.push("x", &xn, &xu, m.pts.iter().map(|q| q[0]).collect());
                t.push("y", &yn, &yu, m.pts.iter().map(|q| q[1]).collect());
            }
            Source::Fit => {
                let (s, res) = self.fit?;
                let sets = fit_sets(s);
                let (kind, pos) = *sets.get(item)?;
                let rows: Vec<(f64, f64, f64)> =
                    s.observations.iter().zip(&res.fitted).filter(|(o, _)| o.kind == kind && o.pos == pos).map(|(o, f)| (o.x, o.value, *f)).collect();
                let xname = match kind {
                    0..=4 => ("Time", tu.to_string()),
                    5 | 6 => ("Pressure head h", lu.to_string()),
                    12..=15 => ("Depth", lu.to_string()),
                    _ => ("x", String::new()),
                };
                t.push("x", xname.0, &xname.1, rows.iter().map(|r| r.0).collect());
                t.push("observed", "Observed", "", rows.iter().map(|r| r.1).collect());
                t.push("fitted", "Fitted", "", rows.iter().map(|r| r.2).collect());
                t.push("residual", "Residual (observed − fitted)", "", rows.iter().map(|r| r.1 - r.2).collect());
            }
            Source::DataFile(i) => {
                let d = fig.data.get(*i)?;
                for (k, name) in d.columns.iter().enumerate() {
                    let (n, u) = split_unit(name);
                    t.push(&format!("col{k}"), &n, &u, d.values.get(k).cloned().unwrap_or_default());
                }
            }
        }
        Some(t)
    }

    /// The series of a layer: one per item and run.
    pub fn series(&self, layer: &Layer, fig: &Figure) -> Vec<Series> {
        let item_labels = self.items(&layer.source);
        let items: Vec<usize> = if has_items(&layer.source) {
            if layer.items.is_empty() {
                (0..item_labels.len()).collect()
            } else {
                layer.items.iter().copied().filter(|k| *k < item_labels.len().max(1)).collect()
            }
        } else {
            vec![0]
        };
        let runs: Vec<String> = if per_run(&layer.source) && !layer.runs.is_empty() { layer.runs.clone() } else { vec![String::new()] };
        let mut out = vec![];
        for (ri, run) in runs.iter().enumerate() {
            for (ii, &k) in items.iter().enumerate() {
                let Some(t) = self.table(&layer.source, k, run, fig) else { continue };
                let (Some((xi, xv)), Some((yi, yv))) = (t.get(&layer.x), t.get(&layer.y)) else { continue };
                let item = if has_items(&layer.source) { item_labels.get(k).cloned().unwrap_or_default() } else { String::new() };
                let run_name = if run.is_empty() { "current run".to_string() } else { run.clone() };
                let label = if layer.label.trim().is_empty() {
                    let mut parts = vec![];
                    if !item.is_empty() {
                        parts.push(item.clone());
                    }
                    if runs.len() > 1 || !run.is_empty() {
                        parts.push(run_name.clone());
                    }
                    if parts.is_empty() {
                        yi.name.clone()
                    } else {
                        parts.join(" – ")
                    }
                } else {
                    layer.label.replace("{item}", &item).replace("{run}", &run_name).replace("{y}", &yi.name)
                };
                let n = xv.len().min(yv.len());
                out.push(Series {
                    label,
                    x: xv[..n].iter().map(|v| v * layer.x_scale).collect(),
                    y: yv[..n].iter().map(|v| v * layer.y_scale).collect(),
                    x_info: xi.clone(),
                    y_info: yi.clone(),
                    item: ii,
                    run: ri,
                });
            }
        }
        out
    }
}

/// "time (d)" → ("time", "d").
fn split_unit(s: &str) -> (String, String) {
    let t = s.trim();
    for (open, close) in [('(', ')'), ('[', ']')] {
        if t.ends_with(close) {
            if let Some(p) = t.rfind(open) {
                return (t[..p].trim().to_string(), t[p + 1..t.len() - 1].trim().to_string());
            }
        }
    }
    (t.to_string(), String::new())
}

/// Names and units of the x and y values of a measured data set.
fn measured_names(m: &Measured, p: &Project) -> (String, String, String, String) {
    let lu = p.units.length_str().to_string();
    let tu = p.units.time_str().to_string();
    match m.target {
        Target::Obs { .. } | Target::Flux { .. } => ("Time".into(), tu, "Measured value".into(), String::new()),
        Target::Profile { .. } => ("Depth".into(), lu, "Measured value".into(), String::new()),
        Target::Retention => ("Pressure head |h|".into(), lu, "Water content θ".into(), "–".into()),
        Target::Conductivity => ("Pressure head |h|".into(), lu.clone(), "Hydraulic conductivity K".into(), format!("{lu}/{tu}")),
    }
}

/// Reads a table from a text or CSV file: the first line with non-numeric fields gives the column names; columns are
/// separated by tabs, semicolons, commas or blanks (with semicolons a decimal comma is accepted).
pub fn read_table(path: &std::path::Path) -> Result<rusteau_core::figure::DataTable, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut names: Vec<String> = vec![];
    let mut rows: Vec<Vec<f64>> = vec![];
    for line in text.lines() {
        let l = line.trim();
        if l.is_empty() || l.starts_with('#') {
            continue;
        }
        let semi = l.contains(';');
        let fields: Vec<String> = if l.contains('\t') {
            l.split('\t').map(|s| s.trim().to_string()).collect()
        } else if semi {
            l.split(';').map(|s| s.trim().to_string()).collect()
        } else if l.contains(',') {
            l.split(',').map(|s| s.trim().to_string()).collect()
        } else {
            l.split_whitespace().map(|s| s.to_string()).collect()
        };
        let nums: Vec<Option<f64>> = fields.iter().map(|f| if semi { f.replace(',', ".") } else { f.clone() }).map(|f| f.replace(['d', 'D'], "e").parse::<f64>().ok()).collect();
        if nums.iter().all(|v| v.is_some()) {
            rows.push(nums.into_iter().map(|v| v.unwrap()).collect());
        } else if rows.is_empty() {
            names = fields;
        }
    }
    if rows.is_empty() {
        return Err(format!("{}: no lines with numbers", path.display()));
    }
    let n = rows.iter().map(|r| r.len()).max().unwrap_or(0);
    let mut columns: Vec<String> = (0..n).map(|k| names.get(k).filter(|s| !s.is_empty()).cloned().unwrap_or_else(|| format!("column {}", k + 1))).collect();
    columns.truncate(n);
    let values = (0..n).map(|k| rows.iter().map(|r| r.get(k).copied().unwrap_or(f64::NAN)).collect()).collect();
    let name = path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| "data".into());
    Ok(rusteau_core::figure::DataTable { name, columns, values })
}

/// A file with a kept run: `{"rusteau_run": 1, "name", "project", "results"}`, or the results alone (the `--json`
/// output of the command line program).
pub fn load_run(path: &std::path::Path) -> Result<SavedRun, String> {
    #[derive(serde::Deserialize)]
    struct File {
        name: String,
        project: Option<Project>,
        results: Results,
    }
    let s = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let stem = path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| "run".into());
    if let Ok(f) = serde_json::from_str::<File>(&s) {
        return Ok(SavedRun { name: f.name, project: f.project.unwrap_or_default(), results: f.results });
    }
    let results: Results = serde_json::from_str(&s).map_err(|e| format!("{}: neither a run nor results ({e})", path.display()))?;
    Ok(SavedRun { name: stem, project: Project::default(), results })
}

/// Writes a kept run (readable by [`load_run`]).
pub fn save_run(run: &SavedRun, path: &std::path::Path) -> Result<(), String> {
    #[derive(serde::Serialize)]
    struct File<'a> {
        rusteau_run: u32,
        name: &'a str,
        project: &'a Project,
        results: &'a Results,
    }
    let s = serde_json::to_string(&File { rusteau_run: 1, name: &run.name, project: &run.project, results: &run.results }).map_err(|e| e.to_string())?;
    std::fs::write(path, s).map_err(|e| format!("{}: {e}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusteau_core::figure::Layer;

    fn run_example() -> (Project, Results) {
        let p = rusteau_core::examples::infiltration_evaporation();
        let mut sim = rusteau_core::Simulation::new(p.clone()).unwrap();
        sim.run(|_| true);
        (p, std::mem::take(&mut sim.res))
    }

    #[test]
    fn tables_and_series_of_all_sources() {
        let (p, r) = run_example();
        let fig = Figure::default();
        let ctx = DataCtx { prj: &p, results: Some(&r), runs: &[], measured: &[], fit: None };
        for src in all_sources(&fig) {
            let cols = ctx.columns(&src, &fig);
            if matches!(src, Source::Measured | Source::Fit | Source::TLevel1 | Source::Meteo) {
                continue;
            }
            assert!(!cols.is_empty(), "{src:?} has no columns");
            for c in &cols {
                assert!(!c.name.is_empty());
            }
        }
        // profiles: one series per print time, depth against water content
        let layer = Layer { source: Source::Profiles, x: "theta".into(), y: "depth".into(), ..Default::default() };
        let s = ctx.series(&layer, &fig);
        assert_eq!(s.len(), r.profiles.len());
        assert_eq!(s[0].x.len(), p.profile.nodes.len());
        assert!(s[1].label.starts_with("t = "));
        // a selection of observation nodes and a run kept for comparison
        let runs = vec![SavedRun { name: "A".into(), project: p.clone(), results: r.clone() }];
        let ctx = DataCtx { prj: &p, results: Some(&r), runs: &runs, measured: &[], fit: None };
        let layer = Layer { source: Source::ObsNodes, items: vec![1], runs: vec![String::new(), "A".into()], x: "t".into(), y: "theta".into(), ..Default::default() };
        let s = ctx.series(&layer, &fig);
        assert_eq!(s.len(), 2);
        assert!(s[1].label.contains("A"), "{}", s[1].label);
        // scaling of the values
        let layer = Layer { source: Source::TimeLevel, x: "t".into(), y: "vTop".into(), y_scale: -10.0, ..Default::default() };
        let s = ctx.series(&layer, &fig);
        assert_eq!(s[0].y[5], -10.0 * r.tlevel[5].v_top);
    }

    #[test]
    fn tables_from_files() {
        let dir = std::env::temp_dir().join(format!("rusteau-fig-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let f = dir.join("obs.csv");
        std::fs::write(&f, "# comment\ntime (d);theta (-)\n1;0,25\n2;0,30\n").unwrap();
        let t = read_table(&f).unwrap();
        assert_eq!(t.columns, vec!["time (d)", "theta (-)"]);
        assert_eq!(t.values[1], vec![0.25, 0.30]);
        assert_eq!(split_unit("time (d)"), ("time".into(), "d".into()));
        let (p, r) = run_example();
        let run = SavedRun { name: "Ks 10".into(), project: p, results: r };
        save_run(&run, &dir.join("run.json")).unwrap();
        let back = load_run(&dir.join("run.json")).unwrap();
        assert_eq!(back.name, "Ks 10");
        assert_eq!(back.results.tlevel.len(), run.results.tlevel.len());
        // the results alone, as written by the command line program
        rusteau_io::save_results(&run.results, &dir.join("cli.json")).unwrap();
        assert_eq!(load_run(&dir.join("cli.json")).unwrap().name, "cli");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
