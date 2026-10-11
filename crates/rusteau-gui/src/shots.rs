//! Screenshot tour for the help (a tool for developers): with the environment variable `RUSTEAU_SCREENSHOTS=<dir>`
//! the program loads the built-in examples, runs them, shows every page that the help illustrates and saves a PNG of
//! the window for each into `<dir>` (the names of `help/img`), then quits. Run it again after changing the interface:
//!
//! ```sh
//! RUSTEAU_SCREENSHOTS=crates/rusteau-gui/help/img cargo run --release -p rusteau-gui
//! ```
//!
//! With `RUSTEAU_SCREENSHOTS_TOPICS=1` the tour also shows every help topic (`topic_<id>.png`, for proof-reading the
//! formulas and tables; not used by the help).

use crate::{App, Page};
use eframe::egui;
use rusteau_core::{examples, MeteoRecord, MeteoSettings};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};

/// The tour is running: input helpers record the rectangles of their labels (to place the mouse for a tooltip).
pub static ACTIVE: AtomicBool = AtomicBool::new(false);

/// Records the rectangle of the label `label` while the tour runs.
pub fn note(ctx: &egui::Context, label: &str, rect: egui::Rect) {
    if ACTIVE.load(Ordering::Relaxed) {
        ctx.data_mut(|d| d.insert_temp(egui::Id::new(("tour_rect", label)), rect));
    }
}

type Setup = fn(&mut App, &egui::Context);

struct Step {
    name: String,
    setup: Setup,
    /// the screenshot waits until this holds (e.g. a run has finished)
    ready: fn(&App) -> bool,
    /// label to rest the mouse on (tooltip)
    hover: Option<&'static str>,
    /// help topic to show (after `setup`)
    topic: Option<&'static str>,
}

enum Phase {
    Setup,
    Wait(u32),
    Hover(u32),
    Requested(u32),
}

pub struct Tour {
    dir: PathBuf,
    steps: Vec<Step>,
    k: usize,
    phase: Phase,
}

fn always(_: &App) -> bool {
    true
}

fn has_results(a: &App) -> bool {
    a.run.is_none() && a.results.is_some()
}

fn has_fit(a: &App) -> bool {
    a.inv.run.is_none() && a.inv.result.is_some()
}

fn page(a: &mut App, p: Page) {
    a.ros = None;
    a.help.open = false;
    a.page = p;
}

/// Meteorological data for the screenshot of the meteo page: ten summer days.
fn meteo_example(a: &mut App) {
    let records = (1..=10)
        .map(|d| MeteoRecord {
            t: d as f64,
            rad: 18.0 + 3.0 * ((d as f64) * 0.9).sin(),
            t_max: 24.0 + 2.0 * ((d as f64) * 0.7).cos(),
            t_min: 12.0 + 1.5 * ((d as f64) * 0.5).sin(),
            rh_mean: 62.0,
            wind_kmd: 160.0,
            sun_hours: 9.0,
            ..Default::default()
        })
        .collect();
    a.prj.atmosphere.meteo = Some(MeteoSettings { records, ..Default::default() });
}

impl Tour {
    /// The tour of `RUSTEAU_SCREENSHOTS`, if set.
    pub fn from_env() -> Option<Tour> {
        let dir = PathBuf::from(std::env::var_os("RUSTEAU_SCREENSHOTS")?);
        std::fs::create_dir_all(&dir).ok()?;
        ACTIVE.store(true, Ordering::Relaxed);
        let s = |name: &str, setup: Setup| Step { name: name.into(), setup, ready: always, hover: None, topic: None };
        let mut steps = vec![
            Step {
                name: "main_window".into(),
                setup: |a, _| {
                    a.set_project(examples::infiltration_evaporation(), None);
                    a.start_run();
                    page(a, Page::ProfileInfo);
                    a.post.prof_var = 1;
                },
                ready: has_results,
                hover: None,
                topic: None,
            },
            s("processes", |a, _| page(a, Page::Main)),
            s("time_output", |a, _| page(a, Page::Time)),
            s("hyd_model", |a, _| page(a, Page::HydModel)),
            s("soil_params", |a, _| {
                page(a, Page::SoilParams);
                a.sel_mat = 0;
            }),
            s("rosetta", |a, _| {
                page(a, Page::SoilParams);
                a.ros = Some(crate::rosetta_ui::RosettaDlg::new(crate::rosetta_ui::RosettaTarget::Material(0)));
            }),
            s("profile_editor", |a, _| {
                page(a, Page::Profile);
                a.pe.table = false;
                a.pe.qty = crate::profile::Qty::Init;
            }),
            s("profile_table", |a, _| {
                page(a, Page::Profile);
                a.pe.table = true;
            }),
            s("water_bc", |a, _| {
                a.pe.table = false;
                page(a, Page::WaterBc);
            }),
            s("time_variable_bc", |a, _| page(a, Page::Atmosphere)),
            s("root_uptake_page", |a, _| page(a, Page::RootUptake)),
            s("iteration", |a, _| page(a, Page::Iteration)),
            s("further_options", |a, _| page(a, Page::Options)),
            s("check_run", |a, _| page(a, Page::Summary)),
            s("res_obs", |a, _| page(a, Page::Obs)),
            s("res_profiles", |a, _| {
                page(a, Page::ProfileInfo);
                a.post.prof_var = 0;
            }),
            s("res_fluxes", |a, _| {
                page(a, Page::Fluxes);
                a.post.flux_var = 1;
            }),
            s("res_soilprops", |a, _| {
                page(a, Page::SoilProps);
                a.post.prop_mats = vec![true, true];
            }),
            s("res_runtime", |a, _| page(a, Page::RunTime)),
            s("res_balance", |a, _| page(a, Page::MassBalance)),
            Step { name: "tooltip".into(), setup: |a, _| page(a, Page::Time), ready: always, hover: Some("Maximum time step"), topic: None },
            s("help_window", |a, _| {
                page(a, Page::SoilParams);
                a.help.show_topic("th_hydraulic_models", None);
            }),
            s("meteo_page", |a, _| {
                meteo_example(a);
                a.sync();
                page(a, Page::Meteo);
            }),
            s("heat_page", |a, _| {
                a.set_project(examples::inverse_heat(), None);
                page(a, Page::Heat);
            }),
            s("solute_general", |a, _| {
                a.set_project(examples::solute_pulse(), None);
                page(a, Page::SoluteGeneral);
            }),
            s("solute_params", |a, _| page(a, Page::SoluteMaterials)),
            s("solute_reactions", |a, _| page(a, Page::SoluteReactions)),
            s("solute_bc", |a, _| page(a, Page::SoluteBc)),
            s("inverse_page", |a, _| {
                a.set_project(examples::inverse_outflow(), None);
                page(a, Page::Inverse);
            }),
            Step {
                name: "res_inverse".into(),
                setup: |a, _| {
                    a.start_run();
                    page(a, Page::InverseResults);
                },
                ready: has_fit,
                hover: None,
                topic: None,
            },
        ];
        if std::env::var_os("RUSTEAU_SCREENSHOTS_TOPICS").is_some() {
            for id in crate::help::topics::order() {
                steps.push(Step { name: format!("topic_{id}"), setup: |a, _| page(a, Page::Main), ready: always, hover: None, topic: Some(id) });
            }
        }
        Some(Tour { dir, steps, k: 0, phase: Phase::Setup })
    }

    /// One frame of the tour; returns false when it has ended.
    pub fn frame(&mut self, app: &mut App, ctx: &egui::Context) -> bool {
        ctx.request_repaint();
        let Some(step) = self.steps.get(self.k) else {
            app.dirty = false;
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            return false;
        };
        self.phase = match self.phase {
            Phase::Setup => {
                // park the mouse in the status bar, where it highlights nothing
                let r = ctx.screen_rect();
                ctx.send_viewport_cmd(egui::ViewportCommand::CursorPosition(egui::pos2(r.right() - 8.0, r.bottom() - 6.0)));
                (step.setup)(app, ctx);
                if let Some(id) = step.topic {
                    app.help.show_topic(id, None);
                }
                Phase::Wait(0)
            }
            Phase::Wait(n) => {
                // a few frames for the layout (plots, scroll areas, windows) to settle
                if n >= 12 && (step.ready)(app) {
                    match step.hover {
                        Some(_) => Phase::Hover(0),
                        None => {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
                            Phase::Requested(0)
                        }
                    }
                } else {
                    Phase::Wait(n + 1)
                }
            }
            Phase::Hover(n) => {
                let label = step.hover.unwrap_or("");
                if let Some(r) = ctx.data(|d| d.get_temp::<egui::Rect>(egui::Id::new(("tour_rect", label)))) {
                    if n < 3 {
                        ctx.send_viewport_cmd(egui::ViewportCommand::CursorPosition(r.center()));
                    }
                }
                std::thread::sleep(std::time::Duration::from_millis(20));
                if n >= 60 {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
                    Phase::Requested(0)
                } else {
                    Phase::Hover(n + 1)
                }
            }
            Phase::Requested(n) => {
                let shot = ctx.input(|i| {
                    i.raw.events.iter().find_map(|e| match e {
                        egui::Event::Screenshot { image, .. } => Some(image.clone()),
                        _ => None,
                    })
                });
                match shot {
                    Some(img) => {
                        let path = self.dir.join(format!("{}.png", step.name));
                        let [w, h] = img.size;
                        let rgb: Vec<u8> = img.pixels.iter().flat_map(|c| [c.r(), c.g(), c.b()]).collect();
                        match image::save_buffer(&path, &rgb, w as u32, h as u32, image::ExtendedColorType::Rgb8) {
                            Ok(()) => println!("screenshot {}", path.display()),
                            Err(e) => eprintln!("could not write {}: {e}", path.display()),
                        }
                        self.k += 1;
                        Phase::Setup
                    }
                    None if n > 600 => {
                        eprintln!("no screenshot for {}", step.name);
                        self.k += 1;
                        Phase::Setup
                    }
                    None => Phase::Requested(n + 1),
                }
            }
        };
        true
    }
}
