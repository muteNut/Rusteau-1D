//! The bundled fonts (Noto Sans in four styles and Noto Sans Math, SIL Open Font License 1.1, see `fonts/README.md`):
//! the fonts of the interface and the default font of the figures; and the fonts installed on the system (offered
//! for the figures).

use std::path::PathBuf;
use std::sync::OnceLock;

pub const NOTO_SANS: &[u8] = include_bytes!("../fonts/NotoSans-Regular.ttf");
pub const NOTO_SANS_BOLD: &[u8] = include_bytes!("../fonts/NotoSans-Bold.ttf");
pub const NOTO_SANS_ITALIC: &[u8] = include_bytes!("../fonts/NotoSans-Italic.ttf");
pub const NOTO_SANS_BOLD_ITALIC: &[u8] = include_bytes!("../fonts/NotoSans-BoldItalic.ttf");
pub const NOTO_SANS_MATH: &[u8] = include_bytes!("../fonts/NotoSansMath-Regular.ttf");

/// License texts of the bundled fonts.
pub const OFL_NOTO_SANS: &str = include_str!("../fonts/OFL-NotoSans.txt");
pub const OFL_NOTO_SANS_MATH: &str = include_str!("../fonts/OFL-NotoSansMath.txt");

/// Name of the bundled font family (the default of the figures).
pub const DEFAULT_FAMILY: &str = "Noto Sans";

/// Name of the bold family of the interface (headings).
pub const BOLD: &str = "bold";

/// Makes the bundled fonts the fonts of the interface, the same on every system: Noto Sans for the text, Noto Sans
/// Bold for headings (family [`BOLD`]), Noto Sans Math for the mathematical signs and arrows. The default fonts of
/// egui stay behind them for emoji and icons; the monospace font of egui stays first in its family.
pub fn install(fonts: &mut egui::FontDefinitions) {
    let add = |fonts: &mut egui::FontDefinitions, name: &str, data: &'static [u8]| {
        fonts.font_data.insert(name.into(), std::sync::Arc::new(egui::FontData::from_static(data)));
    };
    add(fonts, "noto_sans", NOTO_SANS);
    add(fonts, "noto_sans_bold", NOTO_SANS_BOLD);
    add(fonts, "noto_sans_math", NOTO_SANS_MATH);
    let defaults = fonts.families.get(&egui::FontFamily::Proportional).cloned().unwrap_or_default();
    let with = |first: &str| -> Vec<String> {
        let mut v = vec![first.to_string(), "noto_sans_math".to_string()];
        v.extend(defaults.iter().cloned());
        v
    };
    fonts.families.insert(egui::FontFamily::Proportional, with("noto_sans"));
    fonts.families.insert(egui::FontFamily::Name(BOLD.into()), with("noto_sans_bold"));
    if let Some(family) = fonts.families.get_mut(&egui::FontFamily::Monospace) {
        family.push("noto_sans".into());
        family.push("noto_sans_math".into());
    }
}

/// A font family installed on the system: the files of its regular, bold, italic and bold-italic styles.
#[derive(Clone, Debug, Default)]
pub struct SystemFamily {
    pub name: String,
    pub regular: Option<PathBuf>,
    pub bold: Option<PathBuf>,
    pub italic: Option<PathBuf>,
    pub bold_italic: Option<PathBuf>,
}

fn font_dirs() -> Vec<PathBuf> {
    let mut v = vec![];
    if cfg!(windows) {
        let windir = std::env::var_os("WINDIR").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("C:\\Windows"));
        v.push(windir.join("Fonts"));
        if let Some(local) = std::env::var_os("LOCALAPPDATA") {
            v.push(PathBuf::from(local).join("Microsoft\\Windows\\Fonts"));
        }
    } else if cfg!(target_os = "macos") {
        v.extend([PathBuf::from("/System/Library/Fonts"), PathBuf::from("/Library/Fonts")]);
        if let Some(home) = std::env::var_os("HOME") {
            v.push(PathBuf::from(home).join("Library/Fonts"));
        }
    } else {
        v.extend([PathBuf::from("/usr/share/fonts"), PathBuf::from("/usr/local/share/fonts")]);
        if let Some(home) = std::env::var_os("HOME") {
            v.push(PathBuf::from(&home).join(".local/share/fonts"));
            v.push(PathBuf::from(home).join(".fonts"));
        }
    }
    v
}

fn collect(dir: &std::path::Path, depth: usize, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            if depth < 4 {
                collect(&p, depth + 1, out);
            }
        } else if p.extension().map(|x| x.to_string_lossy().to_ascii_lowercase()).is_some_and(|x| x == "ttf" || x == "otf") {
            out.push(p);
        }
    }
}

/// Family name and style (bold, italic) of a font file.
fn describe(path: &std::path::Path) -> Option<(String, bool, bool)> {
    let data = std::fs::read(path).ok()?;
    let face = ttf_parser::Face::parse(&data, 0).ok()?;
    // typographic family (16) first, then the legacy family (1); English names preferred
    let mut name = None;
    for id in [ttf_parser::name_id::TYPOGRAPHIC_FAMILY, ttf_parser::name_id::FAMILY] {
        let found = face
            .names()
            .into_iter()
            .filter(|n| n.name_id == id && n.is_unicode())
            .max_by_key(|n| n.language() == ttf_parser::Language::English_UnitedStates)
            .and_then(|n| n.to_string());
        if found.is_some() {
            name = found;
            break;
        }
    }
    // only fonts that can draw text (a cmap with Latin letters)
    face.glyph_index('a')?;
    Some((name?, face.is_bold(), face.is_italic()))
}

/// The font families installed on the system (scanned once).
pub fn system_families() -> &'static [SystemFamily] {
    static F: OnceLock<Vec<SystemFamily>> = OnceLock::new();
    F.get_or_init(|| {
        let mut files = vec![];
        for d in font_dirs() {
            collect(&d, 0, &mut files);
        }
        files.sort();
        let mut fams: Vec<SystemFamily> = vec![];
        for f in files {
            let Some((name, bold, italic)) = describe(&f) else { continue };
            let k = match fams.iter().position(|x| x.name == name) {
                Some(k) => k,
                None => {
                    fams.push(SystemFamily { name: name.clone(), ..Default::default() });
                    fams.len() - 1
                }
            };
            let slot = match (bold, italic) {
                (false, false) => &mut fams[k].regular,
                (true, false) => &mut fams[k].bold,
                (false, true) => &mut fams[k].italic,
                (true, true) => &mut fams[k].bold_italic,
            };
            // the first file of a style wins (e.g. the static font before a variable one with the same name)
            if slot.is_none() {
                *slot = Some(f);
            }
        }
        fams.retain(|f| f.regular.is_some() && f.name != DEFAULT_FAMILY);
        fams.sort_by_key(|f| f.name.to_lowercase());
        fams
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_fonts_cover_the_symbols_of_scientific_labels() {
        let sans = ttf_parser::Face::parse(NOTO_SANS, 0).unwrap();
        let math = ttf_parser::Face::parse(NOTO_SANS_MATH, 0).unwrap();
        let symbols = "θαβγδλμνπρστφψωΔΘΣΩ°±×·−²³⁻¹⁰₀₁₂ₛᵣₘₐₖ∂√∞≤≥≈∫∑→←↑↓µ‰";
        for c in symbols.chars() {
            assert!(sans.glyph_index(c).is_some() || math.glyph_index(c).is_some(), "no glyph for {c}");
        }
        for f in [NOTO_SANS_BOLD, NOTO_SANS_ITALIC, NOTO_SANS_BOLD_ITALIC] {
            assert!(ttf_parser::Face::parse(f, 0).is_ok());
        }
        assert!(OFL_NOTO_SANS.contains("SIL OPEN FONT LICENSE Version 1.1"));
        assert!(OFL_NOTO_SANS_MATH.contains("SIL OPEN FONT LICENSE Version 1.1"));
    }

    #[test]
    fn the_interface_uses_noto_sans() {
        let mut f = egui::FontDefinitions::default();
        install(&mut f);
        assert_eq!(f.families[&egui::FontFamily::Proportional][0], "noto_sans");
        assert_eq!(f.families[&egui::FontFamily::Name(BOLD.into())][0], "noto_sans_bold");
        // emoji and icons of egui stay available
        assert!(f.families[&egui::FontFamily::Proportional].len() > 2);
    }
}
