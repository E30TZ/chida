//! Chida editor, web edition: a professional design tool UI in the
//! spirit of Figma and FigCraft, Persian first, with Chida's three
//! level simplicity layer (ساده، سازنده، حرفه‌ای) on top.
//!
//! Layout: dark top bar (project, undo and redo, levels, zoom,
//! actions), a Figma style tool rail, a pages and layers panel, a
//! pannable and zoomable canvas with a white artboard, selection
//! handles with corner resize, and a tabbed right panel (Design,
//! Logic, Data, Code). Edits persist to localStorage and restore on
//! reload. The demo bundle in `fixtures/demo-project.json` is
//! generated from the real Chida CLI shop template (project, database
//! and flows), see `generate-fixture.sh`. This same UI is the future
//! desktop editor UI.

use chida_core::{Color, Gradient, LayoutAlign, LayoutDirection, Node, NodeKind, Page, Project, Shadow};
use chida_db::Database;
use chida_logic::{Action, EventKind, Flow, Step};
use leptos::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use wasm_bindgen::JsCast;

const DEMO_JSON: &str = include_str!("../fixtures/demo-project.json");
const STORAGE_KEY: &str = "chida-editor-bundle-v3";

/// A user-saved named style: the paint and text look of one node,
/// captured so it can be reapplied to any other node in one click.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct NamedStyle {
    name: String,
    fill: Option<String>,
    gradient: Option<(String, String, f32)>,
    text_color: Option<String>,
    font_size: f32,
    font_weight: u16,
    radius: f32,
}

/// The `.chida.json` bundle: design, data and logic, same shape as
/// the CLI project file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct Bundle {
    project: Project,
    #[serde(default)]
    database: Database,
    #[serde(default)]
    flows: Vec<Flow>,
    #[serde(default)]
    styles: Vec<NamedStyle>,
}

fn fixture_bundle() -> Bundle {
    serde_json::from_str(DEMO_JSON).expect("bundled demo fixture must parse")
}

fn load_bundle() -> Bundle {
    if let Some(w) = web_sys::window() {
        if let Ok(Some(storage)) = w.local_storage() {
            if let Ok(Some(raw)) = storage.get_item(STORAGE_KEY) {
                if let Ok(b) = serde_json::from_str::<Bundle>(&raw) {
                    return b;
                }
            }
        }
    }
    fixture_bundle()
}

#[derive(Debug, Clone, PartialEq)]
struct FlatNode {
    id: String,
    name: String,
    kind: NodeKind,
    depth: usize,
    ax: f32,
    ay: f32,
    /// Absolute position of the parent (for converting drag deltas).
    pox: f32,
    poy: f32,
    width: f32,
    height: f32,
    fill: Option<String>,
    gradient: Option<(String, String, f32)>,
    shadow: Option<(f32, f32, f32, String, f32)>,
    stroke: Option<String>,
    stroke_width: f32,
    text: Option<String>,
    font_size: f32,
    text_color: Option<String>,
    radius: f32,
    bold: bool,
    weight: u16,
    line_height: f32,
    letter_spacing: f32,
    opacity: f32,
    visible: bool,
    align: Option<String>,
    src: Option<String>,
    /// Preview navigation target from the `link_page` prop.
    link: Option<String>,
}

/// The CSS rgba() form of a hex colour plus an opacity multiplier,
/// used for box shadows in the editor canvas.
fn hex_rgba(hex: &str, opacity: f32) -> String {
    let c = Color::from_hex(hex).unwrap_or(Color::BLACK);
    let a = (c.a as f32 / 255.0) * opacity.clamp(0.0, 1.0);
    format!("rgba({},{},{},{:.3})", c.r, c.g, c.b, a)
}

fn kind_fa(kind: NodeKind) -> &'static str {
    match kind {
        NodeKind::Frame => "فریم",
        NodeKind::Rectangle => "مستطیل",
        NodeKind::Ellipse => "دایره",
        NodeKind::Text => "متن",
        NodeKind::Button => "دکمه",
        NodeKind::Card => "کارت",
        NodeKind::Image => "تصویر",
        NodeKind::Input => "ورودی",
        NodeKind::Divider => "خط جدا کننده",
        NodeKind::Icon => "آیکون",
        NodeKind::List => "فهرست",
        NodeKind::Table => "جدول",
        NodeKind::Chart => "نمودار",
        NodeKind::Custom => "بلوک",
    }
}

fn kind_icon(kind: NodeKind) -> &'static str {
    match kind {
        NodeKind::Frame => "▢",
        NodeKind::Rectangle => "▭",
        NodeKind::Ellipse => "●",
        NodeKind::Text => "T",
        NodeKind::Button => "▣",
        NodeKind::Card => "▤",
        NodeKind::Image => "🖼",
        NodeKind::Input => "⌨",
        NodeKind::Divider => "―",
        NodeKind::Icon => "✦",
        NodeKind::List => "☰",
        NodeKind::Table => "▦",
        NodeKind::Chart => "📊",
        NodeKind::Custom => "◆",
    }
}

fn kind_name(kind: NodeKind) -> &'static str {
    match kind {
        NodeKind::Frame => "frame",
        NodeKind::Rectangle => "rectangle",
        NodeKind::Ellipse => "ellipse",
        NodeKind::Text => "text",
        NodeKind::Button => "button",
        NodeKind::Card => "card",
        NodeKind::Image => "image",
        NodeKind::Input => "input",
        NodeKind::Divider => "divider",
        NodeKind::Icon => "icon",
        NodeKind::List => "list",
        NodeKind::Table => "table",
        NodeKind::Chart => "chart",
        NodeKind::Custom => "custom",
    }
}

/// Flatten a page tree to absolute coordinates. When `include_hidden`
/// is false, hidden subtrees are skipped (the canvas); the layers
/// panel passes true so hidden nodes can be shown again.
fn flatten(node: &Node, ox: f32, oy: f32, depth: usize, include_hidden: bool, out: &mut Vec<FlatNode>) {
    let (ax, ay) = (ox + node.x, oy + node.y);
    if node.id != "root" {
        if !node.visible && !include_hidden {
            return;
        }
        out.push(FlatNode {
            id: node.id.clone(),
            name: node.name.clone(),
            kind: node.kind,
            depth,
            ax,
            ay,
            pox: ox,
            poy: oy,
            width: node.width,
            height: node.height,
            fill: node.fill.map(|c| c.to_hex()),
            gradient: node
                .fill_gradient
                .map(|g| (g.from.to_hex(), g.to.to_hex(), g.angle)),
            shadow: node.shadow.map(|s| (s.x, s.y, s.blur, s.color.to_hex(), s.opacity)),
            stroke: node.stroke.map(|c| c.to_hex()),
            stroke_width: node.stroke_width,
            text: node.text.clone(),
            font_size: node.font_size,
            text_color: node.text_color.map(|c| c.to_hex()),
            radius: node.corner_radius,
            bold: node.bold,
            weight: node.effective_weight(),
            line_height: node.line_height,
            letter_spacing: node.letter_spacing,
            opacity: node.opacity,
            visible: node.visible,
            align: node.props.get("align").cloned(),
            src: node.props.get("src").cloned(),
            link: node.props.get("link_page").cloned(),
        });
    }
    for child in &node.children {
        flatten(child, ax, ay, depth + 1, include_hidden, out);
    }
}

fn page_flat(page: &Page, include_hidden: bool) -> Vec<FlatNode> {
    let mut out = Vec::new();
    for child in &page.root.children {
        flatten(child, 0.0, 0.0, 0, include_hidden, &mut out);
    }
    out
}

fn is_rtl_text(s: &str) -> bool {
    s.chars().any(|c| ('\u{0590}'..='\u{08FF}').contains(&c))
}

fn event_fa(kind: &EventKind) -> &'static str {
    match kind {
        EventKind::Click => "کلیک",
        EventKind::PageLoad => "بارگذاری صفحه",
        EventKind::Submit => "ثبت فرم",
        EventKind::Change => "تغییر مقدار",
        EventKind::Timer => "زمانبندی",
        EventKind::Webhook => "وب هوک",
    }
}

fn action_fa(action: &Action) -> String {
    match action {
        Action::Navigate { route } => format!("برو به صفحه {route}"),
        Action::ShowMessage { text } => format!("پیام نشان بده: {text}"),
        Action::SetValue { key, .. } => format!("مقدار {key} را تنظیم کن"),
        Action::Increment { key, by } => format!("{key} را {by} واحد زیاد کن"),
        Action::SaveRecord { table, .. } => format!("یک رکورد در جدول {table} ذخیره کن"),
        Action::DeleteRecord { table, .. } => format!("رکوردی از جدول {table} حذف کن"),
        Action::CallApi { url, method } => format!("صدا زدن API با {method} به {url}"),
        Action::Log { text } => format!("لاگ بنویس: {text}"),
    }
}

fn step_lines(step: &Step, indent: usize, out: &mut Vec<String>) {
    let pad = "  ".repeat(indent);
    match step {
        Step::Action { action } => out.push(format!("{pad}{}", action_fa(action))),
        Step::Condition { then_steps, else_steps, .. } => {
            out.push(format!("{pad}اگر شرط برقرار بود:"));
            for s in then_steps {
                step_lines(s, indent + 1, out);
            }
            if !else_steps.is_empty() {
                out.push(format!("{pad}در غیر این صورت:"));
                for s in else_steps {
                    step_lines(s, indent + 1, out);
                }
            }
        }
    }
}

/// Remove nodes by id from anywhere in the tree.
fn remove_from_tree(node: &mut Node, ids: &BTreeSet<String>) {
    node.children.retain(|c| !ids.contains(&c.id));
    for child in node.children.iter_mut() {
        remove_from_tree(child, ids);
    }
}

/// Clone a subtree with fresh ids.
fn clone_with_new_ids(node: &Node, counter: &mut u32) -> Node {
    *counter += 1;
    let mut copy = node.clone();
    copy.id = format!("node-{}", counter);
    copy.children = node
        .children
        .iter()
        .map(|c| clone_with_new_ids(c, counter))
        .collect();
    copy
}

/// Move a node one step forward/backward among its siblings, which is
/// what stacking order means in this tree. Returns true when moved.
fn move_in_parent(node: &mut Node, id: &str, forward: bool) -> bool {
    if let Some(pos) = node.children.iter().position(|c| c.id == id) {
        let len = node.children.len();
        if forward && pos + 1 < len {
            node.children.swap(pos, pos + 1);
            return true;
        }
        if !forward && pos > 0 {
            node.children.swap(pos, pos - 1);
            return true;
        }
        return false;
    }
    for child in node.children.iter_mut() {
        if move_in_parent(child, id, forward) {
            return true;
        }
    }
    false
}

/// When a node belongs to an inserted component group (a direct child
/// of the page root marked with comp_group), selecting any part of it
/// selects the whole group: return the group's id.
fn group_root_of(root: &Node, id: &str) -> String {
    for child in &root.children {
        if child.id == id {
            return id.to_string();
        }
        if child.props.contains_key("comp_group") && child.find(id).is_some() {
            return child.id.clone();
        }
    }
    id.to_string()
}

/// The direct parent of a node, when it is not the page root itself.
fn parent_of<'a>(node: &'a Node, id: &str) -> Option<&'a Node> {
    for child in &node.children {
        if child.id == id {
            return Some(node);
        }
        if let Some(p) = parent_of(child, id) {
            return Some(p);
        }
    }
    None
}

/// The route a flow navigates to, searching conditions recursively.
fn find_navigate(steps: &[Step]) -> Option<String> {
    for s in steps {
        match s {
            Step::Action { action: Action::Navigate { route } } => return Some(route.clone()),
            Step::Action { .. } => {}
            Step::Condition { then_steps, else_steps, .. } => {
                if let Some(r) = find_navigate(then_steps) {
                    return Some(r);
                }
                if let Some(r) = find_navigate(else_steps) {
                    return Some(r);
                }
            }
        }
    }
    None
}

/// Preview click target for a node: an explicit `link_page` prop
/// wins; otherwise a click flow bound to this node that navigates.
fn preview_target(b: &Bundle, flat: &FlatNode) -> Option<String> {
    if let Some(link) = &flat.link {
        if !link.is_empty() {
            return Some(link.clone());
        }
    }
    b.flows
        .iter()
        .filter(|f| {
            matches!(f.trigger.kind, EventKind::Click)
                && f.trigger.source_node.as_deref() == Some(flat.id.as_str())
        })
        .find_map(|f| find_navigate(&f.steps))
}

/// The inline CSS for one flattened node on the canvas. Shared by
/// the editing canvas and the clean preview so both paint the same.
fn node_css(n: &FlatNode) -> String {
    let align = n.align.clone().unwrap_or_else(|| {
        if n.text.as_deref().map(is_rtl_text).unwrap_or(false) {
            "right".to_string()
        } else {
            "left".to_string()
        }
    });
    let lh = if n.line_height > 0.0 { n.line_height } else { 1.45 };
    let mut style = format!(
        "left:{}px;top:{}px;width:{}px;height:{}px;color:{};font-size:{}px;border-radius:{}px;opacity:{};text-align:{};font-weight:{};line-height:{};",
        n.ax,
        n.ay,
        n.width,
        n.height,
        n.text_color.clone().unwrap_or_else(|| "#141818".to_string()),
        n.font_size,
        n.radius,
        n.opacity,
        align,
        n.weight,
        lh,
    );
    if n.kind == NodeKind::Ellipse {
        style.push_str("border-radius:50%;");
    }
    if let Some((from, to, angle)) = &n.gradient {
        style.push_str(&format!(
            "background:linear-gradient({}deg,{},{});",
            angle, from, to
        ));
    } else if let Some(f) = &n.fill {
        style.push_str(&format!("background:{f};"));
    }
    if let Some(src) = &n.src {
        style.push_str(&format!(
            "background-image:url({});background-size:cover;background-position:center;",
            src
        ));
    }
    if let Some(st) = &n.stroke {
        style.push_str(&format!("border:{}px solid {};", n.stroke_width, st));
    }
    if let Some((sx, sy, blur, col, op)) = &n.shadow {
        style.push_str(&format!(
            "box-shadow:{}px {}px {}px {};",
            sx,
            sy,
            blur,
            hex_rgba(col, *op)
        ));
    }
    if n.letter_spacing != 0.0 {
        style.push_str(&format!("letter-spacing:{}px;", n.letter_spacing));
    }
    style
}

fn shade(base: Color, factor: f32) -> Color {
    let f = |v: u8| (v as f32 * factor).round().clamp(0.0, 255.0) as u8;
    Color::rgb(f(base.r), f(base.g), f(base.b))
}

fn comp_text(
    id_gen: &mut u32,
    name: &str,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    text: &str,
    size: f32,
    color: Color,
    weight: u16,
    align: &str,
) -> Node {
    *id_gen += 1;
    let mut n = Node::text_node(format!("cmp-{id_gen}"), name, x, y, w, h, text, size, color);
    n.font_weight = weight;
    n.bold = weight >= 600;
    n.props.insert("align".into(), align.into());
    n
}

fn comp_box(
    id_gen: &mut u32,
    kind: NodeKind,
    name: &str,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    fill: Color,
) -> Node {
    *id_gen += 1;
    let mut n = Node::new(format!("cmp-{id_gen}"), name, kind, x, y, w, h);
    n.fill = Some(fill);
    n
}

/// Build one ready component from the library: a real node group with
/// fills, radius, shadows and text, matching the app theme. Photo
/// paths point at the bundled template photos.
pub fn build_component(key: &str, id_gen: &mut u32, primary: Color) -> Node {
    let ink = Color::from_hex("#1a1a1a").unwrap();
    let muted = Color::from_hex("#81858b").unwrap();
    let surface = Color::WHITE;
    let soft = Color::from_hex("#f0f0f1").unwrap();
    let green = Color::from_hex("#00857d").unwrap();
    let mut group = |name: &str, w: f32, h: f32| {
        *id_gen += 1;
        let mut g = Node::new(format!("cmp-{id_gen}"), name, NodeKind::Frame, 0.0, 0.0, w, h);
        g.props.insert("comp_group".into(), "1".into());
        g
    };
    match key {
        "navbar" => {
            let mut g = group("نوبار", 1200.0, 72.0);
            let mut bar = comp_box(id_gen, NodeKind::Frame, "نوبار", 0.0, 0.0, 1200.0, 72.0, surface);
            bar.shadow = Some(Shadow { x: 0.0, y: 2.0, blur: 12.0, color: Color::BLACK, opacity: 0.10 });
            bar.children.push(comp_text(id_gen, "لوگو", 1120.0, 20.0, 64.0, 32.0, "بازارینو", 20.0, primary, 700, "right"));
            for (i, label) in ["خانه", "فروشگاه", "پیشنهادها", "درباره ما"].iter().enumerate() {
                let x = 1000.0 - i as f32 * 96.0;
                bar.children.push(comp_text(id_gen, "پیوند", x, 26.0, 88.0, 20.0, label, 14.0, ink, 500, "center"));
            }
            let mut cta = comp_box(id_gen, NodeKind::Button, "دکمه ورود", 16.0, 16.0, 120.0, 40.0, primary);
            cta.corner_radius = 8.0;
            cta.children.push(comp_text(id_gen, "متن دکمه", 0.0, 9.0, 120.0, 22.0, "ورود | ثبت نام", 13.0, Color::WHITE, 700, "center"));
            bar.children.push(cta);
            g.children.push(bar);
            g
        }
        "hero" => {
            let mut g = group("هیرو با عکس", 1200.0, 420.0);
            let mut bg = comp_box(id_gen, NodeKind::Frame, "پس زمینه هیرو", 0.0, 0.0, 1200.0, 420.0, soft);
            bg.corner_radius = 16.0;
            let mut img = comp_box(id_gen, NodeKind::Image, "عکس هیرو", 620.0, 0.0, 580.0, 420.0, Color::TRANSPARENT);
            img.corner_radius = 16.0;
            img.props.insert("src".into(), "assets/digikala/hero.jpg".into());
            bg.children.push(img);
            let mut overlay = comp_box(id_gen, NodeKind::Rectangle, "پوشش تیره", 620.0, 290.0, 580.0, 130.0, Color::TRANSPARENT);
            overlay.fill_gradient = Some(Gradient::new(Color::rgba(0, 0, 0, 0), Color::rgba(0, 0, 0, 170), 180.0));
            bg.children.push(overlay);
            bg.children.push(comp_text(id_gen, "تیتر روی عکس", 648.0, 318.0, 524.0, 40.0, "جشنواره فروش تابستانه", 27.0, Color::WHITE, 700, "right"));
            bg.children.push(comp_text(id_gen, "زیرتیتر", 648.0, 366.0, 524.0, 24.0, "تا ۴۰٪ تخفیف روی کالاهای منتخب", 15.0, Color::WHITE, 400, "right"));
            let mut card = comp_box(id_gen, NodeKind::Card, "کارت متن", 40.0, 90.0, 480.0, 240.0, surface);
            card.corner_radius = 16.0;
            card.shadow = Some(Shadow::soft());
            card.children.push(comp_text(id_gen, "تیتر کارت", 24.0, 28.0, 432.0, 40.0, "هر آنچه می خواهید، اینجاست", 24.0, ink, 700, "right"));
            card.children.push(comp_text(id_gen, "متن کارت", 24.0, 80.0, 432.0, 56.0, "بیش از ده هزار کالای اصل با ضمانت بازگشت و ارسال سریع به سراسر کشور.", 14.5, muted, 400, "right"));
            let mut cta = comp_box(id_gen, NodeKind::Button, "دکمه خرید", 248.0, 164.0, 208.0, 48.0, primary);
            cta.corner_radius = 10.0;
            cta.fill_gradient = Some(Gradient::new(primary, shade(primary, 1.35), 90.0));
            cta.children.push(comp_text(id_gen, "متن دکمه", 0.0, 12.0, 208.0, 24.0, "شروع خرید", 15.0, Color::WHITE, 700, "center"));
            card.children.push(cta);
            bg.children.push(card);
            g.children.push(bg);
            g
        }
        "btn-primary" => {
            let mut b = comp_box(id_gen, NodeKind::Button, "دکمه اصلی", 0.0, 0.0, 180.0, 48.0, primary);
            b.corner_radius = 10.0;
            b.fill_gradient = Some(Gradient::new(primary, shade(primary, 1.3), 90.0));
            b.shadow = Some(Shadow { x: 0.0, y: 6.0, blur: 16.0, color: primary, opacity: 0.35 });
            b.props.insert("comp_group".into(), "1".into());
            b.children.push(comp_text(id_gen, "متن دکمه", 0.0, 12.0, 180.0, 24.0, "دکمه اصلی", 15.0, Color::WHITE, 700, "center"));
            b
        }
        "btn-secondary" => {
            let mut b = comp_box(id_gen, NodeKind::Button, "دکمه دوم", 0.0, 0.0, 180.0, 48.0, surface);
            b.corner_radius = 10.0;
            b.stroke = Some(Color::from_hex("#d5d8db").unwrap());
            b.stroke_width = 1.5;
            b.props.insert("comp_group".into(), "1".into());
            b.children.push(comp_text(id_gen, "متن دکمه", 0.0, 12.0, 180.0, 24.0, "دکمه دوم", 15.0, ink, 500, "center"));
            b
        }
        "input" => {
            let mut g = group("ورودی فرم", 320.0, 84.0);
            g.children.push(comp_text(id_gen, "برچسب", 0.0, 0.0, 320.0, 20.0, "شماره موبایل", 13.0, ink, 500, "right"));
            let mut field = comp_box(id_gen, NodeKind::Input, "ورودی", 0.0, 28.0, 320.0, 48.0, surface);
            field.corner_radius = 10.0;
            field.stroke = Some(Color::from_hex("#d5d8db").unwrap());
            field.stroke_width = 1.5;
            field.children.push(comp_text(id_gen, "راهنما", 12.0, 13.0, 296.0, 22.0, "۰۹۱۲ ـــ ــــ ـــ", 14.0, muted, 400, "right"));
            g.children.push(field);
            g
        }
        "product-card" => {
            let mut g = group("کارت محصول", 240.0, 344.0);
            let mut card = comp_box(id_gen, NodeKind::Card, "کارت محصول", 0.0, 0.0, 240.0, 344.0, surface);
            card.corner_radius = 12.0;
            card.shadow = Some(Shadow { x: 0.0, y: 4.0, blur: 18.0, color: Color::BLACK, opacity: 0.12 });
            let mut img = comp_box(id_gen, NodeKind::Image, "عکس محصول", 20.0, 16.0, 200.0, 170.0, soft);
            img.props.insert("src".into(), "assets/digikala/phone.jpg".into());
            card.children.push(img);
            let mut badge = comp_box(id_gen, NodeKind::Rectangle, "نشان تخفیف", 172.0, 196.0, 48.0, 24.0, primary);
            badge.corner_radius = 12.0;
            badge.children.push(comp_text(id_gen, "درصد", 0.0, 3.0, 48.0, 18.0, "۱۸٪", 12.5, Color::WHITE, 700, "center"));
            card.children.push(badge);
            card.children.push(comp_text(id_gen, "نام محصول", 16.0, 198.0, 140.0, 44.0, "گوشی موبایل سامسونگ مدل Galaxy A55", 13.5, ink, 500, "right"));
            card.children.push(comp_text(id_gen, "قیمت قبل", 16.0, 262.0, 208.0, 18.0, "۲۴٬۵۰۰٬۰۰۰", 12.0, muted, 400, "left"));
            card.children.push(comp_text(id_gen, "قیمت", 16.0, 284.0, 208.0, 28.0, "۲۰٬۰۹۰٬۰۰۰ تومان", 16.0, ink, 700, "left"));
            g.children.push(card);
            g
        }
        "price-card" => {
            let mut g = group("کارت قیمت", 300.0, 384.0);
            let mut card = comp_box(id_gen, NodeKind::Card, "کارت قیمت", 0.0, 0.0, 300.0, 384.0, surface);
            card.corner_radius = 16.0;
            card.stroke = Some(primary);
            card.stroke_width = 2.0;
            card.shadow = Some(Shadow::soft());
            card.children.push(comp_text(id_gen, "نام طرح", 24.0, 24.0, 252.0, 28.0, "طرح حرفه ای", 18.0, ink, 700, "center"));
            card.children.push(comp_text(id_gen, "قیمت", 24.0, 62.0, 252.0, 44.0, "۴۹۰ هزار تومان", 26.0, primary, 700, "center"));
            card.children.push(comp_text(id_gen, "دوره", 24.0, 110.0, 252.0, 20.0, "ماهیانه", 12.5, muted, 400, "center"));
            for (i, feat) in ["پشتیبانی همیشگی", "دامنه اختصاصی", "گزارش گیری پیشرفته"].iter().enumerate() {
                let y = 152.0 + i as f32 * 34.0;
                card.children.push(comp_text(id_gen, "ویژگی", 24.0, y, 252.0, 22.0, feat, 13.5, ink, 400, "right"));
            }
            let mut cta = comp_box(id_gen, NodeKind::Button, "دکمه انتخاب", 24.0, 296.0, 252.0, 48.0, primary);
            cta.corner_radius = 10.0;
            cta.children.push(comp_text(id_gen, "متن دکمه", 0.0, 12.0, 252.0, 24.0, "انتخاب طرح", 14.5, Color::WHITE, 700, "center"));
            card.children.push(cta);
            g.children.push(card);
            g
        }
        "badge" => {
            let mut b = comp_box(id_gen, NodeKind::Rectangle, "نشان", 0.0, 0.0, 110.0, 30.0, green);
            b.corner_radius = 15.0;
            b.props.insert("comp_group".into(), "1".into());
            b.children.push(comp_text(id_gen, "متن نشان", 0.0, 5.0, 110.0, 20.0, "موجود در انبار", 12.0, Color::WHITE, 700, "center"));
            b
        }
        "avatar" => {
            let mut g = group("آواتار با نام", 220.0, 56.0);
            let mut img = comp_box(id_gen, NodeKind::Ellipse, "عکس پروفایل", 164.0, 0.0, 56.0, 56.0, soft);
            img.props.insert("src".into(), "assets/digikala/fashion.jpg".into());
            g.children.push(img);
            g.children.push(comp_text(id_gen, "نام", 0.0, 6.0, 152.0, 22.0, "سارا محمدی", 14.5, ink, 700, "right"));
            g.children.push(comp_text(id_gen, "نقش", 0.0, 30.0, 152.0, 18.0, "مدیر فروش", 12.0, muted, 400, "right"));
            g
        }
        "table" => {
            let mut g = group("جدول ساده", 560.0, 180.0);
            let mut t = comp_box(id_gen, NodeKind::Table, "جدول", 0.0, 0.0, 560.0, 180.0, surface);
            t.corner_radius = 12.0;
            t.shadow = Some(Shadow { x: 0.0, y: 4.0, blur: 18.0, color: Color::BLACK, opacity: 0.10 });
            let head = comp_box(id_gen, NodeKind::Rectangle, "سربرگ", 0.0, 0.0, 560.0, 44.0, soft);
            t.children.push(head);
            for (x, label) in [(440.0, "نام کالا"), (280.0, "تعداد"), (120.0, "قیمت"), (8.0, "وضعیت")] {
                t.children.push(comp_text(id_gen, "ستون", x, 12.0, 112.0, 20.0, label, 12.5, muted, 700, "right"));
            }
            let rows = [
                ("گوشی موبایل A55", "۲", "۲۰٬۰۹۰٬۰۰۰", "پرداخت شد"),
                ("هدفون بی سیم", "۱", "۳٬۴۰۰٬۰۰۰", "در انتظار"),
                ("ساعت هوشمند", "۳", "۸٬۹۰۰٬۰۰۰", "ارسال شد"),
            ];
            for (i, row) in rows.iter().enumerate() {
                let y = 58.0 + i as f32 * 40.0;
                t.children.push(comp_text(id_gen, "ردیف", 440.0, y, 112.0, 20.0, row.0, 12.5, ink, 400, "right"));
                t.children.push(comp_text(id_gen, "ردیف", 280.0, y, 112.0, 20.0, row.1, 12.5, ink, 400, "right"));
                t.children.push(comp_text(id_gen, "ردیف", 120.0, y, 112.0, 20.0, row.2, 12.5, ink, 400, "right"));
                t.children.push(comp_text(id_gen, "ردیف", 8.0, y, 112.0, 20.0, row.3, 12.5, green, 500, "right"));
            }
            g.children.push(t);
            g
        }
        _ => {
            let mut g = group("بخش ویژگی ها", 1200.0, 260.0);
            g.children.push(comp_text(id_gen, "تیتر بخش", 0.0, 0.0, 1200.0, 36.0, "چرا بازارینو؟", 22.0, ink, 700, "center"));
            let items = [
                ("ضمانت اصالت کالا", "همه کالاها با گارانتی معتبر شرکتی عرضه می شوند."),
                ("ارسال سریع", "سفارش های امروز، فردا به دست شما می رسد."),
                ("۷ روز مهلت بازگشت", "بدون قید و شرط می توانید کالا را برگردانید."),
            ];
            for (i, (title, body)) in items.iter().enumerate() {
                let x = 800.0 - i as f32 * 400.0;
                let mut card = comp_box(id_gen, NodeKind::Card, "کارت ویژگی", x, 64.0, 384.0, 180.0, surface);
                card.corner_radius = 14.0;
                card.shadow = Some(Shadow { x: 0.0, y: 4.0, blur: 18.0, color: Color::BLACK, opacity: 0.10 });
                card.children.push(comp_text(id_gen, "تیتر ویژگی", 20.0, 24.0, 344.0, 26.0, title, 16.0, ink, 700, "right"));
                card.children.push(comp_text(id_gen, "متن ویژگی", 20.0, 60.0, 344.0, 66.0, body, 13.5, muted, 400, "right"));
                g.children.push(card);
            }
            g
        }
    }
}

fn download_text(filename: &str, content: &str, mime: &str) {
    let Some(window) = web_sys::window() else { return };
    let Some(document) = window.document() else { return };
    let parts = js_sys::Array::new();
    parts.push(&wasm_bindgen::JsValue::from_str(content));
    let _ = mime;
    let Ok(blob) = web_sys::Blob::new_with_str_sequence(&parts) else {
        return;
    };
    let Ok(url) = web_sys::Url::create_object_url_with_blob(&blob) else {
        return;
    };
    if let Ok(el) = document.create_element("a") {
        if let Some(anchor) = el.dyn_ref::<web_sys::HtmlAnchorElement>() {
            anchor.set_href(&url);
            anchor.set_download(filename);
            anchor.click();
        }
    }
    let _ = web_sys::Url::revoke_object_url(&url);
}

/// A basic but real SVG render of one page, built from node data in
/// the browser. The CLI renderer (`chida render`) is the full
/// fidelity one; this is the editor's quick export.
fn page_svg(page: &Page) -> String {
    let mut s = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\">",
        page.width,
        page.height,
        page.width,
        page.height
    );
    let bg = page
        .background
        .map(|c| c.to_hex())
        .unwrap_or_else(|| "#ffffff".to_string());
    s.push_str(&format!(
        "<rect width=\"{}\" height=\"{}\" fill=\"{}\"/>",
        page.width, page.height, bg
    ));
    for n in page_flat(page, false) {
        let shadow_style = n
            .shadow
            .map(|(sx, sy, blur, col, op)| {
                format!(
                    " style=\"filter:drop-shadow({}px {}px {}px {})\"",
                    sx,
                    sy,
                    (blur / 2.0).max(0.0),
                    hex_rgba(&col, op)
                )
            })
            .unwrap_or_default();
        let fill = if let Some((from, to, angle)) = &n.gradient {
            let gid: String = n
                .id
                .chars()
                .map(|c| if c.is_alphanumeric() { c } else { '-' })
                .collect();
            let rad = angle.to_radians();
            let (dx, dy) = (rad.sin(), -rad.cos());
            s.push_str(&format!(
                "<defs><linearGradient id=\"g{gid}\" x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\"><stop offset=\"0\" stop-color=\"{}\"/><stop offset=\"1\" stop-color=\"{}\"/></linearGradient></defs>",
                0.5 - dx / 2.0,
                0.5 - dy / 2.0,
                0.5 + dx / 2.0,
                0.5 + dy / 2.0,
                from,
                to
            ));
            format!("url(#g{gid})")
        } else {
            n.fill.clone().unwrap_or_else(|| "none".to_string())
        };
        if n.kind == NodeKind::Ellipse {
            s.push_str(&format!(
                "<ellipse cx=\"{}\" cy=\"{}\" rx=\"{}\" ry=\"{}\" fill=\"{}\"{shadow_style}/>",
                n.ax + n.width / 2.0,
                n.ay + n.height / 2.0,
                n.width / 2.0,
                n.height / 2.0,
                fill
            ));
        } else {
            s.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"{}\" fill=\"{}\"{shadow_style}/>",
                n.ax, n.ay, n.width, n.height, n.radius, fill
            ));
        }
        if let Some(text) = &n.text {
            if !text.is_empty() {
                let color = n.text_color.clone().unwrap_or_else(|| "#141818".to_string());
                let esc = text.replace('&', "&amp;").replace('<', "&lt;");
                s.push_str(&format!(
                    "<text x=\"{}\" y=\"{}\" font-size=\"{}\" fill=\"{}\" font-family=\"sans-serif\">{}</text>",
                    n.ax + 6.0,
                    n.ay + n.font_size + 6.0,
                    n.font_size,
                    color,
                    esc
                ));
            }
        }
    }
    s.push_str("</svg>");
    s
}

fn now_time() -> String {
    let d = js_sys::Date::new_0();
    format!(
        "{:02}:{:02}:{:02}",
        d.get_hours(),
        d.get_minutes(),
        d.get_seconds()
    )
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Tool {
    Move,
    Hand,
    Frame,
    Rect,
    Ellipse,
    Text,
    Image,
    Button,
    Card,
    Input,
}

impl Tool {
    fn shape_kind(self) -> Option<NodeKind> {
        match self {
            Tool::Frame => Some(NodeKind::Frame),
            Tool::Rect => Some(NodeKind::Rectangle),
            Tool::Ellipse => Some(NodeKind::Ellipse),
            Tool::Text => Some(NodeKind::Text),
            Tool::Image => Some(NodeKind::Image),
            Tool::Button => Some(NodeKind::Button),
            Tool::Card => Some(NodeKind::Card),
            Tool::Input => Some(NodeKind::Input),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
enum DragMode {
    None,
    MoveNode {
        start_x: i32,
        start_y: i32,
        /// (id, original relative x, original relative y) for every
        /// node in the selection, so multi moves keep their offsets.
        items: Vec<(String, f32, f32)>,
        /// Primary node absolute rect at drag start, for snapping.
        primary_ax: f32,
        primary_ay: f32,
        primary_w: f32,
        primary_h: f32,
        /// Snap targets: (ax, ay, w, h) of every node not selected.
        targets: Vec<(f32, f32, f32, f32)>,
    },
    Resize { corner: u8, start_x: i32, start_y: i32, orig_x: f32, orig_y: f32, orig_w: f32, orig_h: f32 },
    Pan { start_x: i32, start_y: i32, orig_x: f32, orig_y: f32 },
    /// Marquee (rubber band) selection, in page coordinates.
    Marquee { sx: f32, sy: f32, cx: f32, cy: f32 },
}

#[component]
fn App() -> impl IntoView {
    let bundle = RwSignal::new(load_bundle());
    // Auto-layout containers always show their computed positions,
    // including in a project saved before a layout was enabled.
    bundle.update(|b| {
        for page in &mut b.project.pages {
            page.root.apply_layout();
        }
    });
    let page_index = RwSignal::new(0usize);
    let selected_id = RwSignal::new(String::new());
    let level = RwSignal::new(2u8);
    let tab = RwSignal::new(0u8); // 0 design, 1 logic, 2 data, 3 code
    let tool = RwSignal::new(Tool::Move);
    let counter = RwSignal::new(100u32);
    let locked = RwSignal::new(BTreeSet::<String>::new());
    /// Full multi-selection; `selected_id` is the primary member.
    let sel_set = RwSignal::new(BTreeSet::<String>::new());
    /// Left panel tab: 0 layers, 1 components.
    let left_tab = RwSignal::new(0u8);
    /// Snap guides visible while dragging: (vertical?, position).
    let guides = RwSignal::new(Vec::<(bool, f32)>::new());
    /// Right click context menu: (screen x, screen y, node id).
    let ctx_menu = RwSignal::new(None::<(i32, i32, String)>);
    /// Clean full screen preview: on/off plus the page it shows.
    let preview = RwSignal::new(false);
    let preview_page = RwSignal::new(0usize);
    /// Name typed for a style about to be saved.
    let style_name = RwSignal::new(String::new());
    /// Node whose image an in-flight upload replaces; None means the
    /// upload becomes a new image node.
    let upload_target = RwSignal::new(None::<String>);
    let file_ref = NodeRef::<leptos::html::Input>::new();
    let drag = RwSignal::new(DragMode::None);
    let zoom = RwSignal::new(None::<f32>);
    let pan = RwSignal::new((0.0f32, 0.0f32));
    let past = RwSignal::new(Vec::<Project>::new());
    let future = RwSignal::new(Vec::<Project>::new());
    let saved_stamp = RwSignal::new(String::from("ذخیره خودکار روی این مرورگر فعال است"));
    let last_edit = RwSignal::new((String::new(), 0.0f64));
    let render_note = RwSignal::new(String::new());

    let artboard_ref = NodeRef::<leptos::html::Div>::new();

    // Fit scale from the window width: chrome is rail 48 + panel 240 +
    // right panel 300, plus workspace padding.
    let win_width = RwSignal::new(1440.0f32);
    let win_height = RwSignal::new(900.0f32);
    let measure = move || {
        if let Some(w) = web_sys::window() {
            if let Ok(v) = w.inner_width() {
                if let Some(px) = v.as_f64() {
                    win_width.set(px as f32);
                }
            }
            if let Ok(v) = w.inner_height() {
                if let Some(px) = v.as_f64() {
                    win_height.set(px as f32);
                }
            }
        }
    };
    measure();
    let _resize_handle = window_event_listener(leptos::ev::resize, move |_| measure());

    let page_size = move || {
        bundle.with(|b| {
            b.project
                .pages
                .get(page_index.get())
                .map(|pg| (pg.width, pg.height))
                .unwrap_or((800.0, 600.0))
        })
    };
    let fit_scale = move || {
        let (w, _) = page_size();
        let avail = (win_width.get() - 700.0).max(240.0);
        (avail / w).clamp(0.1, 1.2)
    };
    let scale = move || zoom.get().unwrap_or_else(fit_scale);

    let persist = move || {
        bundle.with_untracked(|b| {
            if let Ok(raw) = serde_json::to_string(b) {
                if let Some(w) = web_sys::window() {
                    if let Ok(Some(storage)) = w.local_storage() {
                        let _ = storage.set_item(STORAGE_KEY, &raw);
                    }
                }
            }
        });
        saved_stamp.set(format!("ذخیره شد، ساعت {}", now_time()));
    };

    // Record the current project in history (grouped for rapid
    // successive edits of the same field, so undo is not per keystroke).
    let record = move |key: &str| {
        let now = js_sys::Date::now();
        let (last_key, last_time) = last_edit.get_untracked();
        if last_key == key && now - last_time < 1500.0 {
            last_edit.set((key.to_string(), now));
            return;
        }
        last_edit.set((key.to_string(), now));
        bundle.with_untracked(|b| {
            past.update(|p| {
                p.push(b.project.clone());
                if p.len() > 60 {
                    p.remove(0);
                }
            });
        });
        future.update(|f| f.clear());
    };

    let apply = move |f: &dyn Fn(&mut Project)| {
        bundle.update(|b| {
            f(&mut b.project);
            // Any edit can change a layout: child added, removed,
            // resized or a layout prop itself. Recompute the current
            // page so canvas, storage and exports agree.
            if let Some(page) = b.project.pages.get_mut(page_index.get_untracked()) {
                page.root.apply_layout();
            }
        });
        persist();
    };

    let undo = move |_| {
        let prev = past.with_untracked(|p| p.last().cloned());
        if let Some(prev) = prev {
            past.update(|p| {
                p.pop();
            });
            bundle.update(|b| {
                future.update(|f| f.push(b.project.clone()));
                b.project = prev;
            });
            persist();
        }
    };
    let redo = move |_| {
        let next = future.with_untracked(|f| f.last().cloned());
        if let Some(next) = next {
            future.update(|f| {
                f.pop();
            });
            bundle.update(|b| {
                past.update(|p| p.push(b.project.clone()));
                b.project = next;
            });
            persist();
        }
    };

    let reset_all = move |_| {
        past.update(|p| p.clear());
        future.update(|f| f.clear());
        locked.update(|l| l.clear());
        selected_id.set(String::new());
        page_index.set(0);
        zoom.set(None);
        pan.set((0.0, 0.0));
        if let Some(w) = web_sys::window() {
            if let Ok(Some(storage)) = w.local_storage() {
                let _ = storage.remove_item(STORAGE_KEY);
            }
        }
        bundle.set(fixture_bundle());
        saved_stamp.set("به نسخه نمونه برگشت و ذخیره پاک شد".to_string());
    };

    let flat_canvas = move || {
        bundle.with(|b| {
            b.project
                .pages
                .get(page_index.get())
                .map(|pg| page_flat(pg, false))
                .unwrap_or_default()
        })
    };
    let flat_all = move || {
        bundle.with(|b| {
            b.project
                .pages
                .get(page_index.get())
                .map(|pg| page_flat(pg, true))
                .unwrap_or_default()
        })
    };
    let selected_node = move || {
        let id = selected_id.get();
        bundle.with(|b| {
            b.project
                .pages
                .get(page_index.get())
                .and_then(|pg| pg.root.find(&id).cloned())
        })
    };
    let page_bg = move || {
        bundle.with(|b| {
            b.project
                .pages
                .get(page_index.get())
                .and_then(|pg| pg.background)
                .unwrap_or(b.project.theme.background)
                .to_hex()
        })
    };
    let page_meta = move || {
        bundle.with(|b| {
            b.project
                .pages
                .get(page_index.get())
                .map(|pg| (pg.name.clone(), pg.route.clone()))
                .unwrap_or_default()
        })
    };

    // ---- node creation -------------------------------------------------
    let make_node = move |kind: NodeKind, px: f32, py: f32| -> Node {
        counter.update(|c| *c += 1);
        let n = counter.get_untracked();
        let id = format!("node-{}-{n}", kind_name(kind));
        let primary = bundle.with_untracked(|b| b.project.theme.primary);
        let ink = bundle.with_untracked(|b| b.project.theme.text);
        let mut node = match kind {
            NodeKind::Frame => {
                let mut f = Node::new(id, "فریم تازه", kind, px - 210.0, py - 150.0, 420.0, 300.0);
                f.fill = Some(Color::WHITE);
                f.stroke = Some(Color::rgb(200, 204, 208));
                f.stroke_width = 1.0;
                f
            }
            NodeKind::Rectangle => {
                let mut r = Node::rect(id, "مستطیل تازه", px - 80.0, py - 50.0, 160.0, 100.0, primary);
                r.corner_radius = 8.0;
                r
            }
            NodeKind::Ellipse => Node::new(id, "دایره تازه", kind, px - 60.0, py - 60.0, 120.0, 120.0),
            NodeKind::Text => Node::text_node(
                id,
                "متن تازه",
                px - 100.0,
                py - 15.0,
                220.0,
                34.0,
                "متن تازه",
                18.0,
                ink,
            ),
            NodeKind::Image => {
                let mut im = Node::new(id, "تصویر تازه", kind, px - 110.0, py - 70.0, 220.0, 140.0);
                im.fill = Some(Color::rgb(214, 226, 237));
                im.corner_radius = 10.0;
                im
            }
            NodeKind::Button => {
                let mut b = Node::new(id, "دکمه تازه", kind, px - 75.0, py - 22.0, 150.0, 44.0);
                b.fill = Some(primary);
                b.corner_radius = 10.0;
                b.text = Some("دکمه".into());
                b.font_size = 15.0;
                b.text_color = Some(Color::WHITE);
                b.bold = true;
                b
            }
            NodeKind::Card => {
                let mut c = Node::new(id, "کارت تازه", kind, px - 120.0, py - 80.0, 240.0, 160.0);
                c.fill = Some(Color::WHITE);
                c.corner_radius = 14.0;
                c.stroke = Some(Color::rgb(210, 214, 218));
                c.stroke_width = 1.0;
                c.text = Some("کارت تازه".into());
                c.font_size = 15.0;
                c.text_color = Some(ink);
                c
            }
            _ => {
                let mut inp = Node::new(id, "ورودی تازه", NodeKind::Input, px - 120.0, py - 22.0, 240.0, 44.0);
                inp.fill = Some(Color::WHITE);
                inp.corner_radius = 8.0;
                inp.stroke = Some(Color::rgb(190, 196, 200));
                inp.stroke_width = 1.0;
                inp.text = Some("اینجا بنویسید".into());
                inp.font_size = 14.0;
                inp.text_color = Some(ink);
                inp
            }
        };
        if node.kind == NodeKind::Ellipse {
            node.fill = Some(primary);
        }
        if node.x < 0.0 {
            node.x = 0.0;
        }
        if node.y < 0.0 {
            node.y = 0.0;
        }
        node
    };

    // ---- mouse interaction ----------------------------------------------
    let artboard_point = move |ev: &leptos::ev::MouseEvent| -> Option<(f32, f32)> {
        let el = artboard_ref.get()?;
        let rect = el.get_bounding_client_rect();
        let s = scale();
        Some((
            (ev.client_x() as f32 - rect.left() as f32) / s,
            (ev.client_y() as f32 - rect.top() as f32) / s,
        ))
    };

    let on_artboard_mousedown = move |ev: leptos::ev::MouseEvent| {
        ev.stop_propagation();
        match tool.get() {
            Tool::Hand => {
                drag.set(DragMode::Pan {
                    start_x: ev.client_x(),
                    start_y: ev.client_y(),
                    orig_x: pan.get_untracked().0,
                    orig_y: pan.get_untracked().1,
                });
            }
            Tool::Move => {
                // Dragging on empty artboard draws a marquee; a plain
                // click (no real drag) still clears the selection,
                // which is decided on mouse up by the drag size.
                ctx_menu.set(None);
                if let Some((px, py)) = artboard_point(&ev) {
                    drag.set(DragMode::Marquee { sx: px, sy: py, cx: px, cy: py });
                }
            }
            t => {
                if let Some(kind) = t.shape_kind() {
                    if let Some((px, py)) = artboard_point(&ev) {
                        record("add");
                        let node = make_node(kind, px, py);
                        let new_id = node.id.clone();
                        apply(&|p: &mut Project| {
                            if let Some(page) = p.pages.get_mut(page_index.get_untracked()) {
                                page.root.children.push(node.clone());
                            }
                        });
                        selected_id.set(new_id.clone());
                        let mut one = BTreeSet::new();
                        one.insert(new_id);
                        sel_set.set(one);
                        tool.set(Tool::Move);
                    }
                }
            }
        }
    };

    let on_workspace_mousedown = move |ev: leptos::ev::MouseEvent| {
        // Only fires for clicks on the workspace background itself,
        // because the artboard and nodes stop propagation or are not
        // children of the listener element chain we check here.
        drag.set(DragMode::Pan {
            start_x: ev.client_x(),
            start_y: ev.client_y(),
            orig_x: pan.get_untracked().0,
            orig_y: pan.get_untracked().1,
        });
        if tool.get() == Tool::Move {
            selected_id.set(String::new());
            sel_set.set(BTreeSet::new());
        }
        ctx_menu.set(None);
    };

    let on_mouse_move = move |ev: leptos::ev::MouseEvent| {
        let s = {
            let (w, _) = bundle.with_untracked(|b| {
                b.project
                    .pages
                    .get(page_index.get_untracked())
                    .map(|pg| (pg.width, pg.height))
                    .unwrap_or((800.0, 600.0))
            });
            let avail = (win_width.get_untracked() - 700.0).max(240.0);
            zoom.get_untracked()
                .unwrap_or_else(|| (avail / w).clamp(0.1, 1.2))
        };
        match drag.get_untracked() {
            DragMode::None => {}
            DragMode::Marquee { sx, sy, .. } => {
                if let Some((px, py)) = artboard_point(&ev) {
                    drag.set(DragMode::Marquee { sx, sy, cx: px, cy: py });
                }
            }
            DragMode::Pan { start_x, start_y, orig_x, orig_y } => {
                pan.set((
                    orig_x + (ev.client_x() - start_x) as f32,
                    orig_y + (ev.client_y() - start_y) as f32,
                ));
            }
            DragMode::MoveNode { start_x, start_y, items, primary_ax, primary_ay, primary_w, primary_h, targets } => {
                let mut dx = (ev.client_x() - start_x) as f32 / s;
                let mut dy = (ev.client_y() - start_y) as f32 / s;
                // Snap the primary node edges and centres to other
                // nodes within 6 canvas units; collect guide lines.
                let nx = primary_ax + dx;
                let ny = primary_ay + dy;
                let x_points = [nx, nx + primary_w / 2.0, nx + primary_w];
                let y_points = [ny, ny + primary_h / 2.0, ny + primary_h];
                let mut best_x: Option<(f32, f32)> = None;
                let mut best_y: Option<(f32, f32)> = None;
                for &(tx, ty, tw, th) in &targets {
                    for tp in [tx, tx + tw / 2.0, tx + tw] {
                        for mp in x_points.iter() {
                            let delta = tp - mp;
                            if delta.abs() <= 6.0
                                && best_x.map(|(b, _)| delta.abs() < b.abs()).unwrap_or(true)
                            {
                                best_x = Some((delta, tp));
                            }
                        }
                    }
                    for tp in [ty, ty + th / 2.0, ty + th] {
                        for mp in y_points.iter() {
                            let delta = tp - mp;
                            if delta.abs() <= 6.0
                                && best_y.map(|(b, _)| delta.abs() < b.abs()).unwrap_or(true)
                            {
                                best_y = Some((delta, tp));
                            }
                        }
                    }
                }
                let mut new_guides: Vec<(bool, f32)> = Vec::new();
                if let Some((delta, pos)) = best_x {
                    dx += delta;
                    new_guides.push((true, pos));
                }
                if let Some((delta, pos)) = best_y {
                    dy += delta;
                    new_guides.push((false, pos));
                }
                guides.set(new_guides);
                bundle.update(|b| {
                    if let Some(page) = b.project.pages.get_mut(page_index.get_untracked()) {
                        for (id, ox, oy) in &items {
                            if let Some(node) = page.root.find_mut(id) {
                                node.x = (ox + dx).max(-2000.0);
                                node.y = (oy + dy).max(-2000.0);
                            }
                        }
                    }
                });
            }
            DragMode::Resize { corner, start_x, start_y, orig_x, orig_y, orig_w, orig_h } => {
                let dx = (ev.client_x() - start_x) as f32 / s;
                let dy = (ev.client_y() - start_y) as f32 / s;
                let id = selected_id.get_untracked();
                bundle.update(|b| {
                    if let Some(page) = b.project.pages.get_mut(page_index.get_untracked()) {
                        if let Some(node) = page.root.find_mut(&id) {
                            let (mut x, mut y, mut w, mut h) = (orig_x, orig_y, orig_w, orig_h);
                            if corner & 1 != 0 {
                                w = (orig_w + dx).max(8.0);
                            }
                            if corner & 2 != 0 {
                                h = (orig_h + dy).max(8.0);
                            }
                            if corner & 4 != 0 {
                                let nw = (orig_w - dx).max(8.0);
                                x = orig_x + (orig_w - nw);
                                w = nw;
                            }
                            if corner & 8 != 0 {
                                let nh = (orig_h - dy).max(8.0);
                                y = orig_y + (orig_h - nh);
                                h = nh;
                            }
                            node.x = x;
                            node.y = y;
                            node.width = w;
                            node.height = h;
                        }
                    }
                });
            }
        }
        // Dragging or resizing inside (or of) a layout container
        // reflows its children live.
        if matches!(
            drag.get_untracked(),
            DragMode::MoveNode { .. } | DragMode::Resize { .. }
        ) {
            bundle.update(|b| {
                if let Some(page) = b.project.pages.get_mut(page_index.get_untracked()) {
                    page.root.apply_layout();
                }
            });
        }
    };
    let on_mouse_up = move |_: leptos::ev::MouseEvent| {
        if let DragMode::Marquee { sx, sy, cx, cy } = drag.get_untracked() {
            let (x0, x1) = (sx.min(cx), sx.max(cx));
            let (y0, y1) = (sy.min(cy), sy.max(cy));
            if x1 - x0 < 4.0 && y1 - y0 < 4.0 {
                // A plain click on empty artboard: clear selection.
                selected_id.set(String::new());
                sel_set.set(BTreeSet::new());
            } else {
                let hits: Vec<String> = flat_canvas()
                    .into_iter()
                    .filter(|n| {
                        n.ax < x1 && n.ax + n.width > x0 && n.ay < y1 && n.ay + n.height > y0
                    })
                    .map(|n| n.id)
                    .collect();
                selected_id.set(hits.first().cloned().unwrap_or_default());
                sel_set.set(hits.into_iter().collect());
            }
        }
        if !matches!(drag.get_untracked(), DragMode::None) {
            drag.set(DragMode::None);
            guides.set(Vec::new());
            persist();
        }
    };

    // ---- inspector edits --------------------------------------------------
    let edit_selected = move |key: &str, f: &dyn Fn(&mut Node)| {
        record(key);
        let id = selected_id.get_untracked();
        apply(&|p: &mut Project| {
            if let Some(page) = p.pages.get_mut(page_index.get_untracked()) {
                if let Some(node) = page.root.find_mut(&id) {
                    f(node);
                }
            }
        });
    };

    // ---- selection actions (keyboard + context menu share these) --------
    let select_only = move |id: String| {
        selected_id.set(id.clone());
        let mut set = BTreeSet::new();
        if !id.is_empty() {
            set.insert(id);
        }
        sel_set.set(set);
    };
    let delete_selection = move || {
        let ids = sel_set.get_untracked();
        if ids.is_empty() {
            return;
        }
        record("delete");
        apply(&|p: &mut Project| {
            if let Some(page) = p.pages.get_mut(page_index.get_untracked()) {
                remove_from_tree(&mut page.root, &ids);
            }
        });
        select_only(String::new());
        ctx_menu.set(None);
    };
    let duplicate_selection = move || {
        let ids = sel_set.get_untracked();
        if ids.is_empty() {
            return;
        }
        record("duplicate");
        let mut new_ids = Vec::new();
        let mut c = counter.get_untracked();
        bundle.update(|b| {
            if let Some(page) = b.project.pages.get_mut(page_index.get_untracked()) {
                // Duplicate top-most selected nodes only: a selected
                // child rides along with its selected ancestor.
                let tops: Vec<Node> = page
                    .root
                    .children
                    .iter()
                    .filter(|ch| ids.contains(&ch.id))
                    .cloned()
                    .collect();
                let mut extra: Vec<Node> = Vec::new();
                let collect_deep = |root: &Node, out: &mut Vec<Node>| {
                    for ch in &root.children {
                        if ids.contains(&ch.id) {
                            out.push(ch.clone());
                        } else {
                            // keep walking: a deep selected node can
                            // still be duplicated on its own
                            let mut sub = Vec::new();
                            // recursion through a helper closure is not
                            // possible; do a manual stack walk instead
                            let mut stack: Vec<&Node> = vec![ch];
                            while let Some(n) = stack.pop() {
                                for cc in &n.children {
                                    if ids.contains(&cc.id) {
                                        sub.push(cc.clone());
                                    } else {
                                        stack.push(cc);
                                    }
                                }
                            }
                            out.extend(sub);
                        }
                    }
                };
                if tops.is_empty() {
                    collect_deep(&page.root, &mut extra);
                }
                let sources: Vec<Node> = if tops.is_empty() { extra } else { tops };
                let mut copies: Vec<Node> = Vec::new();
                for src in &sources {
                    let mut copy = clone_with_new_ids(src, &mut c);
                    copy.x += 24.0;
                    copy.y += 24.0;
                    new_ids.push(copy.id.clone());
                    copies.push(copy);
                }
                page.root.children.extend(copies);
            }
        });
        counter.set(c);
        persist();
        if let Some(first) = new_ids.first() {
            selected_id.set(first.clone());
            sel_set.set(new_ids.into_iter().collect());
        }
        ctx_menu.set(None);
    };
    let nudge_selection = move |dx: f32, dy: f32| {
        let ids = sel_set.get_untracked();
        if ids.is_empty() {
            return;
        }
        record("nudge");
        apply(&|p: &mut Project| {
            if let Some(page) = p.pages.get_mut(page_index.get_untracked()) {
                for id in &ids {
                    if let Some(n) = page.root.find_mut(id) {
                        n.x += dx;
                        n.y += dy;
                    }
                }
            }
        });
    };
    let reorder_selection = move |id: String, forward: bool| {
        record("reorder");
        apply(&|p: &mut Project| {
            if let Some(page) = p.pages.get_mut(page_index.get_untracked()) {
                move_in_parent(&mut page.root, &id, forward);
            }
        });
        ctx_menu.set(None);
    };
    let insert_component = move |key: &str| {
        record("insert-component");
        let mut c = counter.get_untracked();
        let primary = bundle.with_untracked(|b| b.project.theme.primary);
        let mut node = build_component(key, &mut c, primary);
        counter.set(c);
        // Place at the centre of the visible part of the artboard.
        let (mut cx, mut cy) = (200.0f32, 120.0f32);
        if let Some(el) = artboard_ref.get_untracked() {
            let rect = el.get_bounding_client_rect();
            let win = web_sys::window();
            let (vw, vh) = (
                win.as_ref().and_then(|w| w.inner_width().ok()).and_then(|v| v.as_f64()).unwrap_or(1600.0) as f32,
                win.as_ref().and_then(|w| w.inner_height().ok()).and_then(|v| v.as_f64()).unwrap_or(900.0) as f32,
            );
            let s = scale();
            if s > 0.0 {
                let left = rect.left() as f32;
                let top = rect.top() as f32;
                let vis_cx = ((left.max(0.0) + (left + rect.width() as f32).min(vw)) / 2.0 - left) / s;
                let vis_cy = ((top.max(0.0) + (top + rect.height() as f32).min(vh)) / 2.0 - top) / s;
                cx = vis_cx - node.width / 2.0;
                cy = vis_cy - node.height / 2.0;
            }
        }
        node.x = cx.max(0.0);
        node.y = cy.max(0.0);
        let new_id = node.id.clone();
        apply(&|p: &mut Project| {
            if let Some(page) = p.pages.get_mut(page_index.get_untracked()) {
                page.root.children.push(node.clone());
            }
        });
        select_only(new_id);
    };

    // ---- image upload -----------------------------------------------------
    /// Put an uploaded image (a data URL) into the project: replace
    /// the target image node's source, or add a new image node at
    /// the visible centre when there is no target. Uploads live in
    /// the browser project copy (localStorage), as the UI states.
    let apply_upload = move |data_url: String| {
        if let Some(id) = upload_target.get_untracked() {
            upload_target.set(None);
            record("upload");
            apply(&|p: &mut Project| {
                if let Some(page) = p.pages.get_mut(page_index.get_untracked()) {
                    if let Some(node) = page.root.find_mut(&id) {
                        node.props.insert("src".into(), data_url.clone());
                    }
                }
            });
            return;
        }
        record("upload");
        counter.update(|c| *c += 1);
        let n = counter.get_untracked();
        let mut node = Node::new(
            format!("node-image-{n}"),
            "عکس آپلودی",
            NodeKind::Image,
            200.0,
            160.0,
            360.0,
            240.0,
        );
        node.corner_radius = 10.0;
        node.props.insert("src".into(), data_url);
        if let Some(el) = artboard_ref.get_untracked() {
            let rect = el.get_bounding_client_rect();
            let win = web_sys::window();
            let (vw, vh) = (
                win.as_ref().and_then(|w| w.inner_width().ok()).and_then(|v| v.as_f64()).unwrap_or(1600.0) as f32,
                win.as_ref().and_then(|w| w.inner_height().ok()).and_then(|v| v.as_f64()).unwrap_or(900.0) as f32,
            );
            let s = scale();
            if s > 0.0 {
                let left = rect.left() as f32;
                let top = rect.top() as f32;
                node.x = (((left.max(0.0) + (left + rect.width() as f32).min(vw)) / 2.0 - left) / s
                    - node.width / 2.0)
                    .max(0.0);
                node.y = (((top.max(0.0) + (top + rect.height() as f32).min(vh)) / 2.0 - top) / s
                    - node.height / 2.0)
                    .max(0.0);
            }
        }
        let new_id = node.id.clone();
        apply(&|p: &mut Project| {
            if let Some(page) = p.pages.get_mut(page_index.get_untracked()) {
                page.root.children.push(node.clone());
            }
        });
        select_only(new_id);
    };
    let on_file_change = move |ev: leptos::ev::Event| {
        let Some(target) = ev.target() else { return };
        let Ok(input) = target.dyn_into::<web_sys::HtmlInputElement>() else {
            return;
        };
        let Some(files) = input.files() else { return };
        let Some(file) = files.get(0) else { return };
        let Ok(reader) = web_sys::FileReader::new() else {
            return;
        };
        let reader2 = reader.clone();
        let onload = wasm_bindgen::closure::Closure::wrap(Box::new(move || {
            if let Ok(result) = reader2.result() {
                if let Some(text) = result.as_string() {
                    apply_upload(text);
                }
            }
        }) as Box<dyn FnMut()>);
        reader.set_onload(Some(onload.as_ref().unchecked_ref()));
        onload.forget();
        let _ = reader.read_as_data_url(&file);
        input.set_value("");
    };
    let trigger_upload = move |target: Option<String>| {
        upload_target.set(target);
        if let Some(input) = file_ref.get_untracked() {
            input.click();
        }
    };

    // ---- named styles -------------------------------------------------------
    let save_style = move || {
        let name = style_name.get_untracked().trim().to_string();
        if name.is_empty() {
            return;
        }
        let Some(node) = selected_node() else { return };
        let style = NamedStyle {
            name: name.clone(),
            fill: node.fill.map(|c| c.to_hex()),
            gradient: node
                .fill_gradient
                .map(|g| (g.from.to_hex(), g.to.to_hex(), g.angle)),
            text_color: node.text_color.map(|c| c.to_hex()),
            font_size: node.font_size,
            font_weight: node.effective_weight(),
            radius: node.corner_radius,
        };
        bundle.update(|b| {
            b.styles.retain(|s| s.name != name);
            b.styles.push(style);
        });
        persist();
        style_name.set(String::new());
    };
    let apply_style = move |style: NamedStyle| {
        let id = selected_id.get_untracked();
        if id.is_empty() {
            return;
        }
        record("style");
        apply(&|p: &mut Project| {
            if let Some(page) = p.pages.get_mut(page_index.get_untracked()) {
                if let Some(node) = page.root.find_mut(&id) {
                    node.fill_gradient = style.gradient.as_ref().map(|(f, t, a)| {
                        Gradient::new(
                            Color::from_hex(f).unwrap_or(Color::BLACK),
                            Color::from_hex(t).unwrap_or(Color::WHITE),
                            *a,
                        )
                    });
                    if style.gradient.is_none() {
                        if let Some(f) = &style.fill {
                            if let Ok(c) = Color::from_hex(f) {
                                node.fill = Some(c);
                            }
                        }
                    }
                    if let Some(tc) = &style.text_color {
                        if let Ok(c) = Color::from_hex(tc) {
                            node.text_color = Some(c);
                        }
                    }
                    node.font_size = style.font_size;
                    node.font_weight = style.font_weight;
                    node.bold = style.font_weight >= 600;
                    node.corner_radius = style.radius;
                }
            }
        });
    };
    let delete_style = move |name: String| {
        bundle.update(|b| b.styles.retain(|s| s.name != name));
        persist();
    };

    // ---- zoom to selection (Shift+1) -------------------------------------
    let zoom_to_selection = move || {
        let set = sel_set.get_untracked();
        if set.is_empty() {
            return;
        }
        let nodes = flat_all();
        let mut bb: Option<(f32, f32, f32, f32)> = None;
        for n in nodes.iter().filter(|n| set.contains(&n.id)) {
            bb = Some(match bb {
                None => (n.ax, n.ay, n.ax + n.width, n.ay + n.height),
                Some((x0, y0, x1, y1)) => (
                    x0.min(n.ax),
                    y0.min(n.ay),
                    x1.max(n.ax + n.width),
                    y1.max(n.ay + n.height),
                ),
            });
        }
        let Some((x0, y0, x1, y1)) = bb else { return };
        let (bw, bh) = ((x1 - x0).max(1.0), (y1 - y0).max(1.0));
        let avail_w = (win_width.get_untracked() - 640.0).max(200.0);
        let avail_h = (win_height.get_untracked() - 140.0).max(200.0);
        let s_new = (avail_w / bw).min(avail_h / bh).clamp(0.1, 2.0);
        if let Some(el) = artboard_ref.get_untracked() {
            // The artboard's current screen rect anchors the math:
            // after the pan delta, the selection centre must land on
            // the viewport centre at the new scale.
            let rect = el.get_bounding_client_rect();
            let (cx, cy) = ((x0 + x1) / 2.0, (y0 + y1) / 2.0);
            let dx = win_width.get_untracked() / 2.0 - rect.left() as f32 - cx * s_new;
            let dy = win_height.get_untracked() / 2.0 - rect.top() as f32 - cy * s_new;
            let (px, py) = pan.get_untracked();
            pan.set((px + dx, py + dy));
        }
        zoom.set(Some(s_new));
    };

    // ---- keyboard: delete, duplicate, nudge, undo/redo ------------------
    let _key_handle = window_event_listener(leptos::ev::keydown, move |ev: leptos::ev::KeyboardEvent| {
        // Never steal keys from form fields.
        if let Some(target) = ev.target() {
            if let Ok(el) = target.dyn_into::<web_sys::HtmlElement>() {
                let tag = el.tag_name();
                if tag == "INPUT" || tag == "TEXTAREA" || tag == "SELECT" || el.is_content_editable() {
                    return;
                }
            }
        }
        let key = ev.key();
        // While the preview is open, Escape (and only Escape) is ours.
        if preview.get_untracked() {
            if key == "Escape" {
                preview.set(false);
            }
            return;
        }
        if ev.shift_key() && (key == "1" || key == "!") {
            ev.prevent_default();
            zoom_to_selection();
            return;
        }
        if ev.shift_key() && (key == "0" || key == ")") {
            ev.prevent_default();
            zoom.set(None);
            pan.set((0.0, 0.0));
            return;
        }
        let step = if ev.shift_key() { 10.0 } else { 1.0 };
        match key.as_str() {
            "Delete" | "Backspace" => {
                if !sel_set.get_untracked().is_empty() {
                    ev.prevent_default();
                    delete_selection();
                }
            }
            "ArrowLeft" => {
                ev.prevent_default();
                nudge_selection(-step, 0.0);
            }
            "ArrowRight" => {
                ev.prevent_default();
                nudge_selection(step, 0.0);
            }
            "ArrowUp" => {
                ev.prevent_default();
                nudge_selection(0.0, -step);
            }
            "ArrowDown" => {
                ev.prevent_default();
                nudge_selection(0.0, step);
            }
            "Escape" => {
                select_only(String::new());
                ctx_menu.set(None);
            }
            _ => {
                if (ev.ctrl_key() || ev.meta_key()) && (key == "d" || key == "D") {
                    ev.prevent_default();
                    duplicate_selection();
                }
            }
        }
    });

    let zoom_steps = [0.25f32, 0.5, 0.75, 1.0, 1.25, 1.5, 2.0];
    let zoom_in = move |_| {
        let cur = scale();
        let next = zoom_steps
            .iter()
            .copied()
            .find(|z| *z > cur + 0.01)
            .unwrap_or(2.0);
        zoom.set(Some(next));
    };
    let zoom_out = move |_| {
        let cur = scale();
        let next = zoom_steps
            .iter()
            .copied()
            .rev()
            .find(|z| *z < cur - 0.01)
            .unwrap_or(0.25);
        zoom.set(Some(next));
    };
    let zoom_fit = move |_| {
        zoom.set(None);
        pan.set((0.0, 0.0));
    };

    let tools: Vec<(Tool, &'static str, &'static str)> = vec![
        (Tool::Move, "➤", "انتخاب"),
        (Tool::Hand, "✋", "دست"),
        (Tool::Frame, "▢", "فریم"),
        (Tool::Rect, "▭", "مستطیل"),
        (Tool::Ellipse, "●", "دایره"),
        (Tool::Text, "T", "متن"),
        (Tool::Image, "🖼", "تصویر"),
        (Tool::Button, "▣", "دکمه"),
        (Tool::Card, "▤", "کارت"),
        (Tool::Input, "⌨", "ورودی"),
    ];

    view! {
        <main class="shell" on:mousemove=on_mouse_move on:mouseup=on_mouse_up>
            <header class="topbar">
                <div class="brand">
                    <img class="logo-img" src="assets/logo-mark.svg" alt="Chida"/>
                    <strong>"چیدا"</strong>
                    <span class="project-name">{move || bundle.with(|b| b.project.name.clone())}</span>
                </div>
                <div class="history-btns">
                    <button title="واگردانی" disabled=move || past.with(|p| p.is_empty()) on:click=undo>"↩"</button>
                    <button title="از نو" disabled=move || future.with(|f| f.is_empty()) on:click=redo>"↪"</button>
                </div>
                <nav class="levels" aria-label="سطح ویرایشگر">
                    <button class:active=move || level.get() == 1 on:click=move |_| { level.set(1); if tab.get() > 0 { tab.set(0); } }>"ساده"</button>
                    <button class:active=move || level.get() == 2 on:click=move |_| { level.set(2); if tab.get() == 1 || tab.get() == 3 { tab.set(0); } }>"سازنده"</button>
                    <button class:active=move || level.get() == 3 on:click=move |_| level.set(3)>"حرفه‌ای"</button>
                </nav>
                <div class="zoom-box">
                    <button title="کوچک نمایی" on:click=zoom_out>"−"</button>
                    <span class="zoom-label">{move || format!("٪{}", (scale() * 100.0).round() as i32)}</span>
                    <button title="بزرگ نمایی" on:click=zoom_in>"+"</button>
                    <button class="fit-btn" title="جا دادن صفحه در نما" on:click=zoom_fit>"جا دادن"</button>
                </div>
                <div class="top-actions">
                    <button class="preview-btn" on:click=move |_| {
                        preview_page.set(page_index.get_untracked());
                        preview.set(true);
                    }>"▶ پیش‌نمایش"</button>
                    <button on:click=move |_| {
                        let target = selected_node()
                            .filter(|n| n.kind == NodeKind::Image)
                            .map(|n| n.id.clone());
                        trigger_upload(target);
                    }>"آپلود عکس"</button>
                    <button on:click=move |_| {
                        bundle.with(|b| {
                            if let Some(page) = b.project.pages.get(page_index.get_untracked()) {
                                download_text(&format!("{}.svg", page.id), &page_svg(page), "image/svg+xml");
                                render_note.set(format!("رندر SVG صفحه {} دانلود شد", page.name));
                            }
                        });
                    }>"رندر"</button>
                    <button on:click=move |_| { level.set(3); tab.set(3); }>"ساخت کد"</button>
                    <button class="publish" disabled title="انتشار آنلاین در نسخه وب فعال نیست؛ پروژه را با خروجی JSON یا ساخت کد تحویل بگیرید">"انتشار"</button>
                    <button class="export" on:click=move |_| {
                        bundle.with(|b| {
                            if let Ok(raw) = serde_json::to_string_pretty(&b.project) {
                                download_text(&format!("{}.chida.json", b.project.id), &raw, "application/json");
                            }
                        });
                    }>"خروجی JSON"</button>
                </div>
            </header>

            <div class="workbench">
                <aside class="rail">
                    {tools.into_iter().map(|(t, icon, label)| {
                        view! {
                            <button
                                class="tool"
                                class:active=move || tool.get() == t
                                title=label
                                on:click=move |_| tool.set(t)
                            >
                                <span class="tool-icon">{icon}</span>
                                <span class="tool-label">{label}</span>
                            </button>
                        }
                    }).collect_view()}
                </aside>

                <aside class="left-panel">
                    <section class="pages-box">
                        <h3>"صفحات"</h3>
                        {move || bundle.with(|b| {
                            b.project.pages.iter().enumerate().map(|(i, pg)| {
                                let (name, route) = (pg.name.clone(), pg.route.clone());
                                view! {
                                    <button
                                        class="page-row"
                                        class:active=move || page_index.get() == i
                                        on:click=move |_| { page_index.set(i); selected_id.set(String::new()); sel_set.set(BTreeSet::new()); }
                                    >
                                        <span class="page-name">{name}</span>
                                        <span class="page-route">{route}</span>
                                    </button>
                                }
                            }).collect::<Vec<_>>()
                        })}
                    </section>
                    <section class="layers-box">
                        <div class="panel-tabs">
                            <button class:active=move || left_tab.get() == 0 on:click=move |_| left_tab.set(0)>"لایه‌ها"</button>
                            <button class:active=move || left_tab.get() == 1 on:click=move |_| left_tab.set(1)>"اجزا"</button>
                            {move || (level.get() >= 2).then(|| view! {
                                <button class:active=move || left_tab.get() == 2 on:click=move |_| left_tab.set(2)>"استایل‌ها"</button>
                            })}
                        </div>
                        {move || (left_tab.get() == 1).then(|| view! {
                            <div class="components">
                                <p class="hint">"با یک کلیک، جزء آماده با استایل کامل وسط صفحه درج می‌شود."</p>
                                <div class="comp-grid">
                                    {[
                                        ("navbar", "🧭", "نوبار"),
                                        ("hero", "🖼", "هیرو با عکس"),
                                        ("btn-primary", "🔘", "دکمه اصلی"),
                                        ("btn-secondary", "⚪", "دکمه دوم"),
                                        ("input", "⌨", "ورودی فرم"),
                                        ("product-card", "🛍", "کارت محصول با عکس"),
                                        ("price-card", "💳", "کارت قیمت"),
                                        ("badge", "🏷", "نشان"),
                                        ("avatar", "👤", "آواتار با نام"),
                                        ("table", "📋", "جدول ساده"),
                                        ("features", "✨", "بخش ویژگی ها"),
                                    ].into_iter().map(|(key, icon, label)| {
                                        view! {
                                            <button class="comp-card" data-comp=key on:click=move |_| insert_component(key)>
                                                <span class="comp-icon">{icon}</span>
                                                <span class="comp-name">{label}</span>
                                            </button>
                                        }
                                    }).collect_view()}
                                </div>
                            </div>
                        })}
                        {move || (left_tab.get() == 2).then(|| view! {
                            <div class="styles-panel">
                                <p class="hint">"ظاهر عنصر انتخاب شده (رنگ، گرادیان، متن و گردی) را با یک نام ذخیره کنید و با یک کلیک روی هر عنصر دیگری اعمال کنید."</p>
                                <div class="style-save">
                                    <input type="text" placeholder="نام استایل" prop:value=move || style_name.get() on:input=move |ev| style_name.set(event_target_value(&ev)) />
                                    <button class="style-save-btn" disabled=move || selected_id.get().is_empty() on:click=move |_| save_style()>"ذخیره"</button>
                                </div>
                                {move || {
                                    let styles = bundle.with(|b| b.styles.clone());
                                    if styles.is_empty() {
                                        return view! { <p class="hint">"هنوز استایلی ذخیره نشده است."</p> }.into_any();
                                    }
                                    styles.into_iter().map(|st| {
                                        let st_apply = st.clone();
                                        let name_del = st.name.clone();
                                        let swatch = st.gradient.as_ref()
                                            .map(|(f, t, a)| format!("background:linear-gradient({}deg,{},{});", a, f, t))
                                            .or_else(|| st.fill.as_ref().map(|f| format!("background:{};", f)))
                                            .unwrap_or_default();
                                        view! {
                                            <div class="style-row">
                                                <span class="style-swatch" style=swatch></span>
                                                <button class="style-apply" title="اعمال روی عنصر انتخاب شده" on:click=move |_| apply_style(st_apply.clone())>{st.name.clone()}</button>
                                                <button class="mini danger" title="حذف استایل" on:click=move |_| delete_style(name_del.clone())>"✕"</button>
                                            </div>
                                        }
                                    }).collect_view().into_any()
                                }}
                            </div>
                        })}
                        <div class="layers" style=move || if left_tab.get() != 0 { "display:none;" } else { "" }>
                        {move || {
                            let mut nodes = flat_all();
                            nodes.reverse();
                            let is_simple = level.get() == 1;
                            nodes.into_iter().map(|n| {
                                let id_eye = n.id.clone();
                                let id_lock = n.id.clone();
                                let id_lock_class = n.id.clone();
                                let id_lock_icon = n.id.clone();
                                let id_row = n.id.clone();
                                let id_class = n.id.clone();
                                let indent = format!("padding-inline-start:{}px;", 8 + n.depth * 14);
                                view! {
                                    <div
                                        class="layer-row"
                                        data-layer-id=n.id.clone()
                                        class:selected=move || sel_set.get().contains(&id_class)
                                        style=indent
                                        on:click=move |ev: leptos::ev::MouseEvent| {
                                            if ev.shift_key() {
                                                let mut set = sel_set.get_untracked();
                                                if !set.remove(&id_row) {
                                                    set.insert(id_row.clone());
                                                    selected_id.set(id_row.clone());
                                                } else if selected_id.get_untracked() == id_row {
                                                    selected_id.set(set.iter().next().cloned().unwrap_or_default());
                                                }
                                                sel_set.set(set);
                                            } else {
                                                selected_id.set(id_row.clone());
                                                let mut one = BTreeSet::new();
                                                one.insert(id_row.clone());
                                                sel_set.set(one);
                                            }
                                        }
                                    >
                                        <span class="layer-icon">{kind_icon(n.kind)}</span>
                                        <span class="layer-name">{n.name.clone()}</span>
                                        {(!is_simple).then(|| view! { <span class="layer-kind">{kind_fa(n.kind)}</span> })}
                                        <button
                                            class="mini"
                                            class:off=!n.visible
                                            title="نمایش یا پنهان کردن"
                                            on:click=move |ev: leptos::ev::MouseEvent| {
                                                ev.stop_propagation();
                                                record("visibility");
                                                let id = id_eye.clone();
                                                apply(&|p: &mut Project| {
                                                    if let Some(page) = p.pages.get_mut(page_index.get_untracked()) {
                                                        if let Some(node) = page.root.find_mut(&id) {
                                                            node.visible = !node.visible;
                                                        }
                                                    }
                                                });
                                            }
                                        >{if n.visible { "👁" } else { "🚫" }}</button>
                                        <button
                                            class="mini"
                                            class:off=move || !locked.with(|l| l.contains(&id_lock_class))
                                            title="قفل کردن لایه"
                                            on:click=move |ev: leptos::ev::MouseEvent| {
                                                ev.stop_propagation();
                                                let id = id_lock.clone();
                                                locked.update(|l| {
                                                    if !l.remove(&id) {
                                                        l.insert(id);
                                                    }
                                                });
                                            }
                                        >{move || if locked.with(|l| l.contains(&id_lock_icon)) { "🔒" } else { "🔓" }}</button>
                                    </div>
                                }
                            }).collect_view()
                        }}
                        </div>
                    </section>
                </aside>

                <section class="workspace" on:mousedown=on_workspace_mousedown>
                    <div
                        class="world"
                        style=move || format!("transform:translate({}px,{}px);", pan.get().0, pan.get().1)
                    >
                        <div
                            class="ruler-x"
                            style=move || {
                                let (w, _) = page_size();
                                format!("width:{}px;", w * scale())
                            }
                        >
                            {move || {
                                let (w, _) = page_size();
                                let s = scale();
                                (0..=((w / 100.0) as i32)).map(|i| {
                                    let pos = i as f32 * 100.0 * s;
                                    view! {
                                        <span class="tick" style=format!("left:{pos}px;")>
                                            <i></i>{(i * 100).to_string()}
                                        </span>
                                    }
                                }).collect_view()
                            }}
                        </div>
                        <div class="artboard-label">
                            {move || {
                                let (name, route) = page_meta();
                                let (w, h) = page_size();
                                format!("{name}  {route}  {} × {}", w as i32, h as i32)
                            }}
                        </div>
                        <div class="canvas-row">
                        <div
                            class="ruler-y"
                            style=move || {
                                let (_, h) = page_size();
                                format!("height:{}px;", h * scale())
                            }
                        >
                            {move || {
                                let (_, h) = page_size();
                                let s = scale();
                                (0..=((h / 100.0) as i32)).map(|i| {
                                    let pos = i as f32 * 100.0 * s;
                                    view! {
                                        <span class="tick" style=format!("top:{pos}px;")>
                                            <i></i>{(i * 100).to_string()}
                                        </span>
                                    }
                                }).collect_view()
                            }}
                        </div>
                        <div
                            class="artboard-scaler"
                            style=move || {
                                let (w, h) = page_size();
                                let s = scale();
                                format!("width:{}px;height:{}px;", w * s, h * s)
                            }
                        >
                            <div
                                class="artboard"
                                node_ref=artboard_ref
                                style=move || {
                                    let (w, h) = page_size();
                                    format!(
                                        "width:{w}px;height:{h}px;background:{};transform:scale({});",
                                        page_bg(),
                                        scale()
                                    )
                                }
                                on:mousedown=on_artboard_mousedown
                            >
                                {move || flat_canvas().into_iter().map(|n| {
                                    let id_down = n.id.clone();
                                    let id_class = n.id.clone();
                                    let id_ctx = n.id.clone();
                                    let style = node_css(&n);
                                    let label = if n.src.is_some() {
                                        String::new()
                                    } else if n.kind == NodeKind::Image {
                                        "🖼".to_string()
                                    } else {
                                        n.text.clone().unwrap_or_default()
                                    };
                                    view! {
                                        <div
                                            class="node"
                                            dir="auto"
                                            data-kind=kind_name(n.kind)
                                            data-node-id=n.id.clone()
                                            class:selected=move || sel_set.get().contains(&id_class)
                                            style=style
                                            on:contextmenu=move |ev: leptos::ev::MouseEvent| {
                                                ev.prevent_default();
                                                ev.stop_propagation();
                                                let resolved = bundle.with_untracked(|b| {
                                                    b.project
                                                        .pages
                                                        .get(page_index.get_untracked())
                                                        .map(|pg| group_root_of(&pg.root, &id_ctx))
                                                        .unwrap_or_else(|| id_ctx.clone())
                                                });
                                                if !sel_set.get_untracked().contains(&resolved) {
                                                    select_only(resolved.clone());
                                                }
                                                ctx_menu.set(Some((ev.client_x(), ev.client_y(), resolved)));
                                            }
                                            on:mousedown=move |ev: leptos::ev::MouseEvent| {
                                                match tool.get() {
                                                    Tool::Hand => {
                                                        ev.stop_propagation();
                                                        drag.set(DragMode::Pan {
                                                            start_x: ev.client_x(),
                                                            start_y: ev.client_y(),
                                                            orig_x: pan.get_untracked().0,
                                                            orig_y: pan.get_untracked().1,
                                                        });
                                                    }
                                                    Tool::Move => {
                                                        ev.stop_propagation();
                                                        ctx_menu.set(None);
                                                        let resolved = bundle.with_untracked(|b| {
                                                            b.project
                                                                .pages
                                                                .get(page_index.get_untracked())
                                                                .map(|pg| group_root_of(&pg.root, &id_down))
                                                                .unwrap_or_else(|| id_down.clone())
                                                        });
                                                        if ev.shift_key() {
                                                            // Toggle membership, no drag.
                                                            let mut set = sel_set.get_untracked();
                                                            if set.contains(&resolved) {
                                                                set.remove(&resolved);
                                                                if selected_id.get_untracked() == resolved {
                                                                    selected_id.set(set.iter().next().cloned().unwrap_or_default());
                                                                }
                                                            } else {
                                                                set.insert(resolved.clone());
                                                                selected_id.set(resolved);
                                                            }
                                                            sel_set.set(set);
                                                            return;
                                                        }
                                                        let mut set = sel_set.get_untracked();
                                                        if !set.contains(&resolved) {
                                                            set = BTreeSet::new();
                                                            set.insert(resolved.clone());
                                                            sel_set.set(set.clone());
                                                        }
                                                        selected_id.set(resolved.clone());
                                                        if locked.with_untracked(|l| l.contains(&resolved)) {
                                                            return;
                                                        }
                                                        // A child of an auto-layout
                                                        // container is positioned by
                                                        // its parent: it can be
                                                        // selected but not dragged.
                                                        let parent_locked = bundle.with_untracked(|b| {
                                                            b.project
                                                                .pages
                                                                .get(page_index.get_untracked())
                                                                .and_then(|pg| parent_of(&pg.root, &resolved))
                                                                .map(|p| p.has_layout())
                                                                .unwrap_or(false)
                                                        });
                                                        if parent_locked {
                                                            return;
                                                        }
                                                        record("drag");
                                                        let drag_data = bundle.with_untracked(|b| {
                                                            let page = b.project.pages.get(page_index.get_untracked())?;
                                                            let flat = page_flat(page, true);
                                                            let primary = flat.iter().find(|f| f.id == resolved)?;
                                                            let items: Vec<(String, f32, f32)> = set
                                                                .iter()
                                                                .filter_map(|sid| {
                                                                    page.root
                                                                        .find(sid)
                                                                        .map(|node| (sid.clone(), node.x, node.y))
                                                                })
                                                                .collect();
                                                            let targets: Vec<(f32, f32, f32, f32)> = flat
                                                                .iter()
                                                                .filter(|f| !set.contains(&f.id))
                                                                .map(|f| (f.ax, f.ay, f.width, f.height))
                                                                .collect();
                                                            Some((items, primary.ax, primary.ay, primary.width, primary.height, targets))
                                                        });
                                                        if let Some((items, pax, pay, pw, ph, targets)) = drag_data {
                                                            drag.set(DragMode::MoveNode {
                                                                start_x: ev.client_x(),
                                                                start_y: ev.client_y(),
                                                                items,
                                                                primary_ax: pax,
                                                                primary_ay: pay,
                                                                primary_w: pw,
                                                                primary_h: ph,
                                                                targets,
                                                            });
                                                        }
                                                    }
                                                    _ => {}
                                                }
                                            }
                                        >{label}</div>
                                    }
                                }).collect_view()}
                                {move || guides.get().into_iter().map(|(vertical, pos)| {
                                    let (pw, ph) = page_size();
                                    let style = if vertical {
                                        format!("left:{pos}px;top:0;width:2px;height:{ph}px;")
                                    } else {
                                        format!("top:{pos}px;left:0;height:2px;width:{pw}px;")
                                    };
                                    view! { <div class="snap-guide" style=style></div> }
                                }).collect_view()}
                                {move || match drag.get() {
                                    DragMode::Marquee { sx, sy, cx, cy } => {
                                        let style = format!(
                                            "left:{}px;top:{}px;width:{}px;height:{}px;",
                                            sx.min(cx),
                                            sy.min(cy),
                                            (cx - sx).abs(),
                                            (cy - sy).abs()
                                        );
                                        view! { <div class="marquee" style=style></div> }.into_any()
                                    }
                                    _ => ().into_any(),
                                }}
                                {move || {
                                    let nodes = flat_canvas();
                                    let set = sel_set.get();
                                    let primary = selected_id.get();
                                    nodes.iter().filter(|n| set.contains(&n.id)).map(|n| {
                                        let is_primary = n.id == primary;
                                        let handles = if is_primary {
                                            let s = scale();
                                            let hs = 9.0 / s;
                                            let corners: [(u8, &str); 4] = [
                                                (12, "left:0;top:0;transform:translate(-50%,-50%);cursor:nwse-resize;"),
                                                (9, "left:100%;top:0;transform:translate(-50%,-50%);cursor:nesw-resize;"),
                                                (6, "left:0;top:100%;transform:translate(-50%,-50%);cursor:nesw-resize;"),
                                                (3, "left:100%;top:100%;transform:translate(-50%,-50%);cursor:nwse-resize;"),
                                            ];
                                            corners.into_iter().map(|(corner, pos)| {
                                                let sel_h = n.id.clone();
                                                view! {
                                                    <span
                                                        class="handle"
                                                        style=format!("{pos}width:{hs}px;height:{hs}px;")
                                                        on:mousedown=move |ev: leptos::ev::MouseEvent| {
                                                            ev.stop_propagation();
                                                            record("resize");
                                                            let orig = bundle.with_untracked(|b| {
                                                                b.project
                                                                    .pages
                                                                    .get(page_index.get_untracked())
                                                                    .and_then(|pg| pg.root.find(&sel_h))
                                                                    .map(|node| (node.x, node.y, node.width, node.height))
                                                                    .unwrap_or((0.0, 0.0, 0.0, 0.0))
                                                            });
                                                            drag.set(DragMode::Resize {
                                                                corner,
                                                                start_x: ev.client_x(),
                                                                start_y: ev.client_y(),
                                                                orig_x: orig.0,
                                                                orig_y: orig.1,
                                                                orig_w: orig.2,
                                                                orig_h: orig.3,
                                                            });
                                                        }
                                                    ></span>
                                                }
                                            }).collect_view().into_any()
                                        } else {
                                            ().into_any()
                                        };
                                        view! {
                                            <div
                                                class="selection"
                                                class:multi=!is_primary
                                                style=format!(
                                                    "left:{}px;top:{}px;width:{}px;height:{}px;",
                                                    n.ax, n.ay, n.width, n.height
                                                )
                                            >{handles}</div>
                                        }
                                    }).collect_view()
                                }}
                            </div>
                        </div>
                        </div>
                    </div>
                </section>

                <aside class="right-panel">
                    <nav class="tabs">
                        <button class:active=move || tab.get() == 0 on:click=move |_| tab.set(0)>"طراحی"</button>
                        {move || (level.get() >= 2).then(|| view! {
                            <button class:active=move || tab.get() == 2 on:click=move |_| tab.set(2)>"داده"</button>
                        })}
                        {move || (level.get() >= 3).then(|| view! {
                            <button class:active=move || tab.get() == 1 on:click=move |_| tab.set(1)>"منطق"</button>
                            <button class:active=move || tab.get() == 3 on:click=move |_| tab.set(3)>"کد"</button>
                        })}
                    </nav>
                    <div class="tab-body">
                    {move || {
                        let effective = match (level.get(), tab.get()) {
                            (1, _) => 0,
                            (2, 1) | (2, 3) => 0,
                            (_, t) => t,
                        };
                        match effective {
                            1 => view! {
                                <div class="flows">
                                    <h3>"جریان‌های منطق"</h3>
                                    {bundle.with(|b| {
                                        if b.flows.is_empty() {
                                            return view! { <p class="hint">"این پروژه هنوز جریانی ندارد."</p> }.into_any();
                                        }
                                        b.flows.iter().map(|f| {
                                            let mut lines = Vec::new();
                                            for s in &f.steps {
                                                step_lines(s, 0, &mut lines);
                                            }
                                            view! {
                                                <div class="flow-card">
                                                    <strong>{f.name.clone()}</strong>
                                                    <p class="flow-trigger">
                                                        {format!("رویداد: {} از {} در {}",
                                                            event_fa(&f.trigger.kind),
                                                            f.trigger.source_node.clone().unwrap_or_default(),
                                                            f.trigger.page_route.clone().unwrap_or_default())}
                                                    </p>
                                                    {lines.into_iter().map(|l| view! { <p class="flow-step">{l}</p> }).collect_view()}
                                                </div>
                                            }
                                        }).collect_view().into_any()
                                    })}
                                </div>
                            }.into_any(),
                            2 => view! {
                                <div class="tables">
                                    <h3>"جدول‌های داده"</h3>
                                    {bundle.with(|b| {
                                        if b.database.tables.is_empty() {
                                            return view! { <p class="hint">"این پروژه هنوز جدولی ندارد."</p> }.into_any();
                                        }
                                        b.database.tables.iter().map(|t| {
                                            view! {
                                                <div class="table-card">
                                                    <strong>{t.name.clone()}</strong>
                                                    <span class="table-meta">{format!("{} ردیف نمونه", t.seed_rows.len())}</span>
                                                    {t.columns.iter().map(|c| {
                                                        view! { <p class="col-row">{format!("{} : {:?}", c.name, c.col_type)}</p> }
                                                    }).collect_view()}
                                                </div>
                                            }
                                        }).collect_view().into_any()
                                    })}
                                </div>
                            }.into_any(),
                            3 => view! {
                                <div class="code-tab">
                                    <h3>"خروجی و کد"</h3>
                                    <p class="hint">"ساخت کد کامل (بک اند Axum و فرانت Leptos) با دستور chida generate روی فایل پروژه اجرا می‌شود. اینجا می‌توانید JSON پروژه و رندر سریع SVG را بگیرید."</p>
                                    <button class="wide" on:click=move |_| {
                                        bundle.with(|b| {
                                            if let Ok(raw) = serde_json::to_string_pretty(&*b) {
                                                download_text(&format!("{}.chida.json", b.project.id), &raw, "application/json");
                                            }
                                        });
                                    }>"دانلود JSON کامل پروژه"</button>
                                    <button class="wide" on:click=move |_| {
                                        bundle.with(|b| {
                                            if let Some(page) = b.project.pages.get(page_index.get_untracked()) {
                                                download_text(&format!("{}.svg", page.id), &page_svg(page), "image/svg+xml");
                                            }
                                        });
                                    }>"دانلود رندر SVG این صفحه"</button>
                                    <h4>"پیش‌نمایش JSON"</h4>
                                    <pre class="json-preview">{move || bundle.with(|b| {
                                        let raw = serde_json::to_string_pretty(&b.project).unwrap_or_default();
                                        if raw.chars().count() > 6000 {
                                            format!("{}...", raw.chars().take(6000).collect::<String>())
                                        } else {
                                            raw
                                        }
                                    })}</pre>
                                </div>
                            }.into_any(),
                            _ => view! {
                                <div class="design-tab">
                                    {move || match selected_node() {
                                        None => view! { <p class="hint">"یک عنصر روی بوم یا از فهرست لایه‌ها انتخاب کنید."</p> }.into_any(),
                                        Some(node) => {
                                            let simple = level.get() == 1;
                                            let text_value = node.text.clone().unwrap_or_default();
                                            let fill_full = node.fill.map(|c| c.to_hex()).unwrap_or_else(|| "#ffffff".to_string());
                                            let fill_hex = fill_full.chars().take(7).collect::<String>();
                                            let stroke_hex = node.stroke.map(|c| c.to_hex()).unwrap_or_else(|| "#000000".to_string()).chars().take(7).collect::<String>();
                                            let text_hex = node.text_color.map(|c| c.to_hex()).unwrap_or_else(|| "#1a1a1a".to_string()).chars().take(7).collect::<String>();
                                            let (nx, ny, nw, nh) = (node.x, node.y, node.width, node.height);
                                            let fs = node.font_size;
                                            let weight = node.effective_weight();
                                            let lh_val = if node.line_height > 0.0 { node.line_height } else { 1.45 };
                                            let ls_val = node.letter_spacing;
                                            let radius = node.corner_radius;
                                            let opacity_pct = (node.opacity * 100.0).round();
                                            let sw = node.stroke_width;
                                            let align = node.props.get("align").cloned().unwrap_or_default();
                                            let node_id_sel = node.id.clone();
                                            let is_container = matches!(node.kind, NodeKind::Frame | NodeKind::Card | NodeKind::Custom) || !node.children.is_empty();
                                            let layout_dir = node.layout_direction;
                                            let layout_gap = node.layout_gap;
                                            let layout_pad = node.layout_padding;
                                            let layout_align = node.layout_align;
                                            let parent_layout = bundle.with(|b| {
                                                b.project
                                                    .pages
                                                    .get(page_index.get_untracked())
                                                    .and_then(|pg| parent_of(&pg.root, &node_id_sel))
                                                    .map(|p| p.has_layout())
                                                    .unwrap_or(false)
                                            });
                                            let is_button = node.kind == NodeKind::Button;
                                            let is_image = node.kind == NodeKind::Image;
                                            let link_value = node.props.get("link_page").cloned().unwrap_or_default();
                                            let src_short = node.props.get("src").map(|s| {
                                                if s.chars().count() > 42 {
                                                    format!("{}...", s.chars().take(42).collect::<String>())
                                                } else {
                                                    s.clone()
                                                }
                                            });
                                            let has_grad = node.fill_gradient.is_some();
                                            let (g_from, g_to, g_angle) = node.fill_gradient.map(|g| (g.from.to_hex().chars().take(7).collect::<String>(), g.to.to_hex().chars().take(7).collect::<String>(), g.angle)).unwrap_or_else(|| ("#ef4056".to_string(), "#ff8a5c".to_string(), 90.0));
                                            let has_shadow = node.shadow.is_some();
                                            let (sh_x, sh_y, sh_blur, sh_col, sh_op) = node.shadow.map(|s| (s.x, s.y, s.blur, s.color.to_hex().chars().take(7).collect::<String>(), (s.opacity * 100.0).round())).unwrap_or_else(|| (0.0, 8.0, 24.0, "#000000".to_string(), 25.0));
                                            view! {
                                                <div class="fields" class:big=simple>
                                                    <p class="node-name">{node.name.clone()}<span class="node-kind">{kind_fa(node.kind)}</span></p>
                                                    <details class="prop-section" open>
                                                        <summary>"چیدمان"</summary>
                                                        <div class="grid2">
                                                            <label>"X" <input type="number" disabled=parent_layout prop:value=nx.to_string() on:input=move |ev| { let v = event_target_value(&ev); edit_selected("x", &|n: &mut Node| { if let Ok(v) = v.parse::<f32>() { n.x = v; } }); } /></label>
                                                            <label>"Y" <input type="number" disabled=parent_layout prop:value=ny.to_string() on:input=move |ev| { let v = event_target_value(&ev); edit_selected("y", &|n: &mut Node| { if let Ok(v) = v.parse::<f32>() { n.y = v; } }); } /></label>
                                                            <label>"عرض" <input type="number" prop:value=nw.to_string() on:input=move |ev| { let v = event_target_value(&ev); edit_selected("w", &|n: &mut Node| { if let Ok(v) = v.parse::<f32>() { n.width = v.max(1.0); } }); } /></label>
                                                            <label>"ارتفاع" <input type="number" prop:value=nh.to_string() on:input=move |ev| { let v = event_target_value(&ev); edit_selected("h", &|n: &mut Node| { if let Ok(v) = v.parse::<f32>() { n.height = v.max(1.0); } }); } /></label>
                                                        </div>
                                                        {(!simple).then(|| view! {
                                                            <label>"گردی گوشه" <span class="val">{format!("{}", radius as i32)}</span>
                                                                <input type="range" min="0" max="60" prop:value=radius.to_string() on:input=move |ev| { let v = event_target_value(&ev); edit_selected("radius", &|n: &mut Node| { if let Ok(v) = v.parse::<f32>() { n.corner_radius = v; } }); } />
                                                            </label>
                                                            <label>"شفافیت" <span class="val">{format!("٪{}", opacity_pct as i32)}</span>
                                                                <input type="range" min="0" max="100" prop:value=opacity_pct.to_string() on:input=move |ev| { let v = event_target_value(&ev); edit_selected("opacity", &|n: &mut Node| { if let Ok(v) = v.parse::<f32>() { n.opacity = (v / 100.0).clamp(0.0, 1.0); } }); } />
                                                            </label>
                                                        })}
                                                        {(is_container && !simple).then(|| view! {
                                                            <div class="align-row">
                                                                <span>"چیدمان خودکار"</span>
                                                                <button class:on=layout_dir == LayoutDirection::None on:click=move |_| edit_selected("layoutdir", &|n: &mut Node| n.layout_direction = LayoutDirection::None)>"بدون"</button>
                                                                <button class:on=layout_dir == LayoutDirection::Row on:click=move |_| edit_selected("layoutdir", &|n: &mut Node| n.layout_direction = LayoutDirection::Row)>"افقی"</button>
                                                                <button class:on=layout_dir == LayoutDirection::Column on:click=move |_| edit_selected("layoutdir", &|n: &mut Node| n.layout_direction = LayoutDirection::Column)>"عمودی"</button>
                                                            </div>
                                                        })}
                                                        {(is_container && !simple && layout_dir != LayoutDirection::None).then(|| view! {
                                                            <div class="grid2">
                                                                <label>"فاصله بین بچه‌ها" <input type="number" prop:value=layout_gap.to_string() on:input=move |ev| { let v = event_target_value(&ev); edit_selected("layoutgap", &|n: &mut Node| { if let Ok(v) = v.parse::<f32>() { n.layout_gap = v.max(0.0); } }); } /></label>
                                                                <label>"حاشیه داخلی" <input type="number" prop:value=layout_pad.to_string() on:input=move |ev| { let v = event_target_value(&ev); edit_selected("layoutpad", &|n: &mut Node| { if let Ok(v) = v.parse::<f32>() { n.layout_padding = v.max(0.0); } }); } /></label>
                                                            </div>
                                                            <div class="align-row">
                                                                <span>"تراز"</span>
                                                                <button class:on=layout_align == LayoutAlign::Start on:click=move |_| edit_selected("layoutalign", &|n: &mut Node| n.layout_align = LayoutAlign::Start)>"شروع"</button>
                                                                <button class:on=layout_align == LayoutAlign::Center on:click=move |_| edit_selected("layoutalign", &|n: &mut Node| n.layout_align = LayoutAlign::Center)>"وسط"</button>
                                                                <button class:on=layout_align == LayoutAlign::End on:click=move |_| edit_selected("layoutalign", &|n: &mut Node| n.layout_align = LayoutAlign::End)>"پایان"</button>
                                                                <button class:on=layout_align == LayoutAlign::Stretch on:click=move |_| edit_selected("layoutalign", &|n: &mut Node| n.layout_align = LayoutAlign::Stretch)>"کشیده"</button>
                                                            </div>
                                                        })}
                                                        {parent_layout.then(|| view! {
                                                            <p class="hint lock-note">"این عنصر داخل یک چیدمان خودکار است؛ جای آن خودکار حساب می‌شود و با درگ جابه‌جا نمی‌شود."</p>
                                                        })}
                                                    </details>
                                                    <details class="prop-section" open>
                                                        <summary>"متن"</summary>
                                                        <label class="full"><input type="text" prop:value=text_value on:input=move |ev| { let v = event_target_value(&ev); edit_selected("text", &|n: &mut Node| n.text = Some(v.clone())); } /></label>
                                                        <div class="grid2">
                                                            <label>"اندازه فونت" <input type="number" prop:value=fs.to_string() on:input=move |ev| { let v = event_target_value(&ev); edit_selected("fontsize", &|n: &mut Node| { if let Ok(v) = v.parse::<f32>() { n.font_size = v.max(6.0); } }); } /></label>
                                                            {(!simple).then(|| view! {
                                                                <label>"رنگ متن"
                                                                    <input type="color" prop:value=text_hex on:input=move |ev| { let v = event_target_value(&ev); edit_selected("textcolor", &|n: &mut Node| { if let Ok(c) = Color::from_hex(&v) { n.text_color = Some(c); } }); } />
                                                                </label>
                                                            })}
                                                        </div>
                                                        {(!simple).then(|| view! {
                                                            <div class="align-row">
                                                                <span>"وزن"</span>
                                                                {[(400u16, "عادی"), (500, "متوسط"), (700, "درشت")].into_iter().map(|(w, label)| {
                                                                    let is_on = weight == w;
                                                                    view! {
                                                                        <button class:on=is_on on:click=move |_| edit_selected("weight", &|n: &mut Node| { n.font_weight = w; n.bold = w >= 600; })>{label}</button>
                                                                    }
                                                                }).collect_view()}
                                                            </div>
                                                            <div class="grid2">
                                                                <label>"ارتفاع خط" <input type="number" step="0.05" prop:value=lh_val.to_string() on:input=move |ev| { let v = event_target_value(&ev); edit_selected("lineheight", &|n: &mut Node| { if let Ok(v) = v.parse::<f32>() { n.line_height = v.clamp(0.8, 3.0); } }); } /></label>
                                                                <label>"فاصله حروف" <input type="number" prop:value=ls_val.to_string() on:input=move |ev| { let v = event_target_value(&ev); edit_selected("letterspacing", &|n: &mut Node| { if let Ok(v) = v.parse::<f32>() { n.letter_spacing = v; } }); } /></label>
                                                            </div>
                                                            <div class="align-row">
                                                                <span>"تراز"</span>
                                                                {["right", "center", "left"].into_iter().map(|a| {
                                                                    let label = match a { "right" => "راست", "center" => "وسط", _ => "چپ" };
                                                                    let is_on = align == a;
                                                                    view! {
                                                                        <button class:on=is_on on:click=move |_| edit_selected("align", &|n: &mut Node| { n.props.insert("align".into(), a.to_string()); })>{label}</button>
                                                                    }
                                                                }).collect_view()}
                                                            </div>
                                                        })}
                                                    </details>
                                                    <details class="prop-section" open>
                                                        <summary>"پس‌زمینه"</summary>
                                                        {(!simple).then(|| view! {
                                                            <div class="align-row">
                                                                <span>"نوع"</span>
                                                                <button class:on=!has_grad on:click=move |_| edit_selected("fillmode", &|n: &mut Node| n.fill_gradient = None)>"یکدست"</button>
                                                                <button class:on=has_grad on:click=move |_| edit_selected("fillmode", &|n: &mut Node| {
                                                                    let base = n.fill.unwrap_or(Color::rgb(239, 64, 86));
                                                                    n.fill_gradient = Some(Gradient::new(base, shade(base, 1.4), 90.0));
                                                                })>"گرادیان"</button>
                                                            </div>
                                                        })}
                                                        {(!has_grad).then(|| view! {
                                                            <div class="color-row">
                                                                <span>"رنگ"</span>
                                                                <input type="color" prop:value=fill_hex.clone() on:input=move |ev| { let v = event_target_value(&ev); edit_selected("fill", &|n: &mut Node| { if let Ok(c) = Color::from_hex(&v) { n.fill = Some(c); } }); } />
                                                                <input class="hex" type="text" prop:value=fill_hex on:change=move |ev| { let v = event_target_value(&ev); edit_selected("fill", &|n: &mut Node| { if let Ok(c) = Color::from_hex(&v) { n.fill = Some(c); } }); } />
                                                            </div>
                                                        })}
                                                        {(has_grad && !simple).then(|| view! {
                                                            <div class="color-row">
                                                                <span>"از"</span>
                                                                <input type="color" prop:value=g_from on:input=move |ev| { let v = event_target_value(&ev); edit_selected("gradfrom", &|n: &mut Node| { if let (Some(g), Ok(c)) = (&mut n.fill_gradient, Color::from_hex(&v)) { g.from = c; } }); } />
                                                                <span>"به"</span>
                                                                <input type="color" prop:value=g_to on:input=move |ev| { let v = event_target_value(&ev); edit_selected("gradto", &|n: &mut Node| { if let (Some(g), Ok(c)) = (&mut n.fill_gradient, Color::from_hex(&v)) { g.to = c; } }); } />
                                                            </div>
                                                            <label>"زاویه" <span class="val">{format!("{} درجه", g_angle as i32)}</span>
                                                                <input type="range" min="0" max="360" prop:value=g_angle.to_string() on:input=move |ev| { let v = event_target_value(&ev); edit_selected("gradangle", &|n: &mut Node| { if let (Some(g), Ok(v)) = (&mut n.fill_gradient, v.parse::<f32>()) { g.angle = v; } }); } />
                                                            </label>
                                                        })}
                                                        {(!simple).then(|| view! {
                                                            <div class="color-row">
                                                                <span>"خط دور"</span>
                                                                <input type="color" prop:value=stroke_hex on:input=move |ev| { let v = event_target_value(&ev); edit_selected("stroke", &|n: &mut Node| { if let Ok(c) = Color::from_hex(&v) { n.stroke = Some(c); if n.stroke_width <= 0.0 { n.stroke_width = 1.0; } } }); } />
                                                                <input class="hex small" type="number" prop:value=sw.to_string() on:input=move |ev| { let v = event_target_value(&ev); edit_selected("strokew", &|n: &mut Node| { if let Ok(v) = v.parse::<f32>() { n.stroke_width = v.max(0.0); } }); } />
                                                            </div>
                                                        })}
                                                    </details>
                                                    {(!simple).then(|| view! {
                                                        <details class="prop-section" open>
                                                            <summary>"سایه"</summary>
                                                            <label class="check">"سایه"
                                                                <button class="toggle" class:on=has_shadow on:click=move |_| edit_selected("shadowtoggle", &|n: &mut Node| {
                                                                    n.shadow = if n.shadow.is_some() { None } else { Some(Shadow::soft()) };
                                                                })>{if has_shadow { "روشن" } else { "خاموش" }}</button>
                                                            </label>
                                                            {(has_shadow).then(|| view! {
                                                                <label>"افست افقی" <span class="val">{format!("{}", sh_x as i32)}</span>
                                                                    <input type="range" min="-40" max="40" prop:value=sh_x.to_string() on:input=move |ev| { let v = event_target_value(&ev); edit_selected("shadowx", &|n: &mut Node| { if let (Some(s), Ok(v)) = (&mut n.shadow, v.parse::<f32>()) { s.x = v; } }); } />
                                                                </label>
                                                                <label>"افست عمودی" <span class="val">{format!("{}", sh_y as i32)}</span>
                                                                    <input type="range" min="-40" max="40" prop:value=sh_y.to_string() on:input=move |ev| { let v = event_target_value(&ev); edit_selected("shadowy", &|n: &mut Node| { if let (Some(s), Ok(v)) = (&mut n.shadow, v.parse::<f32>()) { s.y = v; } }); } />
                                                                </label>
                                                                <label>"محو" <span class="val">{format!("{}", sh_blur as i32)}</span>
                                                                    <input type="range" min="0" max="80" prop:value=sh_blur.to_string() on:input=move |ev| { let v = event_target_value(&ev); edit_selected("shadowblur", &|n: &mut Node| { if let (Some(s), Ok(v)) = (&mut n.shadow, v.parse::<f32>()) { s.blur = v; } }); } />
                                                                </label>
                                                                <div class="color-row">
                                                                    <span>"رنگ"</span>
                                                                    <input type="color" prop:value=sh_col on:input=move |ev| { let v = event_target_value(&ev); edit_selected("shadowcolor", &|n: &mut Node| { if let (Some(s), Ok(c)) = (&mut n.shadow, Color::from_hex(&v)) { s.color = c; } }); } />
                                                                </div>
                                                                <label>"شفافیت سایه" <span class="val">{format!("٪{}", sh_op as i32)}</span>
                                                                    <input type="range" min="0" max="100" prop:value=sh_op.to_string() on:input=move |ev| { let v = event_target_value(&ev); edit_selected("shadowop", &|n: &mut Node| { if let (Some(s), Ok(v)) = (&mut n.shadow, v.parse::<f32>()) { s.opacity = (v / 100.0).clamp(0.0, 1.0); } }); } />
                                                                </label>
                                                            })}
                                                        </details>
                                                    })}
                                                    {(is_button && !simple).then(|| view! {
                                                        <details class="prop-section" open>
                                                            <summary>"پیوند"</summary>
                                                            <label class="full">
                                                                <select on:change=move |ev| {
                                                                    let v = event_target_value(&ev);
                                                                    edit_selected("linkpage", &|n: &mut Node| {
                                                                        if v.is_empty() {
                                                                            n.props.remove("link_page");
                                                                        } else {
                                                                            n.props.insert("link_page".into(), v.clone());
                                                                        }
                                                                    });
                                                                }>
                                                                    <option value="" selected=link_value.is_empty()>"بدون پیوند"</option>
                                                                    {bundle.with(|b| b.project.pages.iter().map(|pg| {
                                                                        let r = pg.route.clone();
                                                                        let sel = link_value == r;
                                                                        view! { <option value=r.clone() selected=sel>{format!("{} ({})", pg.name, r)}</option> }
                                                                    }).collect_view())}
                                                                </select>
                                                            </label>
                                                            <p class="hint">"در پیش‌نمایش، کلیک روی این دکمه بازدیدکننده را به صفحه انتخاب شده می‌برد."</p>
                                                        </details>
                                                    })}
                                                    {is_image.then(|| view! {
                                                        <details class="prop-section" open>
                                                            <summary>"عکس"</summary>
                                                            {src_short.clone().map(|s| view! { <p class="src-line">{s}</p> })}
                                                            <button class="wide-action" on:click=move |_| trigger_upload(Some(selected_id.get_untracked()))>"تعویض عکس"</button>
                                                            <p class="hint">"عکس آپلودی فقط در نسخه ذخیره شده همین مرورگر می‌ماند و وارد فایل پروژه صادرشده نمی‌شود."</p>
                                                        </details>
                                                    })}
                                                    <p class="node-id">{format!("شناسه: {}", node.id)}</p>
                                                </div>
                                            }.into_any()
                                        }
                                    }}
                                </div>
                            }.into_any(),
                        }
                    }}
                    </div>
                </aside>
            </div>

            <footer class="statusbar">
                <span>{move || format!("{} عنصر در این صفحه", flat_all().len())}</span>
                <span class="status-sel">{move || {
                    let id = selected_id.get();
                    let count = sel_set.get().len();
                    if id.is_empty() {
                        "بدون انتخاب".to_string()
                    } else if count > 1 {
                        format!("{count} عنصر انتخاب شده")
                    } else {
                        bundle.with(|b| {
                            b.project
                                .pages
                                .get(page_index.get())
                                .and_then(|pg| pg.root.find(&id))
                                .map(|n| format!("{} / {}", n.id, kind_fa(n.kind)))
                                .unwrap_or_default()
                        })
                    }
                }}</span>
                <span>{move || format!("بزرگ نمایی ٪{}", (scale() * 100.0).round() as i32)}</span>
                <span class="status-save">{move || saved_stamp.get()}</span>
                {move || {
                    let note = render_note.get();
                    (!note.is_empty()).then(|| view! { <span class="status-note">{note}</span> })
                }}
                <button class="reset" on:click=reset_all>"بازنشانی"</button>
            </footer>
            <input
                type="file"
                accept="image/*"
                class="hidden-file"
                node_ref=file_ref
                on:change=on_file_change
            />
            {move || preview.get().then(|| {
                let (nodes, pname, pw, ph, pbg) = bundle.with(|b| {
                    b.project
                        .pages
                        .get(preview_page.get())
                        .map(|pg| {
                            (
                                page_flat(pg, false),
                                pg.name.clone(),
                                pg.width,
                                pg.height,
                                pg.background.map(|c| c.to_hex()).unwrap_or_else(|| "#ffffff".to_string()),
                            )
                        })
                        .unwrap_or_default()
                });
                let pv = ((win_width.get() - 48.0) / pw.max(1.0))
                    .min((win_height.get() - 150.0) / ph.max(1.0))
                    .clamp(0.05, 1.5);
                view! {
                    <div class="preview-overlay">
                        <div class="preview-head">
                            <strong>{format!("پیش‌نمایش: {pname}")}</strong>
                            <span class="hint">"دکمه‌های پیونددار در پیش‌نمایش کار می‌کنند؛ برای خروج Esc یا بستن"</span>
                            <button class="preview-close" on:click=move |_| preview.set(false)>"بستن ✕"</button>
                        </div>
                        <div class="preview-stage">
                            <div class="preview-scaler" style=format!("width:{}px;height:{}px;", pw * pv, ph * pv)>
                                <div
                                    class="preview-board"
                                    style=format!("width:{pw}px;height:{ph}px;background:{pbg};transform:scale({pv});")
                                >
                                    {nodes.into_iter().map(|n| {
                                        let target = bundle.with_untracked(|b| preview_target(b, &n));
                                        let mut style = node_css(&n);
                                        if target.is_some() {
                                            style.push_str("cursor:pointer;");
                                        }
                                        let label = if n.src.is_some() {
                                            String::new()
                                        } else if n.kind == NodeKind::Image {
                                            "🖼".to_string()
                                        } else {
                                            n.text.clone().unwrap_or_default()
                                        };
                                        view! {
                                            <div
                                                class="node"
                                                dir="auto"
                                                data-kind=kind_name(n.kind)
                                                data-node-id=n.id.clone()
                                                style=style
                                                on:click=move |_| {
                                                    if let Some(route) = &target {
                                                        if let Some(idx) = bundle.with_untracked(|b| {
                                                            b.project.pages.iter().position(|pg| &pg.route == route)
                                                        }) {
                                                            preview_page.set(idx);
                                                        }
                                                    }
                                                }
                                            >{label}</div>
                                        }
                                    }).collect_view()}
                                </div>
                            </div>
                        </div>
                    </div>
                }
            })}
            {move || ctx_menu.get().map(|(x, y, id)| {
                let id_dup = id.clone();
                let id_del = id.clone();
                let id_fwd = id.clone();
                let id_back = id.clone();
                view! {
                    <div
                        class="ctx-menu"
                        style=format!("left:{x}px;top:{y}px;")
                        on:mousedown=|ev: leptos::ev::MouseEvent| ev.stop_propagation()
                    >
                        <button on:click=move |_| { select_only(id_dup.clone()); duplicate_selection(); }>"تکثیر"</button>
                        <button on:click=move |_| { select_only(id_fwd.clone()); reorder_selection(id_fwd.clone(), true); }>"بیار جلو"</button>
                        <button on:click=move |_| { select_only(id_back.clone()); reorder_selection(id_back.clone(), false); }>"ببر عقب"</button>
                        <button class="danger" on:click=move |_| { select_only(id_del.clone()); delete_selection(); }>"حذف"</button>
                    </div>
                }
            })}
        </main>
    }
}

fn main() {
    console_error_panic_hook::set_once();
    leptos::mount::mount_to_body(App);
}
