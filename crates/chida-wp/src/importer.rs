//! WordPress theme import: a theme file map becomes a Chida
//! project. Template PHP is preprocessed into markers, the
//! resulting HTML is parsed into nodes, and the theme's CSS is
//! applied as node styles.

use crate::css::{Style, Stylesheet};
use crate::html::{collapse_ws, parse_html, Dom, DomChild};
use chida_core::{Color, LayoutDirection, Node, NodeKind, Page, Project, Theme};
use std::collections::BTreeMap;

/// A theme as a flat file map: slash separated paths to raw bytes.
/// Zips and plain directories both land in this shape.
pub type ThemeFiles = BTreeMap<String, Vec<u8>>;

/// What the import produced, for CLI and editor summaries.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ImportSummary {
    pub pages: usize,
    pub elements: usize,
    pub dynamic_blocks: usize,
    pub dropped_php: usize,
}

/// The outcome of a successful import.
#[derive(Debug, Clone)]
pub struct ImportedTheme {
    pub project: Project,
    pub summary: ImportSummary,
}

/// Import counters shared by the preprocessor and the converter.
#[derive(Default)]
struct Stats {
    dropped_php: usize,
}

/// State for the PHP preprocessor: loop and conditional nesting.
#[derive(Default)]
struct Pre {
    loop_depth: i32,
    inner_if: i32,
    if_depth: i32,
    loop_else_pending: bool,
}

/// Import a WordPress theme from its file map.
pub fn import_theme(files: &ThemeFiles) -> Result<ImportedTheme, String> {
    let (root, files) = normalize_root(files);
    let style_raw = text_of(&files, "style.css")
        .ok_or("this is not a WordPress theme: style.css is missing")?;
    let sheet = Stylesheet::parse(&style_raw);
    let slug = if !root.is_empty() {
        slugify(root.trim_end_matches('/').rsplit('/').next().unwrap_or(""))
    } else {
        slugify(&sheet.header.name)
    };
    let slug = if slug.is_empty() { "theme".to_string() } else { slug };
    let name = if sheet.header.name.is_empty() {
        slug.clone()
    } else {
        sheet.header.name.clone()
    };
    let text_domain = if sheet.header.text_domain.is_empty() {
        slug.clone()
    } else {
        sheet.header.text_domain.clone()
    };

    let mut stats = Stats::default();
    let body_style = sheet.tag_style("body");
    let mut project = Project::new(slug.clone(), name.clone());
    project.description = sheet.header.description.clone();
    project.meta.insert("wp_theme".into(), "true".into());
    project.meta.insert("wp_slug".into(), slug.clone());
    project.meta.insert("wp_text_domain".into(), text_domain);
    if !sheet.header.version.is_empty() {
        project.meta.insert("wp_version".into(), sheet.header.version.clone());
    }
    let mut theme = Theme::default();
    if let Some(bg) = body_style.background {
        theme.background = bg;
    }
    if let Some(fg) = body_style.color {
        theme.text = fg;
    }
    project.theme = theme;

    // (file candidates in priority order, page id, page name, route)
    let specs: &[(&[&str], &str, &str, &str)] = &[
        (&["header.php"], "header", "سربرگ", "/header"),
        (
            &["front-page.php", "home.php", "index.php"],
            "home",
            "خانه",
            "/",
        ),
        (&["single.php"], "single", "نوشته تکی", "/post"),
        (&["page.php"], "page", "برگه", "/page"),
        (&["footer.php"], "footer", "پاصفحه", "/footer"),
    ];
    let mut counter = 0u32;
    for (candidates, id, page_name, route) in specs {
        let Some(file) = candidates.iter().find_map(|c| text_of(&files, c)) else {
            continue;
        };
        let mut pre = Pre::default();
        let markup = preprocess(&file, &files, 0, &mut pre, &mut stats);
        let dom = parse_html(&markup);
        let mut conv = Conv {
            sheet: &sheet,
            counter: &mut counter,
            body_color: body_style.color,
        };
        let mut ancestors = Vec::new();
        let nodes = convert_children(&dom, &mut ancestors, 1200.0, false, body_style.color, &mut conv);
        let mut page = Page::new(*id, *page_name, *route, 1200.0, 900.0);
        page.background = body_style.background.or(Some(Color::WHITE));
        page.root.layout_direction = LayoutDirection::Column;
        page.root.children = nodes;
        let content_h: f32 = page.root.children.iter().map(|n| n.height).sum();
        page.height = content_h.clamp(240.0, 4000.0);
        page.root.height = page.height;
        page.root.apply_layout();
        project.pages.push(page);
    }
    if project.pages.is_empty() {
        return Err(
            "this theme has no readable template (index.php, front-page.php or home.php)"
                .to_string(),
        );
    }
    let dynamic_blocks = count_dynamic(&project);
    let summary = ImportSummary {
        pages: project.pages.len(),
        elements: project.total_nodes(),
        dynamic_blocks,
        dropped_php: stats.dropped_php,
    };
    Ok(ImportedTheme { project, summary })
}

/// Strip the single wrapping folder most theme zips use, and drop
/// macOS resource fork junk.
fn normalize_root(files: &ThemeFiles) -> (String, ThemeFiles) {
    let style_key = files
        .keys()
        .filter(|k| !k.starts_with("__MACOSX"))
        .filter(|k| k.as_str() == "style.css" || k.ends_with("/style.css"))
        .min_by_key(|k| k.matches('/').count())
        .cloned();
    let Some(style_key) = style_key else {
        return (String::new(), files.clone());
    };
    let root = style_key
        .strip_suffix("style.css")
        .unwrap_or("")
        .to_string();
    if root.is_empty() {
        return (root, files.clone());
    }
    let stripped: ThemeFiles = files
        .iter()
        .filter(|(k, _)| !k.starts_with("__MACOSX"))
        .filter_map(|(k, v)| {
            k.strip_prefix(&root).map(|rest| (rest.to_string(), v.clone()))
        })
        .collect();
    (root, stripped)
}

fn text_of(files: &ThemeFiles, name: &str) -> Option<String> {
    files
        .get(name)
        .map(|b| String::from_utf8_lossy(b).into_owned())
}

/// A URL and file system safe slug.
pub fn slugify(raw: &str) -> String {
    let mut out = String::new();
    let mut last_dash = true;
    for ch in raw.trim().chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
            last_dash = false;
        } else if ch == '-' || ch == '_' {
            out.push(ch);
            last_dash = ch == '-';
        } else if !last_dash && !out.is_empty() {
            out.push('-');
            last_dash = true;
        }
    }
    out.trim_matches('-').to_string()
}

// ---------------------------------------------------------------------
// PHP preprocessing
// ---------------------------------------------------------------------

/// Replace the PHP in one template with HTML markers and plain
/// marker text the DOM stage understands. Template parts are
/// inlined (recursively, depth limited) before classification.
fn preprocess(
    text: &str,
    files: &ThemeFiles,
    depth: usize,
    pre: &mut Pre,
    stats: &mut Stats,
) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    loop {
        let Some(start) = rest.find("<?") else {
            out.push_str(rest);
            break;
        };
        let is_php = rest[start..].starts_with("<?php") || rest[start..].starts_with("<?=");
        if !is_php {
            // An XML declaration or similar: keep the text, move on.
            out.push_str(&rest[..start + 2]);
            rest = &rest[start + 2..];
            continue;
        }
        out.push_str(&rest[..start]);
        let code_start = start + if rest[start..].starts_with("<?=") { 3 } else { 5 };
        let after = &rest[code_start..];
        let (code, next) = match after.find("?>") {
            Some(end) => (&after[..end], &after[end + 2..]),
            None => (after, ""),
        };
        let replacement = classify(code, files, depth, pre, stats);
        out.push_str(&replacement);
        rest = next;
    }
    if depth == 0 && pre.loop_depth > 0 {
        for _ in 0..pre.loop_depth {
            out.push_str("</chida-loop>");
        }
        pre.loop_depth = 0;
    }
    out
}

/// The replacement markup (or empty string) for one PHP block.
fn classify(
    code: &str,
    files: &ThemeFiles,
    depth: usize,
    pre: &mut Pre,
    stats: &mut Stats,
) -> String {
    let code = code.trim().trim_end_matches(';').trim();
    let low = code.to_lowercase();

    // Template parts are inlined before anything else.
    if low.contains("get_template_part") {
        if depth < 3 {
            let args = quoted_strings(code);
            let mut candidates: Vec<String> = Vec::new();
            if let Some(slug) = args.first() {
                if let Some(name) = args.get(1) {
                    candidates.push(format!("{slug}-{name}.php"));
                }
                candidates.push(format!("{slug}.php"));
            }
            for cand in candidates {
                if let Some(part) = text_of(files, &cand) {
                    return preprocess(&part, files, depth + 1, pre, stats);
                }
            }
        }
        stats.dropped_php += 1;
        return String::new();
    }

    // Loop close comes before loop open: an `endwhile` block must
    // never be read as a new loop.
    if pre.loop_depth > 0 {
        if low.contains("endwhile") {
            pre.loop_depth -= 1;
            if low.contains("endif") {
                pre.loop_else_pending = false;
            }
            return "</chida-loop>".to_string();
        }
        if low.contains("endif") {
            if pre.inner_if > 0 {
                pre.inner_if -= 1;
                return String::new();
            }
            if pre.loop_else_pending {
                pre.loop_else_pending = false;
                return String::new();
            }
            pre.loop_depth -= 1;
            return "</chida-loop>".to_string();
        }
        if low.trim_start().starts_with("else") {
            // The "no posts" branch lives outside the loop node.
            pre.loop_depth -= 1;
            pre.loop_else_pending = true;
            return "</chida-loop>".to_string();
        }
        if is_conditional_open(&low) && !low.contains("have_posts") {
            pre.inner_if += 1;
            return String::new();
        }
    } else {
        if pre.loop_else_pending && low.contains("endif") {
            pre.loop_else_pending = false;
            return String::new();
        }
        if is_conditional_open(&low) && !low.contains("have_posts") {
            pre.if_depth += 1;
            return String::new();
        }
        if low.contains("endif") {
            pre.if_depth = (pre.if_depth - 1).max(0);
            return String::new();
        }
        if low.trim_start().starts_with("else") || low.contains("endwhile") {
            return String::new();
        }
    }

    // The Loop opens.
    if low.contains("have_posts") && low.contains("the_post") {
        pre.loop_depth += 1;
        return "<chida-loop>".to_string();
    }
    if low.contains("have_posts") {
        // `if ( have_posts() ) :` with the while in the next block,
        // or a bare check: no marker, the while block opens the loop.
        return String::new();
    }

    // Structural calls that produce no page content of their own.
    for needle in [
        "wp_head(",
        "wp_footer(",
        "body_class(",
        "language_attributes(",
        "wp_body_open(",
        "get_header(",
        "get_footer(",
        "get_sidebar(",
        "wp_reset_postdata(",
        "wp_reset_query(",
        "the_post(",
    ] {
        if low.contains(needle) {
            return String::new();
        }
    }

    // The navigation menu becomes a flagged dynamic container.
    if low.contains("wp_nav_menu(") {
        return "<chida-menu></chida-menu>".to_string();
    }

    // Conditionals around content stay transparent.
    if low.contains("has_post_thumbnail(") {
        if pre.loop_depth > 0 {
            pre.inner_if += 1;
        } else {
            pre.if_depth += 1;
        }
        return String::new();
    }

    // The featured image.
    if low.contains("the_post_thumbnail(") || low.contains("get_the_post_thumbnail(") {
        return "<img data-chida-dynamic=\"featured\" src=\"\" alt=\"\">".to_string();
    }

    // Template tags that print text.
    let marker = if low.contains("the_content(") {
        Some("{{post_content}}")
    } else if low.contains("the_excerpt(") || low.contains("get_the_excerpt(") {
        Some("{{post_excerpt}}")
    } else if low.contains("the_title(") || low.contains("get_the_title(") {
        Some("{{post_title}}")
    } else if low.contains("the_permalink(") || low.contains("get_permalink(") {
        Some("{{permalink}}")
    } else if low.contains("the_date(")
        || low.contains("get_the_date(")
        || low.contains("the_time(")
        || low.contains("get_the_time(")
    {
        Some("{{post_date}}")
    } else if low.contains("the_author(") || low.contains("get_the_author(") {
        Some("{{post_author}}")
    } else if low.contains("the_category(")
        || low.contains("the_tags(")
        || low.contains("get_the_category_list(")
        || low.contains("get_the_tag_list(")
        || low.contains("the_terms(")
    {
        Some("{{post_terms}}")
    } else if low.contains("get_template_directory_uri(")
        || low.contains("get_stylesheet_directory_uri(")
    {
        Some("{{theme_uri}}")
    } else if low.contains("home_url(") {
        Some("{{site_url}}")
    } else if low.contains("bloginfo(") || low.contains("get_bloginfo(") {
        match quoted_strings(code).first().map(|s| s.to_lowercase()).as_deref() {
            Some("name") | Some("title") => Some("{{site_title}}"),
            Some("description") => Some("{{site_tagline}}"),
            Some("url") | Some("home") | Some("siteurl") => Some("{{site_url}}"),
            Some("template_url") | Some("template_directory") | Some("stylesheet_url") => {
                Some("{{theme_uri}}")
            }
            // charset, language, version and friends leave no trace.
            Some(_) => return String::new(),
            None => None,
        }
    } else {
        None
    };
    if let Some(marker) = marker {
        return marker.to_string();
    }

    // A lone closing brace ends a braced conditional.
    let trimmed = low.trim();
    if trimmed == "}" || trimmed.starts_with("} ") || trimmed.starts_with("};") {
        if pre.inner_if > 0 {
            pre.inner_if -= 1;
        } else if pre.if_depth > 0 {
            pre.if_depth -= 1;
        }
        return String::new();
    }

    // Remaining control structures are transparent too.
    if low.trim_start().starts_with("while")
        || low.trim_start().starts_with("foreach")
        || low.contains("endforeach")
        || low.trim_start().starts_with("if")
    {
        return String::new();
    }

    // Anything else is PHP we cannot reproduce: drop and count it.
    stats.dropped_php += 1;
    String::new()
}

/// An alternate-syntax or braced conditional opening.
fn is_conditional_open(low: &str) -> bool {
    let t = low.trim_start();
    (t.starts_with("if ") || t.starts_with("if(") || t.starts_with("elseif"))
        && (low.contains(':') || low.contains('{'))
}

/// Every single or double quoted string literal in a code snippet.
fn quoted_strings(code: &str) -> Vec<String> {
    let mut out = Vec::new();
    let chars: Vec<char> = code.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '\'' || chars[i] == '"' {
            let q = chars[i];
            i += 1;
            let mut s = String::new();
            while i < chars.len() && chars[i] != q {
                if chars[i] == '\\' && i + 1 < chars.len() {
                    i += 1;
                }
                s.push(chars[i]);
                i += 1;
            }
            out.push(s);
        }
        i += 1;
    }
    out
}

// ---------------------------------------------------------------------
// DOM to nodes
// ---------------------------------------------------------------------

type Ancestor = (String, Vec<String>, Option<String>);

struct Conv<'a> {
    sheet: &'a Stylesheet,
    counter: &'a mut u32,
    body_color: Option<Color>,
}

impl Conv<'_> {
    fn next_id(&mut self) -> String {
        *self.counter += 1;
        format!("wp-{}", self.counter)
    }
    fn style_of(&self, el: &Dom, ancestors: &[Ancestor]) -> Style {
        self.sheet.resolve(
            &el.tag,
            &el.classes(),
            el.attr("id"),
            ancestors,
            el.attr("style"),
        )
    }
}

const SKIP_TAGS: &[&str] = &["head", "script", "style", "title", "meta", "link", "base"];
const TRANSPARENT_TAGS: &[&str] = &["html", "body"];
const INLINE_TAGS: &[&str] = &[
    "span", "a", "strong", "em", "b", "i", "small", "code", "u", "s", "sub", "sup", "abbr",
    "mark", "time", "label",
];

fn convert_children(
    dom: &Dom,
    ancestors: &mut Vec<Ancestor>,
    avail_w: f32,
    in_row: bool,
    inherited_color: Option<Color>,
    conv: &mut Conv,
) -> Vec<Node> {
    let mut out = Vec::new();
    for child in &dom.children {
        match child {
            DomChild::Text(raw) => {
                let text = collapse_ws(raw);
                if !text.is_empty() {
                    let mut n = Node::text_node(
                        conv.next_id(),
                        "متن",
                        0.0,
                        0.0,
                        avail_w,
                        24.0,
                        text,
                        16.0,
                        inherited_color.unwrap_or(Color::rgb(26, 26, 26)),
                    );
                    n.height = text_height(&n.text.clone().unwrap_or_default(), 16.0, avail_w);
                    out.push(n);
                }
            }
            DomChild::El(el) => {
                if TRANSPARENT_TAGS.contains(&el.tag.as_str()) {
                    ancestors.push((el.tag.clone(), el.classes(), el.attr("id").map(|s| s.to_string())));
                    out.extend(convert_children(el, ancestors, avail_w, in_row, inherited_color, conv));
                    ancestors.pop();
                } else if let Some(node) =
                    convert_element(el, ancestors, avail_w, in_row, inherited_color, conv)
                {
                    out.push(node);
                }
            }
        }
    }
    out
}

fn convert_element(
    el: &Dom,
    ancestors: &mut Vec<Ancestor>,
    avail_w: f32,
    in_row: bool,
    inherited_color: Option<Color>,
    conv: &mut Conv,
) -> Option<Node> {
    let tag = el.tag.as_str();
    if SKIP_TAGS.contains(&tag) {
        return None;
    }
    let style = conv.style_of(el, ancestors);
    // Text inside this element inherits its colour unless a rule
    // closer to the text sets one.
    let inner_color = style.color.or(inherited_color);
    match tag {
        "chida-loop" => {
            let mut n = container_node(conv, el, &style, avail_w, "حلقه نوشته‌ها");
            n.props.insert("wp_loop".into(), "posts".into());
            fill_container(&mut n, el, ancestors, &style, inner_color, conv);
            Some(n)
        }
        "chida-menu" => {
            // Menu items are anchors: a rule like `.main-nav a`
            // colours them, so resolve that style for the items.
            let link_color = conv
                .sheet
                .resolve("a", &[], None, ancestors, None)
                .color;
            Some(menu_node(conv, &style, avail_w, inner_color, link_color))
        }
        "hr" => {
            let mut n = Node::new(conv.next_id(), "خط", NodeKind::Divider, 0.0, 0.0, avail_w, 2.0);
            n.fill = Some(Color::rgb(200, 204, 208));
            Some(n)
        }
        "img" => Some(image_node(conv, el, &style, avail_w)),
        "input" => Some(input_node(conv, el, &style, avail_w)),
        "textarea" | "select" => {
            let mut n = input_node(conv, el, &style, avail_w);
            n.name = "ورودی".into();
            Some(n)
        }
        "button" => Some(button_node(conv, el, &style, avail_w)),
        "h1" | "h2" | "h3" | "h4" | "h5" | "h6" | "p" | "figcaption" => {
            Some(text_block_node(conv, el, &style, avail_w, in_row, inner_color))
        }
        _ if INLINE_TAGS.contains(&tag) => {
            Some(inline_node(conv, el, &style, avail_w, in_row, inner_color))
        }
        _ => {
            // Anchors wrapping block content act as containers.
            if tag == "a" && has_block_children(el) {
                let mut n = container_node(conv, el, &style, avail_w, "پیوند");
                apply_link_props(&mut n, el);
                fill_container(&mut n, el, ancestors, &style, inner_color, conv);
                return Some(n);
            }
            let mut n = container_node(conv, el, &style, avail_w, "بخش");
            fill_container(&mut n, el, ancestors, &style, inner_color, conv);
            Some(n)
        }
    }
}

/// A container node with the element's box styles applied; size is
/// finished by `fill_container` once the children are known.
fn container_node(conv: &mut Conv, el: &Dom, style: &Style, avail_w: f32, name: &str) -> Node {
    let mut n = Node::new(
        conv.next_id(),
        name,
        NodeKind::Frame,
        0.0,
        0.0,
        style.width.unwrap_or(avail_w),
        0.0,
    );
    n.fill = style.background;
    n.text_color = style.color;
    if let Some(r) = style.radius {
        n.corner_radius = r;
    }
    n.props.insert("tag".into(), el.tag.clone());
    if let Some(align) = &style.align {
        n.props.insert("align".into(), align.clone());
    }
    if style.flex {
        n.layout_direction = if style.flex_row {
            LayoutDirection::Row
        } else {
            LayoutDirection::Column
        };
    } else {
        n.layout_direction = LayoutDirection::Column;
    }
    n.layout_gap = style.gap.unwrap_or(0.0);
    n.layout_padding = style.padding.unwrap_or(0.0);
    n
}

fn fill_container(
    n: &mut Node,
    el: &Dom,
    ancestors: &mut Vec<Ancestor>,
    style: &Style,
    inner_color: Option<Color>,
    conv: &mut Conv,
) {
    ancestors.push((el.tag.clone(), el.classes(), el.attr("id").map(|s| s.to_string())));
    let row = n.layout_direction == LayoutDirection::Row;
    let content_w = (n.width - 2.0 * n.layout_padding).max(40.0);
    let share = if row && !el.children.is_empty() {
        content_w / el.children.len().max(1) as f32
    } else {
        content_w
    };
    let child_avail = if row { share } else { content_w };
    n.children = convert_children(el, ancestors, child_avail, row, inner_color, conv);
    ancestors.pop();
    // A container holding nothing but one plain text run keeps the
    // text on itself, so it exports as one styled element instead
    // of a box around a paragraph.
    if n.children.len() == 1 {
        let only = &n.children[0];
        let plain_text = only.kind == NodeKind::Text
            && !only.props.contains_key("href")
            && !only.props.contains_key("wp_dynamic")
            && !only.props.contains_key("tag");
        if plain_text {
            let child = n.children.remove(0);
            let child_weight = child.effective_weight();
            let child_height = child.height;
            n.text = child.text;
            if n.text_color.is_none() {
                n.text_color = child.text_color;
            }
            n.font_size = style.font_size.unwrap_or(child.font_size);
            let weight = style.font_weight.unwrap_or(child_weight);
            n.font_weight = weight;
            n.bold = weight >= 600;
            n.height = style
                .height
                .unwrap_or_else(|| child_height + 2.0 * n.layout_padding);
            return;
        }
    }
    let pad = n.layout_padding;
    let computed = if row {
        2.0 * pad + n.children.iter().map(|c| c.height).fold(0.0f32, f32::max)
    } else {
        2.0 * pad
            + n.children.iter().map(|c| c.height).sum::<f32>()
            + n.layout_gap * (n.children.len().saturating_sub(1) as f32)
    };
    n.height = style.height.unwrap_or(if n.children.is_empty() {
        (2.0 * pad).max(24.0)
    } else {
        computed
    });
}

/// The dynamic navigation menu: a row flagged for wp_nav_menu,
/// with placeholder items a user can restyle.
fn menu_node(
    conv: &mut Conv,
    style: &Style,
    avail_w: f32,
    inner_color: Option<Color>,
    link_color: Option<Color>,
) -> Node {
    let mut n = Node::new(
        conv.next_id(),
        "منوی اصلی",
        NodeKind::Frame,
        0.0,
        0.0,
        style.width.unwrap_or(avail_w),
        28.0,
    );
    n.layout_direction = LayoutDirection::Row;
    n.layout_gap = 24.0;
    n.props.insert("wp_dynamic".into(), "menu".into());
    for label in ["خانه", "برگه نمونه", "تماس"] {
        let mut item = Node::text_node(
            conv.next_id(),
            "پیوند منو",
            0.0,
            0.0,
            text_width_est(label, 16.0),
            24.0,
            label,
            16.0,
            style
                .color
                .or(link_color)
                .or(inner_color)
                .unwrap_or_else(|| conv.body_color.unwrap_or(Color::rgb(26, 26, 26))),
        );
        item.props.insert("href".into(), "#".into());
        n.children.push(item);
    }
    n
}

fn image_node(conv: &mut Conv, el: &Dom, style: &Style, avail_w: f32) -> Node {
    let featured = el.attr("data-chida-dynamic") == Some("featured");
    let raw_src = el.attr("src").unwrap_or("");
    let mut src = clean_url(raw_src).0;
    if featured {
        src = String::new();
    }
    let w = style
        .width
        .or_else(|| el.attr("width").and_then(|v| v.parse().ok()))
        .unwrap_or(if featured { 800.0 } else { 480.0 })
        .min(avail_w.max(480.0));
    let h = style
        .height
        .or_else(|| el.attr("height").and_then(|v| v.parse().ok()))
        .unwrap_or(if featured { 400.0 } else { 300.0 });
    let mut n = Node::image(conv.next_id(), src, 0.0, 0.0, w, h);
    n.name = if featured { "عکس شاخص".into() } else { "عکس".into() };
    if featured {
        n.props.insert("wp_dynamic".into(), "featured".into());
    }
    if let Some(alt) = el.attr("alt") {
        if !alt.is_empty() {
            n.props.insert("alt".into(), alt.to_string());
        }
    }
    if let Some(r) = style.radius {
        n.corner_radius = r;
    }
    n
}

fn input_node(conv: &mut Conv, el: &Dom, style: &Style, avail_w: f32) -> Node {
    let input_type = el.attr("type").unwrap_or("text").to_lowercase();
    if input_type == "submit" {
        let label = el.attr("value").unwrap_or("ارسال");
        let mut b = Node::new(
            conv.next_id(),
            "دکمه",
            NodeKind::Button,
            0.0,
            0.0,
            style.width.unwrap_or(140.0),
            style.height.unwrap_or(44.0),
        );
        b.fill = style.background;
        if let Some(r) = style.radius {
            b.corner_radius = r;
        }
        b.props.insert("tag".into(), "input".into());
        let mut t = Node::text_node(
            conv.next_id(),
            "متن دکمه",
            0.0,
            (b.height - 24.0) / 2.0,
            b.width,
            24.0,
            label,
            15.0,
            style.color.unwrap_or(Color::WHITE),
        );
        t.props.insert("align".into(), "center".into());
        b.children.push(t);
        return b;
    }
    let w = style.width.unwrap_or(avail_w.min(320.0));
    let h = style.height.unwrap_or(44.0);
    let mut n = Node::new(conv.next_id(), "ورودی", NodeKind::Input, 0.0, 0.0, w, h);
    if let Some(r) = style.radius {
        n.corner_radius = r;
    }
    n.props.insert("type".into(), input_type);
    let placeholder = el.attr("placeholder").unwrap_or("");
    if !placeholder.is_empty() {
        n.props.insert("placeholder".into(), placeholder.to_string());
        let mut t = Node::text_node(
            conv.next_id(),
            "راهنما",
            12.0,
            (h - 22.0) / 2.0,
            (w - 24.0).max(20.0),
            22.0,
            placeholder,
            14.0,
            Color::rgb(120, 126, 130),
        );
        t.props.remove("align");
        n.children.push(t);
    }
    n
}

fn button_node(conv: &mut Conv, el: &Dom, style: &Style, avail_w: f32) -> Node {
    let (text, _, _) = gather_text(el);
    let font_size = style.font_size.unwrap_or(15.0);
    let pad = style.padding.unwrap_or(14.0);
    let w = style
        .width
        .unwrap_or_else(|| (text_width_est(&text, font_size) + 2.0 * pad).max(100.0))
        .min(avail_w.max(100.0));
    let h = style.height.unwrap_or_else(|| font_size * 1.45 + 2.0 * (pad * 0.6));
    let mut b = Node::new(conv.next_id(), "دکمه", NodeKind::Button, 0.0, 0.0, w, h);
    b.fill = style.background;
    if let Some(r) = style.radius {
        b.corner_radius = r;
    }
    b.props.insert("tag".into(), "button".into());
    apply_link_props(&mut b, el);
    if !text.is_empty() {
        let mut t = Node::text_node(
            conv.next_id(),
            "متن دکمه",
            0.0,
            (h - font_size * 1.45) / 2.0,
            w,
            font_size * 1.45,
            text,
            font_size,
            style.color.unwrap_or(Color::WHITE),
        );
        t.props.insert("align".into(), "center".into());
        if let Some(weight) = style.font_weight {
            t.font_weight = weight;
            t.bold = weight >= 600;
        }
        b.children.push(t);
    }
    b
}

/// A heading or paragraph: one text node with the element's whole
/// text (inline descendants merged in).
fn text_block_node(
    conv: &mut Conv,
    el: &Dom,
    style: &Style,
    avail_w: f32,
    in_row: bool,
    inner_color: Option<Color>,
) -> Node {
    let (text, permalink, href) = gather_text(el);
    let (def_size, def_weight) = match el.tag.as_str() {
        "h1" => (32.0, 700),
        "h2" => (26.0, 700),
        "h3" => (22.0, 700),
        "h4" => (18.0, 700),
        "h5" => (16.0, 700),
        "h6" => (14.0, 700),
        _ => (16.0, 400),
    };
    let font_size = style.font_size.unwrap_or(def_size);
    let width = style.width.unwrap_or(if in_row {
        text_width_est(&text, font_size).min(avail_w).max(30.0)
    } else {
        avail_w
    });
    let mut n = Node::text_node(
        conv.next_id(),
        kind_label(&el.tag),
        0.0,
        0.0,
        width,
        0.0,
        text.clone(),
        font_size,
        style
            .color
            .or(inner_color)
            .unwrap_or_else(|| conv.body_color.unwrap_or(Color::rgb(26, 26, 26))),
    );
    n.height = style
        .height
        .unwrap_or_else(|| text_height(&text, font_size, width));
    let weight = style.font_weight.unwrap_or(def_weight);
    n.font_weight = weight;
    n.bold = weight >= 600;
    n.props.insert("tag".into(), el.tag.clone());
    if let Some(align) = &style.align {
        n.props.insert("align".into(), align.clone());
    }
    if permalink {
        n.props.insert("wp_dynamic".into(), "permalink".into());
    } else if let Some(href) = href {
        n.props.insert("href".into(), href);
    }
    n
}

/// An inline element (span, a, strong, ...) becomes a text node.
fn inline_node(
    conv: &mut Conv,
    el: &Dom,
    style: &Style,
    avail_w: f32,
    in_row: bool,
    inner_color: Option<Color>,
) -> Node {
    let (text, permalink, href) = gather_text(el);
    let font_size = style.font_size.unwrap_or(16.0);
    let width = style.width.unwrap_or(if in_row {
        text_width_est(&text, font_size).min(avail_w).max(30.0)
    } else {
        avail_w
    });
    let mut n = Node::text_node(
        conv.next_id(),
        kind_label(&el.tag),
        0.0,
        0.0,
        width,
        0.0,
        text.clone(),
        font_size,
        style
            .color
            .or(inner_color)
            .unwrap_or_else(|| conv.body_color.unwrap_or(Color::rgb(26, 26, 26))),
    );
    n.height = text_height(&text, font_size, width);
    let weight = style
        .font_weight
        .unwrap_or(if matches!(el.tag.as_str(), "strong" | "b") {
            700
        } else {
            400
        });
    n.font_weight = weight;
    n.bold = weight >= 600;
    if el.tag == "a" {
        if permalink {
            n.props.insert("wp_dynamic".into(), "permalink".into());
        } else {
            n.props
                .insert("href".into(), href.unwrap_or_else(|| "#".into()));
        }
    }
    if let Some(align) = &style.align {
        n.props.insert("align".into(), align.clone());
    }
    n
}

fn kind_label(tag: &str) -> &'static str {
    match tag {
        "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => "تیتر",
        "a" => "پیوند",
        _ => "متن",
    }
}

fn has_block_children(el: &Dom) -> bool {
    el.children.iter().any(|c| match c {
        DomChild::El(child) => {
            !INLINE_TAGS.contains(&child.tag.as_str())
                && !matches!(child.tag.as_str(), "img" | "br" | "input")
        }
        DomChild::Text(_) => false,
    })
}

/// Set link props on a node from an element's href attribute.
fn apply_link_props(n: &mut Node, el: &Dom) {
    if let Some(raw) = el.attr("href") {
        let (href, permalink) = clean_url(raw);
        if permalink {
            n.props.insert("wp_dynamic".into(), "permalink".into());
        } else if !href.is_empty() {
            n.props.insert("href".into(), href);
        }
    }
}

/// Clean a URL attribute that may contain importer markers:
/// returns the stored href plus whether it is the permalink marker.
fn clean_url(raw: &str) -> (String, bool) {
    let raw = raw.trim();
    if raw.contains("{{permalink}}") {
        return ("#".to_string(), true);
    }
    if raw == "{{site_url}}" {
        return ("/".to_string(), false);
    }
    if let Some(rest) = raw.strip_prefix("{{theme_uri}}") {
        return (rest.trim_start_matches('/').to_string(), false);
    }
    if raw.contains("{{") {
        return ("#".to_string(), false);
    }
    (raw.to_string(), false)
}

/// The full text of an element for a merged text node, plus the
/// permalink flag and the single descendant link target, if any.
fn gather_text(el: &Dom) -> (String, bool, Option<String>) {
    let mut href: Option<String> = None;
    let mut permalink = false;
    fn scan(el: &Dom, href: &mut Option<String>, permalink: &mut bool) {
        for child in &el.children {
            if let DomChild::El(c) = child {
                if c.tag == "a" {
                    if let Some(raw) = c.attr("href") {
                        let (h, p) = clean_url(raw);
                        if p {
                            *permalink = true;
                        } else if href.is_none() {
                            *href = Some(h);
                        }
                    }
                }
                scan(c, href, permalink);
            }
        }
    }
    scan(el, &mut href, &mut permalink);
    (el.text_content(), permalink, href)
}

/// The display length of a text once dynamic markers stand in for
/// the content they will print.
fn display_len(text: &str) -> usize {
    let mut len = 0usize;
    let mut rest = text;
    while let Some(start) = rest.find("{{") {
        len += rest[..start].chars().count();
        match rest[start..].find("}}") {
            Some(end) => {
                len += match &rest[start + 2..start + end] {
                    "post_content" => 400,
                    "post_excerpt" => 160,
                    "post_title" => 30,
                    _ => 12,
                };
                rest = &rest[start + end + 2..];
            }
            None => {
                len += rest[start..].chars().count();
                rest = "";
            }
        }
    }
    len + rest.chars().count()
}

fn text_width_est(text: &str, font_size: f32) -> f32 {
    display_len(text) as f32 * font_size * 0.52
}

fn text_height(text: &str, font_size: f32, width: f32) -> f32 {
    let char_w = (font_size * 0.52).max(1.0);
    let per_line = ((width / char_w) as usize).max(1);
    let lines = display_len(text).div_ceil(per_line).max(1);
    (lines as f32 * font_size * 1.45).round()
}

/// Count the nodes that stand for live WordPress output: loop
/// containers, dynamic blocks and texts holding markers.
fn count_dynamic(project: &Project) -> usize {
    let mut count = 0;
    for page in &project.pages {
        page.root.walk(&mut |n| {
            let dynamic = n.props.contains_key("wp_loop")
                || n.props.contains_key("wp_dynamic")
                || n.text
                    .as_deref()
                    .map(|t| t.contains("{{"))
                    .unwrap_or(false);
            if dynamic {
                count += 1;
            }
        });
    }
    count
}
