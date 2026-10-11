//! The markup of the help topics: a small subset of Markdown with sub- and superscripts and citations.
//!
//! Blocks:
//! * `# Title` (first line of a topic), then optionally `keywords: a; b; c` (entries of the index)
//! * `## Heading` and `### Heading`, optionally with an anchor `{#name}` at the end
//! * paragraphs (lines joined until a blank line)
//! * lists: `- item` or `1. item`; indented lines continue the item
//! * tables: `| a | b |` rows, the second row `|---|---|` separates the header
//! * formulas: `$$ … $$` on one line, or `$$` lines around several formula lines
//! * images: `![caption](img/file.png)`
//! * notes: `> text`
//! * `---`: horizontal rule
//! * `{#name}` on a line of its own: link target
//! * code: lines between two lines starting with three back-ticks
//!
//! Inline: `**bold**`, `*italic*`, `` `code` ``, `x_{sub}`, `x^{sup}`, `[text](topic)`, `[text](topic#anchor)`,
//! `[text](https://…)`, citations `[@key]` or `[@key1; @key2]` (see [`super::refs`]), `\` escapes the next character.

use super::refs;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Script {
    #[default]
    Normal,
    Sub,
    Sup,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Link {
    /// Another topic (`id`, optional anchor of a heading)
    Topic { id: String, anchor: Option<String> },
    /// External address
    Url(String),
    /// Entry of the references (key of [`refs::REFS`])
    Cite(String),
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Span {
    pub text: String,
    pub bold: bool,
    pub italic: bool,
    pub code: bool,
    pub script: Script,
    pub link: Option<Link>,
}

/// A run of text with its formatting.
pub type Inline = Vec<Span>;

#[derive(Clone, Debug, PartialEq)]
pub enum Block {
    Heading { level: u8, text: Inline, anchor: String },
    Para(Inline),
    List { ordered: bool, items: Vec<Inline> },
    Table { head: Vec<Inline>, rows: Vec<Vec<Inline>> },
    /// Displayed formula, one entry per line
    Formula(Vec<Inline>),
    Image { src: String, caption: Inline },
    Note(Inline),
    Rule,
    /// Invisible link target (`{#name}` on a line of its own)
    Anchor(String),
    /// Preformatted text (lines between ``` fences)
    Code(String),
}

#[derive(Clone, Debug, Default)]
pub struct Doc {
    pub title: String,
    pub keywords: Vec<String>,
    pub blocks: Vec<Block>,
}

/// Plain text of an inline run.
pub fn plain(inl: &Inline) -> String {
    inl.iter().map(|s| s.text.as_str()).collect()
}

/// Anchor name of a heading text: lower case, words joined by `-`.
pub fn slug(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        if c.is_alphanumeric() {
            out.extend(c.to_lowercase());
        } else if !out.is_empty() && !out.ends_with('-') {
            out.push('-');
        }
    }
    out.trim_end_matches('-').to_string()
}

#[derive(Clone, Copy, Default)]
struct Style {
    bold: bool,
    italic: bool,
    script: Script,
}

struct InlineParser {
    chars: Vec<char>,
    out: Inline,
}

impl InlineParser {
    fn push(&mut self, text: &str, st: Style, code: bool, link: &Option<Link>) {
        if text.is_empty() {
            return;
        }
        if let Some(last) = self.out.last_mut() {
            if last.bold == st.bold && last.italic == st.italic && last.script == st.script && last.code == code && last.link == *link {
                last.text.push_str(text);
                return;
            }
        }
        self.out.push(Span { text: text.to_string(), bold: st.bold, italic: st.italic, code, script: st.script, link: link.clone() });
    }

    /// Position of the `close` character that ends a group opened just before `from` (nested `open`/`close` count).
    fn matching(&self, from: usize, open: char, close: char) -> Option<usize> {
        let mut depth = 0;
        let mut i = from;
        while i < self.chars.len() {
            let c = self.chars[i];
            if c == '\\' {
                i += 2;
                continue;
            }
            if c == open {
                depth += 1;
            } else if c == close {
                if depth == 0 {
                    return Some(i);
                }
                depth -= 1;
            }
            i += 1;
        }
        None
    }

    /// Parses `chars[start..end]` with style `st` and link `link`.
    fn run(&mut self, start: usize, end: usize, mut st: Style, link: &Option<Link>) {
        let mut buf = String::new();
        let mut i = start;
        macro_rules! flush {
            () => {
                let b = std::mem::take(&mut buf);
                self.push(&b, st, false, link);
            };
        }
        while i < end {
            let c = self.chars[i];
            let next = if i + 1 < end { Some(self.chars[i + 1]) } else { None };
            match c {
                '\\' if next.is_some() => {
                    buf.push(next.unwrap());
                    i += 2;
                }
                '*' if next == Some('*') => {
                    flush!();
                    st.bold = !st.bold;
                    i += 2;
                }
                '*' => {
                    flush!();
                    st.italic = !st.italic;
                    i += 1;
                }
                '`' => {
                    flush!();
                    let close = (i + 1..end).find(|&k| self.chars[k] == '`').unwrap_or(end);
                    let code: String = self.chars[i + 1..close].iter().collect();
                    self.push(&code, Style { script: st.script, ..Default::default() }, true, link);
                    i = close + 1;
                }
                '_' | '^' if next == Some('{') => {
                    if let Some(close) = self.matching(i + 2, '{', '}').filter(|&k| k < end) {
                        flush!();
                        let inner = Style { script: if c == '_' { Script::Sub } else { Script::Sup }, ..st };
                        self.run(i + 2, close, inner, link);
                        i = close + 1;
                    } else {
                        buf.push(c);
                        i += 1;
                    }
                }
                '[' if link.is_none() => {
                    if next == Some('@') {
                        if let Some(close) = (i + 1..end).find(|&k| self.chars[k] == ']') {
                            flush!();
                            let inner: String = self.chars[i + 1..close].iter().collect();
                            let keys: Vec<&str> = inner.split(';').map(|k| k.trim().trim_start_matches('@').trim()).filter(|k| !k.is_empty()).collect();
                            for (k, key) in keys.iter().enumerate() {
                                if k > 0 {
                                    self.push("; ", st, false, &None);
                                }
                                let text = refs::find(key).map(|r| r.short.to_string()).unwrap_or_else(|| format!("[?{key}]"));
                                self.push(&text, st, false, &Some(Link::Cite(key.to_string())));
                            }
                            i = close + 1;
                            continue;
                        }
                    }
                    // [text](target)
                    let close = self.matching(i + 1, '[', ']').filter(|&k| k < end);
                    match close {
                        Some(cl) if cl + 1 < end && self.chars[cl + 1] == '(' => {
                            let paren = (cl + 2..end).find(|&k| self.chars[k] == ')');
                            if let Some(p) = paren {
                                flush!();
                                let target: String = self.chars[cl + 2..p].iter().collect();
                                let l = Some(parse_target(target.trim()));
                                self.run(i + 1, cl, st, &l);
                                i = p + 1;
                            } else {
                                buf.push(c);
                                i += 1;
                            }
                        }
                        _ => {
                            buf.push(c);
                            i += 1;
                        }
                    }
                }
                _ => {
                    buf.push(c);
                    i += 1;
                }
            }
        }
        flush!();
    }
}

fn parse_target(t: &str) -> Link {
    if t.starts_with("http://") || t.starts_with("https://") || t.starts_with("mailto:") {
        return Link::Url(t.to_string());
    }
    match t.split_once('#') {
        Some((id, a)) => Link::Topic { id: id.to_string(), anchor: Some(a.to_string()) },
        None => Link::Topic { id: t.to_string(), anchor: None },
    }
}

/// Parses inline markup.
pub fn inline(s: &str) -> Inline {
    let mut p = InlineParser { chars: s.chars().collect(), out: vec![] };
    let n = p.chars.len();
    p.run(0, n, Style::default(), &None);
    p.out
}

fn table_cells(line: &str) -> Vec<String> {
    let t = line.trim();
    let t = t.strip_prefix('|').unwrap_or(t);
    let t = t.strip_suffix('|').unwrap_or(t);
    // split at '|' that are not escaped
    let mut cells = vec![];
    let mut cur = String::new();
    let mut esc = false;
    for c in t.chars() {
        if esc {
            if c != '|' {
                cur.push('\\');
            }
            cur.push(c);
            esc = false;
        } else if c == '\\' {
            esc = true;
        } else if c == '|' {
            cells.push(cur.trim().to_string());
            cur.clear();
        } else {
            cur.push(c);
        }
    }
    cells.push(cur.trim().to_string());
    cells
}

fn is_rule_row(line: &str) -> bool {
    let t = line.trim();
    t.starts_with('|') && t.chars().all(|c| matches!(c, '|' | '-' | ':' | ' '))
}

fn list_item(line: &str) -> Option<(bool, &str)> {
    let t = line.trim_start();
    if line.len() - t.len() > 1 {
        return None;
    }
    if let Some(r) = t.strip_prefix("- ") {
        return Some((false, r));
    }
    let digits = t.chars().take_while(|c| c.is_ascii_digit()).count();
    if digits > 0 && t[digits..].starts_with(". ") {
        return Some((true, &t[digits + 2..]));
    }
    None
}

/// Parses a topic.
pub fn parse(src: &str) -> Doc {
    let mut doc = Doc::default();
    let lines: Vec<&str> = src.lines().collect();
    let mut i = 0;
    let mut para: Vec<&str> = vec![];
    let flush_para = |para: &mut Vec<&str>, blocks: &mut Vec<Block>| {
        if !para.is_empty() {
            blocks.push(Block::Para(inline(&para.join(" "))));
            para.clear();
        }
    };
    while i < lines.len() {
        let line = lines[i];
        let t = line.trim();
        if t.is_empty() {
            flush_para(&mut para, &mut doc.blocks);
            i += 1;
            continue;
        }
        if let Some(title) = t.strip_prefix("# ") {
            flush_para(&mut para, &mut doc.blocks);
            doc.title = title.trim().to_string();
            i += 1;
            continue;
        }
        if let Some(k) = t.strip_prefix("keywords:") {
            doc.keywords.extend(k.split(';').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()));
            i += 1;
            continue;
        }
        if t.starts_with("## ") || t.starts_with("### ") {
            flush_para(&mut para, &mut doc.blocks);
            let level = if t.starts_with("### ") { 3 } else { 2 };
            let mut text = t[level as usize + 1..].trim().to_string();
            let mut anchor = None;
            if let Some(p) = text.rfind("{#") {
                if text.ends_with('}') {
                    anchor = Some(text[p + 2..text.len() - 1].to_string());
                    text = text[..p].trim().to_string();
                }
            }
            let inl = inline(&text);
            let anchor = anchor.unwrap_or_else(|| slug(&plain(&inl)));
            doc.blocks.push(Block::Heading { level, text: inl, anchor });
            i += 1;
            continue;
        }
        if t.starts_with("```") {
            flush_para(&mut para, &mut doc.blocks);
            let mut code = vec![];
            i += 1;
            while i < lines.len() && !lines[i].trim_start().starts_with("```") {
                code.push(lines[i]);
                i += 1;
            }
            i += 1;
            doc.blocks.push(Block::Code(code.join("\n")));
            continue;
        }
        if t.starts_with("{#") && t.ends_with('}') {
            flush_para(&mut para, &mut doc.blocks);
            doc.blocks.push(Block::Anchor(t[2..t.len() - 1].to_string()));
            i += 1;
            continue;
        }
        if t == "---" {
            flush_para(&mut para, &mut doc.blocks);
            doc.blocks.push(Block::Rule);
            i += 1;
            continue;
        }
        if t.starts_with("$$") {
            flush_para(&mut para, &mut doc.blocks);
            let rest = t[2..].trim();
            if rest.ends_with("$$") && !rest.is_empty() {
                doc.blocks.push(Block::Formula(vec![inline(rest[..rest.len() - 2].trim())]));
                i += 1;
            } else {
                let mut f = vec![];
                if !rest.is_empty() {
                    f.push(inline(rest));
                }
                i += 1;
                while i < lines.len() && !lines[i].trim().starts_with("$$") {
                    if !lines[i].trim().is_empty() {
                        f.push(inline(lines[i].trim()));
                    }
                    i += 1;
                }
                i += 1;
                doc.blocks.push(Block::Formula(f));
            }
            continue;
        }
        if t.starts_with("![") {
            if let (Some(cl), true) = (t.find("]("), t.ends_with(')')) {
                flush_para(&mut para, &mut doc.blocks);
                let caption = inline(&t[2..cl]);
                let src = t[cl + 2..t.len() - 1].trim().to_string();
                doc.blocks.push(Block::Image { src, caption });
                i += 1;
                continue;
            }
        }
        if t.starts_with('>') {
            flush_para(&mut para, &mut doc.blocks);
            let mut text = vec![];
            while i < lines.len() && lines[i].trim().starts_with('>') {
                text.push(lines[i].trim()[1..].trim());
                i += 1;
            }
            doc.blocks.push(Block::Note(inline(&text.join(" "))));
            continue;
        }
        if t.starts_with('|') {
            flush_para(&mut para, &mut doc.blocks);
            let mut rows: Vec<Vec<String>> = vec![];
            let mut has_head = false;
            while i < lines.len() && lines[i].trim().starts_with('|') {
                if is_rule_row(lines[i]) {
                    has_head = rows.len() == 1;
                } else {
                    rows.push(table_cells(lines[i]));
                }
                i += 1;
            }
            let conv = |r: &Vec<String>| r.iter().map(|c| inline(c)).collect::<Vec<Inline>>();
            let head = if has_head && !rows.is_empty() { conv(&rows.remove(0)) } else { vec![] };
            doc.blocks.push(Block::Table { head, rows: rows.iter().map(conv).collect() });
            continue;
        }
        if let Some((ordered, first)) = list_item(line) {
            flush_para(&mut para, &mut doc.blocks);
            let mut items: Vec<String> = vec![first.trim().to_string()];
            i += 1;
            while i < lines.len() {
                let l = lines[i];
                if l.trim().is_empty() {
                    // a blank line ends the list unless the next line is another item of it
                    if i + 1 < lines.len() && list_item(lines[i + 1]).is_some_and(|(o, _)| o == ordered) {
                        i += 1;
                        continue;
                    }
                    break;
                }
                if let Some((o, rest)) = list_item(l) {
                    if o != ordered {
                        break;
                    }
                    items.push(rest.trim().to_string());
                } else if l.starts_with("  ") {
                    let last = items.last_mut().unwrap();
                    last.push(' ');
                    last.push_str(l.trim());
                } else {
                    break;
                }
                i += 1;
            }
            doc.blocks.push(Block::List { ordered, items: items.iter().map(|s| inline(s)).collect() });
            continue;
        }
        para.push(t);
        i += 1;
    }
    flush_para(&mut para, &mut doc.blocks);
    doc
}

/// Plain text of a document (search).
pub fn doc_text(doc: &Doc) -> String {
    let mut s = String::new();
    s.push_str(&doc.title);
    s.push('\n');
    for b in &doc.blocks {
        match b {
            Block::Heading { text, .. } | Block::Para(text) | Block::Note(text) => {
                s.push_str(&plain(text));
            }
            Block::List { items, .. } => {
                for it in items {
                    s.push_str(&plain(it));
                    s.push('\n');
                }
            }
            Block::Table { head, rows } => {
                for c in head.iter().chain(rows.iter().flatten()) {
                    s.push_str(&plain(c));
                    s.push(' ');
                }
            }
            Block::Formula(lines) => {
                for l in lines {
                    s.push_str(&plain(l));
                    s.push('\n');
                }
            }
            Block::Image { caption, .. } => s.push_str(&plain(caption)),
            Block::Code(c) => s.push_str(c),
            Block::Rule | Block::Anchor(_) => {}
        }
        s.push('\n');
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inline_styles() {
        let v = inline("a **b** *c* `d_{x}` θ_{r} h^{2} \\*e");
        let texts: Vec<(&str, bool, bool, bool, Script)> = v.iter().map(|s| (s.text.as_str(), s.bold, s.italic, s.code, s.script)).collect();
        assert_eq!(
            texts,
            vec![
                ("a ", false, false, false, Script::Normal),
                ("b", true, false, false, Script::Normal),
                (" ", false, false, false, Script::Normal),
                ("c", false, true, false, Script::Normal),
                (" ", false, false, false, Script::Normal),
                ("d_{x}", false, false, true, Script::Normal),
                (" θ", false, false, false, Script::Normal),
                ("r", false, false, false, Script::Sub),
                (" h", false, false, false, Script::Normal),
                ("2", false, false, false, Script::Sup),
                (" *e", false, false, false, Script::Normal),
            ]
        );
    }

    #[test]
    fn links_and_citations() {
        let v = inline("see [the model](th_hydraulic_models#vg) and [@vanGenuchten1980], [web](https://x.org)");
        assert_eq!(v[1].text, "the model");
        assert_eq!(v[1].link, Some(Link::Topic { id: "th_hydraulic_models".into(), anchor: Some("vg".into()) }));
        assert_eq!(v[3].link, Some(Link::Cite("vanGenuchten1980".into())));
        assert_eq!(v[3].text, "van Genuchten (1980)");
        assert_eq!(v[5].link, Some(Link::Url("https://x.org".into())));
        // underscores without braces stay text
        assert_eq!(plain(&inline("T_Level.out")), "T_Level.out");
    }

    #[test]
    fn blocks() {
        let d = parse("# T\nkeywords: a; b\n\npara\nline\n\n## H {#x}\n- one\n  two\n- three\n\n| a | b |\n|---|---|\n| 1 | 2 |\n\n$$ x^{2} $$\n\n![cap](img/a.png)\n> note\n```\nrun *x*\n```\n");
        assert_eq!(d.title, "T");
        assert_eq!(d.keywords, vec!["a", "b"]);
        assert_eq!(d.blocks[0], Block::Para(inline("para line")));
        assert!(matches!(&d.blocks[1], Block::Heading { anchor, level: 2, .. } if anchor == "x"));
        assert!(matches!(&d.blocks[2], Block::List { items, ordered: false } if items.len() == 2 && plain(&items[0]) == "one two"));
        assert!(matches!(&d.blocks[3], Block::Table { head, rows } if head.len() == 2 && rows.len() == 1));
        assert!(matches!(&d.blocks[4], Block::Formula(l) if l.len() == 1));
        assert!(matches!(&d.blocks[5], Block::Image { src, .. } if src == "img/a.png"));
        assert!(matches!(&d.blocks[6], Block::Note(_)));
        assert_eq!(d.blocks[7], Block::Code("run *x*".into()));
        assert_eq!(slug("Soil Hydraulic Model – van Genuchten"), "soil-hydraulic-model-van-genuchten");
    }
}
