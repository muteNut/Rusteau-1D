//! Help of the interface: a help window with contents, keyword index and full-text search (modelled on the compiled
//! HTML help of HYDRUS-1D), context help for every page (F1 or the "?" button next to the page title) and the same
//! topics as HTML pages for a web browser.
//!
//! The topics are the files `help/<id>.md` of this crate (see [`markup`] for their format); citations refer to
//! [`refs::REFS`].

pub mod html;
pub mod markup;
pub mod refs;
pub mod render;
pub mod topics;

use crate::Page;
use egui::{Context, RichText, Ui};
use render::{Action, ImageCache, Renderer};

/// Topic of an input or results page (context help).
pub fn topic_of(page: Page) -> &'static str {
    match page {
        Page::Main => "processes",
        Page::Time => "time_output",
        Page::Iteration => "iteration",
        Page::Options => "further_options",
        Page::HydModel => "hyd_model",
        Page::SoilParams => "soil_params",
        Page::WaterBc => "water_bc",
        Page::Atmosphere => "time_variable_bc",
        Page::Meteo => "meteo_page",
        Page::RootUptake => "root_uptake_page",
        Page::SoluteGeneral => "solute_general",
        Page::SoluteMaterials => "solute_params",
        Page::SoluteReactions => "solute_reactions",
        Page::SoluteAdvanced => "solute_advanced",
        Page::SoluteBc => "solute_bc",
        Page::Heat => "heat_page",
        Page::Profile => "profile_editor",
        Page::Summary => "check_run",
        Page::Obs => "res_obs",
        Page::ProfileInfo => "res_profiles",
        Page::Fluxes => "res_fluxes",
        Page::SoilProps => "res_soilprops",
        Page::RunTime => "res_runtime",
        Page::MassBalance => "res_balance",
        Page::Inverse => "inverse_page",
        Page::InverseResults => "res_inverse",
    }
}

/// Id of the egui memory flag set by the "?" button of a page title: show the help of the current page.
fn request_id() -> egui::Id {
    egui::Id::new("rusteau_help_request")
}

/// Asks for the help of the page shown (the "?" next to the page title).
pub fn request_page_help(ctx: &Context) {
    ctx.data_mut(|d| d.insert_temp(request_id(), true));
}

/// Asks for a given topic (e.g. from a dialog).
pub fn request_topic(ctx: &Context, id: &'static str) {
    ctx.data_mut(|d| d.insert_temp(egui::Id::new("rusteau_help_topic"), id));
}

/// Takes a pending request: `Some(None)` for the help of the current page, `Some(Some(id))` for a topic.
pub fn take_request(ctx: &Context) -> Option<Option<&'static str>> {
    ctx.data_mut(|d| {
        if let Some(id) = d.get_temp::<&'static str>(egui::Id::new("rusteau_help_topic")) {
            d.remove::<&'static str>(egui::Id::new("rusteau_help_topic"));
            return Some(Some(id));
        }
        if d.get_temp::<bool>(request_id()).unwrap_or(false) {
            d.remove::<bool>(request_id());
            return Some(None);
        }
        None
    })
}

/// Entries of the keyword index: (keyword, topic, anchor), sorted. Keywords of the topics and their section headings.
pub fn keyword_index() -> Vec<(String, &'static str, Option<String>)> {
    let mut v: Vec<(String, &'static str, Option<String>)> = vec![];
    for t in topics::all() {
        v.push((t.doc.title.clone(), t.id, None));
        for k in &t.doc.keywords {
            v.push((k.clone(), t.id, None));
        }
        for b in &t.doc.blocks {
            if let markup::Block::Heading { level: 2, text, anchor } = b {
                let h = markup::plain(text);
                if anchor != "literature" {
                    v.push((format!("{h} ({})", t.doc.title), t.id, Some(anchor.clone())));
                }
            }
        }
    }
    v.sort_by(|a, b| a.0.to_lowercase().cmp(&b.0.to_lowercase()).then(a.1.cmp(b.1)));
    v.dedup_by(|a, b| a.0.eq_ignore_ascii_case(&b.0) && a.1 == b.1);
    v
}

/// Search hits: (topic, score, snippet).
pub fn search(query: &str) -> Vec<(&'static str, usize, String)> {
    let terms: Vec<String> = query.split_whitespace().map(|w| w.to_lowercase()).collect();
    if terms.is_empty() {
        return vec![];
    }
    let mut hits = vec![];
    for t in topics::all() {
        let title = t.doc.title.to_lowercase();
        let mut score = 0;
        let mut ok = true;
        for w in &terms {
            let n = t.text.matches(w.as_str()).count();
            if n == 0 {
                ok = false;
                break;
            }
            score += n + if title.contains(w.as_str()) { 20 } else { 0 };
        }
        if ok {
            let i = t.text.find(terms[0].as_str()).unwrap_or(0);
            let chars: Vec<char> = t.text.chars().collect();
            let ci = t.text[..i].chars().count();
            let a = ci.saturating_sub(60);
            let b = (ci + 100).min(chars.len());
            let snip: String = chars[a..b].iter().collect::<String>().split_whitespace().collect::<Vec<_>>().join(" ");
            hits.push((t.id, score, format!("{}{snip}…", if a > 0 { "…" } else { "" })));
        }
    }
    hits.sort_by(|a, b| b.1.cmp(&a.1));
    hits
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tab {
    Contents,
    Index,
    Search,
}

/// What the help window asks the application to do.
pub enum HelpEvent {
    /// Write the HTML help and open it in the web browser
    OpenInBrowser(String),
}

pub struct HelpState {
    pub open: bool,
    current: String,
    back: Vec<String>,
    forward: Vec<String>,
    tab: Tab,
    query: String,
    index_filter: String,
    scroll_to: Option<String>,
    images: ImageCache,
    zoom: Option<String>,
    size: f32,
}

impl Default for HelpState {
    fn default() -> Self {
        HelpState {
            open: false,
            current: "contents".into(),
            back: vec![],
            forward: vec![],
            tab: Tab::Contents,
            query: String::new(),
            index_filter: String::new(),
            scroll_to: Some(String::new()),
            images: ImageCache::default(),
            zoom: None,
            size: 15.0,
        }
    }
}

impl HelpState {
    /// Opens the window at topic `id` (and anchor).
    pub fn show_topic(&mut self, id: &str, anchor: Option<&str>) {
        self.open = true;
        if topics::get(id).is_none() {
            return;
        }
        if self.current != id {
            self.back.push(self.current.clone());
            self.forward.clear();
            self.current = id.to_string();
        }
        self.scroll_to = Some(anchor.unwrap_or("").to_string());
    }

    pub fn current(&self) -> &str {
        &self.current
    }

    fn go_back(&mut self) {
        if let Some(p) = self.back.pop() {
            self.forward.push(std::mem::replace(&mut self.current, p));
            self.scroll_to = Some(String::new());
        }
    }

    fn go_forward(&mut self) {
        if let Some(n) = self.forward.pop() {
            self.back.push(std::mem::replace(&mut self.current, n));
            self.scroll_to = Some(String::new());
        }
    }

    /// The help window.
    pub fn window(&mut self, ctx: &Context) -> Option<HelpEvent> {
        if !self.open {
            return None;
        }
        let mut event = None;
        let mut open = true;
        let screen = ctx.screen_rect();
        egui::Window::new("Rusteau-1D Help")
            .id(egui::Id::new("help_window"))
            .open(&mut open)
            .collapsible(false)
            .resizable(true)
            .default_size([(screen.width() * 0.8).min(1180.0), (screen.height() * 0.85).min(860.0)])
            .default_pos(screen.center() - egui::vec2((screen.width() * 0.4).min(590.0), (screen.height() * 0.42).min(430.0)))
            .show(ctx, |ui| {
                event = self.contents_ui(ui);
            });
        if !open {
            self.open = false;
        }
        if let Some(src) = self.zoom.clone() {
            let mut zopen = true;
            egui::Window::new("Image").id(egui::Id::new("help_zoom")).open(&mut zopen).resizable(true).default_size(screen.size() * 0.9).show(ctx, |ui| {
                let mut r = Renderer { images: &mut self.images, action: None, scroll_to: None, size: self.size };
                r.zoomed(ui, &src);
            });
            if !zopen {
                self.zoom = None;
            }
        }
        // keys of the help window: Alt+Left / Alt+Right
        if ctx.input(|i| i.modifiers.alt && i.key_pressed(egui::Key::ArrowLeft)) {
            self.go_back();
        }
        if ctx.input(|i| i.modifiers.alt && i.key_pressed(egui::Key::ArrowRight)) {
            self.go_forward();
        }
        event
    }

    fn toolbar(&mut self, ui: &mut Ui) -> Option<HelpEvent> {
        let mut event = None;
        ui.horizontal(|ui| {
            if ui.add_enabled(!self.back.is_empty(), egui::Button::new("◀ Back")).on_hover_text("Previous topic (Alt+←)").clicked() {
                self.go_back();
            }
            if ui.add_enabled(!self.forward.is_empty(), egui::Button::new("Forward ▶")).on_hover_text("Next topic in the history (Alt+→)").clicked() {
                self.go_forward();
            }
            if ui.button("⌂ Contents").on_hover_text("Start page of the help").clicked() {
                self.show_topic("contents", None);
            }
            let order = topics::order();
            let k = order.iter().position(|t| *t == self.current);
            if let Some(k) = k {
                if ui.add_enabled(k > 0, egui::Button::new("↑")).on_hover_text("Previous topic in the table of contents").clicked() {
                    self.show_topic(order[k - 1], None);
                }
                if ui.add_enabled(k + 1 < order.len(), egui::Button::new("↓")).on_hover_text("Next topic in the table of contents").clicked() {
                    self.show_topic(order[k + 1], None);
                }
            }
            if ui.button("📚 References").on_hover_text("All scientific sources of the program").clicked() {
                self.show_topic("references", None);
            }
            ui.separator();
            if ui.button("A−").on_hover_text("Smaller text").clicked() {
                self.size = (self.size - 1.0).max(11.0);
            }
            if ui.button("A+").on_hover_text("Larger text").clicked() {
                self.size = (self.size + 1.0).min(24.0);
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .button("🌐 Open in web browser")
                    .on_hover_text("Writes the help as HTML pages (offline) and opens this topic in the web browser")
                    .clicked()
                {
                    event = Some(HelpEvent::OpenInBrowser(self.current.clone()));
                }
            });
        });
        event
    }

    fn contents_ui(&mut self, ui: &mut Ui) -> Option<HelpEvent> {
        let event = self.toolbar(ui);
        ui.separator();
        egui::SidePanel::left("help_nav").resizable(true).default_width(270.0).min_width(200.0).show_inside(ui, |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.tab, Tab::Contents, "Contents");
                ui.selectable_value(&mut self.tab, Tab::Index, "Index");
                ui.selectable_value(&mut self.tab, Tab::Search, "Search");
            });
            ui.separator();
            egui::ScrollArea::vertical().id_salt("help_nav_scroll").auto_shrink([false, false]).show(ui, |ui| match self.tab {
                Tab::Contents => self.toc_ui(ui),
                Tab::Index => self.index_ui(ui),
                Tab::Search => self.search_ui(ui),
            });
        });
        let mut action = None;
        egui::CentralPanel::default().show_inside(ui, |ui| {
            let path = topics::path_of(&self.current);
            if !path.is_empty() {
                ui.label(RichText::new(path.join(" › ")).weak().small());
            }
            egui::ScrollArea::vertical().id_salt(("help_page", self.current.clone())).auto_shrink([false, false]).show(ui, |ui| {
                ui.set_max_width(980.0);
                if let Some(t) = topics::get(&self.current) {
                    let mut r = Renderer { images: &mut self.images, action: None, scroll_to: self.scroll_to.take(), size: self.size };
                    r.doc(ui, &t.doc);
                    // an anchor that was not drawn this frame (e.g. below the visible part) is kept for the next one
                    self.scroll_to = r.scroll_to.filter(|a| !a.is_empty());
                    action = r.action;
                }
            });
        });
        match action {
            Some(Action::Go { id, anchor }) => self.show_topic(&id, anchor.as_deref()),
            Some(Action::Url(u)) => ui.ctx().open_url(egui::OpenUrl::new_tab(u)),
            Some(Action::Zoom(src)) => self.zoom = Some(src),
            None => {}
        }
        event
    }

    fn toc_ui(&mut self, ui: &mut Ui) {
        fn walk(s: &mut HelpState, ui: &mut Ui, t: &'static [topics::Toc], path: &[&str]) {
            for e in t {
                match e {
                    topics::Toc::Book(name, c) => {
                        let open_now = path.contains(name);
                        let mut h = egui::CollapsingHeader::new(RichText::new(format!("📖 {name}")).strong()).id_salt(("toc", *name)).default_open(open_now);
                        if open_now {
                            h = h.open(Some(true));
                        }
                        h.show(ui, |ui| walk(s, ui, c, path));
                    }
                    topics::Toc::Page(id) => {
                        let sel = s.current == *id;
                        if ui.selectable_label(sel, format!("📄 {}", topics::title(id))).clicked() && !sel {
                            s.show_topic(id, None);
                        }
                    }
                }
            }
        }
        // keep the book of the current topic open only when the topic changes (the user may close books)
        let key = egui::Id::new("help_toc_last");
        let last: Option<String> = ui.ctx().data(|d| d.get_temp(key));
        let path = if last.as_deref() != Some(self.current.as_str()) {
            ui.ctx().data_mut(|d| d.insert_temp(key, self.current.clone()));
            topics::path_of(&self.current)
        } else {
            vec![]
        };
        walk(self, ui, topics::TOC, &path);
    }

    fn index_ui(&mut self, ui: &mut Ui) {
        ui.add(egui::TextEdit::singleline(&mut self.index_filter).hint_text("Type a keyword…").desired_width(f32::INFINITY));
        let f = self.index_filter.to_lowercase();
        let mut go = None;
        for (k, id, anchor) in keyword_index() {
            if !f.is_empty() && !k.to_lowercase().contains(&f) {
                continue;
            }
            if ui.link(&k).on_hover_text(topics::title(id)).clicked() {
                go = Some((id, anchor));
            }
        }
        if let Some((id, a)) = go {
            self.show_topic(id, a.as_deref());
        }
    }

    fn search_ui(&mut self, ui: &mut Ui) {
        let r = ui.add(egui::TextEdit::singleline(&mut self.query).hint_text("Search words…").desired_width(f32::INFINITY));
        if self.query.is_empty() && !r.has_focus() {
            ui.label(RichText::new("All words must occur in a topic; topics with the words in the title come first.").weak());
            return;
        }
        let hits = search(&self.query);
        ui.label(RichText::new(format!("{} topic(s)", hits.len())).weak());
        let mut go = None;
        for (id, _, snip) in hits {
            if ui.selectable_label(self.current == id, RichText::new(topics::title(id)).strong()).clicked() {
                go = Some(id);
            }
            ui.label(RichText::new(snip).weak().small());
            ui.add_space(4.0);
        }
        if let Some(id) = go {
            self.show_topic(id, None);
        }
    }
}

/// Writes the HTML help into the cache folder and returns the address of topic `id`.
pub fn html_url(id: &str) -> Result<String, String> {
    let dir = std::env::temp_dir().join(format!("rusteau-help-{}", env!("CARGO_PKG_VERSION")));
    html::export(&dir).map_err(|e| format!("Could not write the HTML help to {}: {e}", dir.display()))?;
    let file = dir.join(format!("{id}.html"));
    Ok(file_url(&file))
}

/// `file://` address of a local file.
pub fn file_url(p: &std::path::Path) -> String {
    let s = p.to_string_lossy().replace('\\', "/");
    let s = s.replace(' ', "%20");
    if s.starts_with('/') {
        format!("file://{s}")
    } else {
        format!("file:///{s}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_page_has_a_help_topic() {
        use Page::*;
        for p in [
            Main, Time, Iteration, Options, HydModel, SoilParams, WaterBc, Atmosphere, Meteo, RootUptake, SoluteGeneral, SoluteMaterials,
            SoluteReactions, SoluteAdvanced, SoluteBc, Heat, Profile, Summary, Obs, ProfileInfo, Fluxes, SoilProps, RunTime, MassBalance,
            Inverse, InverseResults,
        ] {
            assert!(topics::get(topic_of(p)).is_some(), "{p:?}");
        }
    }

    #[test]
    fn search_finds_topics() {
        let h = search("van Genuchten");
        assert!(h.iter().any(|(id, _, _)| *id == "th_hydraulic_models"));
        assert!(search("").is_empty());
        assert!(!keyword_index().is_empty());
    }

    #[test]
    fn file_urls() {
        assert_eq!(file_url(std::path::Path::new("/tmp/a b/x.html")), "file:///tmp/a%20b/x.html");
    }
}
