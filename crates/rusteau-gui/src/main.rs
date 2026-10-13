#![cfg_attr(all(not(debug_assertions), windows), windows_subsystem = "windows")]

mod figure;
mod files;
mod fonts;
mod help;
mod hydro;
mod inverse_ui;
mod measured;
mod post;
mod pre;
mod profile;
mod rosetta_ui;
mod shots;
mod tips;
mod util;

use eframe::egui;
use files::{DialogResult, FileDialog, Mode, Purpose};
use tips::TipExt;
use rusteau_core::{Project, Results, Simulation};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{channel, Receiver};
use std::sync::Arc;
use std::time::Instant;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Page {
    Main,
    Time,
    Iteration,
    Options,
    HydModel,
    SoilParams,
    WaterBc,
    Atmosphere,
    Meteo,
    RootUptake,
    SoluteGeneral,
    SoluteMaterials,
    SoluteReactions,
    SoluteAdvanced,
    SoluteBc,
    Heat,
    Profile,
    Summary,
    Obs,
    ProfileInfo,
    Fluxes,
    SoilProps,
    RunTime,
    MassBalance,
    Inverse,
    InverseResults,
}

struct RunHandle {
    rx: Receiver<Result<Results, String>>,
    progress: Arc<AtomicU64>,
    cancel: Arc<AtomicBool>,
    t_init: f64,
    t_max: f64,
    started: Instant,
}



pub struct PostState {
    pub obs_var: usize,
    pub obs_sel: Vec<bool>,
    /// observation node (index) a loaded measured time series belongs to
    pub obs_meas: usize,
    pub prof_var: usize,
    pub prof_sel: Vec<bool>,
    pub flux_var: usize,
    pub solute: usize,
    pub run_var: usize,
    pub prop_mat: usize,
    pub prop_kind: usize,
    /// profile page: one time with a slider instead of several selected times
    pub prof_single: bool,
    pub prof_idx: usize,
    pub prof_play: bool,
    pub prof_tick: f64,
    /// soil hydraulic property viewer
    pub prop_mats: Vec<bool>,
    pub prop_log_x: bool,
    pub prop_log_y: bool,
    pub prop_wetting: bool,
}
impl Default for PostState {
    fn default() -> Self {
        PostState {
            obs_var: 0,
            obs_sel: vec![],
            obs_meas: 0,
            prof_var: 0,
            prof_sel: vec![],
            flux_var: 0,
            solute: 0,
            run_var: 0,
            prop_mat: 0,
            prop_kind: 0,
            prof_single: false,
            prof_idx: 0,
            prof_play: false,
            prof_tick: 0.0,
            prop_mats: vec![],
            prop_log_x: true,
            prop_log_y: true,
            prop_wetting: true,
        }
    }
}

fn load_system_font(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();

    let candidate_paths: &[&str] = &[
        // Linux / Ubuntu / GNOME
        "/usr/share/fonts/truetype/ubuntu/Ubuntu-R.ttf",
        "/usr/share/fonts/opentype/cantarell/Cantarell-Regular.otf",
        "/usr/share/fonts/truetype/noto/NotoSans-Regular.ttf",
        "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
        "/usr/share/fonts/TTF/DejaVuSans.ttf",
        // Windows
        "C:\\Windows\\Fonts\\segoeui.ttf",
        "C:\\Windows\\Fonts\\arial.ttf",
        // macOS
        "/System/Library/Fonts/SFProText-Regular.otf",
        "/System/Library/Fonts/HelveticaNeue.ttc",
        "/Library/Fonts/Arial.ttf",
    ];

    let mut found = false;
    for path in candidate_paths {
        if let Ok(data) = std::fs::read(path) {
            fonts.font_data.insert(
                "os_default".to_owned(),
                std::sync::Arc::new(egui::FontData::from_owned(data)),
            );

            if let Some(family) = fonts.families.get_mut(&egui::FontFamily::Proportional) {
                family.insert(0, "os_default".to_owned());
            }
            found = true;
            break;
        }
    }
    // symbols the system font lacks come from the bundled Noto fonts
    fonts::add_fallbacks(&mut fonts, found);

    ctx.set_fonts(fonts);
}

pub fn apply_custom_dark_theme(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::dark();
    visuals.panel_fill = egui::Color32::from_rgb(26, 28, 32);
    visuals.window_fill = egui::Color32::from_rgb(32, 34, 39);
    visuals.faint_bg_color = egui::Color32::from_rgb(38, 41, 48);
    visuals.extreme_bg_color = egui::Color32::from_rgb(18, 19, 22);

    visuals.window_corner_radius = egui::CornerRadius::same(8);
    visuals.widgets.noninteractive.corner_radius = egui::CornerRadius::same(4);
    visuals.widgets.inactive.corner_radius = egui::CornerRadius::same(5);
    visuals.widgets.hovered.corner_radius = egui::CornerRadius::same(5);
    visuals.widgets.active.corner_radius = egui::CornerRadius::same(5);

    visuals.widgets.noninteractive.bg_stroke =
        egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(45, 48, 56));
    visuals.widgets.inactive.bg_stroke =
        egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(52, 56, 65));
    visuals.widgets.hovered.bg_stroke =
        egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(80, 140, 220));
    // selected labels / tabs: white text on the selection colour
    visuals.selection.stroke = egui::Stroke::new(1.0_f32, egui::Color32::WHITE);

    ctx.set_visuals(visuals);
}

pub fn apply_custom_light_theme(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::light();
    visuals.panel_fill = egui::Color32::from_rgb(245, 246, 248);
    visuals.window_fill = egui::Color32::from_rgb(255, 255, 255);
    visuals.faint_bg_color = egui::Color32::from_rgb(238, 240, 244);
    visuals.extreme_bg_color = egui::Color32::from_rgb(255, 255, 255);

    visuals.window_corner_radius = egui::CornerRadius::same(8);
    visuals.widgets.noninteractive.corner_radius = egui::CornerRadius::same(4);
    visuals.widgets.inactive.corner_radius = egui::CornerRadius::same(5);
    visuals.widgets.hovered.corner_radius = egui::CornerRadius::same(5);
    visuals.widgets.active.corner_radius = egui::CornerRadius::same(5);

    visuals.widgets.noninteractive.bg_stroke =
        egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(215, 220, 228));
    visuals.widgets.inactive.bg_stroke =
        egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(200, 205, 215));
    visuals.widgets.hovered.bg_stroke =
        egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(60, 120, 210));

    ctx.set_visuals(visuals);
}

pub struct App {
    pub prj: Project,
    pub path: Option<PathBuf>,
    pub dirty: bool,
    pub page: Page,
    pub results: Option<Results>,
    run: Option<RunHandle>,
    /// parameter estimation (run, result)
    pub inv: inverse_ui::InvState,
    /// waiting for clipboard text for the observation table
    pub inv_paste: bool,
    pub status: String,
    pub dialog: Option<FileDialog>,
    pub problems: Vec<String>,
    pub sel_mat: usize,
    /// soil profile editor state
    pub pe: profile::ProfEdit,
    pub post: PostState,
    pub print_text: String,
    pub print_text_for: Vec<f64>,
    pub show_about: bool,
    pub last_dir: Option<PathBuf>,
    pub run_seconds: f64,
    pub first_frame: bool,
    pub show_close_confirm: bool,
    pub pending_action: Option<PendingAction>,
    /// measured data shown with the results
    pub measured: Vec<measured::Measured>,
    pub measured_target: Option<measured::Target>,
    /// convert all values when a unit changes (otherwise only the unit label changes)
    pub convert_units: bool,
    /// Save also writes a HYDRUS-1D project (`<name>.h1d` + folder `<name>`) next to the .h1dr
    pub legacy_on_save: bool,
    /// Rosetta dialog
    pub ros: Option<rosetta_ui::RosettaDlg>,
    /// Help window
    pub help: help::HelpState,
    /// Screenshot tour for the help (`RUSTEAU_SCREENSHOTS`)
    tour: Option<shots::Tour>,
    /// figure editor (its own window)
    pub figs: figure::ui::FigState,
}

#[derive(Clone, Debug)]
pub enum PendingAction {
    Quit,
    NewProject,
    OpenProject,
    /// open this project (dropped onto the window)
    OpenPath(PathBuf),
}

impl App {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        load_system_font(&cc.egui_ctx);

        #[cfg(target_os = "linux")]
        {
            if cc.egui_ctx.pixels_per_point() < 1.1 {
                cc.egui_ctx.set_pixels_per_point(1.15);
            }
        }

        apply_custom_dark_theme(&cc.egui_ctx);

        let mut style = (*cc.egui_ctx.style()).clone();
        style.spacing.item_spacing = egui::vec2(10.0, 8.0);
        style.spacing.button_padding = egui::vec2(10.0, 6.0);
        style.spacing.window_margin = egui::Margin::same(12);
        cc.egui_ctx.set_style(style);

        let prj = rusteau_core::examples::infiltration_evaporation();
        let mut app = App {
            prj,
            path: None,
            dirty: false,
            page: Page::Main,
            results: None,
            run: None,
            inv: Default::default(),
            inv_paste: false,
            status: "Ready. Load an example, open a project (or drag a .h1dr, .h1d or HYDRUS-1D project folder onto the window), or edit the current project.".into(),
            dialog: None,
            problems: vec![],
            sel_mat: 0,
            pe: profile::ProfEdit::default(),
            post: PostState::default(),
            print_text: String::new(),
            print_text_for: vec![],
            show_about: false,
            last_dir: None,
            run_seconds: 0.0,
            first_frame: true,
            show_close_confirm: false,
            pending_action: None,
            measured: vec![],
            measured_target: None,
            convert_units: true,
            legacy_on_save: false,
            ros: None,
            help: help::HelpState::default(),
            tour: shots::Tour::from_env(),
            figs: Default::default(),
        };
        if let Some(arg) = std::env::args().nth(1) {
            app.open_path(&PathBuf::from(&arg));
        }
        app
    }

    /// Opens a project: a Rusteau project (`.h1dr`, `.json`) or a HYDRUS-1D project (the `.h1d` file, its folder or any
    /// file in the folder; as File → Import, a Fit.in in the folder gives measured data).
    fn open_path(&mut self, path: &std::path::Path) {
        self.last_dir = if path.is_dir() { Some(path.to_path_buf()) } else { path.parent().map(|d| d.to_path_buf()) };
        if rusteau_io::is_legacy_project_path(path) {
            self.import_legacy(path);
            return;
        }
        match rusteau_io::load_project(path) {
            Ok(prj) => {
                self.set_project(prj, Some(path.to_path_buf()));
                self.status = format!("Opened {}", path.display());
            }
            Err(e) => self.status = format!("Could not open {}: {}", path.display(), e),
        }
    }

    /// Files and folders dragged onto the window: the first project among them is opened (after asking about unsaved
    /// changes).
    fn handle_dropped_files(&mut self, ctx: &egui::Context) {
        let dropped: Vec<PathBuf> = ctx.input(|i| i.raw.dropped_files.iter().filter_map(|f| f.path.clone()).collect());
        if dropped.is_empty() {
            return;
        }
        let Some(path) = dropped.iter().find(|p| is_project_path(p)).cloned() else {
            let names: Vec<String> = dropped.iter().map(|p| p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default()).collect();
            self.status = format!("Not a project: {} (drop a .h1dr, a .h1d or a HYDRUS-1D project folder)", names.join(", "));
            return;
        };
        if self.run.is_some() || self.inv.run.is_some() {
            self.status = "A calculation is running: stop it before opening another project.".into();
            return;
        }
        if self.dirty {
            self.pending_action = Some(PendingAction::OpenPath(path));
            self.show_close_confirm = true;
        } else {
            self.open_path(&path);
        }
    }

    /// While files are dragged over the window: a veil with what a drop would do.
    fn drop_overlay(&self, ctx: &egui::Context) {
        let hovered: Vec<egui::HoveredFile> = ctx.input(|i| i.raw.hovered_files.clone());
        if hovered.is_empty() {
            return;
        }
        // the path is not known on every platform while dragging
        let text = match hovered.iter().find_map(|f| f.path.clone()) {
            Some(p) if is_project_path(&p) => format!("Drop to open {}", p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default()),
            Some(p) => format!("{} is not a project\n(.h1dr, .h1d or a HYDRUS-1D project folder)", p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default()),
            None => "Drop to open the project".into(),
        };
        let painter = ctx.layer_painter(egui::LayerId::new(egui::Order::Foreground, egui::Id::new("drop_overlay")));
        let rect = ctx.screen_rect();
        painter.rect_filled(rect, 0.0, egui::Color32::from_black_alpha(170));
        painter.rect_stroke(rect.shrink(12.0), 12.0, egui::Stroke::new(3.0_f32, egui::Color32::from_rgb(90, 160, 230)), egui::StrokeKind::Inside);
        painter.text(rect.center(), egui::Align2::CENTER_CENTER, text, egui::FontId::proportional(24.0), egui::Color32::WHITE);
    }

    pub fn set_project(&mut self, p: Project, path: Option<PathBuf>) {
        self.prj = p;
        self.path = path;
        self.dirty = false;
        self.results = None;
        self.sel_mat = 0;
        self.problems.clear();
        self.print_text_for.clear();
        self.pe.reset();
        self.post = PostState::default();
        self.measured.clear();
        self.inv.result = None;
        self.inv.settings = None;
        self.page = Page::Main;
        self.figs.project_changed();
        self.sync();
    }

    /// Measured data from the observations of a Fit.in (shown with the results).
    fn load_fit_measurements(&mut self, path: &std::path::Path) -> Result<usize, String> {
        let settings = rusteau_io::read_fit_in(path).map_err(|e| e.to_string())?;
        let sets = measured::from_fit(&settings);
        self.measured.retain(|m| !m.name.starts_with("Fit.in"));
        let n = sets.iter().map(|m| m.pts.len()).sum();
        self.measured.extend(sets);
        Ok(n)
    }

    fn start_run(&mut self) {
        if self.prj.processes.inverse {
            self.start_inverse();
            return;
        }
        self.problems = self.prj.validate();
        if !self.problems.is_empty() {
            self.status = "The project has problems – see the Summary page.".into();
            self.page = Page::Summary;
            return;
        }
        let prj = self.prj.clone();
        let (tx, rx) = channel();
        let progress = Arc::new(AtomicU64::new(prj.time.t_init.to_bits()));
        let cancel = Arc::new(AtomicBool::new(false));
        let (p2, c2) = (progress.clone(), cancel.clone());
        let (t_init, t_max) = (prj.time.t_init, prj.time.t_max);
        std::thread::spawn(move || {
            let res = match Simulation::new(prj) {
                Ok(mut sim) => {
                    sim.run(|s| {
                        p2.store(s.t.to_bits(), Ordering::Relaxed);
                        !c2.load(Ordering::Relaxed)
                    });
                    Ok(std::mem::take(&mut sim.res))
                }
                Err(e) => Err(e.to_string()),
            };
            let _ = tx.send(res);
        });
        self.run = Some(RunHandle {
            rx,
            progress,
            cancel,
            t_init,
            t_max,
            started: Instant::now(),
        });
        self.status = "Running…".into();
    }

    fn poll_run(&mut self, ctx: &egui::Context) {
        let mut finished = None;
        if let Some(r) = &self.run {
            ctx.request_repaint_after(std::time::Duration::from_millis(80));
            if let Ok(res) = r.rx.try_recv() {
                finished = Some((
                    res,
                    r.started.elapsed().as_secs_f64(),
                    r.cancel.load(Ordering::Relaxed),
                ));
            }
        }
        if let Some((res, secs, cancelled)) = finished {
            self.run = None;
            self.run_seconds = secs;
            match res {
                Ok(r) => {
                    let msg = if r.failed {
                        format!("Calculation stopped: {}", r.messages.join(" "))
                    } else if cancelled {
                        "Calculation cancelled (partial results).".to_string()
                    } else {
                        format!(
                            "Calculation finished in {:.2} s – {} time levels.",
                            secs,
                            r.tlevel.len()
                        )
                    };
                    self.status = msg;
                    self.post.obs_sel.clear();
                    self.post.prof_sel.clear();
                    self.results = Some(r);
                    if matches!(self.page, Page::Main | Page::Summary) {
                        self.page = Page::ProfileInfo;
                    }
                }
                Err(e) => self.status = format!("Error: {}", e),
            }
        }
    }

    fn request_new_project(&mut self) {
        if self.dirty {
            self.pending_action = Some(PendingAction::NewProject);
            self.show_close_confirm = true;
        } else {
            self.do_new_project();
        }
    }

    fn do_new_project(&mut self) {
        let mut p = Project::default();
        p.profile = rusteau_core::examples::infiltration_evaporation().profile;
        for n in p.profile.nodes.iter_mut() {
            n.beta = 0.0;
            n.mat = 1;
            n.layer = 1;
            n.h = -100.0;
        }
        self.set_project(p, None);
    }

    /// Import a HYDRUS-1D project: its folder, the 4.x project file `<name>.h1d` (the data are in the sibling folder
    /// `<name>`) or a file inside the folder.  The imported project has no file name yet (Save asks for a .h1dr).
    fn import_legacy(&mut self, path: &std::path::Path) {
        let r = rusteau_io::legacy_project_dir(path).and_then(|dir| rusteau_io::read_legacy_project(&dir).map(|p| (dir, p)));
        match r {
            Ok((dir, mut p)) => {
                if p.title.trim().is_empty() {
                    p.title = dir.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
                }
                self.set_project(p, None);
                self.dirty = true;
                self.last_dir = Some(dir.clone());
                self.status = format!("Imported HYDRUS-1D project from {}", dir.display());
                // observations of an inverse problem are shown with the results
                let fit = std::fs::read_dir(&dir).ok().and_then(|rd| rd.flatten().map(|e| e.path()).find(|p| p.file_name().is_some_and(|n| n.to_string_lossy().eq_ignore_ascii_case("fit.in"))));
                if let Some(fit) = fit {
                    if let Ok(n) = self.load_fit_measurements(&fit) {
                        self.status = format!("Imported HYDRUS-1D project from {} ({} measured values from Fit.in)", dir.display(), n);
                    }
                }
            }
            Err(e) => {
                self.status = format!("Import failed: {}", e);
                self.problems = vec![format!("Failed to import from {}: {}", path.display(), e)];
            }
        }
    }

    fn request_open_project(&mut self) {
        if self.dirty {
            self.pending_action = Some(PendingAction::OpenProject);
            self.show_close_confirm = true;
        } else {
            self.dialog = Some(FileDialog::new(
                Purpose::OpenProject,
                Mode::OpenFile,
                "Open project",
                self.last_dir.clone(),
                "",
                &["h1dr", "h1d", "json"],
            ));
        }
    }

    fn request_quit(&mut self, ctx: &egui::Context) {
        if self.dirty {
            self.pending_action = Some(PendingAction::Quit);
            self.show_close_confirm = true;
        } else {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }

    fn handle_shortcuts(&mut self, ctx: &egui::Context) {
        ctx.input(|i| {
            let ctrl = i.modifiers.command || i.modifiers.ctrl;
            if ctrl && !i.modifiers.shift && i.key_pressed(egui::Key::N) {
                self.request_new_project();
            }
            if ctrl && !i.modifiers.shift && i.key_pressed(egui::Key::O) {
                self.request_open_project();
            }
            if ctrl && i.modifiers.shift && i.key_pressed(egui::Key::S) {
                let name = self
                    .path
                    .as_ref()
                    .and_then(|p| p.file_name())
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_else(|| "project.h1dr".into());
                self.dialog = Some(FileDialog::new(
                    Purpose::SaveProject,
                    Mode::SaveFile,
                    "Save project",
                    self.last_dir.clone(),
                    &name,
                    &["h1dr"],
                ));
            } else if ctrl && !i.modifiers.shift && i.key_pressed(egui::Key::S) {
                if self.path.is_some() {
                    self.save();
                } else {
                    self.dialog = Some(FileDialog::new(
                        Purpose::SaveProject,
                        Mode::SaveFile,
                        "Save project",
                        self.last_dir.clone(),
                        "project.h1dr",
                        &["h1dr"],
                    ));
                }
            }
        });
    }

    fn menu(&mut self, ctx: &egui::Context) {
        let help_before = (self.help.open, self.help.current().to_string());
        self.menu_bar(ctx);
        // help asked for in the main window is shown there, also while the figure window shows it
        if (self.help.open, self.help.current().to_string()) != help_before {
            self.figs.help_here = false;
        }
    }

    fn menu_bar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("menu").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("New project   (Ctrl+N)").clicked() {
                        ui.close_menu();
                        self.request_new_project();
                    }
                    ui.menu_button("Examples", |ui| {
                        for (name, f) in rusteau_core::examples::all() {
                            if ui.button(name).clicked() {
                                self.set_project(f(), None);
                                ui.close_menu();
                            }
                        }
                    });
                    ui.separator();
                    if ui.button("Open project (.h1dr, HYDRUS-1D .h1d)…   (Ctrl+O)").clicked() {
                        ui.close_menu();
                        self.request_open_project();
                    }
                    if ui.button("Import HYDRUS-1D project folder…").clicked() {
                        self.dialog = Some(FileDialog::new(
                            Purpose::ImportLegacy,
                            Mode::PickDir,
                            "Select a HYDRUS-1D project folder (contains Selector.in)",
                            self.last_dir.clone(),
                            "",
                            &[],
                        ));
                        ui.close_menu();
                    }
					if ui.button("Import optimization file (Fit.in)…").clicked() {
                        self.dialog = Some(FileDialog::new(
                            Purpose::ImportFitIn,
                            Mode::OpenFile,
                            "Select optimization input file (Fit.in)",
                            self.last_dir.clone(),
                            "Fit.in",
                            &["in"],
                        ));
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui
                        .add_enabled(self.path.is_some(), egui::Button::new("Save   (Ctrl+S)"))
                        .clicked()
                    {
                        self.save();
                        ui.close_menu();
                    }
                    if ui.button("Save as…   (Ctrl+Shift+S)").clicked() {
                        let name = self
                            .path
                            .as_ref()
                            .and_then(|p| p.file_name())
                            .map(|s| s.to_string_lossy().to_string())
                            .unwrap_or_else(|| "project.h1dr".into());
                        self.dialog = Some(FileDialog::new(
                            Purpose::SaveProject,
                            Mode::SaveFile,
                            "Save project",
                            self.last_dir.clone(),
                            &name,
                            &["h1dr"],
                        ));
                        ui.close_menu();
                    }
                    if ui.button("Save as HYDRUS-1D project (.h1d + folder)…").clicked() {
                        let name = self
                            .path
                            .as_ref()
                            .and_then(|p| p.file_stem())
                            .map(|s| format!("{}.h1d", s.to_string_lossy()))
                            .unwrap_or_else(|| "project.h1d".into());
                        self.dialog = Some(FileDialog::new(
                            Purpose::SaveLegacy,
                            Mode::SaveFile,
                            "Save as HYDRUS-1D project (the folder <name> with the input files is created next to <name>.h1d)",
                            self.last_dir.clone(),
                            &name,
                            &["h1d"],
                        ));
                        ui.close_menu();
                    }
                    ui.checkbox(&mut self.legacy_on_save, "Save also writes a HYDRUS-1D project (.h1d)")
                        .on_hover_text("Every Save also writes <name>.h1d and the folder <name> with the HYDRUS-1D input files next to the .h1dr, so the project opens in HYDRUS-1D 4.x.");
                    ui.separator();
                    if ui
                        .add_enabled(
                            self.results.is_some(),
                            egui::Button::new("Export results (CSV)…"),
                        )
                        .clicked()
                    {
                        self.dialog = Some(FileDialog::new(
                            Purpose::ExportCsv,
                            Mode::PickDir,
                            "Choose a folder for the CSV files",
                            self.last_dir.clone(),
                            "",
                            &[],
                        ));
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("Quit").clicked() {
                        ui.close_menu();
                        self.request_quit(ctx);
                    }
                });
                ui.menu_button("Calculation", |ui| {
                    if ui
                        .add_enabled(self.run.is_none(), egui::Button::new("▶ Run   (F5)"))
                        .clicked()
                    {
                        self.start_run();
                        ui.close_menu();
                    }
                    if ui
                        .add_enabled(self.run.is_some() || self.inv.run.is_some(), egui::Button::new("■ Stop"))
                        .clicked()
                    {
                        if let Some(r) = &self.run {
                            r.cancel.store(true, Ordering::Relaxed);
                        }
                        if let Some(r) = &self.inv.run {
                            r.cancel.store(true, Ordering::Relaxed);
                        }
                        ui.close_menu();
                    }
                });
                ui.menu_button("View", |ui| {
                    if ui.button("Figures (new window)…").on_hover_text("Diagrams of the results, the input and measured data for publications: titles, annotations, several panels, comparison of runs; PNG and SVG").clicked() {
                        self.figs.open = true;
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("Dark theme").clicked() {
                        apply_custom_dark_theme(ctx);
                        ui.close_menu();
                    }
                    if ui.button("Light theme").clicked() {
                        apply_custom_light_theme(ctx);
                        ui.close_menu();
                    }
                });
                ui.menu_button("Help", |ui| {
                    if ui.button("Help on this page   (F1)").clicked() {
                        self.help.show_topic(help::topic_of(self.page), None);
                        ui.close_menu();
                    }
                    if ui.button("Contents").clicked() {
                        self.help.show_topic("contents", None);
                        ui.close_menu();
                    }
                    if ui.button("Getting started").clicked() {
                        self.help.show_topic("getting_started", None);
                        ui.close_menu();
                    }
                    if ui.button("Theory and equations").clicked() {
                        self.help.show_topic("th_richards", None);
                        ui.close_menu();
                    }
                    if ui.button("References (scientific sources)").clicked() {
                        self.help.show_topic("references", None);
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui
                        .button("Open the help in the web browser")
                        .on_hover_text("Writes the help as HTML pages (offline) and opens them in the web browser")
                        .clicked()
                    {
                        self.open_help_in_browser(ctx, help::topic_of(self.page));
                        ui.close_menu();
                    }
                    if ui.button("Export the HTML help…").on_hover_text("Writes the help as HTML pages into the folder \"Rusteau-1D help\" of a folder of your choice").clicked() {
                        self.dialog = Some(FileDialog::new(
                            Purpose::ExportHelp,
                            Mode::PickDir,
                            "Choose a folder for the HTML help (a folder \"Rusteau-1D help\" is created in it)",
                            self.last_dir.clone(),
                            "",
                            &[],
                        ));
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("Keyboard and mouse").clicked() {
                        self.help.show_topic("shortcuts", None);
                        ui.close_menu();
                    }
                    if ui.button("About Rusteau-1D").clicked() {
                        self.show_about = true;
                        ui.close_menu();
                    }
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let running = self.run.is_some() || self.inv.run.is_some();
                    if running {
                        if ui.button("■ Stop").clicked() {
                            if let Some(r) = &self.run {
                                r.cancel.store(true, Ordering::Relaxed);
                            }
                            if let Some(r) = &self.inv.run {
                                r.cancel.store(true, Ordering::Relaxed);
                            }
                        }
                    } else if ui.button(if self.prj.processes.inverse { "▶ Run (inverse)" } else { "▶ Run" }).clicked() {
                        self.start_run();
                    }
                    let title = format!("{}{}", self.prj.title, if self.dirty { " *" } else { "" });
                    ui.label(egui::RichText::new(title).weak());
                });
            });
        });
        if ctx.input(|i| i.key_pressed(egui::Key::F5)) && self.run.is_none() && self.inv.run.is_none() {
            self.start_run();
        }
        if ctx.input(|i| i.key_pressed(egui::Key::F1)) {
            let id = if self.ros.is_some() { "rosetta" } else { help::topic_of(self.page) };
            self.help.show_topic(id, None);
        }
    }

    /// Writes the HTML help into the cache folder and opens topic `id` in the web browser.
    fn open_help_in_browser(&mut self, ctx: &egui::Context, id: &str) {
        match help::html_url(id) {
            Ok(url) => {
                ctx.open_url(egui::OpenUrl::new_tab(&url));
                self.status = format!("Help opened in the web browser ({url})");
            }
            Err(e) => self.status = e,
        }
    }

    /// The help window and the help requests of the pages ("?" next to a page title, help buttons of dialogs).
    fn help_ui(&mut self, ctx: &egui::Context) {
        match help::take_request(ctx) {
            Some(None) => {
                self.figs.help_here = false;
                self.help.show_topic(help::topic_of(self.page), None);
            }
            Some(Some(id)) => {
                self.figs.help_here = false;
                self.help.show_topic(id, None);
            }
            None => {}
        }
        // the figure window shows the help itself when it was asked for there
        if self.figs.open && self.figs.help_here {
            return;
        }
        if let Some(help::HelpEvent::OpenInBrowser(id)) = self.help.window(ctx) {
            self.open_help_in_browser(ctx, &id);
        }
    }

    fn save(&mut self) {
        if let Some(p) = self.path.clone() {
            match rusteau_io::save_project(&self.prj, &p) {
                Ok(_) => {
                    self.dirty = false;
                    self.status = format!("Saved {}", p.display());
                    if self.legacy_on_save {
                        let msg = self.save_legacy(&p.with_extension("h1d"));
                        self.status = format!("Saved {} – {}", p.display(), msg);
                    }
                }
                Err(e) => self.status = e.to_string(),
            }
        }
    }

    /// Writes the project as a HYDRUS-1D 4.x project (`<name>.h1d` and the folder `<name>` with the input files) and
    /// returns a status message; settings HYDRUS-1D cannot store are listed with the problems.
    fn save_legacy(&mut self, path: &std::path::Path) -> String {
        match rusteau_io::save_h1d_project(&self.prj, path) {
            Ok((files, warnings)) => {
                let (h1d, dir) = rusteau_io::h1d_paths(path);
                if let Some(d) = h1d.parent() {
                    self.last_dir = Some(d.to_path_buf());
                }
                if !warnings.is_empty() {
                    self.problems = warnings.iter().map(|w| format!("HYDRUS-1D project: {w}")).collect();
                }
                format!(
                    "HYDRUS-1D project written: {} and {} files in {}{}",
                    h1d.display(),
                    files.len().saturating_sub(1),
                    dir.display(),
                    if warnings.is_empty() { String::new() } else { format!(" ({} settings not stored, see the problems)", warnings.len()) }
                )
            }
            Err(e) => format!("Writing the HYDRUS-1D project failed: {e}"),
        }
    }

    fn dialogs(&mut self, ctx: &egui::Context) {
        if self.show_close_confirm {
            egui::Window::new("Unsaved Changes")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.label("The current project has unsaved changes.\nDo you want to discard them and continue?");
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        if ui.button("Discard & Continue").clicked() {
                            self.show_close_confirm = false;
                            self.dirty = false;
                            match self.pending_action.take() {
                                Some(PendingAction::Quit) => {
                                    ctx.send_viewport_cmd(egui::ViewportCommand::Close)
                                }
                                Some(PendingAction::NewProject) => self.do_new_project(),
                                Some(PendingAction::OpenProject) => {
                                    self.dialog = Some(FileDialog::new(
                                        Purpose::OpenProject,
                                        Mode::OpenFile,
                                        "Open project",
                                        self.last_dir.clone(),
                                        "",
                                        &["h1dr", "h1d", "json"],
                                    ));
                                }
                                Some(PendingAction::OpenPath(p)) => self.open_path(&p),
                                None => {}
                            }
                        }
                        if ui.button("Cancel").clicked() {
                            self.show_close_confirm = false;
                            self.pending_action = None;
                        }
                    });
                });
        }

        if let Some(mut d) = self.dialog.take() {
            match d.show(ctx) {
                DialogResult::Open => self.dialog = Some(d),
                DialogResult::Cancel => {}
                DialogResult::Chosen(path) => {
                    self.last_dir = if path.is_dir() {
                        Some(path.clone())
                    } else {
                        path.parent().map(|p| p.to_path_buf())
                    };
                    match d.purpose {
                        Purpose::OpenProject => self.open_path(&path),
                        Purpose::ImportLegacy => {
							if path.file_name().map_or(false, |n| n.to_string_lossy().eq_ignore_ascii_case("Fit.in")) {
								match self.load_fit_measurements(&path) {
									Ok(n) => {
										self.status = format!("Loaded calibration file with {} observations from {} (shown with the results)", n, path.display());
									}
									Err(e) => {
										self.status = format!("Failed to parse Fit.in: {}", e);
									}
								}
							} else {
                                self.import_legacy(&path);
                            }
                        }
                        Purpose::SaveProject => {
                            let mut p = path.clone();
                            if p.extension().is_none() {
                                p.set_extension("h1dr");
                            }
                            self.path = Some(p);
                            self.save();
                        }
                        Purpose::SaveLegacy => {
                            self.status = self.save_legacy(&path);
                        }
                        Purpose::ExportCsv => {
                            if let Some(r) = &self.results {
                                self.status = match rusteau_io::writer::write_all(r, &path) {
                                    Ok(_) => format!("Results written to {}", path.display()),
                                    Err(e) => e.to_string(),
                                };
                            }
                        }
                        Purpose::ImportMeasured => self.import_measured(&path),
                        Purpose::ImportFitIn => match rusteau_io::read_fit_in(&path) {
                            Ok(settings) => {
                                let n = settings.observations.len();
                                let sets = measured::from_fit(&settings);
                                self.measured.retain(|m| !m.name.starts_with("Fit.in"));
                                self.measured.extend(sets);
                                self.prj.inverse = Some(settings);
                                self.prj.processes.inverse = true;
                                self.dirty = true;
                                self.page = Page::Inverse;
                                self.status = format!("Inverse settings with {} observations read from {}", n, path.display());
                            }
                            Err(e) => self.status = format!("Failed to read Fit.in: {}", e),
                        },
                        Purpose::ExportFitIn => {
                            if let Some(s) = &self.prj.inverse {
                                self.status = match rusteau_io::write_fit_in(&path, s) {
                                    Ok(_) => format!("Fit.in written to {}", path.display()),
                                    Err(e) => e.to_string(),
                                };
                            }
                        }
                        Purpose::SaveFitOut => {
                            if let (Some(r), Some(s)) = (&self.inv.result, &self.inv.settings) {
                                let text = rusteau_io::fit_out_text(&self.prj, s, r);
                                self.status = match std::fs::write(&path, text) {
                                    Ok(_) => format!("Fit.out written to {}", path.display()),
                                    Err(e) => e.to_string(),
                                };
                            }
                        }
                        Purpose::ExportHelp => {
                            let path = path.join("Rusteau-1D help");
                            self.status = match help::html::export(&path) {
                                Ok(start) => format!("HTML help written to {} (start page {})", path.display(), start.display()),
                                Err(e) => format!("Could not write the HTML help: {e}"),
                            };
                        }
                        Purpose::ImportAtmosphere => {
                            self.status = match self.import_atmosphere_text(&path) {
                                Ok(n) => format!("Imported {} atmospheric records", n),
                                Err(e) => e,
                            };
                        }
                        // dialogs of the figure window
                        Purpose::FigurePng | Purpose::FigureSvg | Purpose::FigureData | Purpose::FigureLoadRun | Purpose::FigureSaveRun => {}
                    }
                }
            }
        }
        if self.show_about {
            egui::Window::new("About Rusteau-1D")
                .open(&mut self.show_about)
                .collapsible(false)
                .show(ctx, |ui| {
                    ui.heading("Rusteau-1D");
                    ui.label("Modern, performant porous media & vadose zone simulator.\nNumerical engine ported and verified against HYDRUS-1D Fortran routines:\nvariably saturated flow (Richards), solute transport, and heat flow.");
                    ui.add_space(4.0);
                    ui.hyperlink_to(
                        "Original HYDRUS-1D: Šimůnek, van Genuchten & Šejna",
                        "https://www.pc-progress.com/en/Default.aspx?hydrus-1d",
                    );
                    ui.label(format!("Version {} – free software under the GNU General Public License, version 3 or later.", env!("CARGO_PKG_VERSION")));
                    ui.add_space(4.0);
                    ui.label("Fonts: Noto Sans 2.015 and Noto Sans Math 3.000, © The Noto Project Authors, bundled unmodified under the SIL Open Font License 1.1.");
                    ui.collapsing("Font license (SIL Open Font License 1.1)", |ui| {
                        egui::ScrollArea::vertical().max_height(260.0).show(ui, |ui| {
                            ui.label(egui::RichText::new(fonts::OFL_NOTO_SANS).monospace().small());
                            ui.separator();
                            ui.label(egui::RichText::new(fonts::OFL_NOTO_SANS_MATH).monospace().small());
                        });
                    });
                });
        }
    }

    /// Input pages in the order of the workflow, grouped in steps.
    fn pre_steps(&self) -> Vec<(&'static str, Vec<(Page, &'static str)>)> {
        let pr = &self.prj.processes;
        let bc = &self.prj.water.bc;
        let mut steps: Vec<(&'static str, Vec<(Page, &'static str)>)> = vec![
            ("Project", vec![(Page::Main, "Processes & units"), (Page::Time, "Time & output")]),
            ("Soil", vec![(Page::HydModel, "Hydraulic model"), (Page::SoilParams, "Materials & parameters"), (Page::Profile, "Soil profile")]),
        ];
        let mut b = vec![(Page::WaterBc, "Water flow")];
        if bc.atmospheric || bc.top_time_variable || bc.bot_time_variable {
            b.push((Page::Atmosphere, "Time-variable data"));
        }
        if self.prj.atmosphere.meteo.is_some() {
            b.push((Page::Meteo, "Meteorological data"));
        }
        steps.push(("Boundary conditions", b));
        let mut t = vec![];
        if pr.root_water_uptake {
            t.push((Page::RootUptake, "Root water & solute uptake"));
        }
        if pr.heat {
            t.push((Page::Heat, "Heat transport"));
        }
        if pr.solute {
            t.extend([
                (Page::SoluteGeneral, "Solute: general"),
                (Page::SoluteMaterials, "Solute: transport parameters"),
                (Page::SoluteReactions, "Solute: reactions"),
                (Page::SoluteAdvanced, "Solute: T / θ dependence, colloids"),
                (Page::SoluteBc, "Solute: boundary conditions"),
            ]);
        }
        if !t.is_empty() {
            steps.push(("Uptake & transport", t));
        }
        steps.push(("Run", vec![(Page::Iteration, "Iteration criteria"), (Page::Options, "Further options"), (Page::Summary, "Check & run")]));
        steps.push(("Parameter estimation", vec![(Page::Inverse, "Inverse problem (Fit.in)")]));
        steps
    }

    fn post_pages() -> [(Page, &'static str); 6] {
        [
            (Page::Obs, "Observation points"),
            (Page::ProfileInfo, "Profiles"),
            (Page::Fluxes, "Boundary fluxes & heads"),
            (Page::SoilProps, "Soil hydraulic properties"),
            (Page::RunTime, "Run-time information"),
            (Page::MassBalance, "Mass balance"),
        ]
    }

    fn nav(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("nav")
            .resizable(true)
            .default_width(250.0)
            .min_width(210.0)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.spacing_mut().item_spacing.y = 4.0;
                    ui.add_space(4.0);
                    ui.label(egui::RichText::new("Input").strong().size(16.0));
                    ui.separator();
                    for (k, (step, pages)) in self.pre_steps().into_iter().enumerate() {
                        ui.add_space(if k == 0 { 0.0 } else { 6.0 });
                        let head = if step == "Parameter estimation" { step.to_uppercase() } else { format!("{} · {}", k + 1, step.to_uppercase()) };
                        ui.label(egui::RichText::new(head).small().weak());
                        for (p, label) in pages {
                            ui.horizontal(|ui| {
                                ui.add_space(8.0);
                                if ui.selectable_label(self.page == p, label).clicked() {
                                    self.page = p;
                                }
                            });
                        }
                    }
                    ui.add_space(12.0);
                    ui.label(egui::RichText::new("Results").strong().size(16.0));
                    ui.separator();
                    let has = self.results.is_some();
                    for (p, label) in Self::post_pages() {
                        let enabled = has || p == Page::SoilProps;
                        ui.horizontal(|ui| {
                            ui.add_space(8.0);
                            if ui.add_enabled(enabled, egui::SelectableLabel::new(self.page == p, label)).clicked() {
                                self.page = p;
                            }
                        });
                    }
                    if self.inv.result.is_some() || self.inv.run.is_some() || self.prj.processes.inverse {
                        let enabled = self.inv.result.is_some() || self.inv.run.is_some();
                        ui.horizontal(|ui| {
                            ui.add_space(8.0);
                            if ui.add_enabled(enabled, egui::SelectableLabel::new(self.page == Page::InverseResults, "Parameter estimation")).clicked() {
                                self.page = Page::InverseResults;
                            }
                        });
                    }
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        ui.add_space(8.0);
                        let n = self.prj.figures.len();
                        let label = if n == 0 { "📈 Figures ↗".to_string() } else { format!("📈 Figures ({n}) ↗") };
                        if ui.selectable_label(self.figs.open, label).tip("Figures").clicked() {
                            self.figs.open = true;
                        }
                    });
                });
            });
    }

    /// Previous / next page of the input workflow.
    fn flow_bar(&mut self, ctx: &egui::Context) {
        let flat: Vec<(Page, &'static str)> = self.pre_steps().into_iter().flat_map(|(_, p)| p).filter(|(p, _)| *p != Page::Inverse).collect();
        let Some(i) = flat.iter().position(|(p, _)| *p == self.page) else {
            return;
        };
        egui::TopBottomPanel::bottom("flow").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if i > 0 {
                    let (p, l) = flat[i - 1];
                    if ui.button(format!("← {l}")).clicked() {
                        self.page = p;
                    }
                }
                ui.label(egui::RichText::new(format!("step {} of {}", i + 1, flat.len())).weak());
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if i + 1 < flat.len() {
                        let (p, l) = flat[i + 1];
                        if ui.button(egui::RichText::new(format!("{l} →")).strong()).clicked() {
                            self.page = p;
                        }
                    } else if ui.add_enabled(self.run.is_none(), egui::Button::new(egui::RichText::new("▶ Run").strong())).clicked() {
                        self.start_run();
                    }
                });
            });
        });
    }

    fn status_bar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::bottom("status").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if let Some(r) = &self.run {
                    let t = f64::from_bits(r.progress.load(Ordering::Relaxed));
                    let f = ((t - r.t_init) / (r.t_max - r.t_init)).clamp(0.0, 1.0) as f32;
                    ui.add(egui::ProgressBar::new(f).desired_width(220.0).show_percentage());
                    ui.label(format!("t = {:.4}", t));
                }
                ui.label(&self.status);
            });
        });
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if let Some(mut tour) = self.tour.take() {
            if tour.frame(self, ctx) {
                self.tour = Some(tour);
            }
        }
        if self.first_frame {
            self.first_frame = false;
            // 80 % of the monitor (the viewport itself is not the monitor); without monitor information the default
            // size of the native options stays; the screenshots of the tour keep the default size
            if let Some(m) = ctx.input(|i| i.viewport().monitor_size).filter(|_| self.tour.is_none()) {
                if m.x > 0.0 && m.y > 0.0 {
                    let target_w = (m.x * 0.80).clamp(960.0_f32.min(m.x), 1920.0);
                    let target_h = (m.y * 0.82).clamp(620.0_f32.min(m.y), 1200.0);
                    ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(target_w, target_h)));
                }
            }
        }

        if ctx.input(|i| i.viewport().close_requested()) && self.dirty {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.pending_action = Some(PendingAction::Quit);
            self.show_close_confirm = true;
        }

        self.handle_dropped_files(ctx);
        self.handle_shortcuts(ctx);
        self.poll_run(ctx);
        self.poll_inverse(ctx);
        self.menu(ctx);
        self.status_bar(ctx);
        self.flow_bar(ctx);
        self.nav(ctx);
        egui::CentralPanel::default().show(ctx, |ui| {
            if self.page == Page::Profile {
                // the profile editor fits the window and scrolls its own parts
                self.page_ui(ui);
                return;
            }
            egui::ScrollArea::both()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    self.page_ui(ui);
                });
        });
        self.dialogs(ctx);
        self.rosetta_window(ctx);
        self.help_ui(ctx);
        self.figure_window(ctx);
        self.drop_overlay(ctx);
    }
}

impl App {
    fn page_ui(&mut self, ui: &mut egui::Ui) {
        let before = self.dirty;
        let mut ch = false;
        match self.page {
            Page::Main => self.page_main(ui, &mut ch),
            Page::Time => self.page_time(ui, &mut ch),
            Page::Iteration => self.page_iteration(ui, &mut ch),
            Page::Options => self.page_options(ui, &mut ch),
            Page::HydModel => self.page_hydmodel(ui, &mut ch),
            Page::SoilParams => self.page_soil(ui, &mut ch),
            Page::WaterBc => self.page_waterbc(ui, &mut ch),
            Page::Atmosphere => self.page_atmosphere(ui, &mut ch),
            Page::Meteo => self.page_meteo(ui, &mut ch),
            Page::RootUptake => self.page_root(ui, &mut ch),
            Page::SoluteGeneral => self.page_sol_general(ui, &mut ch),
            Page::SoluteMaterials => self.page_sol_materials(ui, &mut ch),
            Page::SoluteReactions => self.page_sol_reactions(ui, &mut ch),
            Page::SoluteAdvanced => self.page_sol_advanced(ui, &mut ch),
            Page::SoluteBc => self.page_sol_bc(ui, &mut ch),
            Page::Heat => self.page_heat(ui, &mut ch),
            Page::Profile => self.page_profile(ui, &mut ch),
            Page::Inverse => self.page_inverse(ui, &mut ch),
            Page::InverseResults => self.page_inverse_results(ui),
            Page::Summary => self.page_summary(ui),
            Page::Obs => self.page_obs(ui),
            Page::ProfileInfo => self.page_profile_info(ui),
            Page::Fluxes => self.page_fluxes(ui),
            Page::SoilProps => self.page_soil_props(ui),
            Page::RunTime => self.page_runtime(ui),
            Page::MassBalance => self.page_balance(ui),
        }
        if ch {
            self.dirty = true;
        }
        let _ = before;
    }
}

/// A project file or folder: Rusteau (`.h1dr`, `.json`) or HYDRUS-1D (`.h1d`, a project folder or a file in one).
fn is_project_path(p: &std::path::Path) -> bool {
    let ext = p.extension().map(|e| e.to_string_lossy().to_ascii_lowercase()).unwrap_or_default();
    if matches!(ext.as_str(), "h1dr" | "json" | "h1d") {
        return true;
    }
    // a HYDRUS-1D project folder, or a file inside one
    let dir = if p.is_dir() { Some(p) } else { p.parent() };
    dir.and_then(|d| std::fs::read_dir(d).ok())
        .is_some_and(|rd| rd.flatten().any(|e| e.file_name().to_string_lossy().eq_ignore_ascii_case("selector.in")))
}

fn main() -> eframe::Result<()> {
    // rusteau --export-help DIR: write the HTML help and exit (no window)
    let args: Vec<String> = std::env::args().collect();
    if let Some(k) = args.iter().position(|a| a == "--export-help") {
        let dir = PathBuf::from(args.get(k + 1).map(|s| s.as_str()).unwrap_or("help"));
        match help::html::export(&dir) {
            Ok(start) => {
                println!("HTML help written to {} (start page {})", dir.display(), start.display());
                std::process::exit(0);
            }
            Err(e) => {
                eprintln!("Could not write the HTML help to {}: {e}", dir.display());
                std::process::exit(1);
            }
        }
    }
    let opts = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 800.0])
            .with_min_inner_size([920.0, 600.0])
            .with_resizable(true)
            .with_drag_and_drop(true)
            .with_title("Rusteau-1D"),
        ..Default::default()
    };
    eframe::run_native("Rusteau-1D", opts, Box::new(|cc| Ok(Box::new(App::new(cc)))))
}