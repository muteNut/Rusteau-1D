//! Ready-made figures: the usual diagrams of a simulation, a start for one's own figures.

use rusteau_core::figure::{Axis, Color, Dash, Figure, Geom, Layer, LegendPos, Marker, Palette, Panel, Source};
use rusteau_core::Project;

/// What the templates may use.
pub struct TemplateCtx<'a> {
    pub prj: &'a Project,
    /// names of the kept runs
    pub runs: Vec<String>,
    pub has_fit: bool,
    pub has_measured: bool,
}

pub struct Template {
    pub name: &'static str,
    pub desc: &'static str,
    /// whether the template fits the project (e.g. solutes simulated)
    pub fits: fn(&TemplateCtx) -> bool,
    pub make: fn(&TemplateCtx) -> Figure,
}

fn layer(source: Source, x: &str, y: &str) -> Layer {
    Layer { source, x: x.into(), y: y.into(), ..Default::default() }
}

fn depth_axis() -> Axis {
    Axis { reverse: true, ..Default::default() }
}

fn panel(title: &str, layers: Vec<Layer>) -> Panel {
    Panel { title: title.into(), layers, ..Default::default() }
}

fn one(name: &str, p: Panel) -> Figure {
    Figure { name: name.into(), panels: vec![p], ..Default::default() }
}

fn profiles(var: &str, title: &str) -> Panel {
    Panel { title: title.into(), y: depth_axis(), legend: LegendPos::BottomRight, ..panel("", vec![layer(Source::Profiles, var, "depth")]) }
}

fn always(_: &TemplateCtx) -> bool {
    true
}

fn solutes(c: &TemplateCtx) -> bool {
    c.prj.processes.solute && c.prj.n_solutes() > 0
}

pub fn all() -> Vec<Template> {
    vec![
        Template { name: "Empty figure", desc: "One panel with one layer: choose the data yourself.", fits: always, make: |_| Figure::default() },
        Template {
            name: "Water content profiles",
            desc: "θ against depth at the print times.",
            fits: always,
            make: |_| Figure { palette: Palette::Viridis, ..one("Water content profiles", profiles("theta", "")) },
        },
        Template {
            name: "Pressure head profiles",
            desc: "h against depth at the print times.",
            fits: always,
            make: |_| Figure { palette: Palette::Viridis, ..one("Pressure head profiles", profiles("h", "")) },
        },
        Template {
            name: "Water content and pressure head profiles",
            desc: "Two panels side by side, one legend.",
            fits: always,
            make: |_| {
                let mut a = profiles("theta", "(a) Water content");
                a.legend = LegendPos::None;
                let mut b = profiles("h", "(b) Pressure head");
                b.legend = LegendPos::Right;
                Figure { name: "Profiles".into(), cols: 2, palette: Palette::Viridis, panels: vec![a, b], ..Default::default() }
            },
        },
        Template {
            name: "Concentration profiles",
            desc: "Liquid concentration of solute 1 against depth at the print times.",
            fits: solutes,
            make: |_| Figure { palette: Palette::Viridis, ..one("Concentration profiles", profiles("c1", "")) },
        },
        Template {
            name: "Observation nodes: water content",
            desc: "θ(t) at the observation nodes.",
            fits: |c| !c.prj.profile.observation_nodes.is_empty(),
            make: |_| one("Observation nodes", panel("", vec![layer(Source::ObsNodes, "t", "theta")])),
        },
        Template {
            name: "Observation nodes: pressure head",
            desc: "h(t) at the observation nodes.",
            fits: |c| !c.prj.profile.observation_nodes.is_empty(),
            make: |_| one("Observation nodes, pressure head", panel("", vec![layer(Source::ObsNodes, "t", "h")])),
        },
        Template {
            name: "Breakthrough curves",
            desc: "Concentration of solute 1 at the observation nodes and at the bottom.",
            fits: solutes,
            make: |c| {
                let mut ls = vec![];
                if !c.prj.profile.observation_nodes.is_empty() {
                    ls.push(layer(Source::ObsNodes, "t", "c1"));
                }
                ls.push(Layer { label: "bottom".into(), dash: Dash::Dashed, ..layer(Source::TimeLevel, "t", "cBot1") });
                one("Breakthrough curves", panel("", ls))
            },
        },
        Template {
            name: "Surface and bottom fluxes",
            desc: "Actual surface flux and bottom flux against time (negative: downward), precipitation on the right axis.",
            fits: always,
            make: |c| {
                let mut ls = vec![layer(Source::TimeLevel, "t", "vTop"), layer(Source::TimeLevel, "t", "vBot")];
                if !c.prj.atmosphere.records.is_empty() && c.prj.water.bc.atmospheric {
                    ls.push(Layer { geom: Geom::Steps, y2: true, color: Some(Color::rgb(150, 150, 150)), ..layer(Source::AtmInput, "t", "prec") });
                }
                let mut p = panel("", ls);
                p.y.label_auto = false;
                p.y.label = "Flux (cm d^{−1})".replace("cm", c.prj.units.length_str()).replace(" d^", &format!(" {}^", c.prj.units.time_str()));
                p.legend = LegendPos::Bottom;
                one("Boundary fluxes", p)
            },
        },
        Template {
            name: "Cumulative water balance",
            desc: "Cumulative infiltration, evaporation, transpiration and bottom flux.",
            fits: always,
            make: |c| {
                let mut ls = vec![layer(Source::TimeLevel, "t", "sum_infil"), layer(Source::TimeLevel, "t", "sum_evap"), layer(Source::TimeLevel, "t", "sum_vBot")];
                if c.prj.processes.root_water_uptake {
                    ls.push(layer(Source::TimeLevel, "t", "sum_vRoot"));
                }
                let mut p = panel("", ls);
                p.y.label_auto = false;
                p.y.label = format!("Cumulative flux ({})", c.prj.units.length_str());
                p.legend = LegendPos::TopLeft;
                one("Cumulative fluxes", p)
            },
        },
        Template {
            name: "Atmospheric forcing",
            desc: "Precipitation as bars, potential evaporation and transpiration as steps (input data).",
            fits: |c| {
                let bc = &c.prj.water.bc;
                !c.prj.atmosphere.records.is_empty() && (bc.atmospheric || bc.top_time_variable || bc.bot_time_variable)
            },
            make: |c| {
                let mut ls = vec![Layer { geom: Geom::Bars, color: Some(Color::rgb(0, 114, 178)), alpha: 0.6, ..layer(Source::AtmInput, "t", "prec") }];
                ls.push(Layer { geom: Geom::Steps, color: Some(Color::rgb(213, 94, 0)), ..layer(Source::AtmInput, "t", "evap") });
                if !c.prj.atmosphere.lai_partitioning {
                    ls.push(Layer { geom: Geom::Steps, color: Some(Color::rgb(0, 158, 115)), dash: Dash::Dashed, ..layer(Source::AtmInput, "t", "transp") });
                }
                let mut p = panel("", ls);
                p.y.label_auto = false;
                p.y.label = format!("Rate ({}/{})", c.prj.units.length_str(), c.prj.units.time_str());
                p.legend = LegendPos::TopRight;
                one("Atmospheric forcing", p)
            },
        },
        Template {
            name: "Retention curve and conductivity",
            desc: "θ(|h|) and K(|h|) of the materials on logarithmic axes.",
            fits: always,
            make: |_| {
                let log = Axis { log: true, minor: true, ..Default::default() };
                let a = Panel { x: log.clone(), legend: LegendPos::BottomLeft, ..panel("(a) Retention curve", vec![layer(Source::Hydraulic, "h", "theta")]) };
                let b = Panel { x: log.clone(), y: log, legend: LegendPos::None, ..panel("(b) Hydraulic conductivity", vec![layer(Source::Hydraulic, "h", "K")]) };
                Figure { name: "Soil hydraulic properties".into(), cols: 2, panels: vec![a, b], ..Default::default() }
            },
        },
        Template {
            name: "Mass balance error",
            desc: "Relative water balance error at the print times.",
            fits: always,
            make: |_| {
                let mut p = panel("", vec![Layer { geom: Geom::LinePoints, ..layer(Source::Balance, "t", "WatBalR") }]);
                p.legend = LegendPos::None;
                one("Mass balance error", p)
            },
        },
        Template {
            name: "Simulated and measured (observation nodes)",
            desc: "θ(t) at the observation nodes as lines with the measured data as points.",
            fits: |c| c.has_measured && !c.prj.profile.observation_nodes.is_empty(),
            make: |_| {
                let p = panel("", vec![layer(Source::ObsNodes, "t", "theta"), Layer { geom: Geom::Points, marker: Marker::OpenCircle, ..layer(Source::Measured, "x", "y") }]);
                one("Simulated and measured", p)
            },
        },
        Template {
            name: "Observed and fitted (parameter estimation)",
            desc: "The measurement sets of the inverse problem: observed values as points, fitted values as lines.",
            fits: |c| c.has_fit,
            make: |_| {
                let p = panel(
                    "",
                    vec![
                        Layer { geom: Geom::Points, marker: Marker::OpenCircle, label: "{item}: observed".into(), ..layer(Source::Fit, "x", "observed") },
                        Layer { label: "{item}: fitted".into(), ..layer(Source::Fit, "x", "fitted") },
                    ],
                );
                let mut f = one("Observed and fitted", p);
                f.panels[0].y.label_auto = false;
                f.panels[0].y.label = "Observed and fitted value".into();
                f
            },
        },
        Template {
            name: "Comparison of runs",
            desc: "Surface flux and cumulative bottom flux of the current results and of all kept runs.",
            fits: |c| !c.runs.is_empty(),
            make: |c| {
                let mut runs = vec![String::new()];
                runs.extend(c.runs.iter().cloned());
                let a = Panel { legend: LegendPos::BottomRight, ..panel("(a) Surface flux", vec![Layer { runs: runs.clone(), label: "{run}".into(), ..layer(Source::TimeLevel, "t", "vTop") }]) };
                let b = Panel { legend: LegendPos::None, ..panel("(b) Cumulative bottom flux", vec![Layer { runs, label: "{run}".into(), ..layer(Source::TimeLevel, "t", "sum_vBot") }]) };
                Figure { name: "Comparison of runs".into(), cols: 2, panels: vec![a, b], ..Default::default() }
            },
        },
        Template {
            name: "Overview (four panels)",
            desc: "Water content and pressure head profiles, boundary fluxes and cumulative fluxes.",
            fits: always,
            make: |_| {
                let mut a = profiles("theta", "(a) Water content");
                a.legend = LegendPos::None;
                let mut b = profiles("h", "(b) Pressure head");
                b.legend = LegendPos::Right;
                let mut c = panel("(c) Fluxes", vec![layer(Source::TimeLevel, "t", "vTop"), layer(Source::TimeLevel, "t", "vBot")]);
                c.legend = LegendPos::BottomRight;
                let mut d = panel("(d) Cumulative fluxes", vec![layer(Source::TimeLevel, "t", "sum_infil"), layer(Source::TimeLevel, "t", "sum_evap"), layer(Source::TimeLevel, "t", "sum_vBot")]);
                d.legend = LegendPos::Right;
                Figure { name: "Overview".into(), rows: 2, cols: 2, height_mm: 150.0, palette: Palette::Viridis, panels: vec![a, b, c, d], ..Default::default() }
            },
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn templates_make_consistent_figures() {
        let p = rusteau_core::examples::infiltration_evaporation();
        let c = TemplateCtx { prj: &p, runs: vec!["A".into()], has_fit: true, has_measured: true };
        for t in all() {
            let f = (t.make)(&c);
            assert_eq!(f.panels.len(), f.cells(), "{}", t.name);
            assert!(!t.desc.is_empty());
        }
    }

    /// Every template that fits a project shows data in every panel once the project has been run.
    #[test]
    fn templates_show_the_results() {
        use super::super::data::{DataCtx, SavedRun};
        for p in [rusteau_core::examples::infiltration_evaporation(), rusteau_core::examples::solute_pulse()] {
            let mut sim = rusteau_core::Simulation::new(p.clone()).unwrap();
            sim.run(|_| true);
            let r = std::mem::take(&mut sim.res);
            let runs = vec![SavedRun { name: "A".into(), project: p.clone(), results: r.clone() }];
            let c = TemplateCtx { prj: &p, runs: vec!["A".into()], has_fit: false, has_measured: false };
            let dc = DataCtx { prj: &p, results: Some(&r), runs: &runs, measured: &[], fit: None };
            for t in all().iter().filter(|t| (t.fits)(&c) && t.name != "Empty figure") {
                let f = (t.make)(&c);
                for (k, panel) in f.panels.iter().enumerate() {
                    let n: usize = panel.layers.iter().map(|l| dc.series(l, &f).iter().filter(|s| s.x.iter().any(|v| v.is_finite())).count()).sum();
                    assert!(n > 0, "{} ({}): panel {} has no data", t.name, p.title, k + 1);
                }
                let r = super::super::render::render(&f, &f.panels.iter().map(|p| p.layers.iter().map(|l| dc.series(l, &f)).collect()).collect::<Vec<_>>());
                assert_eq!(r.panels.len(), f.panels.len());
            }
        }
    }
}
