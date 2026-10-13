//! Smoke test of the interface: every page, every help topic and every figure template is drawn for every example,
//! before and after a run, without a window. Internal errors of the drawing are caught by the interface (see
//! [`crate::guard`]) and recorded; the test fails on any of them.

use crate::figure::data::SavedRun;
use crate::figure::templates::{self, TemplateCtx};
use crate::{App, Page};
use eframe::egui;

const PAGES: [Page; 26] = [
    Page::Main,
    Page::Time,
    Page::Iteration,
    Page::Options,
    Page::HydModel,
    Page::SoilParams,
    Page::WaterBc,
    Page::Atmosphere,
    Page::Meteo,
    Page::RootUptake,
    Page::SoluteGeneral,
    Page::SoluteMaterials,
    Page::SoluteReactions,
    Page::SoluteAdvanced,
    Page::SoluteBc,
    Page::Heat,
    Page::Profile,
    Page::Summary,
    Page::Obs,
    Page::ProfileInfo,
    Page::Fluxes,
    Page::SoilProps,
    Page::RunTime,
    Page::MassBalance,
    Page::Inverse,
    Page::InverseResults,
];

fn context() -> egui::Context {
    let ctx = egui::Context::default();
    let mut fonts = egui::FontDefinitions::default();
    crate::fonts::install(&mut fonts);
    ctx.set_fonts(fonts);
    // the figure window inside the main window (no second window without a display)
    ctx.set_embed_viewports(true);
    ctx
}

fn frame(ctx: &egui::Context, app: &mut App) {
    let input = egui::RawInput { screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1280.0, 800.0))), ..Default::default() };
    let _ = ctx.run(input, |ctx| app.frame(ctx));
}

/// Draws all pages, the figure window with every template, and checks that nothing failed.
fn draw_everything(ctx: &egui::Context, app: &mut App, what: &str) {
    for page in PAGES {
        app.page = page;
        frame(ctx, app);
        frame(ctx, app);
        assert!(app.broken_page.is_none(), "{what}, page {page:?}: {:?}", app.broken_page);
    }
    let runs: Vec<String> = app.figs.runs.iter().map(|r| r.name.clone()).collect();
    let tc = TemplateCtx { prj: &app.prj, runs, has_fit: app.inv.result.is_some(), has_measured: !app.measured.is_empty() };
    let figs: Vec<_> = templates::all().iter().map(|t| (t.make)(&tc)).collect();
    app.prj.figures = figs;
    app.figs.open = true;
    for k in 0..app.prj.figures.len() {
        app.figs.fig = k;
        for panel in 0..app.prj.figures[k].panels.len() {
            app.figs.panel = panel;
            frame(ctx, app);
            frame(ctx, app);
        }
        assert!(app.figs_failure().is_none(), "{what}, figure {:?}: {:?}", app.prj.figures[k].name, app.figs_failure());
    }
    app.figs.open = false;
    app.prj.figures.clear();
}

/// The examples, an empty project and variants with the other hydraulic models, hysteresis, two solutes with heat
/// transport and meteorological data.
fn projects() -> Vec<(String, rusteau_core::Project)> {
    use rusteau_core::{examples, Hysteresis, MeteoRecord, MeteoSettings, Project, SoilModel};
    let mut v: Vec<(String, Project)> = examples::all().into_iter().map(|(n, f)| (n.to_string(), f())).collect();
    v.push(("empty project".into(), Project::default()));
    for model in [
        SoilModel::ModifiedVG,
        SoilModel::BrooksCorey,
        SoilModel::VGAirEntry,
        SoilModel::Kosugi,
        SoilModel::Durner,
        SoilModel::DualPorosityW,
        SoilModel::DualPorosityH,
        SoilModel::DualPermeability,
        SoilModel::Tabular,
    ] {
        let mut p = examples::infiltration_evaporation();
        p.water.model = model;
        v.push((format!("model {model:?}"), p.clone()));
        // the same with valid parameters of the second region (the defaults are those of the modified van Genuchten)
        let extra = match model {
            SoilModel::DualPorosityW => Some([0.02, 0.15, 0.01, 0.0, 0.0]),
            SoilModel::DualPorosityH => Some([0.02, 0.15, 0.03, 1.5, 0.01]),
            SoilModel::DualPermeability => Some([0.05, 0.4, 0.02, 1.4, 1.0]),
            _ => None,
        };
        if let Some(extra) = extra {
            for m in &mut p.water.materials {
                m.extra = extra;
            }
            v.push((format!("model {model:?} with its parameters"), p));
        }
    }
    for h in [Hysteresis::Retention, Hysteresis::RetentionAndConductivity, Hysteresis::Lenhard] {
        let mut p = examples::infiltration_evaporation();
        p.water.hysteresis = h;
        v.push((format!("hysteresis {h:?}"), p));
    }
    let mut p = examples::solute_pulse();
    p.processes.heat = true;
    if let Some(s) = p.solute.species.first().cloned() {
        p.solute.species.push(s);
    }
    v.push(("two solutes with heat".into(), p));
    let mut p = examples::infiltration_evaporation();
    let records = (1..=10).map(|d| MeteoRecord { t: d as f64, rad: 18.0, t_max: 24.0, t_min: 12.0, rh_mean: 62.0, wind_kmd: 160.0, sun_hours: 9.0, ..Default::default() }).collect();
    p.atmosphere.meteo = Some(MeteoSettings { records, ..Default::default() });
    v.push(("meteorological data".into(), p));
    v
}

#[test]
fn every_page_figure_and_help_topic_draws() {
    let ctx = context();
    for (name, p) in projects() {
        let mut app = App::with_project(rusteau_core::examples::infiltration_evaporation());
        let inverse = p.inverse.clone();
        app.set_project(p, None);
        draw_everything(&ctx, &mut app, &format!("{name} (no results)"));
        // drawing the pages keeps the inverse settings
        assert_eq!(app.prj.inverse.is_some(), inverse.is_some(), "{name}");
        // a direct run as the interface starts it (only a valid project; the inverse examples too; at most a few
        // seconds) and a kept run for the comparisons
        let mut p = app.prj.clone();
        p.processes.inverse = false;
        if !p.validate().is_empty() {
            continue;
        }
        let Ok(mut sim) = rusteau_core::Simulation::new(p) else { continue };
        let start = std::time::Instant::now();
        sim.run(|_| start.elapsed() < std::time::Duration::from_secs(5));
        let r = std::mem::take(&mut sim.res);
        app.figs.runs = vec![SavedRun { name: "kept".into(), project: app.prj.clone(), results: r.clone() }];
        app.results = Some(r);
        draw_everything(&ctx, &mut app, &format!("{name} (with results)"));
    }
    // every help topic in the help window
    let mut app = App::with_project(rusteau_core::examples::infiltration_evaporation());
    for id in crate::help::topics::order() {
        app.help.show_topic(id, None);
        frame(&ctx, &mut app);
    }
}
