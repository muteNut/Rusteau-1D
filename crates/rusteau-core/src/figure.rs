//! Figures (diagrams) of a project, defined in the figure editor of the interface and stored with the project. Data
//! only: the simulation does not use them. A figure has one or more panels in a grid; a panel has axes, data layers
//! (series of x–y values taken from the results, the input, measured data or data files) and annotations.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// A colour with opacity (sRGB and alpha, 0–255). Stored as `"#rrggbb"` or `"#rrggbbaa"`; `[r, g, b]` and
/// `[r, g, b, a]` are read, too.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Color(pub [u8; 4]);

impl Color {
    pub const BLACK: Color = Color([0, 0, 0, 255]);

    pub const fn rgb(r: u8, g: u8, b: u8) -> Color {
        Color([r, g, b, 255])
    }

    /// `#rrggbb`, or `#rrggbbaa` when the colour is not opaque.
    pub fn hex(&self) -> String {
        let [r, g, b, a] = self.0;
        if a == 255 {
            format!("#{r:02x}{g:02x}{b:02x}")
        } else {
            format!("#{r:02x}{g:02x}{b:02x}{a:02x}")
        }
    }

    /// Reads `#rgb`, `#rrggbb` or `#rrggbbaa` (the `#` is optional).
    pub fn parse(s: &str) -> Option<Color> {
        let h = s.trim().trim_start_matches('#');
        if !h.chars().all(|c| c.is_ascii_hexdigit()) {
            return None;
        }
        let byte = |i: usize| u8::from_str_radix(&h[i..i + 2], 16).ok();
        match h.len() {
            3 => {
                let d = |i: usize| u8::from_str_radix(&h[i..i + 1], 16).ok().map(|v| v * 17);
                Some(Color([d(0)?, d(1)?, d(2)?, 255]))
            }
            6 => Some(Color([byte(0)?, byte(2)?, byte(4)?, 255])),
            8 => Some(Color([byte(0)?, byte(2)?, byte(4)?, byte(6)?])),
            _ => None,
        }
    }
}

impl Serialize for Color {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.hex())
    }
}

impl<'de> Deserialize<'de> for Color {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Color, D::Error> {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Repr {
            Hex(String),
            Values(Vec<u8>),
        }
        match Repr::deserialize(d)? {
            Repr::Hex(s) => Color::parse(&s).ok_or_else(|| serde::de::Error::custom(format!("not a colour: {s}"))),
            Repr::Values(v) if v.len() == 3 => Ok(Color([v[0], v[1], v[2], 255])),
            Repr::Values(v) if v.len() == 4 => Ok(Color([v[0], v[1], v[2], v[3]])),
            Repr::Values(v) => Err(serde::de::Error::custom(format!("a colour has 3 or 4 values, not {}", v.len()))),
        }
    }
}

/// Reads the colours of the custom palette: colours, or (as written by earlier versions) objects with a colour.
fn custom_colors<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Vec<Color>, D::Error> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Entry {
        Plain(Color),
        Named { color: Color },
    }
    Ok(Vec::<Entry>::deserialize(d)?
        .into_iter()
        .map(|e| match e {
            Entry::Plain(c) | Entry::Named { color: c } => c,
        })
        .collect())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Theme {
    /// white background, frame around the panel, light grid
    #[default]
    Grid,
    /// white background, axis lines left and bottom, no grid (the classic journal style)
    Classic,
    /// no frame, light grid (like ggplot's theme_minimal)
    Minimal,
    /// dark background (presentations, the dark theme of the interface)
    Dark,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Palette {
    /// colour-blind safe (Okabe and Ito)
    #[default]
    OkabeIto,
    /// the colours of the interface (Tableau 10)
    Tableau,
    /// perceptually uniform sequence (viridis), for series in an order (print times, depths)
    Viridis,
    /// shades of grey
    Grey,
    /// the colours of `Figure::colors`
    Custom,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum LegendPos {
    #[default]
    TopRight,
    TopLeft,
    BottomRight,
    BottomLeft,
    /// outside the plot area, on the right
    Right,
    /// outside the plot area, below
    Bottom,
    None,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Geom {
    #[default]
    Line,
    Points,
    LinePoints,
    /// step function (the value holds until the next x, like the records of the atmospheric data)
    Steps,
    /// filled area between the curve and y = 0
    Area,
    Bars,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Dash {
    #[default]
    Solid,
    Dashed,
    Dotted,
    DashDot,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Marker {
    #[default]
    Circle,
    Square,
    Triangle,
    Diamond,
    Cross,
    Plus,
    OpenCircle,
    OpenSquare,
    OpenTriangle,
}

/// Where the values of a layer come from. `items` of the layer select the observation nodes, print times, materials,
/// measured sets or measurement sets (empty: all).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub enum Source {
    /// time-level information (T_Level): boundary fluxes, heads, cumulative values, solute fluxes
    #[default]
    TimeLevel,
    /// observation nodes (Obs_Node), one series per node
    ObsNodes,
    /// profiles at the print times (Nod_Inf), one series per print time
    Profiles,
    /// mass balance at the print times (Balance)
    Balance,
    /// cumulative values at the ends of the atmospheric records (A_Level)
    ALevel,
    /// extended fluxes (T_Level1)
    TLevel1,
    /// meteorological output (Meteo.out)
    Meteo,
    /// soil hydraulic properties θ(h), K(h), C(h), one series per material
    Hydraulic,
    /// input: the time-variable boundary records (Atmosph.in)
    AtmInput,
    /// input: the initial condition and nodal values of the soil profile
    ProfileInput,
    /// measured data loaded on the results pages, one series per set
    Measured,
    /// parameter estimation: observed and fitted values, one series per measurement set
    Fit,
    /// a data file imported into the figure (index into `Figure::data`)
    DataFile(usize),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Layer {
    pub source: Source,
    /// selected nodes, print times, materials or sets (0-based); empty: all
    pub items: Vec<usize>,
    /// runs to show: "" is the current results, other names are kept runs (comparison); empty: the current results
    pub runs: Vec<String>,
    /// column of the x values (key of the data table)
    pub x: String,
    /// column of the y values
    pub y: String,
    /// factors applied to the x and y values (unit conversion, e.g. 10 for cm → mm or −1 to flip a sign)
    pub x_scale: f64,
    pub y_scale: f64,
    pub geom: Geom,
    /// line width [pt]
    pub width: f64,
    pub dash: Dash,
    pub marker: Marker,
    /// marker size [pt]
    pub marker_size: f64,
    /// opacity of areas and bars, 0–1
    pub alpha: f64,
    /// legend label; empty: automatic. `{item}` (print time, node, material…), `{run}` and `{y}` (the variable)
    /// are replaced
    pub label: String,
    /// plotted against the right (secondary) y axis
    pub y2: bool,
    pub visible: bool,
    /// shown in the legend
    pub in_legend: bool,
}

impl Default for Layer {
    fn default() -> Self {
        Layer {
            source: Source::TimeLevel,
            items: vec![],
            runs: vec![],
            x: "t".into(),
            y: "vTop".into(),
            x_scale: 1.0,
            y_scale: 1.0,
            geom: Geom::Line,
            width: 1.2,
            dash: Dash::Solid,
            marker: Marker::Circle,
            marker_size: 4.0,
            alpha: 0.35,
            label: String::new(),
            y2: false,
            visible: true,
            in_legend: true,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Axis {
    /// title from the variable of the first layer with its unit
    pub label_auto: bool,
    pub label: String,
    /// range from the data (with a small margin); otherwise `min`–`max`
    pub auto: bool,
    pub min: f64,
    pub max: f64,
    pub log: bool,
    /// values increase downward (depth) or to the left
    pub reverse: bool,
    /// distance of the major ticks; 0: automatic
    pub step: f64,
    pub grid: bool,
    pub minor: bool,
}

impl Default for Axis {
    fn default() -> Self {
        Axis { label_auto: true, label: String::new(), auto: true, min: 0.0, max: 1.0, log: false, reverse: false, step: 0.0, grid: true, minor: false }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum AnnotationKind {
    #[default]
    Text,
    /// horizontal line at y (with an optional text)
    HLine,
    /// vertical line at x (with an optional text)
    VLine,
    /// arrow from (x, y) to (x2, y2), with an optional text at its start
    Arrow,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Anchor {
    #[default]
    Left,
    Center,
    Right,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Annotation {
    pub kind: AnnotationKind,
    /// text with the markup of the help: `x_{s}` subscript, `x^{2}` superscript, `*italic*`, `**bold**`
    pub text: String,
    pub x: f64,
    pub y: f64,
    pub x2: f64,
    pub y2: f64,
    /// coordinates relative to the plot area (0–1 from the bottom left) instead of data coordinates
    pub relative: bool,
    /// font size [pt]; 0: the size of the tick labels
    pub size: f64,
    /// black follows the text colour of the theme
    pub color: Color,
    pub anchor: Anchor,
    pub dash: Dash,
    /// line width [pt]
    pub width: f64,
    /// refers to the right y axis
    pub on_y2: bool,
}

impl Default for Annotation {
    fn default() -> Self {
        Annotation {
            kind: AnnotationKind::Text,
            text: "Text".into(),
            x: 0.05,
            y: 0.95,
            x2: 0.3,
            y2: 0.7,
            relative: true,
            size: 0.0,
            color: Color::BLACK,
            anchor: Anchor::Left,
            dash: Dash::Dashed,
            width: 0.8,
            on_y2: false,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Panel {
    /// title above the panel, e.g. "(a) Loam"
    pub title: String,
    pub x: Axis,
    pub y: Axis,
    /// right y axis (used when a layer is plotted against it)
    pub y2: Axis,
    pub layers: Vec<Layer>,
    pub annotations: Vec<Annotation>,
    pub legend: LegendPos,
    /// maximum width of the legend (below the plot: of an entry) in percent of the width of the panel; longer labels
    /// are wrapped at blanks
    pub legend_width: f64,
    /// legend labels given to single series, in place of their automatic labels (the keys)
    pub legend_names: BTreeMap<String, String>,
    /// palette of this panel (used when `Figure::palette_per_panel` is on and the figure has several panels)
    pub palette: Palette,
    /// colours of the palette "Custom" of this panel
    #[serde(deserialize_with = "custom_colors")]
    pub colors: Vec<Color>,
}

impl Panel {
    /// The legend label of a series with the automatic label `auto`: the name given to it, or `auto`.
    pub fn legend_label<'a>(&'a self, auto: &'a str) -> &'a str {
        self.legend_names.get(auto).map(String::as_str).unwrap_or(auto)
    }

    /// Gives the series with the automatic label `auto` the legend label `name`; an empty name or `auto` itself
    /// restores the automatic label.
    pub fn rename_series(&mut self, auto: &str, name: &str) {
        let name = name.trim();
        if name.is_empty() || name == auto {
            self.legend_names.remove(auto);
        } else {
            self.legend_names.insert(auto.to_string(), name.to_string());
        }
    }
}

impl Default for Panel {
    fn default() -> Self {
        Panel {
            title: String::new(),
            x: Axis::default(),
            y: Axis::default(),
            y2: Axis { grid: false, ..Axis::default() },
            layers: vec![Layer::default()],
            annotations: vec![],
            legend: LegendPos::TopRight,
            legend_width: 40.0,
            legend_names: BTreeMap::new(),
            palette: Palette::OkabeIto,
            colors: vec![],
        }
    }
}

/// Font sizes [pt].
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct TextSizes {
    pub title: f64,
    pub subtitle: f64,
    pub caption: f64,
    pub panel_title: f64,
    pub axis_title: f64,
    pub ticks: f64,
    pub legend: f64,
}

impl Default for TextSizes {
    fn default() -> Self {
        TextSizes { title: 13.0, subtitle: 10.0, caption: 8.0, panel_title: 10.0, axis_title: 10.0, ticks: 8.5, legend: 8.5 }
    }
}

/// A table imported from a file: named columns of equal length.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct DataTable {
    pub name: String,
    pub columns: Vec<String>,
    pub values: Vec<Vec<f64>>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Figure {
    pub name: String,
    /// title, subtitle and caption: the markup of the help (`θ_{s}`, `cm^{3}`, `*italic*`, `**bold**`)
    pub title: String,
    pub subtitle: String,
    pub caption: String,
    /// size of the figure [mm]
    pub width_mm: f64,
    pub height_mm: f64,
    /// resolution of the PNG export [dots per inch]
    pub dpi: f64,
    /// font family ("Noto Sans" is bundled with the program)
    pub font: String,
    pub sizes: TextSizes,
    pub title_bold: bool,
    pub theme: Theme,
    pub palette: Palette,
    /// colours of the palette "Custom", in the order of the series
    #[serde(deserialize_with = "custom_colors")]
    pub colors: Vec<Color>,
    /// every panel has its own palette (`Panel::palette`, `Panel::colors`) instead of `palette` and `colors`
    pub palette_per_panel: bool,
    /// transparent background in the exported files
    pub transparent: bool,
    /// panels in a grid of `rows` × `cols`, row by row
    pub rows: usize,
    pub cols: usize,
    pub panels: Vec<Panel>,
    pub data: Vec<DataTable>,
    /// SVG export: text as outlines (looks the same everywhere) instead of text elements (editable)
    pub svg_text_as_paths: bool,
}

impl Default for Figure {
    fn default() -> Self {
        Figure {
            name: "Figure 1".into(),
            title: String::new(),
            subtitle: String::new(),
            caption: String::new(),
            width_mm: 160.0,
            height_mm: 100.0,
            dpi: 300.0,
            font: "Noto Sans".into(),
            sizes: TextSizes::default(),
            title_bold: true,
            theme: Theme::Grid,
            palette: Palette::OkabeIto,
            colors: vec![],
            palette_per_panel: false,
            transparent: false,
            rows: 1,
            cols: 1,
            panels: vec![Panel::default()],
            data: vec![],
            svg_text_as_paths: false,
        }
    }
}

impl Figure {
    /// Number of panel cells of the grid (at least 1).
    pub fn cells(&self) -> usize {
        (self.rows.max(1) * self.cols.max(1)).max(1)
    }

    /// Whether the panels use their own palettes: `palette_per_panel` with several panels.
    pub fn own_panel_palettes(&self) -> bool {
        self.palette_per_panel && self.cells() > 1
    }

    /// The palette of panel `k`: its own with [`Figure::own_panel_palettes`], otherwise the figure's.
    pub fn panel_palette(&self, k: usize) -> (Palette, &[Color]) {
        match self.panels.get(k) {
            Some(p) if self.own_panel_palettes() => (p.palette, &p.colors),
            _ => (self.palette, &self.colors),
        }
    }

    /// Switches between one palette for all panels and one per panel; the panels start from the palette of the figure,
    /// so the figure looks the same after the switch.
    pub fn set_palette_per_panel(&mut self, on: bool) {
        if on && !self.palette_per_panel {
            for p in &mut self.panels {
                p.palette = self.palette;
                p.colors = self.colors.clone();
            }
        }
        self.palette_per_panel = on;
    }

    /// Adds or removes panels so that every cell of the grid has one.
    pub fn sync_panels(&mut self) {
        let n = self.cells();
        while self.panels.len() < n {
            let mut p = self.panels.last().cloned().unwrap_or_default();
            p.title.clear();
            self.panels.push(p);
        }
        self.panels.truncate(n);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn figures_round_trip_and_old_projects_load() {
        let mut p = crate::Project::default();
        let mut f = Figure { rows: 2, cols: 2, ..Default::default() };
        f.sync_panels();
        f.panels[1].layers[0].source = Source::DataFile(0);
        f.data.push(DataTable { name: "x".into(), columns: vec!["a".into(), "b".into()], values: vec![vec![1.0], vec![2.0]] });
        p.figures.push(f.clone());
        let s = serde_json::to_string(&p).unwrap();
        let q: crate::Project = serde_json::from_str(&s).unwrap();
        assert_eq!(q.figures, vec![f]);
        // a project written before figures existed
        let mut v: serde_json::Value = serde_json::from_str(&s).unwrap();
        v.as_object_mut().unwrap().remove("figures");
        let q: crate::Project = serde_json::from_value(v).unwrap();
        assert!(q.figures.is_empty());
        // names given to series; panels written before them take the defaults
        let mut panel = Panel::default();
        panel.rename_series("Bottom flux", " Drainage ");
        assert_eq!(panel.legend_label("Bottom flux"), "Drainage");
        assert_eq!(panel.legend_label("Runoff"), "Runoff");
        panel.rename_series("Bottom flux", "");
        assert!(panel.legend_names.is_empty());
        panel.rename_series("Runoff", "Runoff");
        assert!(panel.legend_names.is_empty());
        let old: Panel = serde_json::from_str(r#"{"title":"(a)"}"#).unwrap();
        assert_eq!((old.legend_width, old.legend_names.len()), (40.0, 0));
        // a figure with missing fields takes the defaults
        let g: Figure = serde_json::from_str(r#"{"name":"F","panels":[{"layers":[{"y":"theta"}]}]}"#).unwrap();
        assert_eq!(g.panels[0].layers[0].y, "theta");
        assert_eq!(g.panels[0].layers[0].x, "t");
        assert_eq!(g.width_mm, 160.0);
    }

    #[test]
    fn colours_read_hex_and_arrays() {
        assert_eq!(Color::parse("#34517d"), Some(Color([0x34, 0x51, 0x7d, 255])));
        assert_eq!(Color::parse("34517d80"), Some(Color([0x34, 0x51, 0x7d, 0x80])));
        assert_eq!(Color::parse("#fff"), Some(Color([255, 255, 255, 255])));
        assert_eq!(Color::parse("#12345"), None);
        assert_eq!(Color::parse("#gg0000"), None);
        assert_eq!(Color([1, 2, 3, 255]).hex(), "#010203");
        let a: Annotation = serde_json::from_str(r#"{"color":[10,20,30]}"#).unwrap();
        assert_eq!(a.color, Color([10, 20, 30, 255]));
        let a: Annotation = serde_json::from_str(r##"{"color":"#0a141e80"}"##).unwrap();
        assert_eq!(a.color, Color([10, 20, 30, 128]));
        assert!(serde_json::from_str::<Annotation>(r#"{"color":[1,2]}"#).is_err());
        // the palette of the figure, or of every panel
        let mut two = Figure { cols: 2, palette: Palette::Viridis, ..Default::default() };
        two.sync_panels();
        assert_eq!(two.panel_palette(1).0, Palette::Viridis);
        two.set_palette_per_panel(true);
        assert_eq!(two.panel_palette(1).0, Palette::Viridis, "the panels start from the scheme of the figure");
        two.panels[1].palette = Palette::Grey;
        assert_eq!((two.panel_palette(0).0, two.panel_palette(1).0), (Palette::Viridis, Palette::Grey));
        two.cols = 1;
        two.sync_panels();
        assert_eq!(two.panel_palette(0).0, Palette::Viridis, "one panel: the scheme of the figure");
        // a fixed layer colour of earlier versions is ignored
        let l: Layer = serde_json::from_str(r##"{"y":"h","color":"#0a0b0c"}"##).unwrap();
        assert_eq!(l.y, "h");
        let f = Figure { palette: Palette::Custom, colors: vec![Color([1, 2, 3, 4])], ..Default::default() };
        let back: Figure = serde_json::from_str(&serde_json::to_string(&f).unwrap()).unwrap();
        assert_eq!(back, f);
        // custom colours with names (earlier versions) are read, too
        let g: Figure = serde_json::from_str(r##"{"colors":[{"name":"loam","color":"#010203"},"#040506"]}"##).unwrap();
        assert_eq!(g.colors, vec![Color([1, 2, 3, 255]), Color([4, 5, 6, 255])]);
    }
}
