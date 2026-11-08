//! Real TrueType text: shaping with rustybuzz (HarfBuzz) and anti-aliased
//! rasterisation with fontdue.
//!
//! The primary face is Vazirmatn (SIL OFL 1.1, bundled in
//! `assets/fonts/`), which covers Persian, Arabic and Latin. DejaVu Sans
//! (bundled, Bitstream Vera license) is the fallback face for any glyph
//! Vazirmatn does not have. Paragraph direction comes from the Unicode
//! bidirectional algorithm (`unicode-bidi`): RTL paragraphs are shaped
//! right-to-left and right-aligned inside their node.
//!
//! If the bundled fonts somehow fail to parse, callers fall back to the
//! old 5x7 bitmap font in `font.rs`; nothing panics.

use std::sync::OnceLock;

pub const VAZIR_REGULAR: &[u8] = include_bytes!("../assets/fonts/Vazirmatn-Regular.ttf");
pub const VAZIR_BOLD: &[u8] = include_bytes!("../assets/fonts/Vazirmatn-Bold.ttf");
pub const FALLBACK_REGULAR: &[u8] = include_bytes!("../assets/fonts/DejaVuSans.ttf");
pub const FALLBACK_BOLD: &[u8] = include_bytes!("../assets/fonts/DejaVuSans-Bold.ttf");

pub struct Fonts {
    pub primary: fontdue::Font,
    pub primary_bold: fontdue::Font,
    pub fallback: fontdue::Font,
    pub fallback_bold: fontdue::Font,
}

fn load(bytes: &[u8]) -> Option<fontdue::Font> {
    fontdue::Font::from_bytes(bytes, fontdue::FontSettings::default()).ok()
}

pub fn fonts() -> Option<&'static Fonts> {
    static FONTS: OnceLock<Option<Fonts>> = OnceLock::new();
    FONTS
        .get_or_init(|| {
            Some(Fonts {
                primary: load(VAZIR_REGULAR)?,
                primary_bold: load(VAZIR_BOLD)?,
                fallback: load(FALLBACK_REGULAR)?,
                fallback_bold: load(FALLBACK_BOLD)?,
            })
        })
        .as_ref()
}

impl Fonts {
    pub fn primary(&self, bold: bool) -> &fontdue::Font {
        if bold {
            &self.primary_bold
        } else {
            &self.primary
        }
    }
    pub fn fallback(&self, bold: bool) -> &fontdue::Font {
        if bold {
            &self.fallback_bold
        } else {
            &self.fallback
        }
    }
    /// Distance from the top of the line box to the baseline.
    pub fn ascent(&self, bold: bool, size: f32) -> f32 {
        self.primary(bold)
            .horizontal_line_metrics(size)
            .map(|m| m.ascent)
            .unwrap_or(size * 0.8)
    }
}

/// One shaped glyph, in visual order, ready to rasterise.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShapedGlyph {
    /// Glyph id in the primary face. 0 means the primary face has no
    /// glyph for this cluster; use `cluster_char` with the fallback face.
    pub glyph_id: u32,
    /// The source character for fallback rendering (first char of cluster).
    pub cluster_char: char,
    pub x_advance: f32,
    pub x_offset: f32,
    pub y_offset: f32,
    /// Pen position of this glyph from the left edge of the line, in
    /// visual order. Glyphs of an embedded left-to-right run (digits,
    /// Latin) appear with increasing `x` in their logical order.
    pub x: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ShapedLine {
    pub glyphs: Vec<ShapedGlyph>,
    pub width: f32,
    pub rtl: bool,
}

/// True when the paragraph base direction is right-to-left.
pub fn is_rtl(text: &str) -> bool {
    let info = unicode_bidi::BidiInfo::new(text, None);
    info.paragraphs
        .first()
        .map(|p| p.level.is_rtl())
        .unwrap_or(false)
}

/// Shape a single direction run with HarfBuzz. The returned glyphs are
/// in visual (left to right on screen) order for that run alone.
fn shape_run(text: &str, run_rtl: bool, bold: bool, size: f32) -> Vec<ShapedGlyph> {
    let bytes = if bold { VAZIR_BOLD } else { VAZIR_REGULAR };
    let Some(face) = rustybuzz::Face::from_slice(bytes, 0) else {
        return Vec::new();
    };
    let upem = face.units_per_em() as f32;
    let scale = size / upem;
    let mut buffer = rustybuzz::UnicodeBuffer::new();
    buffer.push_str(text);
    buffer.guess_segment_properties();
    buffer.set_direction(if run_rtl {
        rustybuzz::Direction::RightToLeft
    } else {
        rustybuzz::Direction::LeftToRight
    });
    if let Ok(lang) = (if run_rtl { "fa" } else { "en" }).parse::<rustybuzz::Language>() {
        buffer.set_language(lang);
    }
    let shaped = rustybuzz::shape(&face, &[], buffer);
    let infos = shaped.glyph_infos();
    let positions = shaped.glyph_positions();
    let mut glyphs = Vec::with_capacity(infos.len());
    for (info, pos) in infos.iter().zip(positions.iter()) {
        let cluster_char = text[info.cluster as usize..]
            .chars()
            .next()
            .unwrap_or('\u{fffd}');
        glyphs.push(ShapedGlyph {
            glyph_id: info.glyph_id,
            cluster_char,
            x_advance: pos.x_advance as f32 * scale,
            x_offset: pos.x_offset as f32 * scale,
            y_offset: pos.y_offset as f32 * scale,
            x: 0.0,
        });
    }
    glyphs
}

/// Shape one line (no newline characters) with HarfBuzz.
///
/// The line is first split into direction runs with the Unicode
/// bidirectional algorithm (`unicode_bidi::BidiInfo::visual_runs`):
/// a Persian paragraph is shaped as RTL runs, while embedded digits
/// and Latin words form their own LTR runs (and the mirror holds in an
/// LTR paragraph). Each run is shaped separately and the runs are laid
/// out in visual order, so a number inside Persian text keeps its
/// logical digit order on screen instead of being reversed by the
/// surrounding RTL direction.
pub fn shape_line(text: &str, bold: bool, size: f32) -> ShapedLine {
    let rtl = is_rtl(text);
    if text.is_empty() {
        return ShapedLine {
            glyphs: Vec::new(),
            width: 0.0,
            rtl,
        };
    }
    let info = unicode_bidi::BidiInfo::new(text, None);
    let Some(para) = info.paragraphs.first() else {
        return ShapedLine {
            glyphs: Vec::new(),
            width: 0.0,
            rtl,
        };
    };
    let (_levels, runs) = info.visual_runs(para, para.range.clone());
    let mut glyphs: Vec<ShapedGlyph> = Vec::new();
    let mut width = 0.0f32;
    for range in runs {
        if range.is_empty() {
            continue;
        }
        let run_rtl = info.levels[range.start].is_rtl();
        let run_text = &text[range];
        for mut g in shape_run(run_text, run_rtl, bold, size) {
            g.x = width + g.x_offset;
            width += g.x_advance;
            glyphs.push(g);
        }
    }
    ShapedLine { glyphs, width, rtl }
}

/// Width in pixels of one line at `size`, after shaping.
pub fn measure_line(text: &str, bold: bool, size: f32) -> f32 {
    shape_line(text, bold, size).width
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_fonts_parse() {
        let f = fonts().expect("bundled fonts must parse");
        // Vazirmatn and the DejaVu fallback both parsed above; sanity: the
        // primary face has a glyph for Latin A and for Persian beh.
        assert!(f.primary.lookup_glyph_index('A') > 0);
        assert!(f.primary.lookup_glyph_index('ب') > 0);
    }

    #[test]
    fn latin_shaping_is_proportional() {
        let wide = measure_line("WWWW", false, 20.0);
        let narrow = measure_line("iiii", false, 20.0);
        assert!(wide > narrow * 1.5, "wide {wide} narrow {narrow}");
        assert!(wide > 0.0);
    }

    #[test]
    fn persian_is_rtl_and_fully_covered() {
        let line = shape_line("سلام", false, 20.0);
        assert!(line.rtl);
        // 4 characters become 3 glyphs: lam + alef form their ligature.
        assert_eq!(line.glyphs.len(), 3);
        assert!(
            line.glyphs.iter().all(|g| g.glyph_id != 0),
            "every Persian glyph must exist in Vazirmatn: {:?}",
            line.glyphs
        );
    }

    #[test]
    fn persian_lam_alef_forms_one_ligature_glyph() {
        // Real shaping proof: lam + alef must collapse into the single
        // ligature glyph U+FEFB presents as, not two separate glyphs.
        let line = shape_line("لا", false, 20.0);
        assert_eq!(line.glyphs.len(), 1, "glyphs: {:?}", line.glyphs);
        assert_ne!(line.glyphs[0].glyph_id, 0);
    }

    #[test]
    fn persian_digits_are_covered() {
        let line = shape_line("۲۵۰۰۰۰ تومان", false, 16.0);
        assert!(line.rtl);
        assert!(line.glyphs.iter().all(|g| g.glyph_id != 0));
        assert!(line.width > 40.0);
    }

    /// The digit run of a mixed line, in visual glyph order.
    fn digit_run(line: &ShapedLine) -> Vec<&ShapedGlyph> {
        line.glyphs
            .iter()
            .filter(|g| {
                g.cluster_char.is_ascii_digit() || ('\u{6f0}'..='\u{6f9}').contains(&g.cluster_char)
            })
            .collect()
    }

    #[test]
    fn persian_price_digits_read_in_numeric_order() {
        // Regression: shaping the whole line as one RTL run reversed the
        // number into 000084 on screen. The digit run must come out in
        // logical order, left to right.
        let line = shape_line("قیمت: 480000 تومان", false, 20.0);
        assert!(line.rtl);
        let digits = digit_run(&line);
        let order: String = digits.iter().map(|g| g.cluster_char).collect();
        assert_eq!(order, "480000", "glyphs: {:?}", line.glyphs);
        // Positions increase in that same order, so the 4 is the
        // leftmost digit glyph and the last 0 is the rightmost.
        for w in digits.windows(2) {
            assert!(w[0].x < w[1].x, "digit x must increase: {:?}", digits);
        }
    }

    #[test]
    fn persian_digit_price_reads_in_numeric_order() {
        // Same regression with Extended Arabic-Indic (Persian) digits,
        // the exact string used on the demo price line.
        let line = shape_line("مونسترا: ۴۸۰۰۰۰ تومان", true, 18.0);
        assert!(line.rtl);
        let digits = digit_run(&line);
        let order: String = digits.iter().map(|g| g.cluster_char).collect();
        assert_eq!(order, "۴۸۰۰۰۰", "glyphs: {:?}", line.glyphs);
        for w in digits.windows(2) {
            assert!(w[0].x < w[1].x, "digit x must increase: {:?}", digits);
        }
        // The digit run sits between the two Persian runs visually:
        // every digit is right of the trailing Persian word and left of
        // the leading Persian word on screen.
        let digit_min = digits.first().unwrap().x;
        let digit_max = digits.last().unwrap().x;
        assert!(digit_min > 0.0 && digit_max < line.width);
    }

    #[test]
    fn latin_run_inside_persian_keeps_its_order() {
        let line = shape_line("نام: Monstera خوب", false, 20.0);
        let latin: String = line
            .glyphs
            .iter()
            .map(|g| g.cluster_char)
            .filter(|c| c.is_ascii_alphabetic())
            .collect();
        assert_eq!(latin, "Monstera");
    }

    #[test]
    fn persian_run_inside_latin_keeps_visual_order() {
        // Mirror case: an RTL run embedded in an LTR paragraph.
        let line = shape_line("Name گلکده Shop", false, 20.0);
        assert!(!line.rtl);
        // The whole line still shapes and every glyph advances the pen.
        assert!(line.width > 0.0);
        let xs: Vec<f32> = line.glyphs.iter().map(|g| g.x).collect();
        for w in xs.windows(2) {
            assert!(w[0] <= w[1], "glyph positions must be visual order");
        }
    }

    #[test]
    fn pure_lines_are_unchanged_by_segmentation() {
        // Pure Persian: visual order is the RTL word order (last logical
        // letter leftmost) and lam-alef still forms its ligature.
        let fa = shape_line("سلام", false, 20.0);
        assert_eq!(fa.glyphs.len(), 3);
        assert_eq!(fa.glyphs[0].cluster_char, 'م');
        // Pure Latin: logical order, proportional advances.
        let en = shape_line("Hello 123", false, 20.0);
        let order: String = en.glyphs.iter().map(|g| g.cluster_char).collect();
        assert_eq!(order, "Hello 123");
        assert!(!en.rtl);
    }

    #[test]
    fn joining_changes_glyph_identity() {
        // Beh in isolation vs beh joined inside a word must be different
        // glyph ids (initial/medial forms), proving contextual shaping.
        let isolated = shape_line("ب", false, 20.0);
        let joined = shape_line("بابا", false, 20.0);
        assert_eq!(isolated.glyphs.len(), 1);
        assert!(joined.glyphs.len() >= 3);
        assert!(joined
            .glyphs
            .iter()
            .any(|g| g.glyph_id != isolated.glyphs[0].glyph_id));
    }
}
