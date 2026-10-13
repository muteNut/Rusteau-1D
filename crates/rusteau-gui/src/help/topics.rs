//! The help topics (files `help/<id>.md` of the crate, compiled into the program), their table of contents, the
//! embedded images and the generated topic "References".

use super::markup::{self, Block, Doc, Inline, Link, Span};
use super::refs;
use std::sync::OnceLock;

/// Entry of the table of contents: a book (chapter) with entries, or a topic.
pub enum Toc {
    Book(&'static str, &'static [Toc]),
    Page(&'static str),
}

use Toc::{Book, Page};

/// Table of contents, in the order of the HYDRUS-1D help: introduction, the input pages, the results, the theory and
/// the reference part.
pub const TOC: &[Toc] = &[
    Book(
        "Introduction",
        &[Page("contents"), Page("introduction"), Page("getting_started"), Page("interface"), Page("workflow"), Page("units"), Page("hydrus_relation")],
    ),
    Book("Projects and files", &[Page("projects"), Page("examples"), Page("output_files"), Page("cli")]),
    Book(
        "Input (pre-processing)",
        &[
            Page("processes"),
            Page("time_output"),
            Page("hyd_model"),
            Page("soil_params"),
            Page("rosetta"),
            Page("profile_editor"),
            Page("water_bc"),
            Page("time_variable_bc"),
            Page("meteo_page"),
            Page("root_uptake_page"),
            Page("heat_page"),
            Book("Solute transport", &[Page("solute_general"), Page("solute_params"), Page("solute_reactions"), Page("solute_advanced"), Page("solute_bc")]),
            Page("iteration"),
            Page("further_options"),
            Page("check_run"),
            Page("inverse_page"),
        ],
    ),
    Book(
        "Results (post-processing)",
        &[
            Page("res_obs"),
            Page("res_profiles"),
            Page("res_fluxes"),
            Page("res_soilprops"),
            Page("res_runtime"),
            Page("res_balance"),
            Page("res_inverse"),
            Page("measured_data"),
            Page("figures"),
            Page("run_comparison"),
        ],
    ),
    Book(
        "Theory and equations",
        &[
            Page("th_richards"),
            Page("th_hydraulic_models"),
            Page("th_hysteresis"),
            Page("th_scaling"),
            Page("th_dual_porosity"),
            Page("th_dual_permeability"),
            Page("th_root_uptake"),
            Page("th_root_solute"),
            Page("th_bc_water"),
            Page("th_atmosphere"),
            Page("th_penman"),
            Page("th_hargreaves"),
            Page("th_energy_balance"),
            Page("th_vapor"),
            Page("th_heat"),
            Page("th_solute"),
            Page("th_nonequilibrium"),
            Page("th_colloids"),
            Page("th_reaction_dependence"),
            Page("th_numerics"),
            Page("th_discretization"),
            Page("th_inverse"),
            Page("th_advanced_options"),
            Page("th_rosetta"),
        ],
    ),
    Book("Reference", &[Page("glossary"), Page("shortcuts"), Page("faq"), Page("references"), Page("credits")]),
];

/// Topic sources.
pub struct Source {
    pub id: &'static str,
    pub src: &'static str,
}

macro_rules! topics {
    ($($id:literal),* $(,)?) => {
        &[$(Source { id: $id, src: include_str!(concat!("../../help/", $id, ".md")) }),*]
    };
}

pub const SOURCES: &[Source] = topics![
    "contents", "introduction", "getting_started", "interface", "workflow", "units", "hydrus_relation",
    "projects", "examples", "output_files", "cli",
    "processes", "time_output", "hyd_model", "soil_params", "rosetta", "profile_editor", "water_bc", "time_variable_bc",
    "meteo_page", "root_uptake_page", "heat_page", "solute_general", "solute_params", "solute_reactions",
    "solute_advanced", "solute_bc", "iteration", "further_options", "check_run", "inverse_page",
    "res_obs", "res_profiles", "res_fluxes", "res_soilprops", "res_runtime", "res_balance", "res_inverse", "measured_data",
    "figures", "run_comparison",
    "th_richards", "th_hydraulic_models", "th_hysteresis", "th_scaling", "th_dual_porosity", "th_dual_permeability",
    "th_root_uptake", "th_root_solute", "th_bc_water", "th_atmosphere", "th_penman", "th_hargreaves",
    "th_energy_balance", "th_vapor", "th_heat", "th_solute", "th_nonequilibrium", "th_colloids",
    "th_reaction_dependence", "th_numerics", "th_discretization", "th_inverse", "th_advanced_options", "th_rosetta",
    "glossary", "shortcuts", "faq", "credits",
];

macro_rules! images {
    ($($name:literal),* $(,)?) => {
        &[$((concat!("img/", $name), include_bytes!(concat!("../../help/img/", $name)) as &[u8])),*]
    };
}

/// Images of the topics (screenshots), referenced as `img/<name>`.
pub const IMAGES: &[(&str, &[u8])] = images![
    "main_window.png",
    "processes.png",
    "time_output.png",
    "hyd_model.png",
    "soil_params.png",
    "rosetta.png",
    "profile_editor.png",
    "profile_table.png",
    "water_bc.png",
    "time_variable_bc.png",
    "meteo_page.png",
    "root_uptake_page.png",
    "heat_page.png",
    "solute_general.png",
    "solute_params.png",
    "solute_reactions.png",
    "solute_bc.png",
    "iteration.png",
    "further_options.png",
    "check_run.png",
    "inverse_page.png",
    "res_obs.png",
    "res_profiles.png",
    "res_fluxes.png",
    "res_soilprops.png",
    "res_runtime.png",
    "res_balance.png",
    "res_inverse.png",
    "help_window.png",
    "tooltip.png",
    "figures.png",
    "figures_layer.png",
    "figures_comparison.png",
    "figure_example.png",
];

pub fn image(src: &str) -> Option<&'static [u8]> {
    IMAGES.iter().find(|(n, _)| *n == src).map(|(_, b)| *b)
}

/// A parsed topic.
pub struct Topic {
    pub id: &'static str,
    pub doc: Doc,
    /// Lower-case plain text (search)
    pub text: String,
}

/// All topics, parsed once.
pub fn all() -> &'static [Topic] {
    static T: OnceLock<Vec<Topic>> = OnceLock::new();
    T.get_or_init(|| {
        let mut v: Vec<Topic> = SOURCES
            .iter()
            .map(|s| {
                let mut doc = markup::parse(s.src);
                append_cited(&mut doc);
                let text = markup::doc_text(&doc).to_lowercase();
                Topic { id: s.id, doc, text }
            })
            .collect();
        let doc = references_doc();
        let text = markup::doc_text(&doc).to_lowercase();
        v.push(Topic { id: "references", doc, text });
        v
    })
}

pub fn get(id: &str) -> Option<&'static Topic> {
    all().iter().find(|t| t.id == id)
}

pub fn title(id: &str) -> &'static str {
    get(id).map(|t| t.doc.title.as_str()).unwrap_or("?")
}

/// Topic ids in the order of the table of contents.
pub fn order() -> Vec<&'static str> {
    fn walk(t: &'static [Toc], out: &mut Vec<&'static str>) {
        for e in t {
            match e {
                Book(_, c) => walk(c, out),
                Page(id) => out.push(id),
            }
        }
    }
    let mut v = vec![];
    walk(TOC, &mut v);
    v
}

/// Titles of the books that contain topic `id` (outermost first).
pub fn path_of(id: &str) -> Vec<&'static str> {
    fn walk(t: &'static [Toc], id: &str, path: &mut Vec<&'static str>) -> bool {
        for e in t {
            match e {
                Book(name, c) => {
                    path.push(name);
                    if walk(c, id, path) {
                        return true;
                    }
                    path.pop();
                }
                Page(p) if *p == id => return true,
                _ => {}
            }
        }
        false
    }
    let mut v = vec![];
    walk(TOC, id, &mut v);
    v
}

/// Citation of a reference as an inline run: the full text and a DOI or web link.
pub fn citation(r: &refs::Ref) -> Inline {
    let mut v = markup::inline(r.full);
    if let Some(doi) = r.doi {
        v.push(Span { text: " doi:".into(), ..Default::default() });
        v.push(Span { text: doi.into(), link: Some(Link::Url(format!("https://doi.org/{doi}"))), ..Default::default() });
    } else if let Some(url) = r.url {
        v.push(Span { text: " ".into(), ..Default::default() });
        v.push(Span { text: url.into(), link: Some(Link::Url(url.into())), ..Default::default() });
    }
    v
}

/// Keys of all citations in the blocks, in the order of their first appearance.
pub fn cited(blocks: &[Block]) -> Vec<String> {
    let mut keys: Vec<String> = vec![];
    let mut add = |inl: &Inline| {
        for s in inl {
            if let Some(Link::Cite(k)) = &s.link {
                if !keys.contains(k) {
                    keys.push(k.clone());
                }
            }
        }
    };
    for b in blocks {
        match b {
            Block::Heading { text, .. } | Block::Para(text) | Block::Note(text) => add(text),
            Block::List { items, .. } => items.iter().for_each(&mut add),
            Block::Table { head, rows } => head.iter().chain(rows.iter().flatten()).for_each(&mut add),
            Block::Formula(lines) => lines.iter().for_each(&mut add),
            Block::Image { caption, .. } => add(caption),
            Block::Rule | Block::Anchor(_) | Block::Code(_) => {}
        }
    }
    keys
}

/// Appends the list of the references cited in the topic ("Literature"), as at the end of a paper.
fn append_cited(doc: &mut Doc) {
    let mut keys = cited(&doc.blocks);
    if keys.is_empty() {
        return;
    }
    keys.sort_by_key(|k| refs::find(k).map(|r| r.full.to_lowercase()).unwrap_or_default());
    doc.blocks.push(Block::Heading { level: 2, text: markup::inline("Literature cited in this topic"), anchor: "literature".into() });
    let items = keys.iter().filter_map(|k| refs::find(k)).map(citation).collect();
    doc.blocks.push(Block::List { ordered: false, items });
    let mut tail = markup::inline("All sources: ");
    tail.push(Span { text: "References".into(), link: Some(Link::Topic { id: "references".into(), anchor: None }), ..Default::default() });
    tail.push(Span { text: ".".into(), ..Default::default() });
    doc.blocks.push(Block::Para(tail));
}

/// The topic "References": all sources, alphabetically, each with what Rusteau-1D bases on it.
fn references_doc() -> Doc {
    let mut blocks = vec![Block::Para(markup::inline(
        "The equations and methods of Rusteau-1D come from the HYDRUS-1D code (version 4.08) and its technical manual \
         [@simunek2009], which the numerical engine ports, and from the publications below. Each entry names what \
         Rusteau-1D bases on it. Citations in the help topics link to this list; each topic also ends with the \
         sources it cites.",
    ))];
    let mut letter = ' ';
    for r in refs::sorted() {
        let f = refs::letter(r);
        if f != letter {
            letter = f;
            blocks.push(Block::Heading { level: 3, text: markup::inline(&f.to_string()), anchor: format!("letter-{}", f.to_ascii_lowercase()) });
        }
        blocks.push(Block::Anchor(r.key.to_string()));
        blocks.push(Block::Para(citation(r)));
        let mut used = vec![Span { text: "Basis for: ".into(), italic: true, ..Default::default() }];
        used.extend(markup::inline(r.used_for).into_iter().map(|mut s| {
            s.italic = true;
            s
        }));
        blocks.push(Block::Note(used));
    }
    Doc { title: "References".into(), keywords: vec!["references".into(), "literature".into(), "bibliography".into(), "citations".into(), "sources".into()], blocks }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn links(doc: &Doc) -> Vec<Link> {
        let mut out = vec![];
        let mut add = |inl: &Inline| out.extend(inl.iter().filter_map(|s| s.link.clone()));
        for b in &doc.blocks {
            match b {
                Block::Heading { text, .. } | Block::Para(text) | Block::Note(text) => add(text),
                Block::List { items, .. } => items.iter().for_each(&mut add),
                Block::Table { head, rows } => head.iter().chain(rows.iter().flatten()).for_each(&mut add),
                Block::Formula(lines) => lines.iter().for_each(&mut add),
                Block::Image { caption, .. } => add(caption),
                Block::Rule | Block::Anchor(_) | Block::Code(_) => {}
            }
        }
        out
    }

    fn anchors(doc: &Doc) -> Vec<String> {
        doc.blocks
            .iter()
            .filter_map(|b| match b {
                Block::Heading { anchor, .. } => Some(anchor.clone()),
                Block::Anchor(a) => Some(a.clone()),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn toc_and_sources_agree() {
        let order = order();
        for s in SOURCES {
            assert!(order.contains(&s.id), "topic {} is not in the table of contents", s.id);
        }
        for id in &order {
            assert!(get(id).is_some(), "table of contents names the missing topic {id}");
        }
        let mut o = order.clone();
        o.sort_unstable();
        o.dedup();
        assert_eq!(o.len(), order.len(), "a topic appears twice in the table of contents");
    }

    #[test]
    fn every_topic_has_a_title() {
        for t in all() {
            assert!(!t.doc.title.is_empty(), "topic {} has no title", t.id);
        }
    }

    #[test]
    fn links_citations_and_images_resolve() {
        for t in all() {
            for l in links(&t.doc) {
                match l {
                    Link::Topic { id, anchor } => {
                        let target = get(&id).unwrap_or_else(|| panic!("{}: link to the missing topic {id}", t.id));
                        if let Some(a) = anchor {
                            assert!(anchors(&target.doc).contains(&a), "{}: link to the missing anchor {id}#{a}", t.id);
                        }
                    }
                    Link::Cite(k) => assert!(refs::find(&k).is_some(), "{}: citation of the missing reference {k}", t.id),
                    Link::Url(u) => assert!(u.starts_with("https://") || u.starts_with("http://") || u.starts_with("mailto:"), "{}: {u}", t.id),
                }
            }
            for b in &t.doc.blocks {
                if let Block::Image { src, .. } = b {
                    assert!(image(src).is_some(), "{}: missing image {src}", t.id);
                }
            }
        }
    }

    #[test]
    fn every_reference_is_cited() {
        let mut keys: Vec<String> = vec![];
        for t in all().iter().filter(|t| t.id != "references") {
            keys.extend(cited(&t.doc.blocks));
        }
        for r in refs::REFS {
            assert!(keys.iter().any(|k| k == r.key), "reference {} is cited in no topic", r.key);
        }
    }

    #[test]
    fn images_are_png() {
        for (n, b) in IMAGES {
            assert!(b.starts_with(b"\x89PNG"), "{n} is not a PNG file");
        }
    }
}
