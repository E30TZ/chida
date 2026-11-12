//! Layout engine for generated templates: turns a [`Pack`] (JSON data)
//! into a real [`ProjectFile`] built from chida-core nodes, with database
//! tables and one optional flow.
//!
//! Layout is RTL-first (Persian copy): the canvas is 1440 wide, the
//! content column runs from x=80 to x=1360, rows lay out right-to-left,
//! and Persian text nodes right-anchor by default in the renderer.

use crate::project_file::ProjectFile;
use crate::templates::{fa_num, fa_price};
use chida_core::{Color, Gradient, Node, NodeKind, Page, Project, Shadow};
use chida_db::{Column, ColumnType, Database, Table};
use chida_logic::{Action, Event, EventKind, Flow, Step};
use serde_json::Map;

use super::pack::{Pack, Section};

// ---------------------------------------------------------------------
// Geometry.
// ---------------------------------------------------------------------

const W: f32 = 1440.0;
const MX: f32 = 80.0;
const CW: f32 = 1280.0;
const RX: f32 = 1360.0;
const NAV_H: f32 = 72.0;
const FOOT_H: f32 = 300.0;
const SEC_GAP: f32 = 56.0;

// ---------------------------------------------------------------------
// Palettes.
// ---------------------------------------------------------------------

#[derive(Clone, Copy)]
pub struct Pal {
    pub primary: Color,
    pub dark: Color,
    pub accent: Color,
    pub ink: Color,
    pub muted: Color,
    pub bg: Color,
    pub surface: Color,
    pub line: Color,
    pub hero_bg: Color,
}

const fn rgb(r: u8, g: u8, b: u8) -> Color {
    Color::rgb(r, g, b)
}

fn tint(c: Color, f: f32) -> Color {
    let a = ((1.0 - f) * 255.0) as u8;
    Color::rgba(c.r, c.g, c.b, a).over(Color::WHITE)
}

fn resolve_palette(name: &str, dark: bool) -> Pal {
    let (primary, dark_c, hero_bg): (Color, Color, Color) = match name {
        "emerald" => (rgb(0, 130, 90), rgb(0, 90, 63), rgb(6, 60, 44)),
        "forest" => (rgb(21, 101, 52), rgb(13, 71, 36), rgb(10, 48, 26)),
        "pine" => (rgb(6, 95, 70), rgb(4, 66, 49), rgb(4, 47, 35)),
        "teal" => (rgb(15, 118, 110), rgb(10, 84, 78), rgb(9, 58, 54)),
        "turquoise" => (rgb(13, 148, 136), rgb(9, 106, 97), rgb(8, 74, 68)),
        "ocean" => (rgb(13, 110, 158), rgb(9, 79, 113), rgb(8, 55, 80)),
        "sky" => (rgb(2, 132, 199), rgb(2, 94, 141), rgb(2, 66, 99)),
        "navy" => (rgb(30, 64, 175), rgb(21, 46, 126), rgb(15, 32, 89)),
        "royal" => (rgb(67, 56, 202), rgb(47, 39, 141), rgb(33, 28, 99)),
        "violet" => (rgb(124, 58, 237), rgb(88, 41, 168), rgb(62, 29, 119)),
        "grape" => (rgb(109, 40, 217), rgb(78, 28, 154), rgb(55, 20, 109)),
        "plum" => (rgb(157, 23, 77), rgb(112, 16, 55), rgb(79, 11, 39)),
        "berry" => (rgb(162, 28, 175), rgb(115, 20, 124), rgb(81, 14, 88)),
        "rose" => (rgb(225, 29, 72), rgb(160, 21, 51), rgb(113, 15, 36)),
        "cherry" => (rgb(190, 18, 60), rgb(135, 13, 43), rgb(95, 9, 30)),
        "crimson" => (rgb(153, 27, 27), rgb(109, 19, 19), rgb(77, 13, 13)),
        "terracotta" => (rgb(194, 65, 12), rgb(138, 46, 9), rgb(97, 32, 6)),
        "coral" => (rgb(234, 88, 12), rgb(166, 63, 9), rgb(117, 44, 6)),
        "amber" => (rgb(180, 83, 9), rgb(128, 59, 6), rgb(90, 42, 4)),
        "gold" => (rgb(161, 98, 7), rgb(115, 70, 5), rgb(81, 49, 3)),
        "sand" => (rgb(146, 64, 14), rgb(104, 46, 10), rgb(73, 32, 7)),
        "coffee" => (rgb(120, 53, 15), rgb(85, 38, 11), rgb(60, 27, 8)),
        "olive" => (rgb(77, 124, 15), rgb(55, 88, 11), rgb(39, 62, 8)),
        "mint" => (rgb(5, 150, 105), rgb(4, 107, 75), rgb(3, 75, 53)),
        "charcoal" => (rgb(31, 41, 55), rgb(17, 24, 39), rgb(10, 14, 23)),
        "slate" => (rgb(51, 65, 85), rgb(30, 41, 59), rgb(15, 23, 42)),
        "steel" => (rgb(71, 85, 105), rgb(51, 65, 85), rgb(30, 41, 59)),
        "midnight" => (rgb(99, 102, 241), rgb(67, 56, 202), rgb(15, 23, 42)),
        _ => (rgb(16, 122, 87), rgb(11, 87, 62), rgb(8, 61, 43)),
    };
    let accent = tint(primary, 0.88);
    if dark {
        Pal {
            primary,
            dark: dark_c,
            accent: rgb(30, 41, 59),
            ink: rgb(241, 245, 249),
            muted: rgb(148, 163, 184),
            bg: rgb(15, 23, 42),
            surface: rgb(30, 41, 59),
            line: rgb(51, 65, 85),
            hero_bg,
        }
    } else {
        Pal {
            primary,
            dark: dark_c,
            accent,
            ink: rgb(28, 25, 23),
            muted: rgb(100, 106, 112),
            bg: Color::WHITE,
            surface: rgb(249, 250, 251),
            line: rgb(229, 231, 235),
            hero_bg,
        }
    }
}

// ---------------------------------------------------------------------
// Text measuring (conservative estimate; verified against renders).
// ---------------------------------------------------------------------

fn text_lines(s: &str, size: f32, width: f32) -> usize {
    let cpl = (width / (size * 0.55)).max(4.0) as usize;
    let chars = s.chars().count();
    ((chars + cpl - 1) / cpl).max(1)
}

fn text_h(s: &str, size: f32, width: f32) -> f32 {
    text_lines(s, size, width) as f32 * (size + 12.0)
}

// ---------------------------------------------------------------------
// Node helpers.
// ---------------------------------------------------------------------

fn txt(id: &str, x: f32, y: f32, w: f32, s: &str, size: f32, color: Color, bold: bool) -> Node {
    let h = text_h(s, size, w);
    let mut n = Node::text_node(id, s, x, y, w, h, s, size, color);
    n.bold = bold;
    n
}

fn ctxt(id: &str, x: f32, y: f32, w: f32, s: &str, size: f32, color: Color, bold: bool) -> Node {
    let mut n = txt(id, x, y, w, s, size, color, bold);
    n.props.insert("align".into(), "center".into());
    n
}

fn rect(id: &str, name: &str, x: f32, y: f32, w: f32, h: f32, fill: Color, radius: f32) -> Node {
    let mut n = Node::rect(id, name, x, y, w, h, fill);
    n.corner_radius = radius;
    n
}

fn card_rect(id: &str, x: f32, y: f32, w: f32, h: f32, fill: Color, line: Color) -> Node {
    let mut n = rect(id, "Card", x, y, w, h, fill, 14.0);
    n.stroke = Some(line);
    n.stroke_width = 1.0;
    n.shadows = vec![Shadow::soft()];
    n
}

fn btn(
    id: &str,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    label: &str,
    size: f32,
    fill: Color,
    tc: Color,
    radius: f32,
) -> Node {
    let mut b = Node::new(id, label, NodeKind::Button, x, y, w, h);
    b.fill = Some(fill);
    b.corner_radius = radius;
    b.text = Some(label.into());
    b.font_size = size;
    b.text_color = Some(tc);
    b.bold = true;
    b.props.insert("align".into(), "center".into());
    b
}

fn link_btn(mut b: Node, route: &str) -> Node {
    b.props
        .insert("interactions".into(), format!("click|route|{route}"));
    b
}

fn link_text(mut n: Node, route: &str) -> Node {
    n.props
        .insert("interactions".into(), format!("click|route|{route}"));
    n
}

/// Right-to-left grid column x: first column sits at the right edge.
fn grid_x(i: usize, cols: usize, gap: f32) -> (f32, f32) {
    let col_w = (CW - gap * (cols as f32 - 1.0)) / cols as f32;
    let x = RX - col_w - (i as f32) * (col_w + gap);
    (x, col_w)
}

const GLYPHS: [&str; 8] = ["◆", "●", "▲", "■", "★", "✚", "⬢", "◐"];

// ---------------------------------------------------------------------
// Page builder.
// ---------------------------------------------------------------------

struct Builder<'a> {
    pack: &'a Pack,
    pal: Pal,
    page_id: String,
    counter: usize,
    nodes: Vec<Node>,
}

impl<'a> Builder<'a> {
    fn new(pack: &'a Pack, pal: Pal, page_id: &str) -> Self {
        Self {
            pack,
            pal,
            page_id: page_id.to_string(),
            counter: 0,
            nodes: Vec::new(),
        }
    }

    fn id(&mut self, tag: &str) -> String {
        self.counter += 1;
        format!("{}-{}-{}", self.page_id, tag, self.counter)
    }

    fn fixed_id(&self, tag: &str) -> String {
        format!("{}-{}", self.page_id, tag)
    }

    /// The route a generic call-to-action should go to: the contact page
    /// when the template has one, else the second page, else home.
    fn cta_target(&self) -> String {
        let pages = &self.pack.pages;
        if let Some(p) = pages.iter().find(|p| p.id == "contact") {
            return p.route.clone();
        }
        if pages.len() > 1 {
            return pages[1].route.clone();
        }
        "/".into()
    }

    /// Section heading: title right-aligned + optional subtitle. Returns
    /// the height consumed (including the gap below).
    fn heading(&mut self, y: f32, title: &str, subtitle: Option<&str>) -> f32 {
        let mut h = 0.0;
        let th = text_h(title, 28.0, 900.0);
        let id = self.id("st");
        self.nodes.push(txt(
            &id,
            RX - 900.0,
            y,
            900.0,
            title,
            28.0,
            self.pal.ink,
            true,
        ));
        h += th + 8.0;
        if let Some(sub) = subtitle {
            let sh = text_h(sub, 15.0, 900.0);
            let id = self.id("ss");
            self.nodes.push(txt(
                &id,
                RX - 900.0,
                y + h,
                900.0,
                sub,
                15.0,
                self.pal.muted,
                false,
            ));
            h += sh + 8.0;
        }
        h + 20.0
    }

    fn centered_heading(&mut self, y: f32, title: &str, subtitle: Option<&str>) -> f32 {
        let mut h = 0.0;
        let th = text_h(title, 28.0, 900.0);
        let id = self.id("st");
        self.nodes.push(ctxt(
            &id,
            (W - 900.0) / 2.0,
            y,
            900.0,
            title,
            28.0,
            self.pal.ink,
            true,
        ));
        h += th + 8.0;
        if let Some(sub) = subtitle {
            let sh = text_h(sub, 15.0, 900.0);
            let id = self.id("ss");
            self.nodes.push(ctxt(
                &id,
                (W - 900.0) / 2.0,
                y + h,
                900.0,
                sub,
                15.0,
                self.pal.muted,
                false,
            ));
            h += sh + 8.0;
        }
        h + 20.0
    }
}

fn build_navbar(b: &mut Builder, y: f32) -> f32 {
    let p = b.pal;
    let id = b.id("nav");
    b.nodes
        .push(rect(&id, "Navbar", 0.0, y, W, NAV_H, p.bg, 0.0));
    let id = b.id("navline");
    b.nodes
        .push(rect(&id, "", 0.0, y + NAV_H - 1.0, W, 1.0, p.line, 0.0));
    // Brand at the right.
    let id = b.id("brand");
    b.nodes.push(txt(
        &id,
        RX - 320.0,
        y + 20.0,
        320.0,
        &b.pack.brand.clone(),
        24.0,
        p.primary,
        true,
    ));
    // Page links, right to left after the brand.
    let mut lx = RX - 340.0;
    for page in &b.pack.pages {
        let w = (page.name.chars().count() as f32 * 11.0 + 28.0).max(64.0);
        lx -= w;
        let id = b.id("navlink");
        let node = txt(&id, lx, y + 26.0, w, &page.name.clone(), 15.0, p.ink, false);
        let route = page.route.clone();
        b.nodes.push(link_text(node, &route));
    }
    // Optional CTA at the far left.
    if let Some(cta) = &b.pack.nav_cta {
        let id = b.id("navcta");
        let node = btn(
            &id,
            MX,
            y + 16.0,
            150.0,
            40.0,
            &cta.label,
            14.0,
            p.primary,
            Color::WHITE,
            10.0,
        );
        let route = cta.route.clone();
        b.nodes.push(link_btn(node, &route));
    }
    NAV_H
}

fn build_footer(b: &mut Builder, y: f32) -> f32 {
    let p = b.pal;
    let id = b.id("foot");
    b.nodes
        .push(rect(&id, "Footer", 0.0, y, W, FOOT_H, p.hero_bg, 0.0));
    // Brand + tagline.
    let id = b.id("fbrand");
    b.nodes.push(txt(
        &id,
        RX - 420.0,
        y + 36.0,
        420.0,
        &b.pack.brand.clone(),
        24.0,
        Color::WHITE,
        true,
    ));
    let id = b.id("ftag");
    b.nodes.push(txt(
        &id,
        RX - 420.0,
        y + 80.0,
        420.0,
        &b.pack.tagline.clone(),
        14.0,
        rgb(203, 213, 225),
        false,
    ));
    // Page links column.
    let id = b.id("fhead");
    b.nodes.push(txt(
        &id,
        760.0,
        y + 36.0,
        200.0,
        "دسترسی سریع",
        16.0,
        Color::WHITE,
        true,
    ));
    let mut ly = y + 72.0;
    for page in &b.pack.pages {
        let id = b.id("flink");
        let node = txt(
            &id,
            760.0,
            ly,
            200.0,
            &page.name.clone(),
            14.0,
            rgb(203, 213, 225),
            false,
        );
        let route = page.route.clone();
        b.nodes.push(link_text(node, &route));
        ly += 30.0;
    }
    // Contact column (sample values).
    let id = b.id("fchead");
    b.nodes.push(txt(
        &id,
        MX,
        y + 36.0,
        300.0,
        "تماس با ما",
        16.0,
        Color::WHITE,
        true,
    ));
    let id = b.id("fc1");
    b.nodes.push(txt(
        &id,
        MX,
        y + 72.0,
        300.0,
        "تلفن: ۰۲۱-۹۱۰۰۰۰۰۰",
        14.0,
        rgb(203, 213, 225),
        false,
    ));
    let id = b.id("fc2");
    b.nodes.push(txt(
        &id,
        MX,
        y + 102.0,
        300.0,
        "تهران، خیابان نمونه، پلاک ۱",
        14.0,
        rgb(203, 213, 225),
        false,
    ));
    // Divider + copyright.
    let id = b.id("fdiv");
    b.nodes.push(rect(
        &id,
        "",
        MX,
        y + FOOT_H - 56.0,
        CW,
        1.0,
        rgb(51, 65, 85),
        0.0,
    ));
    let copy = format!(
        "© ۱۴۰۵ {}. تمامی حقوق محفوظ است. نمونه ساخته‌شده با چیدا.",
        b.pack.brand
    );
    let id = b.id("fcopy");
    b.nodes.push(ctxt(
        &id,
        220.0,
        y + FOOT_H - 40.0,
        1000.0,
        &copy,
        13.0,
        rgb(148, 163, 184),
        false,
    ));
    FOOT_H
}

// ---------------------------------------------------------------------
// Sections.
// ---------------------------------------------------------------------

fn sec_hero(b: &mut Builder, s: &Section, y: f32) -> f32 {
    let p = b.pal;
    let variant = s.variant.as_deref().unwrap_or("center");
    let title = s.title.clone().unwrap_or_else(|| b.pack.tagline.clone());
    let subtitle = s.subtitle.clone().unwrap_or_default();
    let cta = s.cta.clone().unwrap_or_else(|| "شروع کنید".into());
    let cta_route = s.cta_route.clone().unwrap_or_else(|| b.cta_target());
    // A hero button with nowhere to go is a dead control: the default
    // button only renders when the pack has another page to link to, or
    // the author set an explicit cta. Single-page templates (coming-soon)
    // get no button; page-404 keeps its explicit «back home» button.
    let multi_page = b.pack.pages.len() > 1;
    let show_cta = s.cta.is_some() || s.cta_route.is_some() || multi_page;
    let show_cta2 = s.cta2.is_some() && (s.cta2_route.is_some() || multi_page);
    match variant {
        "split" => {
            let h = 480.0;
            // Visual panel on the left.
            let id = b.id("hvis");
            let mut panel = rect(
                &id,
                "Hero visual",
                MX,
                y + 24.0,
                560.0,
                h - 48.0,
                p.primary,
                20.0,
            );
            panel.fill_gradient = Some(Gradient::new(p.primary, p.dark, 135.0));
            b.nodes.push(panel);
            let id = b.id("hcirc");
            let mut glow = Node::new(
                &id,
                "Glow",
                NodeKind::Ellipse,
                MX + 330.0,
                y + 60.0,
                180.0,
                180.0,
            );
            glow.fill = Some(Color::rgba(255, 255, 255, 40));
            b.nodes.push(glow);
            let id = b.id("hcard");
            b.nodes.push(card_rect(
                &id,
                MX + 60.0,
                y + 150.0,
                300.0,
                150.0,
                Color::WHITE,
                p.line,
            ));
            let id = b.id("hcardt");
            b.nodes.push(txt(
                &id,
                MX + 84.0,
                y + 178.0,
                252.0,
                &b.pack.brand.clone(),
                20.0,
                p.ink,
                true,
            ));
            let id = b.id("hcards");
            b.nodes.push(txt(
                &id,
                MX + 84.0,
                y + 222.0,
                252.0,
                &b.pack.tagline.clone(),
                14.0,
                p.muted,
                false,
            ));
            // Text on the right.
            let tx = 700.0;
            let tw = 660.0;
            let mut cy = y + 48.0;
            if let Some(badge) = &s.badge {
                let id = b.id("hbadge");
                b.nodes.push(btn(
                    &id,
                    RX - 180.0,
                    cy,
                    180.0,
                    32.0,
                    badge,
                    13.0,
                    p.accent,
                    p.primary,
                    16.0,
                ));
                cy += 52.0;
            }
            let th = text_h(&title, 40.0, tw);
            let id = b.id("htitle");
            b.nodes
                .push(txt(&id, tx, cy, tw, &title, 40.0, p.ink, true));
            cy += th + 16.0;
            if !subtitle.is_empty() {
                let sh = text_h(&subtitle, 17.0, tw);
                let id = b.id("hsub");
                b.nodes
                    .push(txt(&id, tx, cy, tw, &subtitle, 17.0, p.muted, false));
                cy += sh + 16.0;
            }
            for point in &s.points {
                let id = b.id("hchk");
                b.nodes
                    .push(txt(&id, RX - 30.0, cy, 30.0, "✓", 16.0, p.primary, true));
                let id = b.id("hpt");
                b.nodes
                    .push(txt(&id, tx, cy, tw - 36.0, point, 15.0, p.ink, false));
                cy += 32.0;
            }
            cy += 12.0;
            if show_cta {
                let id = b.fixed_id("hero-cta");
                let node = btn(
                    &id,
                    RX - 190.0,
                    cy,
                    190.0,
                    48.0,
                    &cta,
                    16.0,
                    p.primary,
                    Color::WHITE,
                    12.0,
                );
                b.nodes.push(link_btn(node, &cta_route));
            }
            if show_cta2 {
                if let Some(c2) = &s.cta2 {
                let r2 = s.cta2_route.clone().unwrap_or_else(|| b.cta_target());
                let id = b.id("hcta2");
                let mut node = btn(
                    &id,
                    RX - 390.0,
                    cy,
                    180.0,
                    48.0,
                    c2,
                    16.0,
                    p.bg,
                    p.primary,
                    12.0,
                );
                node.stroke = Some(p.primary);
                node.stroke_width = 1.5;
                b.nodes.push(link_btn(node, &r2));
                }
            }
            h
        }
        "band" => {
            // Height from content: badge? + title + subtitle? + button?.
            let mut h = 64.0 + 64.0;
            if show_cta {
                h += 48.0;
            }
            if s.badge.is_some() {
                h += 52.0;
            }
            h += text_h(&title, 38.0, 900.0) + 16.0;
            if !subtitle.is_empty() {
                h += text_h(&subtitle, 17.0, 900.0) + if show_cta { 24.0 } else { 0.0 };
            }
            let id = b.id("hband");
            let mut band = rect(&id, "Hero band", 0.0, y, W, h, p.hero_bg, 0.0);
            band.fill_gradient = Some(Gradient::new(p.hero_bg, p.dark, 120.0));
            b.nodes.push(band);
            let mut cy = y + 64.0;
            if let Some(badge) = &s.badge {
                let id = b.id("hbadge");
                b.nodes.push(btn(
                    &id,
                    RX - 180.0,
                    cy,
                    180.0,
                    32.0,
                    badge,
                    13.0,
                    Color::rgba(255, 255, 255, 30),
                    Color::WHITE,
                    16.0,
                ));
                cy += 52.0;
            }
            let th = text_h(&title, 38.0, 900.0);
            let id = b.id("htitle");
            b.nodes.push(txt(
                &id,
                RX - 900.0,
                cy,
                900.0,
                &title,
                38.0,
                Color::WHITE,
                true,
            ));
            cy += th + 16.0;
            if !subtitle.is_empty() {
                let sh = text_h(&subtitle, 17.0, 900.0);
                let id = b.id("hsub");
                b.nodes.push(txt(
                    &id,
                    RX - 900.0,
                    cy,
                    900.0,
                    &subtitle,
                    17.0,
                    rgb(203, 213, 225),
                    false,
                ));
                cy += sh + if show_cta { 24.0 } else { 0.0 };
            }
            if show_cta {
                let id = b.fixed_id("hero-cta");
                let node = btn(
                    &id,
                    RX - 190.0,
                    cy,
                    190.0,
                    48.0,
                    &cta,
                    16.0,
                    Color::WHITE,
                    p.dark,
                    12.0,
                );
                b.nodes.push(link_btn(node, &cta_route));
                cy + 48.0 + 64.0 - y
            } else {
                cy + 64.0 - y
            }
        }
        "minimal" => {
            let mut cy = y + 32.0;
            let th = text_h(&title, 34.0, 900.0);
            let id = b.id("htitle");
            b.nodes
                .push(txt(&id, RX - 900.0, cy, 900.0, &title, 34.0, p.ink, true));
            cy += th + 12.0;
            if !subtitle.is_empty() {
                let sh = text_h(&subtitle, 16.0, 900.0);
                let id = b.id("hsub");
                b.nodes.push(txt(
                    &id,
                    RX - 900.0,
                    cy,
                    900.0,
                    &subtitle,
                    16.0,
                    p.muted,
                    false,
                ));
                cy += sh + 12.0;
            }
            let id = b.id("hdiv");
            b.nodes
                .push(rect(&id, "", MX, cy + 12.0, CW, 1.0, p.line, 0.0));
            cy + 40.0 - y
        }
        _ => {
            // center
            let mut cy = y + 56.0;
            if let Some(badge) = &s.badge {
                let id = b.id("hbadge");
                b.nodes.push(btn(
                    &id,
                    (W - 200.0) / 2.0,
                    cy,
                    200.0,
                    34.0,
                    badge,
                    13.0,
                    p.accent,
                    p.primary,
                    17.0,
                ));
                cy += 56.0;
            }
            let th = text_h(&title, 42.0, 900.0);
            let id = b.id("htitle");
            b.nodes.push(ctxt(
                &id,
                (W - 900.0) / 2.0,
                cy,
                900.0,
                &title,
                42.0,
                p.ink,
                true,
            ));
            cy += th + 18.0;
            if !subtitle.is_empty() {
                let sh = text_h(&subtitle, 18.0, 820.0);
                let id = b.id("hsub");
                b.nodes.push(ctxt(
                    &id,
                    (W - 820.0) / 2.0,
                    cy,
                    820.0,
                    &subtitle,
                    18.0,
                    p.muted,
                    false,
                ));
                cy += sh + if show_cta || show_cta2 { 28.0 } else { 0.0 };
            }
            if show_cta || show_cta2 {
                let id = b.fixed_id("hero-cta");
                // RTL: the primary button sits on the right of the pair.
                let (cta_x, cta2_x) = if show_cta && show_cta2 {
                    ((W - 400.0) / 2.0 + 200.0, (W - 400.0) / 2.0)
                } else if show_cta {
                    ((W - 200.0) / 2.0, 0.0)
                } else {
                    (0.0, (W - 180.0) / 2.0)
                };
                if show_cta {
                    let node = btn(
                        &id,
                        cta_x,
                        cy,
                        200.0,
                        50.0,
                        &cta,
                        16.0,
                        p.primary,
                        Color::WHITE,
                        12.0,
                    );
                    b.nodes.push(link_btn(node, &cta_route));
                }
                if show_cta2 {
                    if let Some(c2) = &s.cta2 {
                        let r2 = s.cta2_route.clone().unwrap_or_else(|| b.cta_target());
                        let id = b.id("hcta2");
                        let mut node = btn(
                            &id, cta2_x, cy, 180.0, 50.0, c2, 16.0, p.bg, p.primary, 12.0,
                        );
                        node.stroke = Some(p.primary);
                        node.stroke_width = 1.5;
                        b.nodes.push(link_btn(node, &r2));
                    }
                }
                cy + 50.0 + 56.0 - y
            } else {
                cy + 56.0 - y
            }
        }
    }
}

fn sec_features(b: &mut Builder, s: &Section, y: f32) -> f32 {
    let p = b.pal;
    let variant = s.variant.as_deref().unwrap_or("grid-3");
    let title = s.title.clone().unwrap_or_else(|| "ویژگی‌ها".into());
    let cy = y + b.heading(y, &title, s.subtitle.as_deref());
    match variant {
        "grid-4" => {
            let cols = 4;
            let card_h = 200.0;
            let rows = s.items.len().div_ceil(cols);
            for (i, it) in s.items.iter().enumerate() {
                let (x, cw) = grid_x(i % cols, cols, 20.0);
                let ry = cy + (i / cols) as f32 * (card_h + 20.0);
                let id = b.id("fcard");
                b.nodes
                    .push(card_rect(&id, x, ry, cw, card_h, p.bg, p.line));
                let id = b.id("fic");
                let mut ic = Node::new(
                    &id,
                    "Icon",
                    NodeKind::Ellipse,
                    x + cw - 64.0,
                    ry + 20.0,
                    44.0,
                    44.0,
                );
                ic.fill = Some(p.accent);
                b.nodes.push(ic);
                let id = b.id("fig");
                b.nodes.push(ctxt(
                    &id,
                    x + cw - 64.0,
                    ry + 28.0,
                    44.0,
                    GLYPHS[i % 8],
                    18.0,
                    p.primary,
                    true,
                ));
                let t = it.title.clone().unwrap_or_default();
                let id = b.id("ft");
                b.nodes.push(txt(
                    &id,
                    x + 16.0,
                    ry + 78.0,
                    cw - 32.0,
                    &t,
                    16.0,
                    p.ink,
                    true,
                ));
                if let Some(tx) = &it.text {
                    let id = b.id("fx");
                    b.nodes.push(txt(
                        &id,
                        x + 16.0,
                        ry + 112.0,
                        cw - 32.0,
                        tx,
                        13.0,
                        p.muted,
                        false,
                    ));
                }
            }
            cy + rows as f32 * (card_h + 20.0) - 20.0 - y
        }
        "rows" => {
            for (i, it) in s.items.iter().enumerate() {
                let rh = 210.0;
                let ry = cy + (i as f32) * (rh + 24.0);
                // Visual panel alternates sides.
                let (vx, tx) = if i % 2 == 0 { (MX, 700.0) } else { (720.0, MX) };
                let id = b.id("fvis");
                let mut panel = rect(&id, "Visual", vx, ry, 560.0, rh, p.accent, 16.0);
                panel.fill_gradient = Some(Gradient::new(
                    tint(p.primary, 0.82),
                    tint(p.primary, 0.94),
                    135.0,
                ));
                b.nodes.push(panel);
                // Mini window mockup inside the panel.
                let wx = vx + 120.0;
                let wy = ry + 45.0;
                let id = b.id("fwin");
                b.nodes
                    .push(card_rect(&id, wx, wy, 320.0, 120.0, Color::WHITE, p.line));
                for d in 0..3 {
                    let id = b.id("fdot");
                    let mut dot = Node::new(
                        &id,
                        "Dot",
                        NodeKind::Ellipse,
                        wx + 280.0 - (d as f32) * 20.0,
                        wy + 14.0,
                        10.0,
                        10.0,
                    );
                    dot.fill = Some(if d == 0 { p.primary } else { p.line });
                    b.nodes.push(dot);
                }
                let id = b.id("fbar1");
                b.nodes.push(rect(
                    &id,
                    "",
                    wx + 24.0,
                    wy + 46.0,
                    200.0,
                    12.0,
                    p.accent,
                    6.0,
                ));
                let id = b.id("fbar2");
                b.nodes.push(rect(
                    &id,
                    "",
                    wx + 24.0,
                    wy + 70.0,
                    140.0,
                    12.0,
                    p.line,
                    6.0,
                ));
                let id = b.id("fbar3");
                b.nodes.push(rect(
                    &id,
                    "",
                    wx + 24.0,
                    wy + 92.0,
                    90.0,
                    10.0,
                    tint(p.primary, 0.5),
                    5.0,
                ));
                let t = it.title.clone().unwrap_or_default();
                let id = b.id("ft");
                b.nodes
                    .push(txt(&id, tx, ry + 48.0, 580.0, &t, 22.0, p.ink, true));
                if let Some(tx2) = &it.text {
                    let id = b.id("fx");
                    b.nodes
                        .push(txt(&id, tx, ry + 96.0, 580.0, tx2, 15.0, p.muted, false));
                }
            }
            cy + s.items.len() as f32 * (210.0 + 24.0) - 24.0 - y
        }
        "list" => {
            let ly = cy;
            let half = s.items.len().div_ceil(2);
            for (i, it) in s.items.iter().enumerate() {
                let col = i / half;
                let row = i % half;
                let x = if col == 0 { RX - 620.0 } else { MX };
                let ry = ly + row as f32 * 76.0;
                let id = b.id("fchk");
                b.nodes
                    .push(txt(&id, x + 590.0, ry, 30.0, "✓", 18.0, p.primary, true));
                let t = it.title.clone().unwrap_or_default();
                let id = b.id("ft");
                b.nodes.push(txt(&id, x, ry, 580.0, &t, 16.0, p.ink, true));
                if let Some(tx) = &it.text {
                    let id = b.id("fx");
                    b.nodes
                        .push(txt(&id, x, ry + 28.0, 580.0, tx, 13.0, p.muted, false));
                }
            }
            ly + half as f32 * 76.0 - y
        }
        _ => {
            // grid-3
            let cols = 3;
            let card_h = 190.0;
            let rows = s.items.len().div_ceil(cols);
            for (i, it) in s.items.iter().enumerate() {
                let (x, cw) = grid_x(i % cols, cols, 24.0);
                let ry = cy + (i / cols) as f32 * (card_h + 24.0);
                let id = b.id("fcard");
                b.nodes
                    .push(card_rect(&id, x, ry, cw, card_h, p.bg, p.line));
                let id = b.id("fic");
                let mut ic = Node::new(
                    &id,
                    "Icon",
                    NodeKind::Ellipse,
                    x + cw - 68.0,
                    ry + 20.0,
                    48.0,
                    48.0,
                );
                ic.fill = Some(p.accent);
                b.nodes.push(ic);
                let id = b.id("fig");
                b.nodes.push(ctxt(
                    &id,
                    x + cw - 68.0,
                    ry + 29.0,
                    48.0,
                    GLYPHS[i % 8],
                    20.0,
                    p.primary,
                    true,
                ));
                let t = it.title.clone().unwrap_or_default();
                let id = b.id("ft");
                b.nodes.push(txt(
                    &id,
                    x + 20.0,
                    ry + 82.0,
                    cw - 40.0,
                    &t,
                    17.0,
                    p.ink,
                    true,
                ));
                if let Some(tx) = &it.text {
                    let id = b.id("fx");
                    b.nodes.push(txt(
                        &id,
                        x + 20.0,
                        ry + 116.0,
                        cw - 40.0,
                        tx,
                        14.0,
                        p.muted,
                        false,
                    ));
                }
            }
            cy + rows as f32 * (card_h + 24.0) - 24.0 - y
        }
    }
}

fn sec_services(b: &mut Builder, s: &Section, y: f32) -> f32 {
    let p = b.pal;
    let title = s.title.clone().unwrap_or_else(|| "خدمات".into());
    let mut cy = y + b.heading(y, &title, s.subtitle.as_deref());
    for it in &s.items {
        let t = it.title.clone().unwrap_or_default();
        let id = b.id("svt");
        b.nodes
            .push(txt(&id, RX - 760.0, cy, 760.0, &t, 17.0, p.ink, true));
        if let Some(tx) = &it.text {
            let id = b.id("svx");
            b.nodes.push(txt(
                &id,
                RX - 760.0,
                cy + 30.0,
                760.0,
                tx,
                13.0,
                p.muted,
                false,
            ));
        }
        if let Some(price) = it.price {
            let id = b.id("svp");
            b.nodes.push(txt(
                &id,
                MX,
                cy + 8.0,
                260.0,
                &fa_price(price),
                16.0,
                p.primary,
                true,
            ));
        }
        let id = b.id("svd");
        b.nodes
            .push(rect(&id, "", MX, cy + 66.0, CW, 1.0, p.line, 0.0));
        cy += 82.0;
    }
    cy - 16.0 - y
}

fn sec_products(b: &mut Builder, s: &Section, y: f32) -> f32 {
    let p = b.pal;
    let title = s.title.clone().unwrap_or_else(|| "محصولات".into());
    let cy = y + b.heading(y, &title, s.subtitle.as_deref());
    let cols = 4;
    let card_h = 300.0;
    let rows = s.items.len().div_ceil(cols);
    for (i, it) in s.items.iter().enumerate() {
        let (x, cw) = grid_x(i % cols, cols, 20.0);
        let ry = cy + (i / cols) as f32 * (card_h + 20.0);
        let id = b.id("pcard");
        b.nodes
            .push(card_rect(&id, x, ry, cw, card_h, p.bg, p.line));
        // Visual tile.
        let id = b.id("pvis");
        let shade = tint(p.primary, 0.80 + (i % 4) as f32 * 0.04);
        b.nodes.push(rect(
            &id,
            "Product visual",
            x + 12.0,
            ry + 12.0,
            cw - 24.0,
            140.0,
            shade,
            10.0,
        ));
        let name = it.name.clone().unwrap_or_default();
        let initial: String = name.chars().take(1).collect();
        let id = b.id("pini");
        b.nodes.push(ctxt(
            &id,
            x + 12.0,
            ry + 52.0,
            cw - 24.0,
            &initial,
            44.0,
            p.primary,
            true,
        ));
        if let Some(tag) = &it.tag {
            let id = b.id("ptag");
            b.nodes.push(btn(
                &id,
                x + cw - 92.0,
                ry + 22.0,
                80.0,
                26.0,
                tag,
                11.0,
                p.primary,
                Color::WHITE,
                13.0,
            ));
        }
        let id = b.id("pn");
        b.nodes.push(txt(
            &id,
            x + 16.0,
            ry + 166.0,
            cw - 32.0,
            &name,
            15.0,
            p.ink,
            true,
        ));
        if let Some(price) = it.price {
            let id = b.id("pp");
            b.nodes.push(txt(
                &id,
                x + 16.0,
                ry + 210.0,
                cw - 32.0,
                &fa_price(price),
                16.0,
                p.primary,
                true,
            ));
        }
        if let Some(old) = it.old_price {
            let id = b.id("po");
            b.nodes.push(txt(
                &id,
                x + 16.0,
                ry + 242.0,
                cw - 32.0,
                &fa_price(old),
                13.0,
                p.muted,
                false,
            ));
            let id = b.id("pst");
            b.nodes.push(rect(
                &id,
                "",
                x + cw - 132.0,
                ry + 252.0,
                116.0,
                1.0,
                p.muted,
                0.0,
            ));
        }
    }
    cy + rows as f32 * (card_h + 20.0) - 20.0 - y
}

fn sec_menu(b: &mut Builder, s: &Section, y: f32) -> f32 {
    let p = b.pal;
    let title = s.title.clone().unwrap_or_else(|| "منو".into());
    let cy = y + b.heading(y, &title, s.subtitle.as_deref());
    let half = s.items.len().div_ceil(2);
    for (i, it) in s.items.iter().enumerate() {
        let col = i / half;
        let row = i % half;
        let x = if col == 0 { RX - 620.0 } else { MX };
        let ry = cy + row as f32 * 84.0;
        let name = it.name.clone().unwrap_or_default();
        let id = b.id("mn");
        b.nodes
            .push(txt(&id, x, ry, 420.0, &name, 16.0, p.ink, true));
        if let Some(price) = it.price {
            let id = b.id("mp");
            b.nodes.push(txt(
                &id,
                x + 440.0,
                ry,
                180.0,
                &fa_price(price),
                15.0,
                p.primary,
                true,
            ));
        }
        if let Some(tx) = &it.text {
            let id = b.id("mx");
            b.nodes
                .push(txt(&id, x, ry + 30.0, 600.0, tx, 13.0, p.muted, false));
        }
        let id = b.id("md");
        b.nodes
            .push(rect(&id, "", x, ry + 68.0, 620.0, 1.0, p.line, 0.0));
    }
    cy + half as f32 * 84.0 - 16.0 - y
}

fn sec_pricing(b: &mut Builder, s: &Section, y: f32) -> f32 {
    let p = b.pal;
    let variant = s.variant.as_deref().unwrap_or("tiers-3");
    let title = s.title.clone().unwrap_or_else(|| "تعرفه‌ها".into());
    let mut cy = y + b.centered_heading(y, &title, s.subtitle.as_deref());
    if variant == "simple" {
        for t in &s.tiers {
            let id = b.id("prt");
            b.nodes
                .push(txt(&id, RX - 700.0, cy, 700.0, &t.name, 17.0, p.ink, true));
            let price_s = if t.price == 0 {
                "رایگان".to_string()
            } else {
                match &t.period {
                    Some(per) => format!("{} / {}", fa_price(t.price), per),
                    None => fa_price(t.price),
                }
            };
            let id = b.id("prp");
            b.nodes
                .push(txt(&id, MX, cy, 300.0, &price_s, 16.0, p.primary, true));
            let id = b.id("prd");
            b.nodes
                .push(rect(&id, "", MX, cy + 44.0, CW, 1.0, p.line, 0.0));
            cy += 60.0;
        }
        return cy - 16.0 - y;
    }
    // tiers-3
    let cols = 3;
    let card_h = 420.0;
    for (i, t) in s.tiers.iter().take(3).enumerate() {
        let (x, cw) = grid_x(i, cols, 24.0);
        let id = b.id("prcard");
        let fill = if t.featured { p.hero_bg } else { p.bg };
        let mut card = card_rect(&id, x, cy, cw, card_h, fill, p.line);
        if t.featured {
            card.stroke = Some(p.primary);
            card.stroke_width = 2.0;
        }
        b.nodes.push(card);
        let name_c = if t.featured { Color::WHITE } else { p.ink };
        let muted_c = if t.featured {
            rgb(203, 213, 225)
        } else {
            p.muted
        };
        let id = b.id("prn");
        b.nodes.push(txt(
            &id,
            x + 24.0,
            cy + 24.0,
            cw - 48.0,
            &t.name,
            19.0,
            name_c,
            true,
        ));
        if t.featured {
            let id = b.id("prf");
            b.nodes.push(btn(
                &id,
                x + 24.0,
                cy + 24.0,
                110.0,
                28.0,
                "پیشنهاد ما",
                12.0,
                p.primary,
                Color::WHITE,
                14.0,
            ));
        }
        let price_s = if t.price == 0 {
            "رایگان".to_string()
        } else {
            fa_price(t.price)
        };
        let id = b.id("prp");
        b.nodes.push(txt(
            &id,
            x + 24.0,
            cy + 64.0,
            cw - 48.0,
            &price_s,
            30.0,
            name_c,
            true,
        ));
        if let Some(per) = &t.period {
            let id = b.id("prper");
            b.nodes.push(txt(
                &id,
                x + 24.0,
                cy + 112.0,
                cw - 48.0,
                per,
                13.0,
                muted_c,
                false,
            ));
        }
        let mut fy = cy + 150.0;
        for f in &t.features {
            let id = b.id("prchk");
            b.nodes.push(txt(
                &id,
                x + cw - 48.0,
                fy,
                24.0,
                "✓",
                15.0,
                p.primary,
                true,
            ));
            let id = b.id("prf2");
            b.nodes
                .push(txt(&id, x + 24.0, fy, cw - 72.0, f, 14.0, muted_c, false));
            fy += 32.0;
        }
        let target = b.cta_target();
        let id = b.id("prbtn");
        let (fill, tc) = if t.featured {
            (Color::WHITE, p.dark)
        } else {
            (p.primary, Color::WHITE)
        };
        let node = btn(
            &id,
            x + 24.0,
            cy + card_h - 64.0,
            cw - 48.0,
            44.0,
            "انتخاب",
            15.0,
            fill,
            tc,
            10.0,
        );
        b.nodes.push(link_btn(node, &target));
    }
    cy + card_h - y
}

fn sec_gallery(b: &mut Builder, s: &Section, y: f32) -> f32 {
    let p = b.pal;
    let title = s.title.clone().unwrap_or_else(|| "گالری".into());
    let cy = y + b.heading(y, &title, s.subtitle.as_deref());
    let cols = 3;
    let tile_h = 230.0;
    let rows = s.items.len().div_ceil(cols);
    for (i, it) in s.items.iter().enumerate() {
        let (x, cw) = grid_x(i % cols, cols, 24.0);
        let ry = cy + (i / cols) as f32 * (tile_h + 24.0);
        let id = b.id("gtile");
        let shade = tint(p.primary, 0.78 + (i % 5) as f32 * 0.04);
        let mut tile = rect(&id, "Gallery tile", x, ry, cw, tile_h, shade, 14.0);
        tile.fill_gradient = Some(Gradient::new(
            tint(p.primary, 0.72 + (i % 3) as f32 * 0.05),
            tint(p.dark, 0.55),
            135.0,
        ));
        b.nodes.push(tile);
        let caption = it.caption.clone().unwrap_or_default();
        if !caption.is_empty() {
            let id = b.id("gcap");
            b.nodes.push(txt(
                &id,
                x + 16.0,
                ry + tile_h - 40.0,
                cw - 32.0,
                &caption,
                15.0,
                Color::WHITE,
                true,
            ));
        }
    }
    cy + rows as f32 * (tile_h + 24.0) - 24.0 - y
}

fn sec_testimonials(b: &mut Builder, s: &Section, y: f32) -> f32 {
    let p = b.pal;
    let variant = s.variant.as_deref().unwrap_or("cards-3");
    let title = s.title.clone().unwrap_or_else(|| "نظر مشتریان".into());
    if variant == "quote" {
        let mut cy = y + b.centered_heading(y, &title, s.subtitle.as_deref());
        if let Some(it) = s.items.first() {
            let text = it.text.clone().unwrap_or_default();
            let th = text_h(&text, 22.0, 820.0);
            let id = b.id("qtext");
            b.nodes.push(ctxt(
                &id,
                (W - 820.0) / 2.0,
                cy,
                820.0,
                &text,
                22.0,
                p.ink,
                false,
            ));
            cy += th + 16.0;
            let author = it.author.clone().unwrap_or_else(|| "مشتری نمونه".into());
            let id = b.id("qauth");
            b.nodes.push(ctxt(
                &id,
                (W - 400.0) / 2.0,
                cy,
                400.0,
                &author,
                14.0,
                p.muted,
                true,
            ));
            cy += 32.0;
        }
        return cy - y;
    }
    let cy = y + b.heading(y, &title, s.subtitle.as_deref());
    let cols = 3;
    let card_h = 210.0;
    for (i, it) in s.items.iter().take(3).enumerate() {
        let (x, cw) = grid_x(i, cols, 24.0);
        let id = b.id("tcard");
        b.nodes
            .push(card_rect(&id, x, cy, cw, card_h, p.bg, p.line));
        let text = it.text.clone().unwrap_or_default();
        let id = b.id("tt");
        b.nodes.push(txt(
            &id,
            x + 20.0,
            cy + 20.0,
            cw - 40.0,
            &text,
            15.0,
            p.ink,
            false,
        ));
        let author = it.author.clone().unwrap_or_else(|| "مشتری نمونه".into());
        let id = b.id("ta");
        b.nodes.push(txt(
            &id,
            x + 20.0,
            cy + card_h - 44.0,
            cw - 40.0,
            &author,
            13.0,
            p.muted,
            true,
        ));
    }
    cy + card_h - y
}

fn sec_team(b: &mut Builder, s: &Section, y: f32) -> f32 {
    let p = b.pal;
    let title = s.title.clone().unwrap_or_else(|| "تیم ما".into());
    let cy = y + b.heading(y, &title, s.subtitle.as_deref());
    let cols = 4;
    let card_h = 220.0;
    let rows = s.items.len().div_ceil(cols);
    for (i, it) in s.items.iter().enumerate() {
        let (x, cw) = grid_x(i % cols, cols, 20.0);
        let ry = cy + (i / cols) as f32 * (card_h + 20.0);
        let id = b.id("tmcard");
        b.nodes
            .push(card_rect(&id, x, ry, cw, card_h, p.bg, p.line));
        let name = it.name.clone().unwrap_or_default();
        let initial: String = name.chars().take(1).collect();
        let id = b.id("tmav");
        let mut av = Node::new(
            &id,
            "Avatar",
            NodeKind::Ellipse,
            x + (cw - 72.0) / 2.0,
            ry + 24.0,
            72.0,
            72.0,
        );
        av.fill = Some(p.accent);
        b.nodes.push(av);
        let id = b.id("tmi");
        b.nodes.push(ctxt(
            &id,
            x + (cw - 72.0) / 2.0,
            ry + 42.0,
            72.0,
            &initial,
            28.0,
            p.primary,
            true,
        ));
        let id = b.id("tmn");
        b.nodes.push(ctxt(
            &id,
            x + 12.0,
            ry + 112.0,
            cw - 24.0,
            &name,
            16.0,
            p.ink,
            true,
        ));
        if let Some(role) = &it.role {
            let id = b.id("tmr");
            b.nodes.push(ctxt(
                &id,
                x + 12.0,
                ry + 144.0,
                cw - 24.0,
                role,
                13.0,
                p.muted,
                false,
            ));
        }
    }
    cy + rows as f32 * (card_h + 20.0) - 20.0 - y
}

fn sec_faq(b: &mut Builder, s: &Section, y: f32) -> f32 {
    let p = b.pal;
    let title = s.title.clone().unwrap_or_else(|| "پرسش‌های متداول".into());
    let mut cy = y + b.heading(y, &title, s.subtitle.as_deref());
    for it in &s.items {
        let q = it.q.clone().unwrap_or_default();
        let qh = text_h(&q, 16.0, 1000.0);
        let id = b.id("fqq");
        b.nodes
            .push(txt(&id, RX - 1000.0, cy, 1000.0, &q, 16.0, p.ink, true));
        cy += qh + 6.0;
        if let Some(a) = &it.a {
            let ah = text_h(a, 14.0, 1000.0);
            let id = b.id("fqa");
            b.nodes
                .push(txt(&id, RX - 1000.0, cy, 1000.0, a, 14.0, p.muted, false));
            cy += ah + 6.0;
        }
        let id = b.id("fqd");
        b.nodes
            .push(rect(&id, "", MX, cy + 8.0, CW, 1.0, p.line, 0.0));
        cy += 28.0;
    }
    cy - 12.0 - y
}

fn sec_cta(b: &mut Builder, s: &Section, y: f32) -> f32 {
    let p = b.pal;
    let h = 220.0;
    let id = b.id("ctaband");
    let mut band = rect(&id, "CTA band", MX, y, CW, h, p.primary, 18.0);
    band.fill_gradient = Some(Gradient::new(p.primary, p.dark, 120.0));
    b.nodes.push(band);
    let title = s.title.clone().unwrap_or_default();
    let id = b.id("ctat");
    b.nodes.push(ctxt(
        &id,
        220.0,
        y + 36.0,
        1000.0,
        &title,
        26.0,
        Color::WHITE,
        true,
    ));
    let mut by = y + 104.0;
    if let Some(text) = &s.text {
        let id = b.id("ctax");
        b.nodes.push(ctxt(
            &id,
            280.0,
            y + 92.0,
            880.0,
            text,
            15.0,
            rgb(226, 232, 240),
            false,
        ));
        by = y + 140.0;
    }
    if let Some(label) = &s.button {
        let route = s.button_route.clone().unwrap_or_else(|| b.cta_target());
        let id = b.fixed_id("cta-btn");
        let node = btn(
            &id,
            (W - 200.0) / 2.0,
            by,
            200.0,
            46.0,
            label,
            15.0,
            Color::WHITE,
            p.dark,
            12.0,
        );
        b.nodes.push(link_btn(node, &route));
    }
    h
}

fn sec_contact(b: &mut Builder, s: &Section, y: f32) -> f32 {
    let p = b.pal;
    let title = s.title.clone().unwrap_or_else(|| "تماس با ما".into());
    let cy = y + b.heading(y, &title, s.subtitle.as_deref());
    let id = b.id("cncard");
    b.nodes
        .push(card_rect(&id, MX, cy, CW, 230.0, p.bg, p.line));
    let rows: Vec<(&str, Option<String>)> = vec![
        ("تلفن", s.phone.clone()),
        ("ایمیل", s.email.clone()),
        ("نشانی", s.address.clone()),
        ("ساعت کاری", s.hours.clone()),
    ];
    let mut ry = cy + 28.0;
    for (label, value) in rows.into_iter().filter(|(_, v)| v.is_some()) {
        let value = value.unwrap();
        // A bare number left-anchors (bidi); lead with a Persian word.
        let value = if label == "تلفن" {
            format!("تلفن: {value}")
        } else {
            value
        };
        let id = b.id("cnl");
        b.nodes
            .push(txt(&id, RX - 200.0, ry, 180.0, label, 14.0, p.muted, true));
        let id = b.id("cnv");
        b.nodes
            .push(txt(&id, MX + 20.0, ry, 900.0, &value, 15.0, p.ink, false));
        ry += 46.0;
    }
    cy + 230.0 - y
}

fn sec_booking(b: &mut Builder, s: &Section, y: f32) -> f32 {
    let p = b.pal;
    let title = s.title.clone().unwrap_or_else(|| "رزرو".into());
    let cy = y + b.centered_heading(y, &title, s.text.as_deref());
    let card_w = 640.0;
    let card_x = (W - card_w) / 2.0;
    // Card height from the content: 28 top pad + 84 per field + 8 gap
    // + 48 submit + 28 bottom pad.
    let card_h = 112.0 + s.fields.len() as f32 * 84.0;
    let id = b.id("bkcard");
    b.nodes
        .push(card_rect(&id, card_x, cy, card_w, card_h, p.bg, p.line));
    let mut fy = cy + 28.0;
    for (i, field) in s.fields.iter().enumerate() {
        let id = b.id("bklabel");
        b.nodes.push(txt(
            &id,
            card_x + 28.0,
            fy,
            card_w - 56.0,
            field,
            14.0,
            p.ink,
            true,
        ));
        let id = format!("{}-field-{}", b.page_id, i);
        let mut input = Node::new(
            &id,
            field,
            NodeKind::Input,
            card_x + 28.0,
            fy + 28.0,
            card_w - 56.0,
            44.0,
        );
        input.fill = Some(p.surface);
        input.corner_radius = 10.0;
        input.stroke = Some(p.line);
        input.stroke_width = 1.0;
        input.text = Some(field.clone());
        input.font_size = 14.0;
        input.text_color = Some(p.muted);
        b.nodes.push(input);
        fy += 84.0;
    }
    let submit = s.submit.clone().unwrap_or_else(|| "ثبت".into());
    let id = b.fixed_id("submit");
    b.nodes.push(btn(
        &id,
        card_x + 28.0,
        fy + 8.0,
        card_w - 56.0,
        48.0,
        &submit,
        16.0,
        p.primary,
        Color::WHITE,
        12.0,
    ));
    cy + card_h - y
}

fn sec_hours(b: &mut Builder, s: &Section, y: f32) -> f32 {
    let p = b.pal;
    let title = s.title.clone().unwrap_or_else(|| "ساعت کاری".into());
    let mut cy = y + b.heading(y, &title, s.subtitle.as_deref());
    for it in &s.items {
        let day = it.day.clone().unwrap_or_default();
        let id = b.id("hday");
        b.nodes
            .push(txt(&id, RX - 600.0, cy, 600.0, &day, 16.0, p.ink, true));
        if let Some(time) = &it.time {
            let id = b.id("htime");
            b.nodes
                .push(txt(&id, MX, cy, 400.0, time, 15.0, p.muted, false));
        }
        let id = b.id("hd");
        b.nodes
            .push(rect(&id, "", MX, cy + 36.0, CW, 1.0, p.line, 0.0));
        cy += 52.0;
    }
    cy - 16.0 - y
}

fn sec_text(b: &mut Builder, s: &Section, y: f32) -> f32 {
    let p = b.pal;
    let mut cy = y;
    if let Some(title) = &s.title {
        cy += b.heading(y, title, s.subtitle.as_deref());
    }
    for para in &s.paragraphs {
        let ph = text_h(para, 16.0, 1000.0);
        let id = b.id("tp");
        b.nodes
            .push(txt(&id, RX - 1000.0, cy, 1000.0, para, 16.0, p.ink, false));
        cy += ph + 14.0;
    }
    cy - y
}

fn sec_schedule(b: &mut Builder, s: &Section, y: f32) -> f32 {
    let p = b.pal;
    let title = s.title.clone().unwrap_or_else(|| "برنامه".into());
    let mut cy = y + b.heading(y, &title, s.subtitle.as_deref());
    for it in &s.items {
        if let Some(time) = &it.time {
            let id = b.id("schchip");
            b.nodes.push(btn(
                &id,
                RX - 190.0,
                cy,
                190.0,
                36.0,
                time,
                13.0,
                p.accent,
                p.primary,
                18.0,
            ));
        }
        let t = it.title.clone().unwrap_or_default();
        let id = b.id("scht");
        b.nodes
            .push(txt(&id, MX, cy + 4.0, 900.0, &t, 16.0, p.ink, true));
        if let Some(tx) = &it.text {
            let id = b.id("schx");
            b.nodes
                .push(txt(&id, MX, cy + 32.0, 900.0, tx, 13.0, p.muted, false));
        }
        let id = b.id("schd");
        b.nodes
            .push(rect(&id, "", MX, cy + 66.0, CW, 1.0, p.line, 0.0));
        cy += 82.0;
    }
    cy - 16.0 - y
}

fn sec_courses(b: &mut Builder, s: &Section, y: f32) -> f32 {
    let p = b.pal;
    let title = s.title.clone().unwrap_or_else(|| "دوره‌ها".into());
    let cy = y + b.heading(y, &title, s.subtitle.as_deref());
    let cols = 3;
    let card_h = 240.0;
    let rows = s.items.len().div_ceil(cols);
    for (i, it) in s.items.iter().enumerate() {
        let (x, cw) = grid_x(i % cols, cols, 24.0);
        let ry = cy + (i / cols) as f32 * (card_h + 24.0);
        let id = b.id("ccard");
        b.nodes
            .push(card_rect(&id, x, ry, cw, card_h, p.bg, p.line));
        let t = it.title.clone().unwrap_or_default();
        let id = b.id("ct");
        b.nodes.push(txt(
            &id,
            x + 20.0,
            ry + 20.0,
            cw - 40.0,
            &t,
            17.0,
            p.ink,
            true,
        ));
        if let Some(tx) = &it.text {
            let id = b.id("cx");
            b.nodes.push(txt(
                &id,
                x + 20.0,
                ry + 56.0,
                cw - 40.0,
                tx,
                13.0,
                p.muted,
                false,
            ));
        }
        if let Some(meta) = &it.meta {
            let id = b.id("cm");
            b.nodes.push(txt(
                &id,
                x + 20.0,
                ry + card_h - 72.0,
                cw - 40.0,
                meta,
                13.0,
                p.muted,
                false,
            ));
        }
        if let Some(price) = it.price {
            let id = b.id("cp");
            b.nodes.push(txt(
                &id,
                x + 20.0,
                ry + card_h - 44.0,
                cw - 40.0,
                &fa_price(price),
                16.0,
                p.primary,
                true,
            ));
        }
    }
    cy + rows as f32 * (card_h + 24.0) - 24.0 - y
}

fn sec_listings(b: &mut Builder, s: &Section, y: f32) -> f32 {
    let p = b.pal;
    let title = s.title.clone().unwrap_or_else(|| "آگهی‌ها".into());
    let cy = y + b.heading(y, &title, s.subtitle.as_deref());
    let cols = 3;
    let card_h = 290.0;
    let rows = s.items.len().div_ceil(cols);
    for (i, it) in s.items.iter().enumerate() {
        let (x, cw) = grid_x(i % cols, cols, 24.0);
        let ry = cy + (i / cols) as f32 * (card_h + 24.0);
        let id = b.id("lcard");
        b.nodes
            .push(card_rect(&id, x, ry, cw, card_h, p.bg, p.line));
        let id = b.id("lvis");
        let mut vis = rect(
            &id,
            "Listing visual",
            x + 12.0,
            ry + 12.0,
            cw - 24.0,
            130.0,
            tint(p.primary, 0.85),
            10.0,
        );
        vis.fill_gradient = Some(Gradient::new(
            tint(p.primary, 0.78),
            tint(p.dark, 0.60),
            135.0,
        ));
        b.nodes.push(vis);
        let t = it.title.clone().unwrap_or_default();
        let id = b.id("lt");
        b.nodes.push(txt(
            &id,
            x + 16.0,
            ry + 156.0,
            cw - 32.0,
            &t,
            16.0,
            p.ink,
            true,
        ));
        if let Some(meta) = &it.meta {
            let id = b.id("lm");
            b.nodes.push(txt(
                &id,
                x + 16.0,
                ry + 190.0,
                cw - 32.0,
                meta,
                13.0,
                p.muted,
                false,
            ));
        }
        if let Some(price) = it.price {
            let id = b.id("lp");
            b.nodes.push(txt(
                &id,
                x + 16.0,
                ry + card_h - 44.0,
                cw - 32.0,
                &format!("{} تومان", fa_num(price)),
                16.0,
                p.primary,
                true,
            ));
        }
    }
    cy + rows as f32 * (card_h + 24.0) - 24.0 - y
}

fn sec_events(b: &mut Builder, s: &Section, y: f32) -> f32 {
    let p = b.pal;
    let title = s.title.clone().unwrap_or_else(|| "رویدادها".into());
    let mut cy = y + b.heading(y, &title, s.subtitle.as_deref());
    for it in &s.items {
        if let Some(date) = &it.date {
            let id = b.id("evchip");
            b.nodes.push(btn(
                &id,
                RX - 150.0,
                cy,
                150.0,
                40.0,
                date,
                13.0,
                p.accent,
                p.primary,
                10.0,
            ));
        }
        let t = it.title.clone().unwrap_or_default();
        let id = b.id("evt");
        b.nodes
            .push(txt(&id, MX, cy + 6.0, 900.0, &t, 17.0, p.ink, true));
        if let Some(tx) = &it.text {
            let id = b.id("evx");
            b.nodes
                .push(txt(&id, MX, cy + 34.0, 900.0, tx, 13.0, p.muted, false));
        }
        let id = b.id("evd");
        b.nodes
            .push(rect(&id, "", MX, cy + 74.0, CW, 1.0, p.line, 0.0));
        cy += 90.0;
    }
    cy - 16.0 - y
}

fn sec_steps(b: &mut Builder, s: &Section, y: f32) -> f32 {
    let p = b.pal;
    let title = s.title.clone().unwrap_or_else(|| "چگونه کار می‌کند؟".into());
    let cy = y + b.centered_heading(y, &title, s.subtitle.as_deref());
    let cols = s.items.len().max(1);
    let card_h = 190.0;
    for (i, it) in s.items.iter().enumerate() {
        let (x, cw) = grid_x(i, cols, 24.0);
        let id = b.id("stcard");
        b.nodes
            .push(card_rect(&id, x, cy, cw, card_h, p.bg, p.line));
        let id = b.id("stnum");
        let mut num = Node::new(
            &id,
            "Step number",
            NodeKind::Ellipse,
            x + cw - 64.0,
            cy + 20.0,
            44.0,
            44.0,
        );
        num.fill = Some(p.primary);
        b.nodes.push(num);
        let id = b.id("stn");
        b.nodes.push(ctxt(
            &id,
            x + cw - 64.0,
            cy + 29.0,
            44.0,
            &fa_num((i + 1) as i64),
            18.0,
            Color::WHITE,
            true,
        ));
        let t = it.title.clone().unwrap_or_default();
        let id = b.id("stt");
        b.nodes.push(txt(
            &id,
            x + 20.0,
            cy + 80.0,
            cw - 40.0,
            &t,
            16.0,
            p.ink,
            true,
        ));
        if let Some(tx) = &it.text {
            let id = b.id("stx");
            b.nodes.push(txt(
                &id,
                x + 20.0,
                cy + 112.0,
                cw - 40.0,
                tx,
                13.0,
                p.muted,
                false,
            ));
        }
    }
    cy + card_h - y
}

fn sec_dashboard(b: &mut Builder, s: &Section, y: f32) -> f32 {
    let p = b.pal;
    let title = s.title.clone().unwrap_or_else(|| "داشبورد".into());
    let mut cy = y + b.heading(y, &title, s.subtitle.as_deref());
    let cols = 4;
    let card_h = 120.0;
    for (i, kpi) in s.kpis.iter().take(4).enumerate() {
        let (x, cw) = grid_x(i, cols, 20.0);
        let id = b.id("kcard");
        b.nodes
            .push(card_rect(&id, x, cy, cw, card_h, p.bg, p.line));
        let id = b.id("kl");
        b.nodes.push(txt(
            &id,
            x + 16.0,
            cy + 18.0,
            cw - 32.0,
            &kpi.label,
            13.0,
            p.muted,
            false,
        ));
        let id = b.id("kv");
        b.nodes.push(txt(
            &id,
            x + 16.0,
            cy + 50.0,
            cw - 32.0,
            &kpi.value,
            22.0,
            p.ink,
            true,
        ));
    }
    cy += card_h + 24.0;
    // Simple bar chart panel.
    let id = b.id("chart");
    b.nodes
        .push(card_rect(&id, MX, cy, CW, 260.0, p.bg, p.line));
    let id = b.id("chartt");
    b.nodes.push(txt(
        &id,
        RX - 400.0,
        cy + 20.0,
        400.0,
        "نمودار فروش",
        16.0,
        p.ink,
        true,
    ));
    let heights = [
        90.0, 140.0, 110.0, 170.0, 130.0, 185.0, 150.0, 200.0, 165.0, 120.0, 175.0, 145.0,
    ];
    for (i, h) in heights.iter().enumerate() {
        let bw = 60.0;
        let gap = 36.0;
        let x = RX - 80.0 - bw - (i as f32) * (bw + gap);
        let id = b.id("bar");
        b.nodes.push(rect(
            &id,
            "Bar",
            x,
            cy + 230.0 - h,
            bw,
            *h,
            tint(p.primary, 0.25 + (i % 3) as f32 * 0.2),
            6.0,
        ));
    }
    cy + 260.0 - y
}

fn sec_table(b: &mut Builder, s: &Section, y: f32) -> f32 {
    let p = b.pal;
    let title = s.title.clone().unwrap_or_else(|| "جدول".into());
    let mut cy = y + b.heading(y, &title, s.subtitle.as_deref());
    let table = s.table.clone().unwrap_or_default();
    let id = b.id("tbl");
    let mut t = Node::new(&id, "Data table", NodeKind::Table, MX, cy, CW, 320.0);
    t.fill = Some(p.surface);
    t.corner_radius = 12.0;
    t.stroke = Some(p.line);
    t.stroke_width = 1.0;
    t.props.insert("table".into(), table);
    b.nodes.push(t);
    cy += 320.0;
    cy - y
}

fn build_section(b: &mut Builder, s: &Section, y: f32) -> f32 {
    match s.kind.as_str() {
        "hero" => sec_hero(b, s, y),
        "features" => sec_features(b, s, y),
        "services" => sec_services(b, s, y),
        "products" => sec_products(b, s, y),
        "menu" => sec_menu(b, s, y),
        "pricing" => sec_pricing(b, s, y),
        "gallery" => sec_gallery(b, s, y),
        "testimonials" => sec_testimonials(b, s, y),
        "team" => sec_team(b, s, y),
        "faq" => sec_faq(b, s, y),
        "cta" => sec_cta(b, s, y),
        "contact" => sec_contact(b, s, y),
        "booking" => sec_booking(b, s, y),
        "hours" => sec_hours(b, s, y),
        "text" => sec_text(b, s, y),
        "schedule" => sec_schedule(b, s, y),
        "courses" => sec_courses(b, s, y),
        "listings" => sec_listings(b, s, y),
        "events" => sec_events(b, s, y),
        "steps" => sec_steps(b, s, y),
        "dashboard" => sec_dashboard(b, s, y),
        "table" => sec_table(b, s, y),
        _ => 0.0,
    }
}

// ---------------------------------------------------------------------
// Database and flows.
// ---------------------------------------------------------------------

fn column_type(t: &str) -> ColumnType {
    match t {
        "integer" => ColumnType::Integer,
        "money" => ColumnType::Money,
        "date" => ColumnType::Date,
        "boolean" => ColumnType::Boolean,
        _ => ColumnType::Text,
    }
}

fn build_database(pack: &Pack) -> Database {
    let mut tables = Vec::new();
    for pt in &pack.tables {
        let mut table = Table::new(&pt.name);
        table.label = pt.label.clone().unwrap_or_else(|| pt.name.clone());
        for col in &pt.columns {
            table = table.column(Column::new(&col.name, column_type(&col.col_type)));
        }
        for row in &pt.rows {
            let mut m = Map::new();
            for (i, col) in pt.columns.iter().enumerate() {
                if let Some(v) = row.get(i) {
                    m.insert(col.name.clone(), v.clone());
                }
            }
            table.seed_rows.push(m);
        }
        tables.push(table);
    }
    Database {
        tables,
        relations: Vec::new(),
    }
}

fn build_flows(pack: &Pack) -> Vec<Flow> {
    let Some(f) = &pack.flow else {
        return Vec::new();
    };
    // The trigger page is the page whose id prefixes the button id.
    let page_route = pack
        .pages
        .iter()
        .find(|pg| f.button.starts_with(&format!("{}-", pg.id)))
        .map(|pg| pg.route.clone());
    let mut steps = Vec::new();
    if f.kind == "save" {
        if let Some(table) = &f.table {
            let mut mapping = Map::new();
            for field in &f.fields {
                mapping.insert(field.column.clone(), field.value.clone());
            }
            steps.push(Step::Action {
                action: Action::SaveRecord {
                    table: table.clone(),
                    mapping,
                },
            });
        }
    }
    if let Some(message) = &f.message {
        steps.push(Step::Action {
            action: Action::ShowMessage {
                text: message.clone(),
            },
        });
    }
    if let Some(route) = &f.navigate {
        steps.push(Step::Action {
            action: Action::Navigate {
                route: route.clone(),
            },
        });
    }
    if steps.is_empty() {
        return Vec::new();
    }
    vec![Flow {
        id: format!("{}-flow", pack.id),
        name: "Form submit".into(),
        trigger: Event {
            kind: EventKind::Click,
            source_node: Some(f.button.clone()),
            page_route,
        },
        steps,
    }]
}

// ---------------------------------------------------------------------
// Entry point.
// ---------------------------------------------------------------------

pub fn build_project(pack: &Pack) -> ProjectFile {
    let pal = resolve_palette(&pack.palette, pack.dark);
    let mut project = Project::new(pack.id.clone(), pack.brand.clone());
    project.description = pack.description.clone();
    project.theme.primary = pal.primary;
    project.theme.background = pal.bg;
    project.theme.surface = pal.surface;
    project.theme.text = pal.ink;
    project.theme.muted_text = pal.muted;
    project.theme.dark = pack.dark;

    for page in &pack.pages {
        let chrome = page.chrome.unwrap_or(true);
        let mut b = Builder::new(pack, pal, &page.id);
        let mut y = 0.0;
        if chrome {
            y += build_navbar(&mut b, y);
        }
        for s in &page.sections {
            let h = build_section(&mut b, s, y + SEC_GAP / 2.0);
            y += h + SEC_GAP;
        }
        if chrome {
            y += build_footer(&mut b, y);
        }
        let height = (y + 24.0).max(600.0);
        let mut pg = Page::new(
            page.id.clone(),
            page.name.clone(),
            page.route.clone(),
            W,
            height,
        );
        pg.background = Some(pal.bg);
        pg.root.children = b.nodes;
        project.pages.push(pg);
    }

    ProjectFile {
        project,
        database: build_database(pack),
        flows: build_flows(pack),
    }
}
