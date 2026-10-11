//! The help as a set of static HTML pages (offline help for a web browser, like the compiled HTML help of
//! HYDRUS-1D): one page per topic with the table of contents, a keyword index, a search page and the images.

use super::markup::{Block, Doc, Inline, Link, Script};
use super::{refs, topics};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

fn esc(s: &str) -> String {
    let mut o = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => o.push_str("&amp;"),
            '<' => o.push_str("&lt;"),
            '>' => o.push_str("&gt;"),
            '"' => o.push_str("&quot;"),
            c => o.push(c),
        }
    }
    o
}

fn href(l: &Link) -> String {
    match l {
        Link::Topic { id, anchor: Some(a) } => format!("{id}.html#{a}"),
        Link::Topic { id, anchor: None } => format!("{id}.html"),
        Link::Url(u) => u.clone(),
        Link::Cite(k) => format!("references.html#{k}"),
    }
}

fn inline(inl: &Inline) -> String {
    let mut o = String::new();
    for s in inl {
        let mut t = esc(&s.text);
        if s.code {
            t = format!("<code>{t}</code>");
        }
        if s.italic {
            t = format!("<em>{t}</em>");
        }
        if s.bold {
            t = format!("<strong>{t}</strong>");
        }
        t = match s.script {
            Script::Sub => format!("<sub>{t}</sub>"),
            Script::Sup => format!("<sup>{t}</sup>"),
            Script::Normal => t,
        };
        if let Some(l) = &s.link {
            let ext = matches!(l, Link::Url(_));
            let title = match l {
                Link::Cite(k) => refs::find(k).map(|r| super::markup::plain(&super::markup::inline(r.full))).unwrap_or_default(),
                _ => String::new(),
            };
            let class = if matches!(l, Link::Cite(_)) { " class=\"cite\"" } else { "" };
            t = format!(
                "<a href=\"{}\"{class}{}{}>{t}</a>",
                esc(&href(l)),
                if title.is_empty() { String::new() } else { format!(" title=\"{}\"", esc(&title)) },
                if ext { " target=\"_blank\" rel=\"noopener\"" } else { "" }
            );
        }
        o.push_str(&t);
    }
    o
}

fn blocks(doc: &Doc) -> String {
    let mut o = String::new();
    for b in &doc.blocks {
        match b {
            Block::Heading { level, text, anchor } => {
                let _ = writeln!(o, "<h{level} id=\"{}\">{}</h{level}>", esc(anchor), inline(text));
            }
            Block::Anchor(a) => {
                let _ = writeln!(o, "<span class=\"anchor\" id=\"{}\"></span>", esc(a));
            }
            Block::Para(t) => {
                let _ = writeln!(o, "<p>{}</p>", inline(t));
            }
            Block::Note(t) => {
                let _ = writeln!(o, "<div class=\"note\">{}</div>", inline(t));
            }
            Block::List { ordered, items } => {
                let tag = if *ordered { "ol" } else { "ul" };
                let _ = writeln!(o, "<{tag}>");
                for it in items {
                    let _ = writeln!(o, "<li>{}</li>", inline(it));
                }
                let _ = writeln!(o, "</{tag}>");
            }
            Block::Formula(lines) => {
                let _ = writeln!(o, "<div class=\"formula\">");
                for l in lines {
                    let _ = writeln!(o, "<div>{}</div>", inline(l));
                }
                let _ = writeln!(o, "</div>");
            }
            Block::Table { head, rows } => {
                let _ = writeln!(o, "<div class=\"table\"><table>");
                if !head.is_empty() {
                    let _ = write!(o, "<thead><tr>");
                    for c in head {
                        let _ = write!(o, "<th>{}</th>", inline(c));
                    }
                    let _ = writeln!(o, "</tr></thead>");
                }
                let _ = writeln!(o, "<tbody>");
                for r in rows {
                    let _ = write!(o, "<tr>");
                    for c in r {
                        let _ = write!(o, "<td>{}</td>", inline(c));
                    }
                    let _ = writeln!(o, "</tr>");
                }
                let _ = writeln!(o, "</tbody></table></div>");
            }
            Block::Image { src, caption } => {
                let cap = inline(caption);
                let alt = esc(&super::markup::plain(caption));
                let _ = writeln!(o, "<figure><a href=\"{0}\"><img src=\"{0}\" alt=\"{alt}\" loading=\"lazy\"></a><figcaption>{cap}</figcaption></figure>", esc(src));
            }
            Block::Rule => {
                let _ = writeln!(o, "<hr>");
            }
            Block::Code(c) => {
                let _ = writeln!(o, "<pre><code>{}</code></pre>", esc(c));
            }
        }
    }
    o
}

fn toc_html(current: &str) -> String {
    fn walk(t: &'static [topics::Toc], current: &str, path: &[&str], o: &mut String) {
        let _ = writeln!(o, "<ul>");
        for e in t {
            match e {
                topics::Toc::Book(name, c) => {
                    let open = if path.contains(name) || current == "index" { " open" } else { "" };
                    let _ = writeln!(o, "<li class=\"book\"><details{open}><summary>{}</summary>", esc(name));
                    walk(c, current, path, o);
                    let _ = writeln!(o, "</details></li>");
                }
                topics::Toc::Page(id) => {
                    let cls = if *id == current { " class=\"current\"" } else { "" };
                    let _ = writeln!(o, "<li{cls}><a href=\"{id}.html\">{}</a></li>", esc(topics::title(id)));
                }
            }
        }
        let _ = writeln!(o, "</ul>");
    }
    let mut o = String::new();
    let path = topics::path_of(current);
    walk(topics::TOC, current, &path, &mut o);
    o
}

fn page(title: &str, current: &str, body: &str, pager: &str) -> String {
    format!(
        "<!doctype html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\">\n<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n\
         <title>{t} – Rusteau-1D Help</title>\n<link rel=\"stylesheet\" href=\"style.css\">\n</head>\n<body>\n\
         <header><a class=\"brand\" href=\"contents.html\">Rusteau-1D Help</a>\
         <nav class=\"top\"><a href=\"contents.html\">Contents</a><a href=\"keywords.html\">Index</a><a href=\"references.html\">References</a></nav>\
         <form class=\"search\" action=\"search.html\"><input type=\"search\" name=\"q\" placeholder=\"Search the help\" aria-label=\"Search the help\"></form></header>\n\
         <div class=\"layout\">\n<nav class=\"toc\" aria-label=\"Contents\">\n{toc}</nav>\n<main>\n<article>\n{body}</article>\n{pager}</main>\n</div>\n\
         <footer class=\"foot\">Rusteau-1D {v} · offline help · free software under the GNU GPL v3 or later</footer>\n</body>\n</html>\n",
        t = esc(title),
        toc = toc_html(current),
        v = env!("CARGO_PKG_VERSION"),
    )
}

fn breadcrumb(id: &str) -> String {
    let p = topics::path_of(id);
    if p.is_empty() {
        return String::new();
    }
    format!("<div class=\"crumbs\">{}</div>\n", p.iter().map(|s| esc(s)).collect::<Vec<_>>().join(" › "))
}

/// Writes the help into `dir` (created if needed) and returns the path of the start page.
pub fn export(dir: &Path) -> std::io::Result<PathBuf> {
    std::fs::create_dir_all(dir.join("img"))?;
    let order = topics::order();
    for (k, id) in order.iter().enumerate() {
        let t = topics::get(id).expect("topic");
        let mut body = breadcrumb(id);
        let _ = writeln!(body, "<h1>{}</h1>", inline(&super::markup::inline(&t.doc.title)));
        body.push_str(&blocks(&t.doc));
        let mut pager = String::from("<div class=\"pager\">");
        if k > 0 {
            let p = order[k - 1];
            let _ = write!(pager, "<a class=\"prev\" href=\"{p}.html\">← {}</a>", esc(topics::title(p)));
        }
        if k + 1 < order.len() {
            let n = order[k + 1];
            let _ = write!(pager, "<a class=\"next\" href=\"{n}.html\">{} →</a>", esc(topics::title(n)));
        }
        pager.push_str("</div>\n");
        std::fs::write(dir.join(format!("{id}.html")), page(&t.doc.title, id, &body, &pager))?;
    }
    // start page
    std::fs::write(
        dir.join("index.html"),
        "<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\"><meta http-equiv=\"refresh\" content=\"0; url=contents.html\">\
         <title>Rusteau-1D Help</title></head><body><a href=\"contents.html\">Rusteau-1D Help</a></body></html>\n",
    )?;
    // keyword index
    let mut body = String::from("<h1>Index</h1>\n<p>Keywords and section headings of all topics.</p>\n<ul class=\"index\">\n");
    for (k, id, anchor) in super::keyword_index() {
        let target = match &anchor {
            Some(a) => format!("{id}.html#{a}"),
            None => format!("{id}.html"),
        };
        let _ = writeln!(body, "<li><a href=\"{}\">{}</a> <span class=\"where\">{}</span></li>", esc(&target), esc(&k), esc(topics::title(id)));
    }
    body.push_str("</ul>\n");
    std::fs::write(dir.join("keywords.html"), page("Index", "keywords", &body, ""))?;
    // search
    let body = "<h1>Search</h1>\n<p id=\"summary\"></p>\n<ol id=\"results\" class=\"results\"></ol>\n\
                <noscript><p>The search needs JavaScript. Use the <a href=\"keywords.html\">index</a> instead.</p></noscript>\n\
                <script src=\"search_index.js\"></script>\n<script src=\"search.js\"></script>\n";
    std::fs::write(dir.join("search.html"), page("Search", "search", body, ""))?;
    let mut idx = String::from("window.HELP_INDEX = [\n");
    for t in topics::all() {
        let _ = writeln!(idx, "  {{\"id\": {}, \"title\": {}, \"text\": {}}},", json(t.id), json(&t.doc.title), json(&super::markup::doc_text(&t.doc)));
    }
    idx.push_str("];\n");
    std::fs::write(dir.join("search_index.js"), idx)?;
    std::fs::write(dir.join("search.js"), SEARCH_JS)?;
    std::fs::write(dir.join("style.css"), STYLE_CSS)?;
    for (name, bytes) in topics::IMAGES {
        std::fs::write(dir.join(name), bytes)?;
    }
    Ok(dir.join("index.html"))
}

fn json(s: &str) -> String {
    let mut o = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            '\n' => o.push_str("\\n"),
            '\r' => {}
            '\t' => o.push(' '),
            '<' => o.push_str("\\u003c"),
            c if (c as u32) < 0x20 => {}
            c => o.push(c),
        }
    }
    o.push('"');
    o
}

const SEARCH_JS: &str = r#"(function () {
  var q = (new URLSearchParams(window.location.search).get("q") || "").trim();
  var box = document.querySelector("form.search input");
  if (box) { box.value = q; }
  var out = document.getElementById("results");
  var summary = document.getElementById("summary");
  var terms = q.toLowerCase().split(/\s+/).filter(function (t) { return t.length > 0; });
  if (terms.length === 0) { summary.textContent = "Enter words to search for."; return; }
  var hits = [];
  window.HELP_INDEX.forEach(function (t) {
    var text = t.text.toLowerCase(), title = t.title.toLowerCase(), score = 0, ok = true;
    terms.forEach(function (w) {
      var n = text.split(w).length - 1;
      if (n === 0) { ok = false; }
      score += n + (title.indexOf(w) >= 0 ? 20 : 0);
    });
    if (ok) { hits.push({ t: t, score: score }); }
  });
  hits.sort(function (a, b) { return b.score - a.score; });
  summary.textContent = hits.length + (hits.length === 1 ? " topic" : " topics") + " contain " + terms.map(function (w) { return "“" + w + "”"; }).join(" and ") + ".";
  hits.forEach(function (h) {
    var li = document.createElement("li");
    var a = document.createElement("a");
    a.href = h.t.id + ".html";
    a.textContent = h.t.title;
    li.appendChild(a);
    var text = h.t.text, i = text.toLowerCase().indexOf(terms[0]);
    var s = Math.max(0, i - 80), snip = text.substring(s, i + 120).replace(/\s+/g, " ");
    var p = document.createElement("p");
    p.textContent = (s > 0 ? "…" : "") + snip + "…";
    li.appendChild(p);
    out.appendChild(li);
  });
})();
"#;

const STYLE_CSS: &str = r#":root {
  --bg: #f7f8fa; --panel: #ffffff; --text: #1d2330; --weak: #5d6575; --line: #d8dde6; --accent: #2f6fcf;
  --faint: #eef1f6; --code: #eef1f6; --formula: #f3f5f9; --note: #f0f5ff; --head: #e7ecf4; --current: #dfe9fb;
}
@media (prefers-color-scheme: dark) {
  :root:not([data-theme="light"]) {
    --bg: #1a1c20; --panel: #202227; --text: #dde1e8; --weak: #9aa1ad; --line: #3a3e47; --accent: #78aef0;
    --faint: #262930; --code: #2a2d35; --formula: #15171a; --note: #1f2a3a; --head: #2b2f38; --current: #24364f;
  }
}
* { box-sizing: border-box; }
html, body { margin: 0; padding: 0; }
body { background: var(--bg); color: var(--text); font: 16px/1.6 "Segoe UI", "Noto Sans", "DejaVu Sans", Ubuntu, Arial, sans-serif; }
a { color: var(--accent); text-decoration: none; }
a:hover { text-decoration: underline; }
header { position: sticky; top: 0; z-index: 2; display: flex; flex-wrap: wrap; gap: 8px 24px; align-items: center;
  padding: 10px 16px; background: var(--panel); border-bottom: 1px solid var(--line); }
header .brand { font-weight: 700; font-size: 18px; color: var(--text); }
header nav.top a { margin-right: 16px; }
header form.search { margin-left: auto; }
header input { width: 240px; max-width: 60vw; padding: 6px 10px; border: 1px solid var(--line); border-radius: 6px; background: var(--bg); color: var(--text); font: inherit; }
.layout { display: grid; grid-template-columns: minmax(220px, 300px) 1fr; min-height: calc(100vh - 110px); }
nav.toc { border-right: 1px solid var(--line); padding: 12px 8px 24px 8px; background: var(--panel); position: sticky; top: 55px; align-self: start; height: calc(100vh - 55px); overflow: auto; font-size: 14.5px; }
nav.toc ul { list-style: none; margin: 0; padding-left: 14px; }
nav.toc > ul { padding-left: 4px; }
nav.toc li { margin: 1px 0; }
nav.toc li.current > a { background: var(--current); border-radius: 4px; padding: 1px 6px; margin-left: -6px; font-weight: 600; }
nav.toc summary { cursor: pointer; font-weight: 600; color: var(--text); padding: 2px 0; }
main { padding: 16px 28px 32px 28px; min-width: 0; }
article { max-width: 920px; }
.crumbs { color: var(--weak); font-size: 14px; }
h1 { font-size: 28px; margin: 4px 0 12px 0; }
h2 { font-size: 21px; margin: 28px 0 8px 0; padding-bottom: 4px; border-bottom: 1px solid var(--line); }
h3 { font-size: 17px; margin: 20px 0 6px 0; }
code { background: var(--code); padding: 1px 5px; border-radius: 4px; font: 14px/1.4 "Cascadia Mono", Consolas, "DejaVu Sans Mono", monospace; }
pre { background: var(--formula); border: 1px solid var(--line); border-radius: 6px; padding: 10px 14px; overflow-x: auto; }
pre code { background: none; padding: 0; }
.formula { background: var(--formula); border: 1px solid var(--line); border-radius: 6px; padding: 10px 18px; margin: 10px 0;
  font-family: "Cambria Math", "STIX Two Math", "Latin Modern Math", "Times New Roman", "DejaVu Serif", serif; font-size: 17px; overflow-x: auto; }
.formula div { white-space: pre; }
.note { background: var(--note); border-left: 4px solid var(--accent); padding: 8px 14px; border-radius: 4px; margin: 10px 0; }
.table { overflow-x: auto; margin: 10px 0; }
table { border-collapse: collapse; font-size: 15px; }
th, td { border: 1px solid var(--line); padding: 5px 9px; vertical-align: top; text-align: left; }
thead th { background: var(--head); }
tbody tr:nth-child(odd) { background: var(--faint); }
figure { margin: 14px 0; }
figure img { max-width: 100%; height: auto; border: 1px solid var(--line); border-radius: 4px; }
figcaption { color: var(--weak); font-style: italic; font-size: 14.5px; }
a.cite { white-space: nowrap; }
.anchor { display: block; position: relative; top: -64px; visibility: hidden; }
.pager { display: flex; justify-content: space-between; gap: 16px; max-width: 920px; margin-top: 28px; padding-top: 12px; border-top: 1px solid var(--line); }
.pager .next { margin-left: auto; }
ul.index { columns: 2 320px; }
ul.index .where { color: var(--weak); font-size: 13.5px; }
ol.results li { margin-bottom: 10px; }
ol.results p { margin: 2px 0; color: var(--weak); font-size: 14.5px; }
.foot { color: var(--weak); font-size: 13px; padding: 10px 16px; border-top: 1px solid var(--line); background: var(--panel); }
@media (max-width: 760px) {
  .layout { grid-template-columns: 1fr; }
  nav.toc { position: static; max-height: none; border-right: none; border-bottom: 1px solid var(--line); }
  main { padding: 12px 16px; }
  header form.search { margin-left: 0; }
}
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn export_writes_every_topic_and_image() {
        let dir = std::env::temp_dir().join(format!("rusteau-help-test-{}", std::process::id()));
        let start = export(&dir).unwrap();
        assert!(start.exists());
        for id in topics::order() {
            let s = std::fs::read_to_string(dir.join(format!("{id}.html"))).unwrap();
            assert!(s.contains("<h1>"), "{id}");
            // every local link of the page points to an existing file
            for part in s.split("href=\"").skip(1) {
                let target = &part[..part.find('"').unwrap()];
                if target.starts_with("http") || target.starts_with("mailto:") {
                    continue;
                }
                let file = target.split('#').next().unwrap();
                assert!(dir.join(file).exists(), "{id}.html links to the missing file {file}");
            }
        }
        for (name, _) in topics::IMAGES {
            assert!(dir.join(name).exists());
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The HTML help in `docs/help` of the repository is generated from the topics: it must be up to date.
    #[test]
    fn committed_html_help_is_current() {
        let committed = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/help");
        if !committed.exists() {
            return;
        }
        let dir = std::env::temp_dir().join(format!("rusteau-help-current-{}", std::process::id()));
        export(&dir).unwrap();
        let mut stale = vec![];
        let mut names: Vec<String> = std::fs::read_dir(&dir).unwrap().flatten().map(|e| e.file_name().to_string_lossy().to_string()).collect();
        names.extend(topics::IMAGES.iter().map(|(n, _)| n.to_string()));
        for name in names.iter().filter(|n| *n != "img") {
            let new = std::fs::read(dir.join(name)).unwrap();
            // a checkout on Windows may have converted the line ends of the text files
            let mut old = std::fs::read(committed.join(name)).unwrap_or_default();
            if !name.ends_with(".png") {
                old.retain(|b| *b != b'\r');
            }
            if old != new {
                stale.push(name.clone());
            }
        }
        let _ = std::fs::remove_dir_all(&dir);
        assert!(
            stale.is_empty(),
            "docs/help is out of date ({}); regenerate it with: cargo run --release -p rusteau-gui -- --export-help docs/help",
            stale.join(", ")
        );
    }

    #[test]
    fn html_escapes_and_scripts() {
        let s = inline(&super::super::markup::inline("a < b, θ_{r}, h^{2}, **x**"));
        assert_eq!(s, "a &lt; b, θ<sub>r</sub>, h<sup>2</sup>, <strong>x</strong>");
    }
}
