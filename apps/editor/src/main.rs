//! Chida editor, web edition: a professional design tool UI in the
//! spirit of Figma and FigCraft, Persian first, with Chida's three
//! level simplicity layer (ساده، سازنده، حرفه‌ای) on top.
//!
//! Layout: dark top bar (project, undo and redo, levels, zoom,
//! actions), a Figma style tool rail, a pages and layers panel, a
//! pannable and zoomable canvas with a white artboard, selection
//! handles with corner resize, and a tabbed right panel (Design,
//! Logic, Data, Code). Edits persist to localStorage and restore on
//! reload. The editor opens on a start screen: a blank project, a
//! project file from disk, or one of the bundled template projects
//! in `fixtures/tpl-*.json`, each generated from the real Chida CLI
//! templates (project, database and flows), see
//! `generate-fixture.sh`. This same UI is the desktop editor UI.

use chida_core::{
    AxisSizing, Color, ComponentDef, Gradient, GradientStop, LayoutAlign, LayoutDirection,
    LayoutJustify, Node, NodeKind, Page, PathPoint, Project, Shadow,
};
use chida_db::Database;
use chida_logic::{Action, EventKind, Flow, Step};
use leptos::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use wasm_bindgen::JsCast;

const STORAGE_KEY: &str = "chida-editor-bundle-v3";

/// The bundled starter templates: key, Persian display name and the
/// thumbnail render (the CLI's own render of the template home page).
const TEMPLATES: [(&str, &str, &str); 8] = [
    ("shop", "فروشگاه ساده", "assets/thumbs/shop.jpg"),
    ("digikala", "فروشگاه بزرگ", "assets/thumbs/digikala.jpg"),
    ("persian", "صفحه فارسی", "assets/thumbs/persian.jpg"),
    ("liquid-glass", "شیشه‌ای", "assets/thumbs/liquid-glass.jpg"),
    ("saas-dashboard", "داشبورد مدیریتی", "assets/thumbs/saas-dashboard.jpg"),
    ("clinic", "درمانگاه", "assets/thumbs/clinic.jpg"),
    ("restaurant", "رستوران", "assets/thumbs/restaurant.jpg"),
    ("portfolio", "نمونه‌کار", "assets/thumbs/portfolio.jpg"),
];

fn template_json(key: &str) -> Option<&'static str> {
    Some(match key {
        "shop" => include_str!("../fixtures/tpl-shop.json"),
        "digikala" => include_str!("../fixtures/tpl-digikala.json"),
        "persian" => include_str!("../fixtures/tpl-persian.json"),
        "liquid-glass" => include_str!("../fixtures/tpl-liquid-glass.json"),
        "saas-dashboard" => include_str!("../fixtures/tpl-saas-dashboard.json"),
        "clinic" => include_str!("../fixtures/tpl-clinic.json"),
        "restaurant" => include_str!("../fixtures/tpl-restaurant.json"),
        "portfolio" => include_str!("../fixtures/tpl-portfolio.json"),
        _ => return None,
    })
}

fn template_bundle(key: &str) -> Option<Bundle> {
    serde_json::from_str(template_json(key)?).ok()
}

/// A from-zero project: one page holding one empty desktop frame.
fn blank_bundle() -> Bundle {
    let mut project = Project::new("my-site", "پروژه تازه");
    let mut page = Page::new("home", "صفحه اصلی", "/", 1440.0, 900.0);
    let mut frame = Node::new("desktop-frame", "فریم دسکتاپ", NodeKind::Frame, 0.0, 0.0, 1440.0, 900.0);
    frame.fill = Some(Color::WHITE);
    // The backdrop frame starts locked: it fills the whole page, and a
    // locked node lets canvas drags fall through to marquee selection,
    // so the user can still rubber-band their own elements over it.
    // It can be unlocked from the layers panel.
    frame.props.insert("locked".into(), "1".into());
    page.root.children.push(frame);
    project.pages.push(page);
    Bundle {
        project,
        database: Database::default(),
        flows: Vec::new(),
        styles: Vec::new(),
    }
}

/// Ids of nodes that carry a `locked = 1` prop (the blank project's
/// backdrop frame): they start in the editor's locked set.
fn seed_locked(project: &Project) -> BTreeSet<String> {
    fn walk(node: &Node, out: &mut BTreeSet<String>) {
        if node.props.get("locked").map(|v| v == "1").unwrap_or(false) {
            out.insert(node.id.clone());
        }
        for child in &node.children {
            walk(child, out);
        }
    }
    let mut out = BTreeSet::new();
    for page in &project.pages {
        walk(&page.root, &mut out);
    }
    out
}

/// Parse a project file picked by the user: the editor bundle shape
/// (which is also the CLI `.chida.json` shape), or a bare design
/// project JSON as exported by the «خروجی JSON» button.
fn parse_bundle_text(text: &str) -> Option<Bundle> {
    if let Ok(b) = serde_json::from_str::<Bundle>(text) {
        if !b.project.pages.is_empty() {
            return Some(b);
        }
    }
    if let Ok(p) = Project::from_json(text) {
        if !p.pages.is_empty() {
            return Some(Bundle {
                project: p,
                database: Database::default(),
                flows: Vec::new(),
                styles: Vec::new(),
            });
        }
    }
    None
}

/// True when a previous session left autosaved work in this browser.
fn has_saved_work() -> bool {
    web_sys::window()
        .and_then(|w| w.local_storage().ok().flatten())
        .and_then(|s| s.get_item(STORAGE_KEY).ok().flatten())
        .is_some()
}

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
    blank_bundle()
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
    gradient: Option<Gradient>,
    shadows: Vec<Shadow>,
    path_points: Vec<PathPoint>,
    path_closed: bool,
    blur_radius: f32,
    background_blur: f32,
    interactions: Option<String>,
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
        NodeKind::Path => "مسیر",
        NodeKind::Arrow => "فلش",
        NodeKind::Component => "نمونه کامپوننت",
        NodeKind::Group => "گروه",
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
        NodeKind::Path => "✒",
        NodeKind::Arrow => "➜",
        NodeKind::Component => "🧩",
        NodeKind::Group => "▦",
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
        NodeKind::Path => "path",
        NodeKind::Arrow => "arrow",
        NodeKind::Component => "component",
        NodeKind::Group => "group",
    }
}

/// Flatten a page tree to absolute coordinates. Component
/// instances are resolved against the project registry before
/// they are flattened, so the canvas always paints master content.
/// When `include_hidden` is false, hidden subtrees are skipped
/// (the canvas); the layers panel passes true so hidden nodes can
/// be shown again.
fn flatten(
    node: &Node,
    project: &Project,
    ox: f32,
    oy: f32,
    depth: usize,
    include_hidden: bool,
    out: &mut Vec<FlatNode>,
) {
    if node.component.is_some()
        && node
            .component
            .as_deref()
            .map(|key| project.component_def(key).is_some())
            .unwrap_or(false)
    {
        let resolved = project.resolve_node(node);
        flatten_resolved(&resolved, ox, oy, depth, include_hidden, out);
        return;
    }
    flatten_resolved(node, ox, oy, depth, include_hidden, out);
}

fn flatten_resolved(
    node: &Node,
    ox: f32,
    oy: f32,
    depth: usize,
    include_hidden: bool,
    out: &mut Vec<FlatNode>,
) {
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
            gradient: node.fill_gradient.clone(),
            shadows: node.shadows.clone(),
            path_points: node.path_points.clone(),
            path_closed: node.path_closed,
            blur_radius: node.blur_radius,
            background_blur: node.background_blur,
            interactions: node.props.get("interactions").cloned(),
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
        flatten_resolved(child, ax, ay, depth + 1, include_hidden, out);
    }
}

fn page_flat(project: &Project, page: &Page, include_hidden: bool) -> Vec<FlatNode> {
    let mut out = Vec::new();
    for child in &page.root.children {
        flatten(child, project, 0.0, 0.0, 0, include_hidden, &mut out);
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

/// Move a node to the very front (end of its sibling list, painted
/// last) or the very back (start of the list) inside its parent.
/// Returns true when the node was found.
fn move_to_edge(node: &mut Node, id: &str, front: bool) -> bool {
    if let Some(pos) = node.children.iter().position(|c| c.id == id) {
        let child = node.children.remove(pos);
        if front {
            node.children.push(child);
        } else {
            node.children.insert(0, child);
        }
        return true;
    }
    for child in node.children.iter_mut() {
        if move_to_edge(child, id, front) {
            return true;
        }
    }
    false
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

/// When a node belongs to a group (an inserted component block
/// or a real group), selecting any part of it selects the highest
/// group below the page root: return that group's id.
fn group_root_of(root: &Node, id: &str) -> String {
    fn is_group(node: &Node) -> bool {
        node.kind == NodeKind::Group
            || node.props.contains_key("comp_group")
            || node.props.contains_key("real_group")
    }
    fn walk(node: &Node, id: &str, group: Option<String>) -> Option<String> {
        let group = if node.id != "root" && is_group(node) {
            Some(node.id.clone())
        } else {
            group
        };
        if node.id == id {
            return Some(group.unwrap_or_else(|| id.to_string()));
        }
        for child in &node.children {
            if let Some(found) = walk(child, id, group.clone()) {
                return Some(found);
            }
        }
        None
    }
    walk(root, id, None).unwrap_or_else(|| id.to_string())
}

/// The selected ids that are not inside another selected node.
fn topmost_selected(root: &Node, ids: &BTreeSet<String>) -> Vec<String> {
    fn walk(node: &Node, ids: &BTreeSet<String>, out: &mut Vec<String>) {
        for child in &node.children {
            if ids.contains(&child.id) {
                out.push(child.id.clone());
            } else {
                walk(child, ids, out);
            }
        }
    }
    let mut out = Vec::new();
    walk(root, ids, &mut out);
    out
}

/// Replace one node anywhere in the tree with another node.
fn replace_in_tree(node: &mut Node, id: &str, replacement: Node) -> bool {
    if let Some(pos) = node.children.iter().position(|child| child.id == id) {
        node.children[pos] = replacement;
        return true;
    }
    for child in node.children.iter_mut() {
        if replace_in_tree(child, id, replacement.clone()) {
            return true;
        }
    }
    false
}

/// Move a dragged layer row next to its drop target. Both nodes
/// must be direct children of the same parent.
fn move_layer_before(root: &mut Node, dragged: &str, target: &str) -> bool {
    if let Some(pos) = root.children.iter().position(|child| child.id == dragged) {
        if root.children.iter().any(|child| child.id == target) {
            let node = root.children.remove(pos);
            let target_pos = root
                .children
                .iter()
                .position(|child| child.id == target)
                .unwrap_or(root.children.len());
            root.children.insert(target_pos, node);
            return true;
        }
    }
    root.children
        .iter_mut()
        .any(|child| move_layer_before(child, dragged, target))
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

#[derive(Debug, Clone, PartialEq)]
enum PreviewAction {
    Route(String),
    Url(String),
}

/// Parse the node's interaction list. Stored lines are
/// `trigger|kind|target`, for example `click|page|/about`.
fn interaction_entries(raw: Option<&str>) -> Vec<(String, String, String)> {
    raw.unwrap_or_default()
        .lines()
        .filter_map(|line| {
            let mut parts = line.splitn(3, '|');
            Some((
                parts.next()?.to_string(),
                parts.next()?.to_string(),
                parts.next()?.to_string(),
            ))
        })
        .collect()
}

fn encode_interactions(entries: &[(String, String, String)]) -> String {
    entries
        .iter()
        .map(|(trigger, kind, target)| format!("{trigger}|{kind}|{target}"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn interaction_action(flat: &FlatNode, trigger: &str) -> Option<PreviewAction> {
    interaction_entries(flat.interactions.as_deref())
        .into_iter()
        .find(|(t, _, _)| t == trigger)
        .map(|(_, kind, target)| {
            if kind == "url" {
                PreviewAction::Url(target)
            } else {
                PreviewAction::Route(target)
            }
        })
}

/// Preview action for one trigger. Click falls back to the
/// legacy page link and click flows; hover uses interactions only.
fn preview_action(b: &Bundle, flat: &FlatNode, trigger: &str) -> Option<PreviewAction> {
    if let Some(action) = interaction_action(flat, trigger) {
        return Some(action);
    }
    if trigger != "click" {
        return None;
    }
    if let Some(link) = &flat.link {
        if !link.is_empty() {
            return Some(PreviewAction::Route(link.clone()));
        }
    }
    b.flows
        .iter()
        .filter(|f| {
            matches!(f.trigger.kind, EventKind::Click)
                && f.trigger.source_node.as_deref() == Some(flat.id.as_str())
        })
        .find_map(|f| find_navigate(&f.steps))
        .map(PreviewAction::Route)
}

/// Backwards compatible route-only click target.
fn preview_target(b: &Bundle, flat: &FlatNode) -> Option<String> {
    match preview_action(b, flat, "click") {
        Some(PreviewAction::Route(route)) => Some(route),
        _ => None,
    }
}

/// Inline SVG markup painted inside a path or arrow node's box.
fn node_svg_html(n: &FlatNode) -> Option<String> {
    if !matches!(n.kind, NodeKind::Path | NodeKind::Arrow) {
        return None;
    }
    let mut temp = Node::new("tmp", "tmp", n.kind, 0.0, 0.0, n.width, n.height);
    temp.path_points = n.path_points.clone();
    temp.path_closed = n.path_closed;
    temp.stroke_width = n.stroke_width;
    let fill = n.fill.clone().unwrap_or_else(|| "none".into());
    let stroke = n.stroke.clone().unwrap_or_else(|| fill.clone());
    if n.kind == NodeKind::Arrow {
        let head = temp.arrow_head_points()?;
        let a = temp.path_points.first()?;
        let b = temp.path_points.get(1)?;
        return Some(format!(
            "<svg viewBox=\"0 0 {} {}\" width=\"100%\" height=\"100%\" preserveAspectRatio=\"none\"><path d=\"M {} {} L {} {}\" fill=\"none\" stroke=\"{}\" stroke-width=\"{}\"/><path d=\"M {} {} L {} {} L {} {} Z\" fill=\"{}\"/></svg>",
            n.width,
            n.height,
            a.x,
            a.y,
            b.x,
            b.y,
            stroke,
            n.stroke_width.max(1.5),
            head[0].0,
            head[0].1,
            head[1].0,
            head[1].1,
            head[2].0,
            head[2].1,
            stroke
        ));
    }
    Some(format!(
        "<svg viewBox=\"0 0 {} {}\" width=\"100%\" height=\"100%\" preserveAspectRatio=\"none\"><path d=\"{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"{}\"/></svg>",
        n.width,
        n.height,
        temp.path_d(0.0, 0.0),
        fill,
        stroke,
        n.stroke_width
    ))
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
    if matches!(n.kind, NodeKind::Path | NodeKind::Arrow) {
        style.push_str("background:transparent;");
    } else if let Some(g) = &n.gradient {
        style.push_str(&format!("background:{};", g.css()));
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
        if !matches!(n.kind, NodeKind::Path | NodeKind::Arrow) {
            style.push_str(&format!("border:{}px solid {};", n.stroke_width, st));
        }
    }
    let shadows: Vec<String> = n
        .shadows
        .iter()
        .filter(|sh| sh.enabled)
        .map(|sh| {
            format!(
                "{}px {}px {}px {}",
                sh.x,
                sh.y,
                sh.blur,
                hex_rgba(&sh.color.to_hex(), sh.opacity)
            )
        })
        .collect();
    if !shadows.is_empty() {
        style.push_str(&format!("box-shadow:{};", shadows.join(",")));
    }
    if n.blur_radius > 0.0 {
        style.push_str(&format!("filter:blur({}px);", n.blur_radius));
    }
    if n.background_blur > 0.0 {
        style.push_str(&format!(
            "backdrop-filter:blur({}px);-webkit-backdrop-filter:blur({}px);",
            n.background_blur, n.background_blur
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
            bar.shadows = vec![Shadow { x: 0.0, y: 2.0, blur: 12.0, color: Color::BLACK, opacity: 0.10 , enabled: true }];
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
            card.shadows = vec![Shadow::soft()];
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
            b.shadows = vec![Shadow { x: 0.0, y: 6.0, blur: 16.0, color: primary, opacity: 0.35 , enabled: true }];
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
            card.shadows = vec![Shadow { x: 0.0, y: 4.0, blur: 18.0, color: Color::BLACK, opacity: 0.12 , enabled: true }];
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
            card.shadows = vec![Shadow::soft()];
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
            t.shadows = vec![Shadow { x: 0.0, y: 4.0, blur: 18.0, color: Color::BLACK, opacity: 0.10 , enabled: true }];
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
        "navbar-search" => {
            let mut g = group("نوبار با جستجو", 1200.0, 72.0);
            let mut bar = comp_box(id_gen, NodeKind::Frame, "نوبار", 0.0, 0.0, 1200.0, 72.0, surface);
            bar.shadows = vec![Shadow::soft()];
            bar.children.push(comp_text(id_gen, "لوگو", 1110.0, 20.0, 74.0, 32.0, "بازارینو", 20.0, primary, 700, "right"));
            let mut search = comp_box(id_gen, NodeKind::Input, "جستجو", 430.0, 14.0, 420.0, 44.0, soft);
            search.corner_radius = 10.0;
            search.children.push(comp_text(id_gen, "راهنمای جستجو", 12.0, 11.0, 396.0, 22.0, "جستجو در بازارینو", 13.0, muted, 400, "right"));
            bar.children.push(search);
            for (i, label) in ["خانه", "فروشگاه", "پشتیبانی"].iter().enumerate() {
                let x = 300.0 - i as f32 * 96.0;
                bar.children.push(comp_text(id_gen, "پیوند", x, 26.0, 88.0, 20.0, label, 14.0, ink, 500, "center"));
            }
            g.children.push(bar);
            g
        }
        "hero-center" => {
            let mut g = group("هیروی وسط چین", 1200.0, 380.0);
            let mut bg = comp_box(id_gen, NodeKind::Frame, "پس زمینه", 0.0, 0.0, 1200.0, 380.0, soft);
            bg.corner_radius = 18.0;
            bg.children.push(comp_text(id_gen, "تیتر", 200.0, 70.0, 800.0, 52.0, "عنوان اصلی صفحه را اینجا بنویسید", 32.0, ink, 700, "center"));
            bg.children.push(comp_text(id_gen, "زیرتیتر", 260.0, 140.0, 680.0, 56.0, "در یک یا دو جمله بگویید این صفحه برای چه کسی است و چه کاری برای او می کند.", 16.0, muted, 400, "center"));
            let mut cta = comp_box(id_gen, NodeKind::Button, "دکمه", 510.0, 230.0, 180.0, 50.0, primary);
            cta.corner_radius = 10.0;
            cta.children.push(comp_text(id_gen, "متن دکمه", 0.0, 13.0, 180.0, 24.0, "شروع کنید", 15.0, Color::WHITE, 700, "center"));
            bg.children.push(cta);
            g.children.push(bg);
            g
        }
        "hero-app" => {
            let mut g = group("هیروی اپ", 1200.0, 440.0);
            let mut bg = comp_box(id_gen, NodeKind::Frame, "پس زمینه", 0.0, 0.0, 1200.0, 440.0, surface);
            bg.corner_radius = 18.0;
            bg.children.push(comp_text(id_gen, "تیتر", 620.0, 80.0, 500.0, 88.0, "اپ خود را ساده و روشن معرفی کنید", 30.0, ink, 700, "right"));
            bg.children.push(comp_text(id_gen, "متن", 620.0, 190.0, 500.0, 72.0, "مهم ترین کاری که کاربر با اپ شما انجام می دهد را کوتاه و بی حاشیه بنویسید.", 15.0, muted, 400, "right"));
            let mut cta = comp_box(id_gen, NodeKind::Button, "دکمه دانلود", 920.0, 290.0, 200.0, 50.0, primary);
            cta.corner_radius = 10.0;
            cta.children.push(comp_text(id_gen, "متن دکمه", 0.0, 13.0, 200.0, 24.0, "دریافت اپ", 15.0, Color::WHITE, 700, "center"));
            bg.children.push(cta);
            let mut phone = comp_box(id_gen, NodeKind::Frame, "قاب گوشی", 120.0, 40.0, 300.0, 360.0, ink);
            phone.corner_radius = 32.0;
            let mut screen = comp_box(id_gen, NodeKind::Image, "صفحه اپ", 136.0, 56.0, 268.0, 328.0, soft);
            screen.corner_radius = 24.0;
            screen.props.insert("src".into(), "assets/digikala/phone.jpg".into());
            bg.children.push(phone);
            bg.children.push(screen);
            g.children.push(bg);
            g
        }
        "features-four" => {
            let mut g = group("چهار ویژگی", 1200.0, 250.0);
            g.children.push(comp_text(id_gen, "تیتر بخش", 0.0, 0.0, 1200.0, 36.0, "ویژگی های اصلی", 22.0, ink, 700, "center"));
            for i in 0..4 {
                let x = 892.0 - i as f32 * 292.0;
                let mut card = comp_box(id_gen, NodeKind::Card, "کارت ویژگی", x, 58.0, 276.0, 170.0, surface);
                card.corner_radius = 14.0;
                card.shadows = vec![Shadow::soft()];
                card.children.push(comp_text(id_gen, "عنوان ویژگی", 18.0, 22.0, 240.0, 26.0, "عنوان ویژگی", 16.0, ink, 700, "right"));
                card.children.push(comp_text(id_gen, "متن ویژگی", 18.0, 60.0, 240.0, 72.0, "توضیح کوتاه این ویژگی را اینجا بنویسید.", 13.5, muted, 400, "right"));
                g.children.push(card);
            }
            g
        }
        "steps" => {
            let mut g = group("سه گام", 1200.0, 250.0);
            g.children.push(comp_text(id_gen, "تیتر بخش", 0.0, 0.0, 1200.0, 36.0, "در سه گام ساده", 22.0, ink, 700, "center"));
            for i in 0..3 {
                let x = 800.0 - i as f32 * 400.0;
                let mut dot = comp_box(id_gen, NodeKind::Ellipse, "شماره گام", x + 164.0, 62.0, 56.0, 56.0, primary);
                dot.children.push(comp_text(id_gen, "شماره", 0.0, 14.0, 56.0, 28.0, &format!("{}", i + 1), 20.0, Color::WHITE, 700, "center"));
                g.children.push(dot);
                g.children.push(comp_text(id_gen, "عنوان گام", x + 20.0, 136.0, 344.0, 26.0, "عنوان این گام", 16.0, ink, 700, "center"));
                g.children.push(comp_text(id_gen, "متن گام", x + 20.0, 170.0, 344.0, 48.0, "کاری که در این گام انجام می شود.", 13.5, muted, 400, "center"));
            }
            g
        }
        "pricing-three" => {
            let mut g = group("قیمت گذاری سه طرح", 1200.0, 390.0);
            g.children.push(comp_text(id_gen, "تیتر بخش", 0.0, 0.0, 1200.0, 36.0, "طرح ها را مقایسه کنید", 22.0, ink, 700, "center"));
            for i in 0..3 {
                let x = 800.0 - i as f32 * 400.0;
                let mut card = comp_box(id_gen, NodeKind::Card, "کارت طرح", x, 56.0, 360.0, 320.0, surface);
                card.corner_radius = 16.0;
                card.shadows = vec![Shadow::soft()];
                if i == 1 { card.stroke = Some(primary); card.stroke_width = 2.0; }
                card.children.push(comp_text(id_gen, "نام طرح", 24.0, 24.0, 312.0, 28.0, "نام طرح", 18.0, ink, 700, "center"));
                card.children.push(comp_text(id_gen, "قیمت", 24.0, 66.0, 312.0, 36.0, "قیمت طرح", 24.0, primary, 700, "center"));
                for j in 0..3 {
                    card.children.push(comp_text(id_gen, "امکان", 24.0, 126.0 + j as f32 * 34.0, 312.0, 22.0, "یکی از امکان های این طرح", 13.5, ink, 400, "right"));
                }
                let mut cta = comp_box(id_gen, NodeKind::Button, "دکمه انتخاب", 24.0, 248.0, 312.0, 46.0, primary);
                cta.corner_radius = 10.0;
                cta.children.push(comp_text(id_gen, "متن دکمه", 0.0, 11.0, 312.0, 24.0, "انتخاب این طرح", 14.0, Color::WHITE, 700, "center"));
                card.children.push(cta);
                g.children.push(card);
            }
            g
        }
        "testimonials" => {
            let mut g = group("نظرات مشتریان", 1200.0, 300.0);
            g.children.push(comp_text(id_gen, "تیتر بخش", 0.0, 0.0, 1200.0, 36.0, "مشتریان چه می گویند", 22.0, ink, 700, "center"));
            for i in 0..3 {
                let x = 800.0 - i as f32 * 400.0;
                let mut card = comp_box(id_gen, NodeKind::Card, "کارت نظر", x, 58.0, 360.0, 210.0, surface);
                card.corner_radius = 14.0;
                card.shadows = vec![Shadow::soft()];
                card.children.push(comp_text(id_gen, "متن نظر", 22.0, 24.0, 316.0, 80.0, "نظر واقعی مشتری را اینجا بنویسید.", 14.0, ink, 400, "right"));
                card.children.push(comp_text(id_gen, "نام مشتری", 22.0, 132.0, 316.0, 24.0, "نام مشتری", 14.0, ink, 700, "right"));
                card.children.push(comp_text(id_gen, "عنوان مشتری", 22.0, 160.0, 316.0, 20.0, "عنوان یا شهر مشتری", 12.0, muted, 400, "right"));
                g.children.push(card);
            }
            g
        }
        "gallery" => {
            let mut g = group("گالری عکس", 1200.0, 520.0);
            g.children.push(comp_text(id_gen, "تیتر بخش", 0.0, 0.0, 1200.0, 36.0, "گالری", 22.0, ink, 700, "center"));
            let photos = ["hero.jpg", "laptop.jpg", "watch.jpg", "headphones.jpg", "fashion.jpg", "coffee.jpg"];
            for (i, photo) in photos.iter().enumerate() {
                let col = (i % 3) as f32;
                let row = (i / 3) as f32;
                let mut img = comp_box(id_gen, NodeKind::Image, "عکس گالری", 800.0 - col * 400.0, 58.0 + row * 224.0, 384.0, 208.0, soft);
                img.corner_radius = 14.0;
                img.props.insert("src".into(), format!("assets/digikala/{photo}"));
                g.children.push(img);
            }
            g
        }
        "contact-form" => {
            let mut g = group("فرم تماس", 560.0, 470.0);
            let mut card = comp_box(id_gen, NodeKind::Card, "فرم تماس", 0.0, 0.0, 560.0, 470.0, surface);
            card.corner_radius = 16.0;
            card.shadows = vec![Shadow::soft()];
            card.children.push(comp_text(id_gen, "تیتر فرم", 28.0, 24.0, 504.0, 30.0, "با ما در تماس باشید", 20.0, ink, 700, "right"));
            for (i, label) in ["نام شما", "شماره تماس", "متن پیام"].iter().enumerate() {
                let y = 76.0 + i as f32 * 92.0;
                card.children.push(comp_text(id_gen, "برچسب", 28.0, y, 504.0, 20.0, label, 13.0, ink, 500, "right"));
                let mut field = comp_box(id_gen, NodeKind::Input, "ورودی", 28.0, y + 26.0, 504.0, 48.0, surface);
                field.corner_radius = 9.0;
                field.stroke = Some(Color::from_hex("#d5d8db").unwrap());
                field.stroke_width = 1.5;
                card.children.push(field);
            }
            let mut cta = comp_box(id_gen, NodeKind::Button, "دکمه ارسال", 28.0, 386.0, 504.0, 50.0, primary);
            cta.corner_radius = 10.0;
            cta.children.push(comp_text(id_gen, "متن دکمه", 0.0, 13.0, 504.0, 24.0, "فرستادن پیام", 15.0, Color::WHITE, 700, "center"));
            card.children.push(cta);
            g.children.push(card);
            g
        }
        "newsletter" => {
            let mut g = group("خبرنامه", 1200.0, 180.0);
            let mut bg = comp_box(id_gen, NodeKind::Frame, "پس زمینه", 0.0, 0.0, 1200.0, 180.0, primary);
            bg.corner_radius = 16.0;
            bg.children.push(comp_text(id_gen, "تیتر", 620.0, 42.0, 500.0, 32.0, "از تازه ها جا نمانید", 22.0, Color::WHITE, 700, "right"));
            bg.children.push(comp_text(id_gen, "متن", 620.0, 86.0, 500.0, 44.0, "ایمیل خود را بنویسید تا خبرهای تازه را برایتان بفرستیم.", 14.0, Color::WHITE, 400, "right"));
            let mut field = comp_box(id_gen, NodeKind::Input, "ایمیل", 250.0, 58.0, 330.0, 52.0, surface);
            field.corner_radius = 10.0;
            field.children.push(comp_text(id_gen, "راهنما", 12.0, 14.0, 306.0, 24.0, "ایمیل شما", 13.0, muted, 400, "right"));
            bg.children.push(field);
            let mut cta = comp_box(id_gen, NodeKind::Button, "دکمه عضویت", 70.0, 58.0, 160.0, 52.0, ink);
            cta.corner_radius = 10.0;
            cta.children.push(comp_text(id_gen, "متن دکمه", 0.0, 14.0, 160.0, 24.0, "عضو می شوم", 14.0, Color::WHITE, 700, "center"));
            bg.children.push(cta);
            g.children.push(bg);
            g
        }
        "footer-simple" => {
            let mut g = group("فوتر ساده", 1200.0, 110.0);
            let mut bg = comp_box(id_gen, NodeKind::Frame, "فوتر", 0.0, 0.0, 1200.0, 110.0, ink);
            bg.children.push(comp_text(id_gen, "لوگو", 1080.0, 38.0, 90.0, 30.0, "بازارینو", 18.0, Color::WHITE, 700, "right"));
            for (i, label) in ["خانه", "فروشگاه", "درباره ما", "تماس"].iter().enumerate() {
                bg.children.push(comp_text(id_gen, "پیوند", 760.0 - i as f32 * 110.0, 42.0, 100.0, 22.0, label, 13.0, Color::WHITE, 400, "center"));
            }
            bg.children.push(comp_text(id_gen, "متن پایانی", 30.0, 42.0, 260.0, 22.0, "متن پایانی فوتر", 12.0, Color::WHITE, 400, "left"));
            g.children.push(bg);
            g
        }
        "footer-cols" => {
            let mut g = group("فوتر چند ستونه", 1200.0, 300.0);
            let mut bg = comp_box(id_gen, NodeKind::Frame, "فوتر", 0.0, 0.0, 1200.0, 300.0, ink);
            bg.children.push(comp_text(id_gen, "لوگو", 1010.0, 36.0, 150.0, 32.0, "بازارینو", 20.0, Color::WHITE, 700, "right"));
            bg.children.push(comp_text(id_gen, "معرفی", 880.0, 84.0, 280.0, 66.0, "معرفی کوتاه کسب و کار را اینجا بنویسید.", 13.0, Color::WHITE, 400, "right"));
            for col in 0..3 {
                let x = 620.0 - col as f32 * 260.0;
                bg.children.push(comp_text(id_gen, "عنوان ستون", x, 36.0, 200.0, 24.0, "عنوان ستون", 14.0, Color::WHITE, 700, "right"));
                for row in 0..4 {
                    bg.children.push(comp_text(id_gen, "پیوند", x, 76.0 + row as f32 * 34.0, 200.0, 20.0, "پیوند فوتر", 12.5, Color::WHITE, 400, "right"));
                }
            }
            g.children.push(bg);
            g
        }
        "cta-banner" => {
            let mut g = group("فراخوان پهن", 1200.0, 190.0);
            let mut bg = comp_box(id_gen, NodeKind::Frame, "پس زمینه", 0.0, 0.0, 1200.0, 190.0, primary);
            bg.corner_radius = 16.0;
            bg.fill_gradient = Some(Gradient::new(primary, shade(primary, 1.35), 90.0));
            bg.children.push(comp_text(id_gen, "تیتر", 560.0, 52.0, 560.0, 38.0, "همین امروز شروع کنید", 26.0, Color::WHITE, 700, "right"));
            bg.children.push(comp_text(id_gen, "متن", 560.0, 100.0, 560.0, 26.0, "یک جمله کوتاه برای کامل کردن فراخوان.", 14.0, Color::WHITE, 400, "right"));
            let mut cta = comp_box(id_gen, NodeKind::Button, "دکمه", 70.0, 64.0, 190.0, 54.0, surface);
            cta.corner_radius = 10.0;
            cta.children.push(comp_text(id_gen, "متن دکمه", 0.0, 15.0, 190.0, 24.0, "شروع می کنم", 15.0, ink, 700, "center"));
            bg.children.push(cta);
            g.children.push(bg);
            g
        }
        "cta-card" => {
            let mut g = group("کارت فراخوان", 640.0, 300.0);
            let mut card = comp_box(id_gen, NodeKind::Card, "کارت فراخوان", 0.0, 0.0, 640.0, 300.0, surface);
            card.corner_radius = 18.0;
            card.shadows = vec![Shadow::soft()];
            card.children.push(comp_text(id_gen, "تیتر", 50.0, 48.0, 540.0, 38.0, "برای قدم بعدی آماده اید؟", 24.0, ink, 700, "center"));
            card.children.push(comp_text(id_gen, "متن", 70.0, 104.0, 500.0, 52.0, "در یک جمله بگویید با زدن این دکمه چه اتفاقی می افتد.", 14.0, muted, 400, "center"));
            let mut cta = comp_box(id_gen, NodeKind::Button, "دکمه", 220.0, 190.0, 200.0, 50.0, primary);
            cta.corner_radius = 10.0;
            cta.children.push(comp_text(id_gen, "متن دکمه", 0.0, 13.0, 200.0, 24.0, "بزن بریم", 15.0, Color::WHITE, 700, "center"));
            card.children.push(cta);
            g.children.push(card);
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
                card.shadows = vec![Shadow { x: 0.0, y: 4.0, blur: 18.0, color: Color::BLACK, opacity: 0.10 , enabled: true }];
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

/// Download raw bytes (the WordPress theme zip) like download_text.
fn download_bytes(filename: &str, bytes: &[u8]) {
    let Some(window) = web_sys::window() else { return };
    let Some(document) = window.document() else { return };
    let array = js_sys::Uint8Array::from(bytes);
    let parts = js_sys::Array::new();
    parts.push(&array);
    let Ok(blob) = web_sys::Blob::new_with_u8_array_sequence(&parts) else {
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
fn page_svg(project: &Project, page: &Page) -> String {
    let mut s = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\">",
        page.width, page.height, page.width, page.height
    );
    let bg = page
        .background
        .map(|c| c.to_hex())
        .unwrap_or_else(|| "#ffffff".to_string());
    s.push_str(&format!(
        "<rect width=\"{}\" height=\"{}\" fill=\"{}\"/>",
        page.width, page.height, bg
    ));
    for n in page_flat(project, page, false) {
        let filters: Vec<String> = n
            .shadows
            .iter()
            .filter(|sh| sh.enabled)
            .map(|sh| {
                format!(
                    "drop-shadow({}px {}px {}px {})",
                    sh.x,
                    sh.y,
                    (sh.blur / 2.0).max(0.0),
                    hex_rgba(&sh.color.to_hex(), sh.opacity)
                )
            })
            .collect();
        let shadow_style = if filters.is_empty() {
            String::new()
        } else {
            format!(" style=\"filter:{}\"", filters.join(" "))
        };
        let fill = if let Some(g) = &n.gradient {
            let gid: String = n
                .id
                .chars()
                .map(|c| if c.is_alphanumeric() { c } else { '-' })
                .collect();
            let (dx, dy) = g.direction();
            let stops = g
                .stops()
                .iter()
                .map(|stop| {
                    format!(
                        "<stop offset=\"{}\" stop-color=\"{}\"/>",
                        stop.offset,
                        stop.color.to_hex()
                    )
                })
                .collect::<Vec<_>>()
                .join("");
            s.push_str(&format!(
                "<defs><linearGradient id=\"g{gid}\" x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\">{stops}</linearGradient></defs>",
                0.5 - dx / 2.0,
                0.5 - dy / 2.0,
                0.5 + dx / 2.0,
                0.5 + dy / 2.0
            ));
            format!("url(#g{gid})")
        } else {
            n.fill.clone().unwrap_or_else(|| "none".to_string())
        };
        if matches!(n.kind, NodeKind::Path | NodeKind::Arrow) {
            let mut temp = Node::new("tmp", "tmp", n.kind, 0.0, 0.0, n.width, n.height);
            temp.path_points = n.path_points.clone();
            temp.path_closed = n.path_closed;
            temp.stroke_width = n.stroke_width;
            if n.kind == NodeKind::Arrow {
                if let Some(head) = temp.arrow_head_points() {
                    let a = temp.path_points[0];
                    let b = temp.path_points[1];
                    s.push_str(&format!(
                        "<path d=\"M {} {} L {} {}\" fill=\"none\" stroke=\"{}\" stroke-width=\"{}\"{shadow_style}/><path d=\"M {} {} L {} {} L {} {} Z\" fill=\"{}\"{shadow_style}/>",
                        n.ax + a.x,
                        n.ay + a.y,
                        n.ax + b.x,
                        n.ay + b.y,
                        n.stroke.clone().unwrap_or_else(|| fill.clone()),
                        n.stroke_width.max(1.5),
                        n.ax + head[0].0,
                        n.ay + head[0].1,
                        n.ax + head[1].0,
                        n.ay + head[1].1,
                        n.ax + head[2].0,
                        n.ay + head[2].1,
                        n.stroke.clone().unwrap_or_else(|| fill.clone())
                    ));
                }
            } else {
                s.push_str(&format!(
                    "<path d=\"{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"{}\"{shadow_style}/>",
                    temp.path_d(n.ax, n.ay),
                    fill,
                    n.stroke.clone().unwrap_or_default(),
                    n.stroke_width
                ));
            }
        } else if n.kind == NodeKind::Ellipse {
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
    Line,
    Text,
    Image,
    Button,
    Card,
    Input,
    Pen,
    Polygon,
    Star,
    Arrow,
}

impl Tool {
    fn shape_kind(self) -> Option<NodeKind> {
        match self {
            Tool::Frame => Some(NodeKind::Frame),
            Tool::Rect => Some(NodeKind::Rectangle),
            Tool::Ellipse => Some(NodeKind::Ellipse),
            Tool::Line => Some(NodeKind::Divider),
            Tool::Text => Some(NodeKind::Text),
            Tool::Image => Some(NodeKind::Image),
            Tool::Button => Some(NodeKind::Button),
            Tool::Card => Some(NodeKind::Card),
            Tool::Input => Some(NodeKind::Input),
            Tool::Polygon => Some(NodeKind::Path),
            Tool::Star => Some(NodeKind::Path),
            Tool::Arrow => Some(NodeKind::Arrow),
            _ => None,
        }
    }

    /// The lowest editor level (1 simple, 2 builder, 3 pro) that
    /// offers this tool on the rail.
    fn min_level(self) -> u8 {
        match self {
            Tool::Move | Tool::Text | Tool::Image => 1,
            Tool::Hand => 2,
            Tool::Frame | Tool::Rect | Tool::Ellipse | Tool::Line => 2,
            Tool::Button | Tool::Card | Tool::Input => 3,
            Tool::Polygon | Tool::Star | Tool::Arrow => 2,
            Tool::Pen => 3,
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
        page_guides_x: Vec<f32>,
        page_guides_y: Vec<f32>,
        page_w: f32,
        page_h: f32,
    },
    Resize {
        corner: u8,
        start_x: i32,
        start_y: i32,
        orig_x: f32,
        orig_y: f32,
        orig_w: f32,
        orig_h: f32,
        targets: Vec<(f32, f32, f32, f32)>,
        page_guides_x: Vec<f32>,
        page_guides_y: Vec<f32>,
        page_w: f32,
        page_h: f32,
        parent_ax: f32,
        parent_ay: f32,
        orig_points: Vec<PathPoint>,
    },
    CreateGuide { vertical: bool, pos: f32 },
    MoveGuide { vertical: bool, index: usize, pos: f32 },
    PathPointDrag {
        node_id: String,
        point: usize,
        handle: u8,
        start_points: Vec<PathPoint>,
    },
    Pan { start_x: i32, start_y: i32, orig_x: f32, orig_y: f32 },
    /// Marquee (rubber band) selection, in page coordinates.
    Marquee { sx: f32, sy: f32, cx: f32, cy: f32 },
    /// Drawing a new shape with a toolbar tool: start and current
    /// point in page coordinates. The node is only created on mouse
    /// up, sized by the drag (a plain click keeps the default size).
    Create { kind: NodeKind, sx: f32, sy: f32, cx: f32, cy: f32 },
    PenDrag { index: usize },
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
    let locked = RwSignal::new(bundle.with_untracked(|b| seed_locked(&b.project)));
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
    /// The start screen overlays the editor on load and from the
    /// «خانه» button; the work underneath stays untouched.
    let home_open = RwSignal::new(true);
    /// Why a picked project file was refused, shown on the start screen.
    let home_error = RwSignal::new(String::new());
    /// Hidden input behind «باز کردن فایل» on the start screen.
    let open_ref = NodeRef::<leptos::html::Input>::new();
    /// Hidden input behind «قالب وردپرس» on the start screen.
    let wp_ref = NodeRef::<leptos::html::Input>::new();
    /// Name typed for a style about to be saved.
    let style_name = RwSignal::new(String::new());
    /// Draft anchors for the pen tool, in page coordinates.
    let pen_points = RwSignal::new(Vec::<PathPoint>::new());
    let pen_current = RwSignal::new(None::<(f32, f32)>);
    /// Path currently open in point editing mode.
    let path_edit_id = RwSignal::new(String::new());
    let path_point_sel = RwSignal::new(None::<usize>);
    /// Spacing labels: (x, y, text).
    let spacing_labels = RwSignal::new(Vec::<(f32, f32, String)>::new());
    /// Selected ruler guide: (vertical?, position).
    let selected_guide = RwSignal::new(None::<(bool, f32)>);
    /// Layer row being renamed and the row being dragged.
    let rename_id = RwSignal::new(String::new());
    let layer_drag_id = RwSignal::new(String::new());
    let layer_anchor = RwSignal::new(String::new());
    /// Master editing session: (component key, return page index).
    let master_edit = RwSignal::new(None::<(String, usize)>);
    /// Gradient stop selected in the stops editor.
    let grad_stop_sel = RwSignal::new(0usize);
    /// Fields of the interaction being composed.
    let inter_trigger = RwSignal::new("click".to_string());
    let inter_kind = RwSignal::new("page".to_string());
    let inter_page = RwSignal::new(String::new());
    let inter_url = RwSignal::new(String::new());
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

    let do_undo = move || {
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
    let do_redo = move || {
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

    /// Replace everything with the given bundle and enter the
    /// editor: history, selection and view state start fresh, and
    /// the new project is autosaved like any other edit.
    let start_project = move |mut b: Bundle| {
        for page in &mut b.project.pages {
            page.root.apply_layout();
        }
        let seeds = seed_locked(&b.project);
        past.update(|p| p.clear());
        future.update(|f| f.clear());
        locked.set(seeds);
        selected_id.set(String::new());
        sel_set.set(BTreeSet::new());
        page_index.set(0);
        zoom.set(None);
        pan.set((0.0, 0.0));
        tool.set(Tool::Move);
        counter.set(100_000);
        bundle.set(b);
        persist();
        home_error.set(String::new());
        home_open.set(false);
    };

    /// Button friendly wrappers around the history moves (the
    /// keyboard handler calls `do_undo` and `do_redo` directly).
    let undo = move |_: leptos::ev::MouseEvent| do_undo();
    let redo = move |_: leptos::ev::MouseEvent| do_redo();

    /// Read the project file picked on the start screen.
    let on_open_change = move |ev: leptos::ev::Event| {
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
                    match parse_bundle_text(&text) {
                        Some(b) => start_project(b),
                        None => home_error.set(
                            "این فایل خوانده نشد؛ یک فایل پروژه چیدا با پسوند chida.json انتخاب کن."
                                .to_string(),
                        ),
                    }
                }
            }
        }) as Box<dyn FnMut()>);
        reader.set_onload(Some(onload.as_ref().unchecked_ref()));
        onload.forget();
        let _ = reader.read_as_text(&file);
        input.set_value("");
    };

    /// Read a WordPress theme zip picked on the start screen and
    /// open it as an editable project.
    let on_wp_change = move |ev: leptos::ev::Event| {
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
                let Ok(buffer) = result.dyn_into::<js_sys::ArrayBuffer>() else {
                    return;
                };
                let bytes = js_sys::Uint8Array::new(&buffer).to_vec();
                let imported = chida_wp::read_zip(&bytes)
                    .and_then(|files| chida_wp::import_theme(&files));
                match imported {
                    Ok(imported) => {
                        let summary = imported.summary.clone();
                        start_project(Bundle {
                            project: imported.project,
                            database: Database::default(),
                            flows: Vec::new(),
                            styles: Vec::new(),
                        });
                        render_note.set(format!(
                            "قالب وردپرس وارد شد: {} صفحه و {} عنصر",
                            summary.pages, summary.elements
                        ));
                    }
                    Err(_) => home_error.set(
                        "این فایل خوانده نشد؛ یک فایل zip قالب وردپرس انتخاب کن.".to_string(),
                    ),
                }
            }
        }) as Box<dyn FnMut()>);
        reader.set_onload(Some(onload.as_ref().unchecked_ref()));
        onload.forget();
        let _ = reader.read_as_array_buffer(&file);
        input.set_value("");
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
        bundle.set(blank_bundle());
        saved_stamp.set("پروژه تازه ساخته شد و ذخیره پاک شد".to_string());
    };

    let flat_canvas = move || {
        bundle.with(|b| {
            b.project
                .pages
                .get(page_index.get())
                .map(|pg| page_flat(&b.project, pg, false))
                .unwrap_or_default()
        })
    };
    let flat_all = move || {
        bundle.with(|b| {
            b.project
                .pages
                .get(page_index.get())
                .map(|pg| page_flat(&b.project, pg, true))
                .unwrap_or_default()
        })
    };
    let selected_node = move || {
        let id = selected_id.get();
        bundle.with(|b| {
            let page_node = b
                .project
                .pages
                .get(page_index.get())
                .and_then(|pg| pg.root.find(&id).cloned());
            if page_node.is_some() {
                return page_node;
            }
            if let Some((key, _)) = &master_edit.get_untracked() {
                if let Some(def) = b.project.component_def(key) {
                    if let Some(node) = def.root.find(&id) {
                        return Some(node.clone());
                    }
                }
            }
            None
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
            NodeKind::Divider => {
                let mut d = Node::new(id, "خط تازه", kind, px - 80.0, py - 1.0, 160.0, 2.0);
                d.fill = Some(ink);
                d
            }
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
            NodeKind::Path => {
                let mut path = Node::polygon(id, "چندضلعی تازه", px - 70.0, py - 70.0, 140.0, 140.0, 5, primary);
                path.props.insert("poly_sides".into(), "5".into());
                path
            }
            NodeKind::Arrow => {
                let mut arrow = Node::new(id, "فلش تازه", NodeKind::Arrow, px - 90.0, py - 20.0, 180.0, 40.0);
                arrow.path_points = vec![PathPoint::anchor(0.0, 20.0), PathPoint::anchor(180.0, 20.0)];
                arrow.stroke = Some(primary);
                arrow.stroke_width = 3.0;
                arrow
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

    /// Finish the pen draft as a real path node on the page.
    let finish_pen = move |closed: bool| {
        let pts = pen_points.get_untracked();
        if pts.len() < 2 {
            pen_points.set(Vec::new());
            return;
        }
        record("pen");
        let mut c = counter.get_untracked();
        let mut new_id = String::new();
        bundle.update(|b| {
            let primary = b.project.theme.primary;
            if let Some(page) = b.project.pages.get_mut(page_index.get_untracked()) {
                let min_x = pts.iter().map(|p| p.x).fold(f32::INFINITY, f32::min);
                let min_y = pts.iter().map(|p| p.y).fold(f32::INFINITY, f32::min);
                let max_x = pts.iter().map(|p| p.x).fold(f32::NEG_INFINITY, f32::max);
                let max_y = pts.iter().map(|p| p.y).fold(f32::NEG_INFINITY, f32::max);
                c += 1;
                let mut node = Node::new(
                    format!("node-path-{c}"),
                    "مسیر تازه",
                    NodeKind::Path,
                    min_x,
                    min_y,
                    (max_x - min_x).max(2.0),
                    (max_y - min_y).max(2.0),
                );
                node.path_points = pts
                    .iter()
                    .map(|p| PathPoint {
                        x: p.x - min_x,
                        y: p.y - min_y,
                        in_x: p.in_x.map(|v| v - min_x),
                        in_y: p.in_y.map(|v| v - min_y),
                        out_x: p.out_x.map(|v| v - min_x),
                        out_y: p.out_y.map(|v| v - min_y),
                    })
                    .collect();
                node.path_closed = closed;
                node.fill = Some(primary);
                node.stroke = Some(primary);
                node.stroke_width = 2.0;
                new_id = node.id.clone();
                page.root.children.push(node);
                page.root.apply_layout();
            }
        });
        counter.set(c);
        pen_points.set(Vec::new());
        pen_current.set(None);
        persist();
        if !new_id.is_empty() {
            selected_id.set(new_id.clone());
            let mut set = BTreeSet::new();
            set.insert(new_id);
            sel_set.set(set);
        }
        if tool.get_untracked() == Tool::Pen {
            tool.set(Tool::Move);
        }
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
        ev.prevent_default();
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
            Tool::Image => {
                // The image tool works through the file picker (the
                // rail button opens it); pressing on the canvas with
                // it active does nothing.
            }
            Tool::Pen => {
                if let Some((px, py)) = artboard_point(&ev) {
                    let pts = pen_points.get_untracked();
                    let s = scale();
                    if pts.len() >= 3 {
                        let first = pts[0];
                        if (px - first.x).abs() <= 10.0 / s && (py - first.y).abs() <= 10.0 / s {
                            finish_pen(true);
                            return;
                        }
                    }
                    pen_points.update(|v| v.push(PathPoint::anchor(px, py)));
                    let idx = pen_points.with_untracked(|v| v.len().saturating_sub(1));
                    drag.set(DragMode::PenDrag { index: idx });
                }
            }
            t => {
                if let Some(kind) = t.shape_kind() {
                    if let Some((px, py)) = artboard_point(&ev) {
                        // Start a draw drag: the node is created on
                        // mouse up with the dragged size, or with the
                        // default size when this turns out to be a
                        // plain click.
                        ctx_menu.set(None);
                        drag.set(DragMode::Create { kind, sx: px, sy: py, cx: px, cy: py });
                    }
                }
            }
        }
    };

    let on_workspace_mousedown = move |ev: leptos::ev::MouseEvent| {
        ev.prevent_default();
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
            DragMode::Create { kind, sx, sy, .. } => {
                if let Some((px, py)) = artboard_point(&ev) {
                    drag.set(DragMode::Create { kind, sx, sy, cx: px, cy: py });
                }
            }
            DragMode::PenDrag { index } => {
                if let Some((px, py)) = artboard_point(&ev) {
                    pen_points.update(|v| {
                        if let Some(pt) = v.get_mut(index) {
                            let (ax, ay) = (pt.x, pt.y);
                            if (px - ax).abs() > 1.0 || (py - ay).abs() > 1.0 {
                                pt.out_x = Some(px);
                                pt.out_y = Some(py);
                                pt.in_x = Some(2.0 * ax - px);
                                pt.in_y = Some(2.0 * ay - py);
                            }
                        }
                    });
                    pen_current.set(Some((px, py)));
                }
            }
            DragMode::CreateGuide { vertical, .. } => {
                if let Some((px, py)) = artboard_point(&ev) {
                    let pos = if vertical { px } else { py };
                    drag.set(DragMode::CreateGuide { vertical, pos });
                    guides.set(vec![(vertical, pos)]);
                }
            }
            DragMode::MoveGuide { vertical, index, .. } => {
                if let Some((px, py)) = artboard_point(&ev) {
                    let pos = if vertical { px } else { py };
                    drag.set(DragMode::MoveGuide { vertical, index, pos });
                    guides.set(vec![(vertical, pos)]);
                }
            }
            DragMode::PathPointDrag { node_id, point, handle, start_points } => {
                if let Some((px, py)) = artboard_point(&ev) {
                    bundle.update(|b| {
                        if let Some(page) = b.project.pages.get_mut(page_index.get_untracked()) {
                            // Parent offset of the edited node.
                            let mut off = (0.0f32, 0.0f32);
                            fn find_off(node: &Node, id: &str, ox: f32, oy: f32) -> Option<(f32, f32)> {
                                for child in &node.children {
                                    if child.id == id {
                                        return Some((ox, oy));
                                    }
                                    if let Some(v) = find_off(child, id, ox + child.x, oy + child.y) {
                                        return Some(v);
                                    }
                                }
                                None
                            }
                            if let Some(v) = find_off(&page.root, &node_id, 0.0, 0.0) {
                                off = v;
                            }
                            if let Some(node) = page.root.find_mut(&node_id) {
                                if let Some(orig) = start_points.get(point).cloned() {
                                    let lx = px - off.0;
                                    let ly = py - off.1;
                                    if let Some(pt) = node.path_points.get_mut(point) {
                                        match handle {
                                            0 => {
                                                let (dx, dy) = (lx - orig.x, ly - orig.y);
                                                pt.x = orig.x + dx;
                                                pt.y = orig.y + dy;
                                                pt.in_x = orig.in_x.map(|v| v + dx);
                                                pt.in_y = orig.in_y.map(|v| v + dy);
                                                pt.out_x = orig.out_x.map(|v| v + dx);
                                                pt.out_y = orig.out_y.map(|v| v + dy);
                                            }
                                            1 => {
                                                pt.in_x = Some(lx);
                                                pt.in_y = Some(ly);
                                            }
                                            _ => {
                                                pt.out_x = Some(lx);
                                                pt.out_y = Some(ly);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    });
                }
            }
            DragMode::Pan { start_x, start_y, orig_x, orig_y } => {
                pan.set((
                    orig_x + (ev.client_x() - start_x) as f32,
                    orig_y + (ev.client_y() - start_y) as f32,
                ));
            }
            DragMode::MoveNode { start_x, start_y, items, primary_ax, primary_ay, primary_w, primary_h, targets, page_guides_x, page_guides_y, page_w, page_h } => {
                let mut dx = (ev.client_x() - start_x) as f32 / s;
                let mut dy = (ev.client_y() - start_y) as f32 / s;
                // Snap the primary node edges and centres to page
                // edges and centre, stored ruler guides and other
                // nodes, within 6 screen pixels.
                let threshold = 6.0 / s;
                let nx = primary_ax + dx;
                let ny = primary_ay + dy;
                let x_points = [nx, nx + primary_w / 2.0, nx + primary_w];
                let y_points = [ny, ny + primary_h / 2.0, ny + primary_h];
                let mut cand_x: Vec<f32> = vec![0.0, page_w / 2.0, page_w];
                cand_x.extend(page_guides_x.iter().copied());
                for &(tx, _, tw, _) in &targets {
                    cand_x.extend([tx, tx + tw / 2.0, tx + tw]);
                }
                let mut cand_y: Vec<f32> = vec![0.0, page_h / 2.0, page_h];
                cand_y.extend(page_guides_y.iter().copied());
                for &(_, ty, _, th) in &targets {
                    cand_y.extend([ty, ty + th / 2.0, ty + th]);
                }
                let mut best_x: Option<(f32, f32)> = None;
                for tp in cand_x {
                    for mp in x_points.iter() {
                        let delta = tp - mp;
                        if delta.abs() <= threshold
                            && best_x.map(|(b, _)| delta.abs() < b.abs()).unwrap_or(true)
                        {
                            best_x = Some((delta, tp));
                        }
                    }
                }
                let mut best_y: Option<(f32, f32)> = None;
                for tp in cand_y {
                    for mp in y_points.iter() {
                        let delta = tp - mp;
                        if delta.abs() <= threshold
                            && best_y.map(|(b, _)| delta.abs() < b.abs()).unwrap_or(true)
                        {
                            best_y = Some((delta, tp));
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
                // Spacing labels: the nearest horizontal and
                // vertical gaps to other nodes after the move.
                let labels = bundle.with_untracked(|b| {
                    let mut out: Vec<(f32, f32, String)> = Vec::new();
                    if let Some(page) = b.project.pages.get(page_index.get_untracked()) {
                        let flat = page_flat(&b.project, page, false);
                        let moved: Vec<String> = items.iter().map(|(id, _, _)| id.clone()).collect();
                        if let Some(pri) = flat.iter().find(|f| moved.first().map(|m| &f.id == m).unwrap_or(false)) {
                            let (mut best_h, mut best_v): (Option<(f32, f32, f32)>, Option<(f32, f32, f32)>) = (None, None);
                            for o in &flat {
                                if moved.contains(&o.id) {
                                    continue;
                                }
                                let v_overlap = pri.ay < o.ay + o.height && o.ay < pri.ay + pri.height;
                                let h_overlap = pri.ax < o.ax + o.width && o.ax < pri.ax + pri.width;
                                if v_overlap {
                                    let gap = if o.ax >= pri.ax + pri.width {
                                        o.ax - (pri.ax + pri.width)
                                    } else if pri.ax >= o.ax + o.width {
                                        pri.ax - (o.ax + o.width)
                                    } else {
                                        -1.0
                                    };
                                    if gap >= 0.0 && gap <= 240.0 && best_h.map(|(g, _, _)| gap < g).unwrap_or(true) {
                                        let gx = if o.ax >= pri.ax + pri.width { pri.ax + pri.width + gap / 2.0 } else { o.ax + o.width + gap / 2.0 };
                                        best_h = Some((gap, gx, pri.ay + pri.height / 2.0));
                                    }
                                }
                                if h_overlap {
                                    let gap = if o.ay >= pri.ay + pri.height {
                                        o.ay - (pri.ay + pri.height)
                                    } else if pri.ay >= o.ay + o.height {
                                        pri.ay - (o.ay + o.height)
                                    } else {
                                        -1.0
                                    };
                                    if gap >= 0.0 && gap <= 240.0 && best_v.map(|(g, _, _)| gap < g).unwrap_or(true) {
                                        let gy = if o.ay >= pri.ay + pri.height { pri.ay + pri.height + gap / 2.0 } else { o.ay + o.height + gap / 2.0 };
                                        best_v = Some((gap, pri.ax + pri.width / 2.0, gy));
                                    }
                                }
                            }
                            if let Some((gap, gx, gy)) = best_h {
                                out.push((gx, gy, format!("{}", gap.round() as i32)));
                            }
                            if let Some((gap, gx, gy)) = best_v {
                                out.push((gx, gy, format!("{}", gap.round() as i32)));
                            }
                        }
                    }
                    out
                });
                spacing_labels.set(labels);
            }
            DragMode::Resize { corner, start_x, start_y, orig_x, orig_y, orig_w, orig_h, targets, page_guides_x, page_guides_y, page_w, page_h, parent_ax, parent_ay, orig_points } => {
                let dx = (ev.client_x() - start_x) as f32 / s;
                let dy = (ev.client_y() - start_y) as f32 / s;
                let id = selected_id.get_untracked();
                let threshold = 6.0 / s;
                let mut cand_x: Vec<f32> = vec![0.0, page_w / 2.0, page_h * 0.0, page_w];
                cand_x.extend(page_guides_x.iter().copied());
                let mut cand_y: Vec<f32> = vec![0.0, page_h / 2.0, page_h];
                cand_y.extend(page_guides_y.iter().copied());
                for &(tx, ty, tw, th) in &targets {
                    cand_x.extend([tx, tx + tw / 2.0, tx + tw]);
                    cand_y.extend([ty, ty + th / 2.0, ty + th]);
                }
                let snap = |edge: f32, cands: &Vec<f32>| -> Option<(f32, f32)> {
                    let mut best: Option<(f32, f32)> = None;
                    for tp in cands {
                        let delta = tp - edge;
                        if delta.abs() <= threshold
                            && best.map(|(b, _)| delta.abs() < b.abs()).unwrap_or(true)
                        {
                            best = Some((delta, *tp));
                        }
                    }
                    best
                };
                let mut new_guides: Vec<(bool, f32)> = Vec::new();
                bundle.update(|b| {
                    if let Some(page) = b.project.pages.get_mut(page_index.get_untracked()) {
                        if let Some(node) = page.root.find_mut(&id) {
                            let (mut x, mut y, mut w, mut h) = (orig_x, orig_y, orig_w, orig_h);
                            if corner & 1 != 0 {
                                w = (orig_w + dx).max(8.0);
                                if let Some((delta, pos)) = snap(parent_ax + orig_x + w, &cand_x) {
                                    w = (w + delta).max(8.0);
                                    new_guides.push((true, pos));
                                }
                            }
                            if corner & 2 != 0 {
                                h = (orig_h + dy).max(8.0);
                                if let Some((delta, pos)) = snap(parent_ay + orig_y + h, &cand_y) {
                                    h = (h + delta).max(8.0);
                                    new_guides.push((false, pos));
                                }
                            }
                            if corner & 4 != 0 {
                                let new_x = orig_x + dx;
                                let mut sx = new_x;
                                if let Some((delta, pos)) = snap(parent_ax + new_x, &cand_x) {
                                    sx = new_x + delta;
                                    new_guides.push((true, pos));
                                }
                                w = (orig_w + (orig_x - sx)).max(8.0);
                                x = orig_x + (orig_w - w);
                            }
                            if corner & 8 != 0 {
                                let new_y = orig_y + dy;
                                let mut sy = new_y;
                                if let Some((delta, pos)) = snap(parent_ay + new_y, &cand_y) {
                                    sy = new_y + delta;
                                    new_guides.push((false, pos));
                                }
                                h = (orig_h + (orig_y - sy)).max(8.0);
                                y = orig_y + (orig_h - h);
                            }
                            if !orig_points.is_empty() && orig_w > 0.0 && orig_h > 0.0 {
                                let (fx, fy) = (w / orig_w, h / orig_h);
                                node.path_points = orig_points
                                    .iter()
                                    .map(|pt| PathPoint {
                                        x: pt.x * fx,
                                        y: pt.y * fy,
                                        in_x: pt.in_x.map(|v| v * fx),
                                        in_y: pt.in_y.map(|v| v * fy),
                                        out_x: pt.out_x.map(|v| v * fx),
                                        out_y: pt.out_y.map(|v| v * fy),
                                    })
                                    .collect();
                            }
                            node.x = x;
                            node.y = y;
                            node.width = w;
                            node.height = h;
                        }
                    }
                });
                guides.set(new_guides);
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
        if let DragMode::Create { kind, sx, sy, cx, cy } = drag.get_untracked() {
            let (dx, dy) = ((cx - sx).abs(), (cy - sy).abs());
            record("add");
            let node = if dx < 6.0 && dy < 6.0 {
                // A plain click: the default sized node, centred on
                // the click point.
                make_node(kind, sx, sy)
            } else {
                let mut n = make_node(kind, (sx + cx) / 2.0, (sy + cy) / 2.0);
                let (mut x, mut y) = (sx.min(cx), sy.min(cy));
                let (mut w, mut h) = (dx, dy);
                match kind {
                    NodeKind::Divider => {
                        // A line follows the longer axis of the drag
                        // and stays thin on the other one.
                        if dx >= dy {
                            h = if dy >= 4.0 { dy } else { 2.0 };
                            y = sy.min(cy) + (dy - h) / 2.0;
                        } else {
                            w = if dx >= 4.0 { dx } else { 2.0 };
                            x = sx.min(cx) + (dx - w) / 2.0;
                        }
                    }
                    NodeKind::Text => {
                        w = w.max(40.0);
                        h = h.max(24.0);
                    }
                    _ => {
                        w = w.max(8.0);
                        h = h.max(8.0);
                    }
                }
                n.x = x;
                n.y = y;
                n.width = w;
                n.height = h;
                match tool.get_untracked() {
                    Tool::Polygon => {
                        let mut poly = Node::polygon(n.id.clone(), "چندضلعی تازه", x, y, w, h, 5, n.fill.unwrap_or(Color::rgb(239, 64, 86)));
                        poly.props.insert("poly_sides".into(), "5".into());
                        poly.fill = n.fill;
                        poly.stroke = n.stroke;
                        poly.stroke_width = n.stroke_width;
                        n = poly;
                    }
                    Tool::Star => {
                        let mut star = Node::star(n.id.clone(), "ستاره تازه", x, y, w, h, 5, 0.5, n.fill.unwrap_or(Color::rgb(239, 64, 86)));
                        star.props.insert("star_points".into(), "5".into());
                        star.props.insert("star_ratio".into(), "0.5".into());
                        star.fill = n.fill;
                        star.stroke = n.stroke;
                        star.stroke_width = n.stroke_width;
                        n = star;
                    }
                    Tool::Arrow => {
                        n.path_points = vec![
                            PathPoint::anchor(0.0, h / 2.0),
                            PathPoint::anchor(w, h / 2.0),
                        ];
                    }
                    _ => {}
                }
                n
            };
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
                    .filter(|n| !locked.with_untracked(|l| l.contains(&n.id)))
                    .map(|n| n.id)
                    .collect();
                selected_id.set(hits.first().cloned().unwrap_or_default());
                sel_set.set(hits.into_iter().collect());
            }
        }
        if let DragMode::CreateGuide { vertical, pos } = drag.get_untracked() {
            let mut committed = false;
            bundle.update(|b| {
                if let Some(page) = b.project.pages.get_mut(page_index.get_untracked()) {
                    let limit = if vertical { page.width } else { page.height };
                    if pos > 0.0 && pos < limit {
                        let list = if vertical { &mut page.guides_x } else { &mut page.guides_y };
                        if !list.iter().any(|v| (v - pos).abs() < 0.5) {
                            list.push(pos);
                            list.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                        }
                        committed = true;
                    }
                }
            });
            if committed {
                selected_guide.set(Some((vertical, pos)));
                record("guide");
            }
        }
        if let DragMode::MoveGuide { vertical, index, pos } = drag.get_untracked() {
            bundle.update(|b| {
                if let Some(page) = b.project.pages.get_mut(page_index.get_untracked()) {
                    let limit = if vertical { page.width } else { page.height };
                    let list = if vertical { &mut page.guides_x } else { &mut page.guides_y };
                    if pos <= 0.0 || pos >= limit {
                        if index < list.len() {
                            list.remove(index);
                        }
                        selected_guide.set(None);
                    } else if let Some(slot) = list.get_mut(index) {
                        *slot = pos;
                        selected_guide.set(Some((vertical, pos)));
                    }
                }
            });
            record("guide");
        }
        if !matches!(drag.get_untracked(), DragMode::None) {
            drag.set(DragMode::None);
            guides.set(Vec::new());
            spacing_labels.set(Vec::new());
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
    /// Bring the primary selected node to the very front of its
    /// siblings, or send it to the very back.
    let front_selection = move |front: bool| {
        let id = selected_id.get_untracked();
        if id.is_empty() {
            return;
        }
        record("frontback");
        apply(&|p: &mut Project| {
            if let Some(page) = p.pages.get_mut(page_index.get_untracked()) {
                move_to_edge(&mut page.root, &id, front);
            }
        });
        ctx_menu.set(None);
    };
    let group_selection = move || {
        let ids = sel_set.get_untracked();
        if ids.len() < 2 {
            return;
        }
        record("group");
        let mut created: Vec<String> = Vec::new();
        let mut c = counter.get_untracked();
        bundle.update(|b| {
            if let Some(page) = b.project.pages.get_mut(page_index.get_untracked()) {
                let tops = topmost_selected(&page.root, &ids);
                let top_set: BTreeSet<String> = tops.iter().cloned().collect();
                created = page.root.group_children(&top_set, "group", &mut c);
                for gid in &created {
                    if let Some(g) = page.root.find_mut(gid) {
                        g.name = "گروه".to_string();
                    }
                }
                page.root.apply_layout();
            }
        });
        counter.set(c);
        persist();
        if created.is_empty() {
            render_note.set("برای گروه بندی، دست کم دو عنصر هم سطح را انتخاب کنید.".to_string());
        } else {
            selected_id.set(created[0].clone());
            sel_set.set(created.iter().cloned().collect());
            render_note.set("گروه ساخته شد.".to_string());
        }
        ctx_menu.set(None);
    };
    let ungroup_selection = move || {
        let primary = selected_id.get_untracked();
        if primary.is_empty() {
            return;
        }
        record("ungroup");
        let mut released: Vec<String> = Vec::new();
        bundle.update(|b| {
            if let Some(page) = b.project.pages.get_mut(page_index.get_untracked()) {
                let gid = group_root_of(&page.root, &primary);
                let mut groups = BTreeSet::new();
                groups.insert(gid);
                released = page.root.ungroup_children(&groups);
                page.root.apply_layout();
            }
        });
        persist();
        if released.is_empty() {
            render_note.set("عنصر انتخاب شده داخل گروه نیست.".to_string());
        } else {
            selected_id.set(released[0].clone());
            sel_set.set(released.iter().cloned().collect());
            render_note.set("گروه باز شد.".to_string());
        }
        ctx_menu.set(None);
    };
    /// Turn the current selection into a master component and
    /// leave an instance of it in place.
    let make_component = move || {
        let ids = sel_set.get_untracked();
        if ids.is_empty() {
            return;
        }
        record("component");
        let mut c = counter.get_untracked();
        let mut instance_id = String::new();
        bundle.update(|b| {
            let Some(page) = b.project.pages.get_mut(page_index.get_untracked()) else {
                return;
            };
            let tops = topmost_selected(&page.root, &ids);
            let Some(first) = tops.first().cloned() else {
                return;
            };
            let Some(mut tree) = page.root.find(&first).cloned() else {
                return;
            };
            c += 1;
            let key = format!("comp-{c}");
            let name = if tree.name.is_empty() {
                "کامپوننت تازه".to_string()
            } else {
                tree.name.clone()
            };
            let (w, h) = (tree.width.max(1.0), tree.height.max(1.0));
            if !tree.props.contains_key("comp_group") && tree.kind != NodeKind::Group {
                // A plain single node master keeps its own box.
                tree.x = 0.0;
                tree.y = 0.0;
            } else {
                tree.x = 0.0;
                tree.y = 0.0;
            }
            b.project.components.push(ComponentDef {
                id: key.clone(),
                name: name.clone(),
                root: tree,
            });
            c += 1;
            let mut inst = Node::new(
                format!("inst-{c}"),
                name,
                NodeKind::Component,
                0.0,
                0.0,
                w,
                h,
            );
            inst.component = Some(key);
            if let Some(orig) = page.root.find(&first) {
                inst.x = orig.x;
                inst.y = orig.y;
                if orig.component.is_none() {
                    // Seed overrides from the original look so the
                    // instance paints exactly like the source.
                    inst.fill = orig.fill;
                    inst.fill_gradient = orig.fill_gradient.clone();
                    inst.text = orig.text.clone();
                }
            }
            instance_id = inst.id.clone();
            replace_in_tree(&mut page.root, &first, inst);
            page.root.apply_layout();
        });
        counter.set(c);
        persist();
        if !instance_id.is_empty() {
            select_only(instance_id);
            render_note.set("کامپوننت ساخته شد و یک نمونه از آن سر جایش ماند.".to_string());
        }
    };
    /// Insert an instance of a master at the page centre area.
    let insert_instance = move |key: String| {
        record("instance");
        let mut c = counter.get_untracked();
        let mut new_id = String::new();
        bundle.update(|b| {
            let Some(def) = b.project.component_def(&key).cloned() else {
                return;
            };
            let Some(page) = b.project.pages.get_mut(page_index.get_untracked()) else {
                return;
            };
            c += 1;
            let mut inst = Node::new(
                format!("inst-{c}"),
                def.name.clone(),
                NodeKind::Component,
                120.0,
                120.0,
                def.root.width.max(1.0),
                def.root.height.max(1.0),
            );
            inst.component = Some(key);
            new_id = inst.id.clone();
            page.root.children.push(inst);
            page.root.apply_layout();
        });
        counter.set(c);
        persist();
        if !new_id.is_empty() {
            select_only(new_id);
        }
    };
    /// Replace the selected instance with its resolved plain copy.
    let detach_instance = move || {
        let id = selected_id.get_untracked();
        record("detach");
        let mut done = false;
        bundle.update(|b| {
            let resolved = b
                .project
                .pages
                .get(page_index.get_untracked())
                .and_then(|pg| pg.root.find(&id))
                .filter(|n| n.component.is_some())
                .map(|n| b.project.resolve_node(n));
            if let Some(mut copy) = resolved {
                copy.id = id.clone();
                copy.component = None;
                if let Some(page) = b.project.pages.get_mut(page_index.get_untracked()) {
                    done = replace_in_tree(&mut page.root, &id, copy);
                    page.root.apply_layout();
                }
            }
        });
        persist();
        if done {
            render_note.set("نمونه جدا شد و به یک کپی ساده تبدیل شد.".to_string());
        }
    };
    /// Open a master on a scratch page for editing. The scratch
    /// page is an ordinary page, so every canvas tool works on it.
    let open_master = move |key: String| {
        if master_edit.get_untracked().is_some() {
            return;
        }
        let return_to = page_index.get_untracked();
        let mut c = counter.get_untracked();
        bundle.update(|b| {
            let Some(def) = b.project.component_def(&key).cloned() else {
                return;
            };
            let mut tree = def.root.clone();
            tree.x = 60.0;
            tree.y = 60.0;
            let mut page = Page::new(
                format!("master-{key}"),
                format!("ویرایش اصلی: {}", def.name),
                "/__master",
                (tree.width + 160.0).max(480.0),
                (tree.height + 160.0).max(360.0),
            );
            page.root.children.push(tree);
            b.project.pages.push(page);
        });
        let _ = c;
        master_edit.set(Some((key, return_to)));
        page_index.set(bundle.with_untracked(|b| b.project.pages.len().saturating_sub(1)));
        selected_id.set(String::new());
        sel_set.set(BTreeSet::new());
        persist();
    };
    /// Write the edited master tree back and return to the page
    /// the user came from. Every instance re-renders from the new
    /// master on the next flatten.
    let finish_master = move || {
        let Some((key, return_to)) = master_edit.get_untracked() else {
            return;
        };
        record("master-edit");
        bundle.update(|b| {
            let idx = b
                .project
                .pages
                .iter()
                .position(|pg| pg.id == format!("master-{key}"));
            if let Some(idx) = idx {
                let page = b.project.pages.remove(idx);
                if let Some(mut tree) = page.root.children.into_iter().next() {
                    tree.x = 0.0;
                    tree.y = 0.0;
                    if let Some(def) = b
                        .project
                        .components
                        .iter_mut()
                        .find(|def| def.id == key || def.name == key)
                    {
                        def.root = tree;
                    }
                }
            }
        });
        master_edit.set(None);
        page_index.set(return_to.min(
            bundle.with_untracked(|b| b.project.pages.len().saturating_sub(1)),
        ));
        selected_id.set(String::new());
        sel_set.set(BTreeSet::new());
        persist();
        render_note.set("نسخه اصلی به روز شد و همه نمونه ها تغییر کردند.".to_string());
    };
    /// Align the whole selection to the page: every selected node
    /// shifts by the same page-space delta, so nested nodes (whose
    /// parents only translate them) land exactly right too.
    let align_selection = move |mode: &str| {
        let ids = sel_set.get_untracked();
        if ids.is_empty() {
            return;
        }
        let (pw, ph) = page_size();
        let flat = flat_all();
        // Per-node deltas: every selected element lands on the same page
        // edge or page middle, like Figma's align tools (a single shared
        // delta would only align the selection bounding box).
        let deltas: Vec<(String, f32, f32)> = flat
            .iter()
            .filter(|n| ids.contains(&n.id))
            .map(|r| {
                let (mut dx, mut dy) = (0.0f32, 0.0f32);
                match mode {
                    "left" => dx = -r.ax,
                    "hcenter" => dx = pw / 2.0 - (r.ax + r.width / 2.0),
                    "right" => dx = pw - (r.ax + r.width),
                    "top" => dy = -r.ay,
                    "vmiddle" => dy = ph / 2.0 - (r.ay + r.height / 2.0),
                    "bottom" => dy = ph - (r.ay + r.height),
                    _ => {}
                }
                (r.id.clone(), dx, dy)
            })
            .collect();
        if deltas.is_empty() || deltas.iter().all(|(_, dx, dy)| *dx == 0.0 && *dy == 0.0) {
            return;
        }
        record("align");
        apply(&|p: &mut Project| {
            if let Some(page) = p.pages.get_mut(page_index.get_untracked()) {
                for (id, dx, dy) in &deltas {
                    if let Some(n) = page.root.find_mut(id) {
                        n.x += dx;
                        n.y += dy;
                    }
                }
            }
        });
    };
    /// Spread 3 or more selected nodes evenly along one axis: the
    /// first and last stay put and the middle ones get equal gaps.
    let distribute_selection = move |horizontal: bool| {
        let ids = sel_set.get_untracked();
        if ids.len() < 3 {
            return;
        }
        let flat = flat_all();
        let mut rects: Vec<(String, f32, f32, f32, f32)> = flat
            .iter()
            .filter(|n| ids.contains(&n.id))
            .map(|n| (n.id.clone(), n.ax, n.ay, n.width, n.height))
            .collect();
        if rects.len() < 3 {
            return;
        }
        rects.sort_by(|a, b| {
            let (ka, kb) = if horizontal { (a.1, b.1) } else { (a.2, b.2) };
            ka.partial_cmp(&kb).unwrap_or(std::cmp::Ordering::Equal)
        });
        let n = rects.len();
        let (first, last) = if horizontal {
            (rects[0].1, rects[n - 1].1 + rects[n - 1].3)
        } else {
            (rects[0].2, rects[n - 1].2 + rects[n - 1].4)
        };
        let total_size: f32 = rects
            .iter()
            .map(|r| if horizontal { r.3 } else { r.4 })
            .sum();
        let gap = ((last - first) - total_size) / (n - 1) as f32;
        let mut deltas: Vec<(String, f32)> = Vec::new();
        let mut cursor = first;
        for r in &rects {
            let cur = if horizontal { r.1 } else { r.2 };
            deltas.push((r.0.clone(), cursor - cur));
            cursor += (if horizontal { r.3 } else { r.4 }) + gap;
        }
        record("distribute");
        apply(&|p: &mut Project| {
            if let Some(page) = p.pages.get_mut(page_index.get_untracked()) {
                for (id, d) in &deltas {
                    if let Some(node) = page.root.find_mut(id) {
                        if horizontal {
                            node.x += d;
                        } else {
                            node.y += d;
                        }
                    }
                }
            }
        });
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
                .as_ref().map(|g| (g.from.to_hex(), g.to.to_hex(), g.angle)),
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
            "Enter" => {
                if tool.get_untracked() == Tool::Pen && !pen_points.get_untracked().is_empty() {
                    ev.prevent_default();
                    finish_pen(false);
                }
            }
            "Delete" | "Backspace" => {
                if !path_edit_id.get_untracked().is_empty() {
                    if let Some(pi) = path_point_sel.get_untracked() {
                        ev.prevent_default();
                        let id = path_edit_id.get_untracked();
                        record("path-del-point");
                        apply(&|p: &mut Project| {
                            if let Some(page) = p.pages.get_mut(page_index.get_untracked()) {
                                if let Some(node) = page.root.find_mut(&id) {
                                    if pi < node.path_points.len() && node.path_points.len() > 1 {
                                        node.path_points.remove(pi);
                                    }
                                }
                            }
                        });
                        path_point_sel.set(None);
                        return;
                    }
                }
                if let Some((vertical, pos)) = selected_guide.get_untracked() {
                    ev.prevent_default();
                    record("guide-del");
                    apply(&|p: &mut Project| {
                        if let Some(page) = p.pages.get_mut(page_index.get_untracked()) {
                            let list = if vertical { &mut page.guides_x } else { &mut page.guides_y };
                            list.retain(|v| (v - pos).abs() > 0.01);
                        }
                    });
                    selected_guide.set(None);
                    return;
                }
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
                if tool.get_untracked() == Tool::Pen && !pen_points.get_untracked().is_empty() {
                    finish_pen(false);
                } else if !path_edit_id.get_untracked().is_empty() {
                    path_edit_id.set(String::new());
                    path_point_sel.set(None);
                } else {
                    select_only(String::new());
                    ctx_menu.set(None);
                }
            }
            _ => {
                if (ev.ctrl_key() || ev.meta_key()) && ev.shift_key() && (key == "g" || key == "G") {
                    ev.prevent_default();
                    ungroup_selection();
                } else if (ev.ctrl_key() || ev.meta_key()) && (key == "g" || key == "G") {
                    ev.prevent_default();
                    group_selection();
                } else if (ev.ctrl_key() || ev.meta_key()) && (key == "d" || key == "D") {
                    ev.prevent_default();
                    duplicate_selection();
                } else if (ev.ctrl_key() || ev.meta_key()) && (key == "z" || key == "Z") {
                    ev.prevent_default();
                    if ev.shift_key() {
                        do_redo();
                    } else {
                        do_undo();
                    }
                } else if (ev.ctrl_key() || ev.meta_key()) && (key == "y" || key == "Y") {
                    ev.prevent_default();
                    do_redo();
                } else if !ev.ctrl_key() && !ev.meta_key() && !ev.alt_key() {
                    // Tool shortcuts by physical key position, so
                    // they work on any keyboard layout.
                    let t = match ev.code().as_str() {
                        "KeyV" => Some(Tool::Move),
                        "KeyH" => Some(Tool::Hand),
                        "KeyF" => Some(Tool::Frame),
                        "KeyR" => Some(Tool::Rect),
                        "KeyO" => Some(Tool::Ellipse),
                        "KeyL" => Some(Tool::Line),
                        "KeyT" => Some(Tool::Text),
                        "KeyP" => Some(Tool::Pen),
                        _ => None,
                    };
                    if let Some(t) = t {
                        if t.min_level() <= level.get_untracked() {
                            tool.set(t);
                        }
                    }
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

    // The tool rail: tool, icon, Persian label, shortcut key (empty
    // for none). Which tools show depends on the editor level.
    let tools: Vec<(Tool, &'static str, &'static str, &'static str)> = vec![
        (Tool::Move, "➤", "انتخاب", "V"),
        (Tool::Hand, "✋", "دست", "H"),
        (Tool::Text, "T", "متن", "T"),
        (Tool::Image, "🖼", "تصویر", ""),
        (Tool::Frame, "▢", "فریم", "F"),
        (Tool::Rect, "▭", "مستطیل", "R"),
        (Tool::Ellipse, "●", "دایره", "O"),
        (Tool::Line, "―", "خط", "L"),
        (Tool::Button, "▣", "دکمه", ""),
        (Tool::Card, "▤", "کارت", ""),
        (Tool::Input, "⌨", "ورودی", ""),
        (Tool::Pen, "✒", "قلم", "P"),
        (Tool::Polygon, "⬠", "چندضلعی", ""),
        (Tool::Star, "★", "ستاره", ""),
        (Tool::Arrow, "➜", "فلش", ""),
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
                    <button class:active=move || level.get() == 1 on:click=move |_| { level.set(1); if tool.get_untracked().min_level() > 1 { tool.set(Tool::Move); } if tab.get() > 0 { tab.set(0); } }>"ساده"</button>
                    <button class:active=move || level.get() == 2 on:click=move |_| { level.set(2); if tool.get_untracked().min_level() > 2 { tool.set(Tool::Move); } if tab.get() == 1 || tab.get() == 3 { tab.set(0); } }>"سازنده"</button>
                    <button class:active=move || level.get() == 3 on:click=move |_| level.set(3)>"حرفه‌ای"</button>
                </nav>
                <div class="zoom-box">
                    <button title="کوچک نمایی" on:click=zoom_out>"−"</button>
                    <span class="zoom-label">{move || format!("٪{}", (scale() * 100.0).round() as i32)}</span>
                    <button title="بزرگ نمایی" on:click=zoom_in>"+"</button>
                    <button class="fit-btn" title="جا دادن صفحه در نما" on:click=zoom_fit>"جا دادن"</button>
                </div>
                <div class="top-actions">
                    <button title="برگرد به صفحه شروع" on:click=move |_| home_open.set(true)>"🏠 خانه"</button>
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
                                download_text(&format!("{}.svg", page.id), &page_svg(&b.project, page), "image/svg+xml");
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
                    <button class="export" on:click=move |_| {
                        bundle.with(|b| {
                            let files = chida_wp::export_theme(&b.project);
                            match chida_wp::write_zip(&files) {
                                Ok(bytes) => {
                                    let slug = b
                                        .project
                                        .meta
                                        .get("wp_slug")
                                        .cloned()
                                        .unwrap_or_else(|| b.project.id.clone());
                                    download_bytes(&format!("{slug}.zip"), &bytes);
                                    render_note.set("قالب وردپرس دانلود شد".to_string());
                                }
                                Err(e) => {
                                    render_note.set(format!("ساخت قالب وردپرس نشد: {e}"));
                                }
                            }
                        });
                    }>"خروجی وردپرس"</button>
                </div>
            </header>

            <div class="workbench">
                <aside class="rail">
                    {move || {
                        let lv = level.get();
                        tools.iter().copied()
                            .filter(|(t, _, _, _)| t.min_level() <= lv)
                            .map(|(t, icon, label, key)| {
                                let title = if key.is_empty() {
                                    label.to_string()
                                } else {
                                    format!("{label} ({key})")
                                };
                                view! {
                                    <button
                                        class="tool"
                                        class:active=move || tool.get() == t
                                        title=title
                                        on:click=move |_| {
                                            if t == Tool::Image {
                                                // The image tool is the
                                                // upload picker itself.
                                                upload_target.set(None);
                                                if let Some(input) = file_ref.get_untracked() {
                                                    input.click();
                                                }
                                            } else {
                                                tool.set(t);
                                            }
                                        }
                                    >
                                        <span class="tool-icon">{icon}</span>
                                        <span class="tool-label">{label}</span>
                                    </button>
                                }
                            }).collect_view()
                    }}
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
                                {move || (level.get() >= 3).then(|| view! {
                                    <div class="masters-box">
                                        <h4 class="cat-title">"کامپوننت ها"</h4>
                                        <p class="hint">"از انتخاب فعلی یک کامپوننت بسازید. نمونه ها از روی نسخه اصلی کشیده می شوند و با ویرایش اصلی، همه با هم عوض می شوند."</p>
                                        <button class="comp-create" disabled=move || selected_id.get().is_empty() on:click=move |_| make_component()>"ساخت کامپوننت از انتخاب"</button>
                                        {move || {
                                            let masters = bundle.with(|b| b.project.components.clone());
                                            if masters.is_empty() {
                                                return view! { <p class="hint">"هنوز کامپوننتی ساخته نشده است."</p> }.into_any();
                                            }
                                            masters.into_iter().map(|def| {
                                                let key_ins = def.id.clone();
                                                let key_edit = def.id.clone();
                                                view! {
                                                    <div class="master-row">
                                                        <span class="master-name">{def.name.clone()}</span>
                                                        <button class="mini" on:click=move |_| insert_instance(key_ins.clone())>"درج نمونه"</button>
                                                        <button class="mini" on:click=move |_| open_master(key_edit.clone())>"ویرایش اصلی"</button>
                                                    </div>
                                                }
                                            }).collect_view().into_any()
                                        }}
                                    </div>
                                })}
                                <h4 class="cat-title">"بخش ها"</h4>
                                <p class="hint">"با یک کلیک، بخش آماده با استایل کامل وسط صفحه درج می شود. بلوک ها کپی ساده اند، نه نمونه کامپوننت."</p>
                                {[
                                    ("اجزای پایه", &[("btn-primary", "🔘", "دکمه اصلی"), ("btn-secondary", "⚪", "دکمه دوم"), ("badge", "🏷", "نشان"), ("avatar", "👤", "آواتار با نام"), ("table", "📋", "جدول ساده"), ("product-card", "🛍", "کارت محصول با عکس")][..]),
                                    ("نوبارها", &[("navbar", "🧭", "نوبار"), ("navbar-search", "🔎", "نوبار با جستجو")][..]),
                                    ("هیروها", &[("hero", "🖼", "هیرو با عکس"), ("hero-center", "🎯", "هیروی وسط چین"), ("hero-app", "📱", "هیروی اپ")][..]),
                                    ("ویژگی ها", &[("features", "✨", "بخش ویژگی ها"), ("features-four", "🧩", "چهار ویژگی"), ("steps", "👣", "سه گام")][..]),
                                    ("قیمت گذاری", &[("price-card", "💳", "کارت قیمت"), ("pricing-three", "💰", "قیمت گذاری سه طرح")][..]),
                                    ("نظرات مشتریان", &[("testimonials", "💬", "نظرات مشتریان")][..]),
                                    ("گالری", &[("gallery", "🖼", "گالری عکس")][..]),
                                    ("فرم ها", &[("input", "⌨", "ورودی فرم"), ("contact-form", "✉", "فرم تماس"), ("newsletter", "📰", "خبرنامه")][..]),
                                    ("فوترها", &[("footer-simple", "⬛", "فوتر ساده"), ("footer-cols", "🗂", "فوتر چند ستونه")][..]),
                                    ("فراخوان ها", &[("cta-banner", "📣", "فراخوان پهن"), ("cta-card", "🪧", "کارت فراخوان")][..]),
                                ].into_iter().map(|(cat, items)| {
                                    view! {
                                        <div class="comp-cat">
                                            <h5 class="cat-head">{cat}</h5>
                                            <div class="comp-grid">
                                                {items.iter().map(|(key, icon, label)| {
                                                    let (key, icon, label) = (*key, *icon, *label);
                                                    view! {
                                                        <button class="comp-card" data-comp=key on:click=move |_| insert_component(key)>
                                                            <span class="comp-icon">{icon}</span>
                                                            <span class="comp-name">{label}</span>
                                                        </button>
                                                    }
                                                }).collect_view()}
                                            </div>
                                        </div>
                                    }
                                }).collect_view()}
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
                                let id_dbl = n.id.clone();
                                let id_drag = n.id.clone();
                                let id_drop = n.id.clone();
                                let id_rename = n.id.clone();
                                let name_val = n.name.clone();
                                let indent = format!("padding-inline-start:{}px;", 8 + n.depth * 14);
                                view! {
                                    <div
                                        class="layer-row"
                                        data-layer-id=n.id.clone()
                                        class:selected=move || sel_set.get().contains(&id_class)
                                        style=indent
                                        on:click=move |ev: leptos::ev::MouseEvent| {
                                            if rename_id.get_untracked() == id_row {
                                                return;
                                            }
                                            if ev.shift_key() {
                                                let anchor = layer_anchor.get_untracked();
                                                let mut done = false;
                                                if !anchor.is_empty() && anchor != id_row {
                                                    let mut list = flat_all();
                                                    list.reverse();
                                                    let ia = list.iter().position(|f| f.id == anchor);
                                                    let ib = list.iter().position(|f| f.id == id_row);
                                                    if let (Some(a), Some(b)) = (ia, ib) {
                                                        let (lo, hi) = if a <= b { (a, b) } else { (b, a) };
                                                        let set: BTreeSet<String> = list[lo..=hi].iter().map(|f| f.id.clone()).collect();
                                                        selected_id.set(id_row.clone());
                                                        sel_set.set(set);
                                                        done = true;
                                                    }
                                                }
                                                if !done {
                                                    let mut set = sel_set.get_untracked();
                                                    if !set.remove(&id_row) {
                                                        set.insert(id_row.clone());
                                                        selected_id.set(id_row.clone());
                                                    } else if selected_id.get_untracked() == id_row {
                                                        selected_id.set(set.iter().next().cloned().unwrap_or_default());
                                                    }
                                                    sel_set.set(set);
                                                }
                                            } else {
                                                layer_anchor.set(id_row.clone());
                                                selected_id.set(id_row.clone());
                                                let mut one = BTreeSet::new();
                                                one.insert(id_row.clone());
                                                sel_set.set(one);
                                            }
                                        }
                                        draggable="true"
                                        on:dragstart=move |_| layer_drag_id.set(id_drag.clone())
                                        on:dragover=move |ev: leptos::ev::DragEvent| ev.prevent_default()
                                        on:drop=move |ev: leptos::ev::DragEvent| {
                                            ev.prevent_default();
                                            let dragged = layer_drag_id.get_untracked();
                                            if !dragged.is_empty() && dragged != id_drop {
                                                record("layer-reorder");
                                                let target = id_drop.clone();
                                                apply(&|p: &mut Project| {
                                                    if let Some(page) = p.pages.get_mut(page_index.get_untracked()) {
                                                        move_layer_before(&mut page.root, &dragged, &target);
                                                    }
                                                });
                                            }
                                            layer_drag_id.set(String::new());
                                        }
                                    >
                                        <span class="layer-icon">{kind_icon(n.kind)}</span>
                                        {move || if rename_id.get() == id_rename {
                                            let id_commit = id_rename.clone();
                                            view! {
                                                <input
                                                    class="layer-rename"
                                                    type="text"
                                                    prop:value=name_val.clone()
                                                    on:click=move |ev: leptos::ev::MouseEvent| ev.stop_propagation()
                                                    on:keydown=move |ev: leptos::ev::KeyboardEvent| {
                                                        if ev.key() == "Enter" {
                                                            let v = event_target_value(&ev);
                                                            if !v.trim().is_empty() {
                                                                record("rename");
                                                                let id = id_commit.clone();
                                                                apply(&|p: &mut Project| {
                                                                    if let Some(page) = p.pages.get_mut(page_index.get_untracked()) {
                                                                        if let Some(node) = page.root.find_mut(&id) {
                                                                            node.name = v.trim().to_string();
                                                                        }
                                                                    }
                                                                });
                                                            }
                                                            rename_id.set(String::new());
                                                        } else if ev.key() == "Escape" {
                                                            rename_id.set(String::new());
                                                        }
                                                    }
                                                />
                                            }.into_any()
                                        } else {
                                            let id_d = id_dbl.clone();
                                            view! {
                                                <span class="layer-name" on:dblclick=move |ev: leptos::ev::MouseEvent| { ev.stop_propagation(); rename_id.set(id_d.clone()); }>{name_val.clone()}</span>
                                            }.into_any()
                                        }}
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
                    {move || master_edit.get().map(|_| view! {
                        <div class="master-banner">
                            <span>"در حال ویرایش نسخه اصلی کامپوننت هستید. هر تغییری روی همه نمونه ها می نشیند."</span>
                            <button on:click=move |_| finish_master()>"تمام شد، برگرد"</button>
                        </div>
                    })}
                    <div
                        class="world"
                        style=move || format!("transform:translate({}px,{}px);", pan.get().0, pan.get().1)
                    >
                        <div
                            class="ruler-x"
                            title="از اینجا به پایین بکشید تا خط راهنمای افقی بسازید"
                            style=move || {
                                let (w, _) = page_size();
                                format!("width:{}px;", w * scale())
                            }
                            on:mousedown=move |ev: leptos::ev::MouseEvent| {
                                ev.prevent_default();
                                ev.stop_propagation();
                                if let Some((_, py)) = artboard_point(&ev) {
                                    drag.set(DragMode::CreateGuide { vertical: false, pos: py });
                                }
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
                            title="از اینجا به کنار بکشید تا خط راهنمای عمودی بسازید"
                            style=move || {
                                let (_, h) = page_size();
                                format!("height:{}px;", h * scale())
                            }
                            on:mousedown=move |ev: leptos::ev::MouseEvent| {
                                ev.prevent_default();
                                ev.stop_propagation();
                                if let Some((px, _)) = artboard_point(&ev) {
                                    drag.set(DragMode::CreateGuide { vertical: true, pos: px });
                                }
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
                                    let svg_html = node_svg_html(&n);
                                    let has_inter = n.interactions.as_ref().map(|v| !v.trim().is_empty()).unwrap_or(false);
                                    let id_dbl = n.id.clone();
                                    let is_path_kind = matches!(n.kind, NodeKind::Path | NodeKind::Arrow);
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
                                                ev.prevent_default();
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
                                                        let resolved = bundle.with_untracked(|b| {
                                                            b.project
                                                                .pages
                                                                .get(page_index.get_untracked())
                                                                .map(|pg| group_root_of(&pg.root, &id_down))
                                                                .unwrap_or_else(|| id_down.clone())
                                                        });
                                                        if locked.with_untracked(|l| l.contains(&resolved)) {
                                                            // A locked node (for example the
                                                            // blank project's backdrop frame)
                                                            // lets the press fall through to
                                                            // the artboard, so a drag becomes
                                                            // a marquee instead of a blocked
                                                            // node drag.
                                                            return;
                                                        }
                                                        ev.stop_propagation();
                                                        ctx_menu.set(None);
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
                                                            let flat = page_flat(&b.project, page, true);
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
                                                            let guides_data = bundle.with_untracked(|b| {
                                                                b.project.pages.get(page_index.get_untracked()).map(|pg| {
                                                                    (pg.guides_x.clone(), pg.guides_y.clone(), pg.width, pg.height)
                                                                }).unwrap_or_default()
                                                            });
                                                            drag.set(DragMode::MoveNode {
                                                                start_x: ev.client_x(),
                                                                start_y: ev.client_y(),
                                                                items,
                                                                primary_ax: pax,
                                                                primary_ay: pay,
                                                                primary_w: pw,
                                                                primary_h: ph,
                                                                targets,
                                                                page_guides_x: guides_data.0,
                                                                page_guides_y: guides_data.1,
                                                                page_w: guides_data.2,
                                                                page_h: guides_data.3,
                                                            });
                                                        }
                                                    }
                                                    _ => {}
                                                }
                                            }
                                            on:dblclick=move |ev: leptos::ev::MouseEvent| {
                                                if is_path_kind {
                                                    ev.stop_propagation();
                                                    let resolved = bundle.with_untracked(|b| {
                                                        b.project
                                                            .pages
                                                            .get(page_index.get_untracked())
                                                            .map(|pg| group_root_of(&pg.root, &id_dbl))
                                                            .unwrap_or_else(|| id_dbl.clone())
                                                    });
                                                    // Point editing works on the
                                                    // path node itself.
                                                    let direct = bundle.with_untracked(|b| {
                                                        b.project
                                                            .pages
                                                            .get(page_index.get_untracked())
                                                            .and_then(|pg| pg.root.find(&id_dbl))
                                                            .map(|_| id_dbl.clone())
                                                    });
                                                    let target = direct.unwrap_or(resolved);
                                                    select_only(target.clone());
                                                    path_edit_id.set(target);
                                                    path_point_sel.set(None);
                                                }
                                            }
                                        >
                                            {label}
                                            {svg_html.map(|html| view! { <span class="node-svg" inner_html=html></span> })}
                                            {has_inter.then(|| view! { <span class="link-badge" title="این عنصر تعامل دارد">"🔗"</span> })}
                                        </div>
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
                                {move || bundle.with(|b| {
                                    b.project.pages.get(page_index.get()).map(|pg| {
                                        let mut items: Vec<(bool, usize, f32)> = Vec::new();
                                        for (i, x) in pg.guides_x.iter().enumerate() {
                                            items.push((true, i, *x));
                                        }
                                        for (i, y) in pg.guides_y.iter().enumerate() {
                                            items.push((false, i, *y));
                                        }
                                        items
                                    }).unwrap_or_default()
                                }).into_iter().map(|(vertical, idx, pos)| {
                                    let (pw, ph) = page_size();
                                    let is_sel = selected_guide.get() == Some((vertical, pos));
                                    let style = if vertical {
                                        format!("left:{pos}px;top:0;width:2px;height:{ph}px;")
                                    } else {
                                        format!("top:{pos}px;left:0;height:2px;width:{pw}px;")
                                    };
                                    view! {
                                        <div
                                            class="page-guide"
                                            class:selected-guide=is_sel
                                            data-guide-axis=if vertical { "x" } else { "y" }
                                            style=style
                                            on:mousedown=move |ev: leptos::ev::MouseEvent| {
                                                ev.prevent_default();
                                                ev.stop_propagation();
                                                selected_guide.set(Some((vertical, pos)));
                                                drag.set(DragMode::MoveGuide { vertical, index: idx, pos });
                                            }
                                        ></div>
                                    }
                                }).collect_view()}
                                {move || spacing_labels.get().into_iter().map(|(x, y, text)| {
                                    view! {
                                        <div class="spacing-label" style=format!("left:{x}px;top:{y}px;")>{text}</div>
                                    }
                                }).collect_view()}
                                {move || {
                                    let pts = pen_points.get();
                                    if pts.is_empty() {
                                        return ().into_any();
                                    }
                                    let (pw, ph) = page_size();
                                    let mut d = String::new();
                                    let mut temp = Node::new("draft", "draft", NodeKind::Path, 0.0, 0.0, pw, ph);
                                    temp.path_points = pts.clone();
                                    d = temp.path_d(0.0, 0.0);
                                    let cur = pen_current.get();
                                    let anchors = pts.iter().enumerate().map(|(i, pt)| {
                                        format!("<circle cx=\"{}\" cy=\"{}\" r=\"6\" class=\"pen-anchor{}\"/>", pt.x, pt.y, if i == 0 { " first" } else { "" })
                                    }).collect::<Vec<_>>().join("");
                                    let cur_line = match cur {
                                        Some((cx, cy)) if !pts.is_empty() => {
                                            let last = pts[pts.len() - 1];
                                            format!("<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" class=\"pen-cur\"/>", last.x, last.y, cx, cy)
                                        }
                                        _ => String::new(),
                                    };
                                    let html = format!("<svg width=\"{pw}\" height=\"{ph}\" viewBox=\"0 0 {pw} {ph}\"><path d=\"{d}\" class=\"pen-line\"/>{cur_line}{anchors}</svg>");
                                    view! { <div class="pen-overlay" inner_html=html></div> }.into_any()
                                }}
                                {move || {
                                    let edit_id = path_edit_id.get();
                                    if edit_id.is_empty() {
                                        return ().into_any();
                                    }
                                    let Some(n) = flat_canvas().into_iter().find(|f| f.id == edit_id) else {
                                        return ().into_any();
                                    };
                                    let pts = n.path_points.clone();
                                    if pts.is_empty() {
                                        return ().into_any();
                                    }
                                    let start_points = pts.clone();
                                    let seg_count = if n.path_closed && pts.len() > 2 { pts.len() } else { pts.len().saturating_sub(1) };
                                    let segments = (0..seg_count).map(|i| {
                                        let a = pts[i].clone();
                                        let b = pts[(i + 1) % pts.len()].clone();
                                        let (x1, y1) = (n.ax + a.x, n.ay + a.y);
                                        let (x2, y2) = (n.ax + b.x, n.ay + b.y);
                                        let node_id = n.id.clone();
                                        view! {
                                            <span
                                                class="path-seg"
                                                style=format!("left:{}px;top:{}px;width:{}px;", x1.min(x2), y1.min(y2), ((x2 - x1).powi(2) + (y2 - y1).powi(2)).sqrt())
                                                on:mousedown=move |ev: leptos::ev::MouseEvent| {
                                                    ev.stop_propagation();
                                                    record("path-add-point");
                                                    let mid = PathPoint::anchor((a.x + b.x) / 2.0, (a.y + b.y) / 2.0);
                                                    let nid = node_id.clone();
                                                    apply(&|p: &mut Project| {
                                                        if let Some(page) = p.pages.get_mut(page_index.get_untracked()) {
                                                            if let Some(node) = page.root.find_mut(&nid) {
                                                                let at = (i + 1).min(node.path_points.len());
                                                                node.path_points.insert(at, mid);
                                                            }
                                                        }
                                                    });
                                                    path_point_sel.set(Some(i + 1));
                                                }
                                            ></span>
                                        }
                                    }).collect_view();
                                    let anchors = pts.iter().enumerate().map(|(i, pt)| {
                                        let node_id = n.id.clone();
                                        let sp = start_points.clone();
                                        let is_sel = path_point_sel.get() == Some(i);
                                        view! {
                                            <span
                                                class="path-anchor"
                                                class:sel=is_sel
                                                data-point=i.to_string()
                                                style=format!("left:{}px;top:{}px;", n.ax + pt.x, n.ay + pt.y)
                                                on:mousedown=move |ev: leptos::ev::MouseEvent| {
                                                    ev.stop_propagation();
                                                    path_point_sel.set(Some(i));
                                                    drag.set(DragMode::PathPointDrag { node_id: node_id.clone(), point: i, handle: 0, start_points: sp.clone() });
                                                }
                                            ></span>
                                        }
                                    }).collect_view();
                                    let handles = pts.iter().enumerate().flat_map(|(i, pt)| {
                                        let mut out = Vec::new();
                                        if let (Some(hx), Some(hy)) = (pt.in_x, pt.in_y) {
                                            out.push((i, 1u8, n.ax + hx, n.ay + hy));
                                        }
                                        if let (Some(hx), Some(hy)) = (pt.out_x, pt.out_y) {
                                            out.push((i, 2u8, n.ax + hx, n.ay + hy));
                                        }
                                        out
                                    }).map(|(i, handle, hx, hy)| {
                                        let node_id = n.id.clone();
                                        let sp = start_points.clone();
                                        view! {
                                            <span
                                                class="path-handle"
                                                style=format!("left:{hx}px;top:{hy}px;")
                                                on:mousedown=move |ev: leptos::ev::MouseEvent| {
                                                    ev.stop_propagation();
                                                    path_point_sel.set(Some(i));
                                                    drag.set(DragMode::PathPointDrag { node_id: node_id.clone(), point: i, handle, start_points: sp.clone() });
                                                }
                                            ></span>
                                        }
                                    }).collect_view();
                                    view! {
                                        <div class="path-edit-layer">
                                            {segments}
                                            {handles}
                                            {anchors}
                                        </div>
                                    }.into_any()
                                }}
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
                                    DragMode::Create { sx, sy, cx, cy, .. } => {
                                        let style = format!(
                                            "left:{}px;top:{}px;width:{}px;height:{}px;",
                                            sx.min(cx),
                                            sy.min(cy),
                                            (cx - sx).abs(),
                                            (cy - sy).abs()
                                        );
                                        view! { <div class="create-ghost" style=style></div> }.into_any()
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
                                                            let data = bundle.with_untracked(|b| {
                                                                let page = b.project.pages.get(page_index.get_untracked())?;
                                                                let node = page.root.find(&sel_h)?;
                                                                let flat = page_flat(&b.project, page, true);
                                                                let primary = flat.iter().find(|f| f.id == sel_h)?;
                                                                let targets: Vec<(f32, f32, f32, f32)> = flat
                                                                    .iter()
                                                                    .filter(|f| f.id != sel_h)
                                                                    .map(|f| (f.ax, f.ay, f.width, f.height))
                                                                    .collect();
                                                                Some((
                                                                    node.x,
                                                                    node.y,
                                                                    node.width,
                                                                    node.height,
                                                                    targets,
                                                                    page.guides_x.clone(),
                                                                    page.guides_y.clone(),
                                                                    page.width,
                                                                    page.height,
                                                                    primary.pox,
                                                                    primary.poy,
                                                                    node.path_points.clone(),
                                                                ))
                                                            }).unwrap_or((0.0, 0.0, 0.0, 0.0, Vec::new(), Vec::new(), Vec::new(), 0.0, 0.0, 0.0, 0.0, Vec::new()));
                                                            drag.set(DragMode::Resize {
                                                                corner,
                                                                start_x: ev.client_x(),
                                                                start_y: ev.client_y(),
                                                                orig_x: data.0,
                                                                orig_y: data.1,
                                                                orig_w: data.2,
                                                                orig_h: data.3,
                                                                targets: data.4,
                                                                page_guides_x: data.5,
                                                                page_guides_y: data.6,
                                                                page_w: data.7,
                                                                page_h: data.8,
                                                                parent_ax: data.9,
                                                                parent_ay: data.10,
                                                                orig_points: data.11,
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
                                                download_text(&format!("{}.svg", page.id), &page_svg(&b.project, page), "image/svg+xml");
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
                                            let (g_from, g_to, g_angle) = node.fill_gradient.as_ref().map(|g| (g.from.to_hex().chars().take(7).collect::<String>(), g.to.to_hex().chars().take(7).collect::<String>(), g.angle)).unwrap_or_else(|| ("#ef4056".to_string(), "#ff8a5c".to_string(), 90.0));
                                            let has_shadow = !node.shadows.is_empty();
                                            let shadows_list = node.shadows.clone();
                                            let grad_stops: Vec<GradientStop> = node.fill_gradient.as_ref().map(|g| g.stops()).unwrap_or_default();
                                            let grad_css = node.fill_gradient.as_ref().map(|g| g.css()).unwrap_or_default();
                                            let width_mode = node.width_mode;
                                            let height_mode = node.height_mode;
                                            let (pad_top, pad_right, pad_bottom, pad_left) = node.padding_values();
                                            let layout_justify = node.layout_justify;
                                            let layout_wrap = node.layout_wrap;
                                            let is_path_node = matches!(node.kind, NodeKind::Path | NodeKind::Arrow);
                                            let path_closed = node.path_closed;
                                            let point_count = node.path_points.len();
                                            let poly_sides: f32 = node.props.get("sides").or_else(|| node.props.get("poly_sides")).and_then(|v| v.parse().ok()).unwrap_or(5.0);
                                            let star_points: f32 = node.props.get("points").or_else(|| node.props.get("star_points")).and_then(|v| v.parse().ok()).unwrap_or(5.0);
                                            let star_ratio_pct: f32 = node.props.get("inner_ratio").or_else(|| node.props.get("star_ratio")).and_then(|v| v.parse::<f32>().ok()).map(|v| v * 100.0).unwrap_or(50.0);
                                            let blur_val = node.blur_radius;
                                            let bg_blur_val = node.background_blur;
                                            let comp_key = node.component.clone();
                                            let inter_entries = interaction_entries(node.props.get("interactions").map(|v| v.as_str()));
                                            view! {
                                                <div class="fields" class:big=simple>
                                                    <p class="node-name">{node.name.clone()}<span class="node-kind">{kind_fa(node.kind)}</span></p>
                                                    {(level.get() == 3).then(|| view! {
                                                        <details class="prop-section" open>
                                                            <summary>"ترتیب و تراز"</summary>
                                                            <div class="align-row">
                                                                <span>"ترتیب"</span>
                                                                <button on:click=move |_| front_selection(true)>"جلوترین"</button>
                                                                <button on:click=move |_| { let id = selected_id.get_untracked(); reorder_selection(id, true); }>"جلو"</button>
                                                                <button on:click=move |_| { let id = selected_id.get_untracked(); reorder_selection(id, false); }>"عقب"</button>
                                                                <button on:click=move |_| front_selection(false)>"عقب‌ترین"</button>
                                                            </div>
                                                            <div class="align-row">
                                                                <span>"گروه"</span>
                                                                <button disabled=move || sel_set.get().len() < 2 on:click=move |_| group_selection()>"گروه‌بندی"</button>
                                                                <button on:click=move |_| ungroup_selection()>"باز کردن گروه"</button>
                                                                <button disabled=move || selected_id.get().is_empty() on:click=move |_| make_component()>"ساخت کامپوننت"</button>
                                                            </div>
                                                            <div class="align-row">
                                                                <span>"تراز با صفحه"</span>
                                                                <button on:click=move |_| align_selection("left")>"چپ"</button>
                                                                <button on:click=move |_| align_selection("hcenter")>"وسط افقی"</button>
                                                                <button on:click=move |_| align_selection("right")>"راست"</button>
                                                            </div>
                                                            <div class="align-row">
                                                                <span></span>
                                                                <button on:click=move |_| align_selection("top")>"بالا"</button>
                                                                <button on:click=move |_| align_selection("vmiddle")>"وسط عمودی"</button>
                                                                <button on:click=move |_| align_selection("bottom")>"پایین"</button>
                                                            </div>
                                                            <div class="align-row">
                                                                <span>"پخش و تکثیر"</span>
                                                                <button disabled=move || sel_set.get().len() < 3 on:click=move |_| distribute_selection(true)>"پخش افقی"</button>
                                                                <button disabled=move || sel_set.get().len() < 3 on:click=move |_| distribute_selection(false)>"پخش عمودی"</button>
                                                                <button on:click=move |_| duplicate_selection()>"تکثیر"</button>
                                                            </div>
                                                        </details>
                                                    })}
                                                    <details class="prop-section" open>
                                                        <summary>"چیدمان"</summary>
                                                        <div class="grid2">
                                                            <label>"X" <input type="number" disabled=parent_layout prop:value=nx.to_string() on:input=move |ev| { let v = event_target_value(&ev); edit_selected("x", &|n: &mut Node| { if let Ok(v) = v.parse::<f32>() { n.x = v; } }); } /></label>
                                                            <label>"Y" <input type="number" disabled=parent_layout prop:value=ny.to_string() on:input=move |ev| { let v = event_target_value(&ev); edit_selected("y", &|n: &mut Node| { if let Ok(v) = v.parse::<f32>() { n.y = v; } }); } /></label>
                                                            <label>"عرض" <input type="number" prop:value=nw.to_string() on:input=move |ev| { let v = event_target_value(&ev); edit_selected("w", &|n: &mut Node| { if let Ok(v) = v.parse::<f32>() { n.width = v.max(1.0); } }); } /></label>
                                                            <label>"ارتفاع" <input type="number" prop:value=nh.to_string() on:input=move |ev| { let v = event_target_value(&ev); edit_selected("h", &|n: &mut Node| { if let Ok(v) = v.parse::<f32>() { n.height = v.max(1.0); } }); } /></label>
                                                        </div>
                                                        {(level.get() >= 2).then(|| view! {
                                                            <div class="grid2">
                                                                <label>"حالت عرض"
                                                                    <select on:change=move |ev| { let v = event_target_value(&ev); edit_selected("wmode", &|n: &mut Node| { n.width_mode = match v.as_str() { "hug" => AxisSizing::Hug, "fill" => AxisSizing::Fill, _ => AxisSizing::Fixed }; }); }>
                                                                        <option value="fixed" selected=width_mode == AxisSizing::Fixed>"ثابت"</option>
                                                                        <option value="hug" selected=width_mode == AxisSizing::Hug>"جمع شدن دور محتوا"</option>
                                                                        <option value="fill" selected=width_mode == AxisSizing::Fill>"پر کردن"</option>
                                                                    </select>
                                                                </label>
                                                                <label>"حالت ارتفاع"
                                                                    <select on:change=move |ev| { let v = event_target_value(&ev); edit_selected("hmode", &|n: &mut Node| { n.height_mode = match v.as_str() { "hug" => AxisSizing::Hug, "fill" => AxisSizing::Fill, _ => AxisSizing::Fixed }; }); }>
                                                                        <option value="fixed" selected=height_mode == AxisSizing::Fixed>"ثابت"</option>
                                                                        <option value="hug" selected=height_mode == AxisSizing::Hug>"جمع شدن دور محتوا"</option>
                                                                        <option value="fill" selected=height_mode == AxisSizing::Fill>"پر کردن"</option>
                                                                    </select>
                                                                </label>
                                                            </div>
                                                        })}
                                                        {(level.get() >= 2).then(|| view! {
                                                            <label>"گردی گوشه" <span class="val">{format!("{}", radius as i32)}</span>
                                                                <input type="range" min="0" max="60" prop:value=radius.to_string() on:input=move |ev| { let v = event_target_value(&ev); edit_selected("radius", &|n: &mut Node| { if let Ok(v) = v.parse::<f32>() { n.corner_radius = v; } }); } />
                                                            </label>
                                                        })}
                                                        {(level.get() == 3).then(|| view! {
                                                            <label>"شفافیت" <span class="val">{format!("٪{}", opacity_pct as i32)}</span>
                                                                <input type="range" min="0" max="100" prop:value=opacity_pct.to_string() on:input=move |ev| { let v = event_target_value(&ev); edit_selected("opacity", &|n: &mut Node| { if let Ok(v) = v.parse::<f32>() { n.opacity = (v / 100.0).clamp(0.0, 1.0); } }); } />
                                                            </label>
                                                        })}
                                                        {(is_container && level.get() >= 2).then(|| view! {
                                                            <div class="align-row">
                                                                <span>"چیدمان خودکار"</span>
                                                                <button class:on=layout_dir == LayoutDirection::None on:click=move |_| edit_selected("layoutdir", &|n: &mut Node| n.layout_direction = LayoutDirection::None)>"بدون"</button>
                                                                <button class:on=layout_dir == LayoutDirection::Row on:click=move |_| edit_selected("layoutdir", &|n: &mut Node| n.layout_direction = LayoutDirection::Row)>"افقی"</button>
                                                                <button class:on=layout_dir == LayoutDirection::Column on:click=move |_| edit_selected("layoutdir", &|n: &mut Node| n.layout_direction = LayoutDirection::Column)>"عمودی"</button>
                                                            </div>
                                                        })}
                                                        {(is_container && level.get() >= 2 && layout_dir != LayoutDirection::None).then(|| view! {
                                                            <div class="grid2">
                                                                <label>"فاصله بین بچه‌ها" <input type="number" prop:value=layout_gap.to_string() on:input=move |ev| { let v = event_target_value(&ev); edit_selected("layoutgap", &|n: &mut Node| { if let Ok(v) = v.parse::<f32>() { n.layout_gap = v.max(0.0); } }); } /></label>
                                                                {(level.get() < 3).then(|| view! { <label>"حاشیه داخلی" <input type="number" prop:value=layout_pad.to_string() on:input=move |ev| { let v = event_target_value(&ev); edit_selected("layoutpad", &|n: &mut Node| { if let Ok(v) = v.parse::<f32>() { n.layout_padding = v.max(0.0); n.layout_padding_top = None; n.layout_padding_right = None; n.layout_padding_bottom = None; n.layout_padding_left = None; } }); } /></label> })}
                                                            </div>
                                                            {(level.get() == 3).then(|| view! {
                                                                <div class="grid2">
                                                                    <label>"حاشیه بالا" <input type="number" prop:value=pad_top.to_string() on:input=move |ev| { let v = event_target_value(&ev); edit_selected("padtop", &|n: &mut Node| { if let Ok(v) = v.parse::<f32>() { n.layout_padding_top = Some(v.max(0.0)); } }); } /></label>
                                                                    <label>"حاشیه پایین" <input type="number" prop:value=pad_bottom.to_string() on:input=move |ev| { let v = event_target_value(&ev); edit_selected("padbottom", &|n: &mut Node| { if let Ok(v) = v.parse::<f32>() { n.layout_padding_bottom = Some(v.max(0.0)); } }); } /></label>
                                                                    <label>"حاشیه راست" <input type="number" prop:value=pad_right.to_string() on:input=move |ev| { let v = event_target_value(&ev); edit_selected("padright", &|n: &mut Node| { if let Ok(v) = v.parse::<f32>() { n.layout_padding_right = Some(v.max(0.0)); } }); } /></label>
                                                                    <label>"حاشیه چپ" <input type="number" prop:value=pad_left.to_string() on:input=move |ev| { let v = event_target_value(&ev); edit_selected("padleft", &|n: &mut Node| { if let Ok(v) = v.parse::<f32>() { n.layout_padding_left = Some(v.max(0.0)); } }); } /></label>
                                                                </div>
                                                            })}
                                                            <div class="align-row">
                                                                <span>"تراز"</span>
                                                                <button class:on=layout_align == LayoutAlign::Start on:click=move |_| edit_selected("layoutalign", &|n: &mut Node| n.layout_align = LayoutAlign::Start)>"شروع"</button>
                                                                <button class:on=layout_align == LayoutAlign::Center on:click=move |_| edit_selected("layoutalign", &|n: &mut Node| n.layout_align = LayoutAlign::Center)>"وسط"</button>
                                                                <button class:on=layout_align == LayoutAlign::End on:click=move |_| edit_selected("layoutalign", &|n: &mut Node| n.layout_align = LayoutAlign::End)>"پایان"</button>
                                                                <button class:on=layout_align == LayoutAlign::Stretch on:click=move |_| edit_selected("layoutalign", &|n: &mut Node| n.layout_align = LayoutAlign::Stretch)>"کشیده"</button>
                                                            </div>
                                                            <div class="align-row">
                                                                <span>"پخش در محور اصلی"</span>
                                                                <button class:on=layout_justify == LayoutJustify::Start on:click=move |_| edit_selected("layoutjustify", &|n: &mut Node| n.layout_justify = LayoutJustify::Start)>"شروع"</button>
                                                                <button class:on=layout_justify == LayoutJustify::Center on:click=move |_| edit_selected("layoutjustify", &|n: &mut Node| n.layout_justify = LayoutJustify::Center)>"وسط"</button>
                                                                <button class:on=layout_justify == LayoutJustify::End on:click=move |_| edit_selected("layoutjustify", &|n: &mut Node| n.layout_justify = LayoutJustify::End)>"پایان"</button>
                                                                <button class:on=layout_justify == LayoutJustify::SpaceBetween on:click=move |_| edit_selected("layoutjustify", &|n: &mut Node| n.layout_justify = LayoutJustify::SpaceBetween)>"فاصله بین"</button>
                                                            </div>
                                                            {(layout_dir == LayoutDirection::Row).then(|| view! {
                                                                <label class="check">"رفتن به ردیف بعد (Wrap)"
                                                                    <button class="toggle" class:on=layout_wrap on:click=move |_| edit_selected("layoutwrap", &|n: &mut Node| n.layout_wrap = !n.layout_wrap)>{if layout_wrap { "روشن" } else { "خاموش" }}</button>
                                                                </label>
                                                            })}
                                                            <div class="align9">
                                                                {[0usize, 1, 2].into_iter().map(|row| {
                                                                    view! {
                                                                        <div class="align9-row">
                                                                            {[0usize, 1, 2].into_iter().map(|col| {
                                                                                let (j, a) = if layout_dir == LayoutDirection::Row {
                                                                                    (match col { 0 => LayoutJustify::Start, 1 => LayoutJustify::Center, _ => LayoutJustify::End }, match row { 0 => LayoutAlign::Start, 1 => LayoutAlign::Center, _ => LayoutAlign::End })
                                                                                } else {
                                                                                    (match row { 0 => LayoutJustify::Start, 1 => LayoutJustify::Center, _ => LayoutJustify::End }, match col { 0 => LayoutAlign::Start, 1 => LayoutAlign::Center, _ => LayoutAlign::End })
                                                                                };
                                                                                let is_on = layout_justify == j && layout_align == a;
                                                                                view! {
                                                                                    <button class="align9-cell" class:on=is_on title="تراز بچه ها" on:click=move |_| edit_selected("layout9", &|n: &mut Node| { n.layout_justify = j; n.layout_align = a; })>"•"</button>
                                                                                }
                                                                            }).collect_view()}
                                                                        </div>
                                                                    }
                                                                }).collect_view()}
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
                                                            {(level.get() >= 2).then(|| view! {
                                                                <label>"رنگ متن"
                                                                    <input type="color" prop:value=text_hex on:input=move |ev| { let v = event_target_value(&ev); edit_selected("textcolor", &|n: &mut Node| { if let Ok(c) = Color::from_hex(&v) { n.text_color = Some(c); } }); } />
                                                                </label>
                                                            })}
                                                        </div>
                                                        {(level.get() >= 2).then(|| view! {
                                                            <div class="align-row">
                                                                <span>"وزن"</span>
                                                                {[(400u16, "عادی"), (500, "متوسط"), (700, "درشت")].into_iter().map(|(w, label)| {
                                                                    let is_on = weight == w;
                                                                    view! {
                                                                        <button class:on=is_on on:click=move |_| edit_selected("weight", &|n: &mut Node| { n.font_weight = w; n.bold = w >= 600; })>{label}</button>
                                                                    }
                                                                }).collect_view()}
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
                                                        {(level.get() == 3).then(|| view! {
                                                            <div class="grid2">
                                                                <label>"ارتفاع خط" <input type="number" step="0.05" prop:value=lh_val.to_string() on:input=move |ev| { let v = event_target_value(&ev); edit_selected("lineheight", &|n: &mut Node| { if let Ok(v) = v.parse::<f32>() { n.line_height = v.clamp(0.8, 3.0); } }); } /></label>
                                                                <label>"فاصله حروف" <input type="number" prop:value=ls_val.to_string() on:input=move |ev| { let v = event_target_value(&ev); edit_selected("letterspacing", &|n: &mut Node| { if let Ok(v) = v.parse::<f32>() { n.letter_spacing = v; } }); } /></label>
                                                            </div>
                                                        })}
                                                    </details>
                                                    <details class="prop-section" open>
                                                        <summary>"پس‌زمینه"</summary>
                                                        {(level.get() >= 2).then(|| view! {
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
                                                        {(has_grad && level.get() >= 2).then(|| view! {
                                                            <div class="color-row">
                                                                <span>"از"</span>
                                                                <input type="color" prop:value=g_from on:input=move |ev| { let v = event_target_value(&ev); edit_selected("gradfrom", &|n: &mut Node| { if let (Some(g), Ok(c)) = (&mut n.fill_gradient, Color::from_hex(&v)) { g.from = c; let mut stops = g.stops(); if let Some(first) = stops.first_mut() { first.color = c; } g.set_stops(stops); } }); } />
                                                                <span>"به"</span>
                                                                <input type="color" prop:value=g_to on:input=move |ev| { let v = event_target_value(&ev); edit_selected("gradto", &|n: &mut Node| { if let (Some(g), Ok(c)) = (&mut n.fill_gradient, Color::from_hex(&v)) { g.to = c; let mut stops = g.stops(); if let Some(last) = stops.last_mut() { last.color = c; } g.set_stops(stops); } }); } />
                                                            </div>
                                                            <div class="grad-editor">
                                                                <div
                                                                    class="grad-bar"
                                                                    style=format!("background:{};", grad_css)
                                                                    title="برای افزودن توقف تازه، روی نوار کلیک کنید"
                                                                    on:click=move |ev: leptos::ev::MouseEvent| {
                                                                        let w = ev.target().and_then(|t| t.dyn_into::<web_sys::HtmlElement>().ok()).map(|el| el.offset_width() as f32).unwrap_or(200.0).max(1.0);
                                                                        let off = ((ev.offset_x() as f32) / w).clamp(0.0, 1.0);
                                                                        edit_selected("gradadd", &|n: &mut Node| {
                                                                            if let Some(g) = &mut n.fill_gradient {
                                                                                let mut stops = g.stops();
                                                                                let color = g.at(off);
                                                                                stops.push(GradientStop { offset: off, color });
                                                                                g.set_stops(stops);
                                                                            }
                                                                        });
                                                                    }
                                                                >
                                                                    {grad_stops.iter().enumerate().map(|(i, stop)| {
                                                                        let hex = stop.color.to_hex().chars().take(7).collect::<String>();
                                                                        view! {
                                                                            <button
                                                                                class="grad-stop"
                                                                                class:sel=move || grad_stop_sel.get() == i
                                                                                style=format!("left:{}%;background:{};", stop.offset * 100.0, hex)
                                                                                title="توقف گرادیان"
                                                                                on:click=move |ev: leptos::ev::MouseEvent| { ev.stop_propagation(); grad_stop_sel.set(i); }
                                                                            ></button>
                                                                        }
                                                                    }).collect_view()}
                                                                </div>
                                                                {move || {
                                                                    let stops = grad_stops.clone();
                                                                    if stops.is_empty() {
                                                                        return ().into_any();
                                                                    }
                                                                    let idx = grad_stop_sel.get().min(stops.len() - 1);
                                                                    let stop = stops[idx].clone();
                                                                    let hex = stop.color.to_hex().chars().take(7).collect::<String>();
                                                                    let off_pct = (stop.offset * 100.0).round();
                                                                    let can_del = stops.len() > 2;
                                                                    view! {
                                                                        <div class="grad-stop-ctrl">
                                                                            <div class="color-row">
                                                                                <span>"رنگ توقف"</span>
                                                                                <input type="color" prop:value=hex on:input=move |ev| { let v = event_target_value(&ev); edit_selected("gradstopcolor", &|n: &mut Node| { if let (Some(g), Ok(c)) = (&mut n.fill_gradient, Color::from_hex(&v)) { let mut stops = g.stops(); if let Some(st) = stops.get_mut(idx) { st.color = c; } g.set_stops(stops); } }); } />
                                                                            </div>
                                                                            <label>"جای توقف" <span class="val">{format!("٪{}", off_pct as i32)}</span>
                                                                                <input type="range" min="0" max="100" prop:value=off_pct.to_string() on:input=move |ev| { let v = event_target_value(&ev); edit_selected("gradstopoff", &|n: &mut Node| { if let (Some(g), Ok(v)) = (&mut n.fill_gradient, v.parse::<f32>()) { let mut stops = g.stops(); if let Some(st) = stops.get_mut(idx) { st.offset = (v / 100.0).clamp(0.0, 1.0); } g.set_stops(stops); } }); } />
                                                                            </label>
                                                                            <button class="mini danger" disabled=!can_del on:click=move |_| edit_selected("gradstopdel", &|n: &mut Node| { if let Some(g) = &mut n.fill_gradient { let mut stops = g.stops(); if stops.len() > 2 && idx < stops.len() { stops.remove(idx); } g.set_stops(stops); } })>"حذف توقف"</button>
                                                                        </div>
                                                                    }.into_any()
                                                                }}
                                                            </div>
                                                            <label>"زاویه" <span class="val">{format!("{} درجه", g_angle as i32)}</span>
                                                                <input type="range" min="0" max="360" prop:value=g_angle.to_string() on:input=move |ev| { let v = event_target_value(&ev); edit_selected("gradangle", &|n: &mut Node| { if let (Some(g), Ok(v)) = (&mut n.fill_gradient, v.parse::<f32>()) { g.angle = v; } }); } />
                                                            </label>
                                                        })}
                                                        {(level.get() >= 2).then(|| view! {
                                                            <div class="color-row">
                                                                <span>"خط دور"</span>
                                                                <input type="color" prop:value=stroke_hex on:input=move |ev| { let v = event_target_value(&ev); edit_selected("stroke", &|n: &mut Node| { if let Ok(c) = Color::from_hex(&v) { n.stroke = Some(c); if n.stroke_width <= 0.0 { n.stroke_width = 1.0; } } }); } />
                                                                <input class="hex small" type="number" prop:value=sw.to_string() on:input=move |ev| { let v = event_target_value(&ev); edit_selected("strokew", &|n: &mut Node| { if let Ok(v) = v.parse::<f32>() { n.stroke_width = v.max(0.0); } }); } />
                                                            </div>
                                                        })}
                                                    </details>
                                                    {(level.get() == 3).then(|| view! {
                                                        <details class="prop-section" open>
                                                            <summary>"سایه و محو"</summary>
                                                            <label class="check">"سایه"
                                                                <button class="toggle" class:on=has_shadow on:click=move |_| edit_selected("shadowtoggle", &|n: &mut Node| {
                                                                    if n.shadows.is_empty() { n.shadows = vec![Shadow::soft()]; } else { n.shadows.clear(); }
                                                                })>{if has_shadow { "روشن" } else { "خاموش" }}</button>
                                                            </label>
                                                            {shadows_list.iter().enumerate().map(|(i, sh)| {
                                                                let sh_col = sh.color.to_hex().chars().take(7).collect::<String>();
                                                                let (sx, sy, sblur, sop, sen) = (sh.x, sh.y, sh.blur, (sh.opacity * 100.0).round(), sh.enabled);
                                                                view! {
                                                                    <div class="shadow-row">
                                                                        <div class="align-row">
                                                                            <span>{format!("سایه {}", i + 1)}</span>
                                                                            <button class="toggle" class:on=sen on:click=move |_| edit_selected("shadowon", &|n: &mut Node| { if let Some(s) = n.shadows.get_mut(i) { s.enabled = !s.enabled; } })>{if sen { "روشن" } else { "خاموش" }}</button>
                                                                            <button class="mini danger" on:click=move |_| edit_selected("shadowdel", &|n: &mut Node| { if i < n.shadows.len() { n.shadows.remove(i); } })>"حذف"</button>
                                                                        </div>
                                                                        <label>"افست افقی" <span class="val">{format!("{}", sx as i32)}</span>
                                                                            <input type="range" min="-40" max="40" prop:value=sx.to_string() on:input=move |ev| { let v = event_target_value(&ev); edit_selected("shadowx", &|n: &mut Node| { if let (Some(s), Ok(v)) = (n.shadows.get_mut(i), v.parse::<f32>()) { s.x = v; } }); } />
                                                                        </label>
                                                                        <label>"افست عمودی" <span class="val">{format!("{}", sy as i32)}</span>
                                                                            <input type="range" min="-40" max="40" prop:value=sy.to_string() on:input=move |ev| { let v = event_target_value(&ev); edit_selected("shadowy", &|n: &mut Node| { if let (Some(s), Ok(v)) = (n.shadows.get_mut(i), v.parse::<f32>()) { s.y = v; } }); } />
                                                                        </label>
                                                                        <label>"محو" <span class="val">{format!("{}", sblur as i32)}</span>
                                                                            <input type="range" min="0" max="80" prop:value=sblur.to_string() on:input=move |ev| { let v = event_target_value(&ev); edit_selected("shadowblur", &|n: &mut Node| { if let (Some(s), Ok(v)) = (n.shadows.get_mut(i), v.parse::<f32>()) { s.blur = v; } }); } />
                                                                        </label>
                                                                        <div class="color-row">
                                                                            <span>"رنگ"</span>
                                                                            <input type="color" prop:value=sh_col on:input=move |ev| { let v = event_target_value(&ev); edit_selected("shadowcolor", &|n: &mut Node| { if let (Some(s), Ok(c)) = (n.shadows.get_mut(i), Color::from_hex(&v)) { s.color = c; } }); } />
                                                                        </div>
                                                                        <label>"شفافیت سایه" <span class="val">{format!("٪{}", sop as i32)}</span>
                                                                            <input type="range" min="0" max="100" prop:value=sop.to_string() on:input=move |ev| { let v = event_target_value(&ev); edit_selected("shadowop", &|n: &mut Node| { if let (Some(s), Ok(v)) = (n.shadows.get_mut(i), v.parse::<f32>()) { s.opacity = (v / 100.0).clamp(0.0, 1.0); } }); } />
                                                                        </label>
                                                                    </div>
                                                                }
                                                            }).collect_view()}
                                                            {(has_shadow).then(|| view! {
                                                                <button class="wide-action" on:click=move |_| edit_selected("shadowadd", &|n: &mut Node| { let mut sh = Shadow::soft(); sh.y += 10.0 + (n.shadows.len() as f32) * 6.0; n.shadows.push(sh); })>"افزودن سایه"</button>
                                                            })}
                                                            <label>"محو شدن خود عنصر" <span class="val">{format!("{}", blur_val as i32)}</span>
                                                                <input type="range" min="0" max="20" prop:value=blur_val.to_string() on:input=move |ev| { let v = event_target_value(&ev); edit_selected("blur", &|n: &mut Node| { if let Ok(v) = v.parse::<f32>() { n.blur_radius = v.max(0.0); } }); } />
                                                            </label>
                                                            <label>"محو شدن پس زمینه" <span class="val">{format!("{}", bg_blur_val as i32)}</span>
                                                                <input type="range" min="0" max="20" prop:value=bg_blur_val.to_string() on:input=move |ev| { let v = event_target_value(&ev); edit_selected("bgblur", &|n: &mut Node| { if let Ok(v) = v.parse::<f32>() { n.background_blur = v.max(0.0); } }); } />
                                                            </label>
                                                            <p class="hint">"محو شدن در پیش نمایش و خروجی وب دیده می شود؛ رندر PNG سی پی یویی شکل را بدون محو می کشد."</p>
                                                        </details>
                                                    })}
                                                    {(is_button && level.get() >= 2).then(|| view! {
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
                                                    {(is_path_node).then(|| { let nid_cls = node_id_sel.clone(); let nid_clk = node_id_sel.clone(); view! {
                                                        <details class="prop-section" open>
                                                            <summary>"مسیر و نقطه ها"</summary>
                                                            <p class="hint">{format!("{} نقطه در این مسیر هست.", point_count)}</p>
                                                            <div class="align-row">
                                                                <span>"نقطه ها"</span>
                                                                <button class:on=move || path_edit_id.get() == nid_cls on:click=move |_| {
                                                                    if path_edit_id.get_untracked() == nid_clk {
                                                                        path_edit_id.set(String::new());
                                                                    } else {
                                                                        path_edit_id.set(nid_clk.clone());
                                                                    }
                                                                    path_point_sel.set(None);
                                                                }>"ویرایش نقطه‌ها"</button>
                                                                {(!path_closed).then(|| view! {
                                                                    <button on:click=move |_| edit_selected("pathclose", &|n: &mut Node| n.path_closed = true)>"بستن مسیر"</button>
                                                                })}
                                                            </div>
                                                            {(node.kind == NodeKind::Path).then(|| view! {
                                                                <div class="grid2">
                                                                    <label>"تعداد ضلع چندضلعی" <input type="number" min="3" max="10" prop:value=poly_sides.to_string() on:change=move |ev| { let v = event_target_value(&ev); edit_selected("polysides", &|n: &mut Node| { if let Ok(v) = v.parse::<f32>() { let sides = (v as usize).clamp(3, 10); let mut poly = Node::polygon(n.id.clone(), n.name.clone(), n.x, n.y, n.width, n.height, sides, n.fill.unwrap_or(Color::rgb(20, 24, 24))); poly.fill = n.fill; poly.stroke = n.stroke; poly.stroke_width = n.stroke_width; poly.props = n.props.clone(); poly.props.remove("points"); poly.props.remove("inner_ratio"); poly.props.insert("sides".into(), sides.to_string()); poly.props.insert("shape".into(), "polygon".into()); let keep_props = poly.props.clone(); *n = poly; n.props = keep_props; } }); } /></label>
                                                                    <label>"تعداد پره ستاره" <input type="number" min="3" max="12" prop:value=star_points.to_string() on:change=move |ev| { let v = event_target_value(&ev); edit_selected("starpoints", &|n: &mut Node| { if let Ok(v) = v.parse::<f32>() { let pts = (v as usize).clamp(3, 12); let ratio: f32 = n.props.get("inner_ratio").or_else(|| n.props.get("star_ratio")).and_then(|r| r.parse().ok()).unwrap_or(0.5); let mut star = Node::star(n.id.clone(), n.name.clone(), n.x, n.y, n.width, n.height, pts, ratio, n.fill.unwrap_or(Color::rgb(20, 24, 24))); star.fill = n.fill; star.stroke = n.stroke; star.stroke_width = n.stroke_width; star.props = n.props.clone(); star.props.remove("sides"); star.props.insert("points".into(), pts.to_string()); star.props.insert("shape".into(), "star".into()); let keep_props = star.props.clone(); *n = star; n.props = keep_props; } }); } /></label>
                                                                </div>
                                                                <label>"نسبت پره داخلی ستاره" <span class="val">{format!("٪{}", star_ratio_pct as i32)}</span>
                                                                    <input type="range" min="15" max="90" prop:value=star_ratio_pct.to_string() on:input=move |ev| { let v = event_target_value(&ev); edit_selected("starratio", &|n: &mut Node| { if let Ok(v) = v.parse::<f32>() { let ratio = (v / 100.0).clamp(0.1, 0.95); let pts: usize = n.props.get("points").or_else(|| n.props.get("star_points")).and_then(|r| r.parse().ok()).unwrap_or(5); let mut star = Node::star(n.id.clone(), n.name.clone(), n.x, n.y, n.width, n.height, pts, ratio, n.fill.unwrap_or(Color::rgb(20, 24, 24))); star.fill = n.fill; star.stroke = n.stroke; star.stroke_width = n.stroke_width; star.props = n.props.clone(); star.props.insert("points".into(), pts.to_string()); star.props.insert("inner_ratio".into(), ratio.to_string()); star.props.insert("shape".into(), "star".into()); let keep_props = star.props.clone(); *n = star; n.props = keep_props; } }); } />
                                                                </label>
                                                            })}
                                                        </details>
                                                    } })}
                                                    {comp_key.clone().map(|key| {
                                                        let key_edit = key.clone();
                                                        view! {
                                                            <details class="prop-section" open>
                                                                <summary>"کامپوننت"</summary>
                                                                <p class="hint">"این عنصر یک نمونه از نسخه اصلی است. متن و رنگی که اینجا عوض کنید فقط روی همین نمونه می نشیند."</p>
                                                                <div class="align-row">
                                                                    <button on:click=move |_| open_master(key_edit.clone())>"ویرایش اصلی"</button>
                                                                    <button on:click=move |_| detach_instance()>"جدا کردن"</button>
                                                                </div>
                                                            </details>
                                                        }
                                                    })}
                                                    {(level.get() == 3).then(|| view! {
                                                        <details class="prop-section" open>
                                                            <summary>"تعامل‌ها"</summary>
                                                            {inter_entries.iter().enumerate().map(|(i, (trigger, kind, target))| {
                                                                let label = format!("{}، {}، {}", if trigger == "hover" { "هاور" } else { "کلیک" }, if kind == "url" { "باز کردن پیوند" } else { "رفتن به صفحه" }, target);
                                                                view! {
                                                                    <div class="inter-row">
                                                                        <span>{label}</span>
                                                                        <button class="mini danger" on:click=move |_| edit_selected("interdel", &|n: &mut Node| { let mut entries = interaction_entries(n.props.get("interactions").map(|v| v.as_str())); if i < entries.len() { entries.remove(i); } if entries.is_empty() { n.props.remove("interactions"); } else { n.props.insert("interactions".into(), encode_interactions(&entries)); } })>"حذف"</button>
                                                                    </div>
                                                                }
                                                            }).collect_view()}
                                                            <div class="grid2">
                                                                <label>"رویداد"
                                                                    <select on:change=move |ev| inter_trigger.set(event_target_value(&ev))>
                                                                        <option value="click">"کلیک"</option>
                                                                        <option value="hover">"هاور"</option>
                                                                    </select>
                                                                </label>
                                                                <label>"کار"
                                                                    <select on:change=move |ev| inter_kind.set(event_target_value(&ev))>
                                                                        <option value="page">"رفتن به صفحه"</option>
                                                                        <option value="url">"باز کردن پیوند"</option>
                                                                    </select>
                                                                </label>
                                                            </div>
                                                            {move || if inter_kind.get() == "url" {
                                                                view! { <label class="full">"نشانی پیوند" <input type="text" placeholder="https://example.com" prop:value=move || inter_url.get() on:input=move |ev| inter_url.set(event_target_value(&ev)) /></label> }.into_any()
                                                            } else {
                                                                view! {
                                                                    <label class="full">"صفحه مقصد"
                                                                        <select on:change=move |ev| inter_page.set(event_target_value(&ev))>
                                                                            {bundle.with(|b| b.project.pages.iter().map(|pg| {
                                                                                view! { <option value=pg.route.clone()>{format!("{} ({})", pg.name, pg.route)}</option> }
                                                                            }).collect_view())}
                                                                        </select>
                                                                    </label>
                                                                }.into_any()
                                                            }}
                                                            <button class="wide-action" on:click=move |_| {
                                                                let kind = inter_kind.get_untracked();
                                                                let target = if kind == "url" {
                                                                    inter_url.get_untracked()
                                                                } else {
                                                                    let chosen = inter_page.get_untracked();
                                                                    if chosen.is_empty() {
                                                                        bundle.with_untracked(|b| b.project.pages.first().map(|pg| pg.route.clone()).unwrap_or_default())
                                                                    } else {
                                                                        chosen
                                                                    }
                                                                };
                                                                if target.is_empty() {
                                                                    render_note.set("اول مقصد تعامل را کامل کنید.".to_string());
                                                                    return;
                                                                }
                                                                let line = format!("{}|{}|{}", inter_trigger.get_untracked(), kind, target);
                                                                edit_selected("interadd", &|n: &mut Node| {
                                                                    let mut entries = interaction_entries(n.props.get("interactions").map(|v| v.as_str()));
                                                                    let mut parts = line.splitn(3, '|');
                                                                    if let (Some(t), Some(k), Some(tg)) = (parts.next(), parts.next(), parts.next()) {
                                                                        entries.push((t.to_string(), k.to_string(), tg.to_string()));
                                                                    }
                                                                    n.props.insert("interactions".into(), encode_interactions(&entries));
                                                                });
                                                            }>"افزودن تعامل"</button>
                                                            <p class="hint">"در پیش نمایش، کلیک و هاور هر دو کار می کنند. در خروجی کد، کلیک به پیوند واقعی تبدیل می شود."</p>
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
            <input
                type="file"
                accept=".json,application/json"
                class="hidden-file"
                node_ref=open_ref
                on:change=on_open_change
            />
            <input
                type="file"
                accept=".zip,application/zip"
                class="hidden-file"
                node_ref=wp_ref
                on:change=on_wp_change
            />
            {move || home_open.get().then(|| {
                let saved = has_saved_work();
                let saved_name = bundle.with_untracked(|b| b.project.name.clone());
                view! {
                    <div class="home-overlay">
                        <div class="home-inner">
                            <img class="home-logo" src="assets/logo-mark.svg" alt="Chida"/>
                            <h1>"چیدا"</h1>
                            <p class="home-sub">"سایتت را از صفر بکش؛ چیدا از آن یک برنامه واقعی می‌سازد."</p>
                            {saved.then(|| view! {
                                <button class="home-continue" on:click=move |_| home_open.set(false)>
                                    {format!("ادامه کار قبلی: {saved_name}")}
                                </button>
                            })}
                            <div class="home-actions">
                                <button class="home-card" on:click=move |_| start_project(blank_bundle())>
                                    <span class="home-card-icon">"＋"</span>
                                    <strong>"پروژه خالی"</strong>
                                    <span>"یک صفحه خالی با یک فریم دسکتاپ؛ از صفر شروع کن."</span>
                                </button>
                                <button class="home-card" on:click=move |_| {
                                    if let Some(input) = open_ref.get_untracked() {
                                        input.click();
                                    }
                                }>
                                    <span class="home-card-icon">"📂"</span>
                                    <strong>"باز کردن فایل"</strong>
                                    <span>"یک فایل پروژه چیدا با پسوند chida.json انتخاب کن."</span>
                                </button>
                                <button class="home-card" on:click=move |_| {
                                    if let Some(input) = wp_ref.get_untracked() {
                                        input.click();
                                    }
                                }>
                                    <span class="home-card-icon">"📦"</span>
                                    <strong>"قالب وردپرس"</strong>
                                    <span>"یک فایل zip قالب وردپرس انتخاب کن؛ ویرایشش کن و دوباره قالب تحویل بگیر."</span>
                                </button>
                            </div>
                            {move || {
                                let e = home_error.get();
                                (!e.is_empty()).then(|| view! { <p class="home-error">{e}</p> })
                            }}
                            <h2 class="home-tpl-title">"قالب آماده"</h2>
                            <p class="home-sub small">"از یک قالب شروع کن و هر جایش را که خواستی عوض کن."</p>
                            <div class="tpl-grid">
                                {TEMPLATES.iter().map(|(key, name, thumb)| {
                                    let key_owned = key.to_string();
                                    view! {
                                        <button class="tpl-card" on:click=move |_| {
                                            if let Some(b) = template_bundle(&key_owned) {
                                                start_project(b);
                                            }
                                        }>
                                            <img src=*thumb alt=*name/>
                                            <span>{*name}</span>
                                        </button>
                                    }
                                }).collect_view()}
                            </div>
                        </div>
                    </div>
                }
            })}
            {move || preview.get().then(|| {
                let (nodes, pname, pw, ph, pbg) = bundle.with(|b| {
                    b.project
                        .pages
                        .get(preview_page.get())
                        .map(|pg| {
                            (
                                page_flat(&b.project, pg, false),
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
                                        let click_action = bundle.with_untracked(|b| preview_action(b, &n, "click"));
                                        let hover_action = bundle.with_untracked(|b| preview_action(b, &n, "hover"));
                                        let mut style = node_css(&n);
                                        if click_action.is_some() || hover_action.is_some() {
                                            style.push_str("cursor:pointer;");
                                        }
                                        let go = move |action: &Option<PreviewAction>| {
                                            if let Some(action) = action {
                                                match action {
                                                    PreviewAction::Route(route) => {
                                                        if let Some(idx) = bundle.with_untracked(|b| {
                                                            b.project.pages.iter().position(|pg| &pg.route == route)
                                                        }) {
                                                            preview_page.set(idx);
                                                        }
                                                    }
                                                    PreviewAction::Url(url) => {
                                                        if let Some(w) = web_sys::window() {
                                                            let _ = w.open_with_url_and_target(url, "_blank");
                                                        }
                                                    }
                                                }
                                            }
                                        };
                                        let label = if n.src.is_some() {
                                            String::new()
                                        } else if n.kind == NodeKind::Image {
                                            "🖼".to_string()
                                        } else {
                                            n.text.clone().unwrap_or_default()
                                        };
                                        let svg_html = node_svg_html(&n);
                                        let has_inter = false;
                                        let id_dbl = n.id.clone();
                                        let is_path_kind = matches!(n.kind, NodeKind::Path | NodeKind::Arrow);
                                        view! {
                                            <div
                                                class="node"
                                                dir="auto"
                                                data-kind=kind_name(n.kind)
                                                data-node-id=n.id.clone()
                                                style=style
                                                on:click=move |_| go(&click_action)
                                                on:mouseenter=move |_| go(&hover_action)
                                                on:dblclick=move |ev: leptos::ev::MouseEvent| {
                                                if is_path_kind {
                                                    ev.stop_propagation();
                                                    let resolved = bundle.with_untracked(|b| {
                                                        b.project
                                                            .pages
                                                            .get(page_index.get_untracked())
                                                            .map(|pg| group_root_of(&pg.root, &id_dbl))
                                                            .unwrap_or_else(|| id_dbl.clone())
                                                    });
                                                    // Point editing works on the
                                                    // path node itself.
                                                    let direct = bundle.with_untracked(|b| {
                                                        b.project
                                                            .pages
                                                            .get(page_index.get_untracked())
                                                            .and_then(|pg| pg.root.find(&id_dbl))
                                                            .map(|_| id_dbl.clone())
                                                    });
                                                    let target = direct.unwrap_or(resolved);
                                                    select_only(target.clone());
                                                    path_edit_id.set(target);
                                                    path_point_sel.set(None);
                                                }
                                            }
                                        >
                                            {label}
                                            {svg_html.map(|html| view! { <span class="node-svg" inner_html=html></span> })}
                                            {has_inter.then(|| view! { <span class="link-badge" title="این عنصر تعامل دارد">"🔗"</span> })}
                                        </div>
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
                        <button on:click=move |_| { group_selection(); }>"گروه بندی"</button>
                        <button on:click=move |_| { ungroup_selection(); }>"باز کردن گروه"</button>
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
