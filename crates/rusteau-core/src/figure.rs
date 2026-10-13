//! Figures (diagrams) of a project, defined in the figure editor of the interface and stored with the project. Data
//! only: the simulation does not use them. A figure has one or more panels in a grid; a panel has axes, data layers
//! (series of x–y values taken from the results, the input, measured data or data files) and annotations.

use serde::{Deserialize, Serialize};

/// RGB colour.
pub type Rgb = [u8; 3];

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
    /// fixed colour; `None`: the next colours of the palette
    pub color: Option<Rgb>,
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
            color: None,
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
    pub color: Rgb,
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
            color: [0, 0, 0],
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
}

impl Default for Panel {
    fn default() -> Self {
        Panel { title: String::new(), x: Axis::default(), y: Axis::default(), y2: Axis { grid: false, ..Axis::default() }, layers: vec![Layer::default()], annotations: vec![], legend: LegendPos::TopRight }
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
        // a figure with missing fields takes the defaults
        let g: Figure = serde_json::from_str(r#"{"name":"F","panels":[{"layers":[{"y":"theta"}]}]}"#).unwrap();
        assert_eq!(g.panels[0].layers[0].y, "theta");
        assert_eq!(g.panels[0].layers[0].x, "t");
        assert_eq!(g.width_mm, 160.0);
    }
}
