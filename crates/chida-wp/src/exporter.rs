//! WordPress theme export: a Chida project becomes the files of
//! an installable theme. Pages are matched to templates by their
//! Persian names (or the ids the importer gives them), node styles
//! become generated CSS classes, and dynamic markers become real
//! WordPress template tags.

use crate::importer::slugify;
use chida_core::{LayoutDirection, Node, NodeKind, Page, Project};
use std::collections::BTreeMap;

/// Export the project as theme files: path to file contents.
pub fn export_theme(project: &Project) -> BTreeMap<String, String> {
    let slug = project
        .meta
        .get("wp_slug")
        .cloned()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| slugify(&project.name));
    let slug = if slug.is_empty() {
        slugify(&project.id)
    } else {
        slug
    };
    let domain = project
        .meta
        .get("wp_text_domain")
        .cloned()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| slug.clone());

    let header_page = find_page(project, &["سربرگ"], &["header"]);
    let footer_page = find_page(project, &["پاصفحه"], &["footer"]);
    let home_page = find_page(project, &["خانه"], &["home"]).or_else(|| {
        project
            .pages
            .iter()
            .find(|p| Some(*p) != header_page && Some(*p) != footer_page)
    });
    let single_page = find_page(project, &["نوشته تکی"], &["single"]);
    let page_page = find_page(project, &["برگه"], &["page"]);

    let mut css = CssGen::default();
    let mut files: BTreeMap<String, String> = BTreeMap::new();

    let header_content = header_page.map(|p| render_page(project, p, &mut css));
    let home_content = home_page.map(|p| render_page(project, p, &mut css));
    let single_content = single_page.map(|p| render_page(project, p, &mut css));
    let page_content = page_page.map(|p| render_page(project, p, &mut css));
    let footer_content = footer_page.map(|p| render_page(project, p, &mut css));

    let mut extras: Vec<(&Page, String)> = Vec::new();
    for p in &project.pages {
        if [header_page, footer_page, home_page, single_page, page_page].contains(&Some(p)) {
            continue;
        }
        let raw = if p.route != "/" && !p.route.is_empty() {
            p.route.trim_matches('/').replace('/', "-")
        } else {
            p.id.clone()
        };
        let s = slugify(&raw);
        if s.is_empty() || ["index", "single", "page", "header", "footer"].contains(&s.as_str()) {
            continue;
        }
        extras.push((p, s));
    }
    let extra_contents: Vec<(String, String)> = extras
        .iter()
        .map(|(p, s)| (s.clone(), render_page(project, p, &mut css)))
        .collect();

    if let Some(content) = &header_content {
        files.insert(
            "header.php".into(),
            format!(
                "<!DOCTYPE html>\n<html <?php language_attributes(); ?>>\n<head>\n<meta charset=\"<?php bloginfo( 'charset' ); ?>\">\n<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n<?php wp_head(); ?>\n</head>\n<body <?php body_class(); ?>>\n{content}\n"
            ),
        );
    }
    if let Some(content) = &footer_content {
        files.insert(
            "footer.php".into(),
            format!("{content}\n<?php wp_footer(); ?>\n</body>\n</html>\n"),
        );
    }
    let wrap = |content: &str| -> String {
        let mut out = String::new();
        if header_page.is_some() {
            out.push_str("<?php get_header(); ?>\n\n");
        }
        out.push_str(content);
        out.push('\n');
        if footer_page.is_some() {
            out.push_str("\n<?php get_footer(); ?>\n");
        }
        out
    };
    if let Some(content) = &home_content {
        files.insert("index.php".into(), wrap(content));
    }
    if let Some(content) = &single_content {
        files.insert("single.php".into(), wrap(content));
    }
    if let Some(content) = &page_content {
        files.insert("page.php".into(), wrap(content));
    }
    for (slug_name, content) in extra_contents {
        files.insert(format!("page-{slug_name}.php"), wrap(&content));
    }
    files.insert(
        "functions.php".into(),
        functions_php(project, &slug, &domain),
    );
    files.insert("style.css".into(), style_css(project, &domain, &css));

    files
}

fn find_page<'a>(project: &'a Project, names: &[&str], ids: &[&str]) -> Option<&'a Page> {
    project
        .pages
        .iter()
        .find(|p| names.contains(&p.name.as_str()))
        .or_else(|| project.pages.iter().find(|p| ids.contains(&p.id.as_str())))
}

fn render_page(project: &Project, page: &Page, css: &mut CssGen) -> String {
    page.root
        .children
        .iter()
        .map(|n| project.resolve_node(n))
        .map(|n| render_node(&n, 0, css))
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

fn render_children(node: &Node, indent: usize, css: &mut CssGen) -> String {
    node.children
        .iter()
        .map(|n| render_node(n, indent, css))
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

fn render_node(node: &Node, indent: usize, css: &mut CssGen) -> String {
    if !node.visible {
        return String::new();
    }
    let pad = "  ".repeat(indent);
    let dynamic = node.props.get("wp_dynamic").map(|s| s.as_str());
    let is_loop = node
        .props
        .get("wp_loop")
        .map(|s| s == "posts")
        .unwrap_or(false);

    if is_loop {
        let inner = render_children(node, indent, css);
        return format!(
            "{pad}<?php if ( have_posts() ) : while ( have_posts() ) : the_post(); ?>\n{inner}\n{pad}<?php endwhile; endif; ?>"
        );
    }
    if dynamic == Some("menu") {
        let class = class_attr(node, css);
        return format!(
            "{pad}<nav{class}>\n{pad}  <?php wp_nav_menu( array( 'theme_location' => 'primary', 'container' => false ) ); ?>\n{pad}</nav>"
        );
    }
    if dynamic == Some("featured") {
        let cls = css.class_for(node);
        return match cls {
            Some(c) => {
                format!("{pad}<?php the_post_thumbnail( 'large', array( 'class' => '{c}' ) ); ?>")
            }
            None => format!("{pad}<?php the_post_thumbnail( 'large' ); ?>"),
        };
    }

    match node.kind {
        NodeKind::Text => render_text_node(node, &pad, css),
        NodeKind::Button => render_button(node, &pad, css),
        NodeKind::Input => render_input(node, &pad, css),
        NodeKind::Image => {
            let src = node.image_src().unwrap_or("");
            if src.is_empty() {
                return String::new();
            }
            let class = class_attr(node, css);
            let alt = node.props.get("alt").map(|s| s.as_str()).unwrap_or("");
            let src_attr = if src.starts_with("http://")
                || src.starts_with("https://")
                || src.starts_with("data:")
                || src.starts_with('/')
            {
                escape_attr(src)
            } else {
                format!(
                    "<?php echo esc_url( get_template_directory_uri() ); ?>/{}",
                    escape_attr(src)
                )
            };
            format!(
                "{pad}<img{class} src=\"{src_attr}\" alt=\"{}\" />",
                escape_attr(alt)
            )
        }
        NodeKind::Path | NodeKind::Arrow => {
            let class = class_attr(node, css);
            let fill = node
                .fill
                .map(|c| c.to_hex())
                .unwrap_or_else(|| "none".into());
            let stroke = node
                .stroke
                .map(|c| c.to_hex())
                .unwrap_or_else(|| fill.clone());
            if node.kind == NodeKind::Arrow {
                let head = node.arrow_head_points().unwrap_or_default();
                if node.path_points.len() < 2 || head.len() < 3 {
                    return String::new();
                }
                let a = node.path_points[0];
                let b = node.path_points[1];
                format!("{pad}<svg{class} viewBox=\"0 0 {} {}\" width=\"{}\" height=\"{}\"><path d=\"M {} {} L {} {}\" fill=\"none\" stroke=\"{}\" stroke-width=\"{}\"/><path d=\"M {} {} L {} {} L {} {} Z\" fill=\"{}\"/></svg>", node.width, node.height, node.width, node.height, a.x, a.y, b.x, b.y, stroke, node.stroke_width.max(1.5), head[0].0, head[0].1, head[1].0, head[1].1, head[2].0, head[2].1, stroke)
            } else {
                format!("{pad}<svg{class} viewBox=\"0 0 {} {}\" width=\"{}\" height=\"{}\"><path d=\"{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"{}\"/></svg>", node.width, node.height, node.width, node.height, node.path_d(0.0, 0.0), fill, stroke, node.stroke_width)
            }
        }
        NodeKind::Divider => {
            let class = class_attr(node, css);
            format!("{pad}<hr{class} />")
        }
        _ => {
            // A container whose only child is the dynamic menu
            // renders as the menu's own nav element, not nav in nav.
            if node.children.len() == 1
                && node.children[0].props.get("wp_dynamic").map(|s| s.as_str()) == Some("menu")
            {
                let class = class_attr(node, css);
                return format!(
                    "{pad}<nav{class}>\n{pad}  <?php wp_nav_menu( array( 'theme_location' => 'primary', 'container' => false ) ); ?>\n{pad}</nav>"
                );
            }
            // Containers and every remaining kind become a box
            // element; the original tag survives when it is a
            // plain HTML structural tag.
            let tag = node
                .props
                .get("tag")
                .map(|s| s.as_str())
                .filter(|t| {
                    [
                        "header", "footer", "nav", "section", "main", "article", "aside", "div",
                        "ul", "ol", "li", "form", "figure",
                    ]
                    .contains(t)
                })
                .unwrap_or("div");
            let class = class_attr(node, css);
            let inner = if !node.children.is_empty() {
                let kids = render_children(node, indent + 1, css);
                format!("\n{kids}\n{pad}")
            } else if let Some(text) = &node.text {
                markers_to_php(text)
            } else {
                String::new()
            };
            format!("{pad}<{tag}{class}>{inner}</{tag}>")
        }
    }
}

fn render_text_node(node: &Node, pad: &str, css: &mut CssGen) -> String {
    let text = node.text.clone().unwrap_or_default();
    let content = markers_to_php(&text);
    let class = class_attr(node, css);
    let tag = node.props.get("tag").map(|s| s.as_str());
    let permalink = node
        .props
        .get("wp_dynamic")
        .map(|s| s == "permalink")
        .unwrap_or(false);
    let href = node.props.get("href").map(|s| s.as_str());
    let link_href: Option<String> = if permalink {
        Some("<?php the_permalink(); ?>".to_string())
    } else {
        href.map(escape_attr)
    };
    match (tag, link_href) {
        (Some(t), Some(h))
            if ["h1", "h2", "h3", "h4", "h5", "h6", "p", "span", "li"].contains(&t) =>
        {
            format!("{pad}<{t}{class}><a href=\"{h}\">{content}</a></{t}>")
        }
        (_, Some(h)) => format!("{pad}<a{class} href=\"{h}\">{content}</a>"),
        (Some(t), None) if ["h1", "h2", "h3", "h4", "h5", "h6", "p", "span"].contains(&t) => {
            format!("{pad}<{t}{class}>{content}</{t}>")
        }
        _ => format!("{pad}<p{class}>{content}</p>"),
    }
}

fn render_button(node: &Node, pad: &str, css: &mut CssGen) -> String {
    let class = class_attr(node, css);
    let label = if !node.children.is_empty() {
        node.children
            .iter()
            .filter(|c| c.kind == NodeKind::Text)
            .map(|c| markers_to_php(c.text.as_deref().unwrap_or("")))
            .collect::<Vec<_>>()
            .join(" ")
    } else {
        markers_to_php(node.text.as_deref().unwrap_or(""))
    };
    match node.props.get("tag").map(|s| s.as_str()) {
        Some("button") => format!("{pad}<button{class}>{label}</button>"),
        Some("input") => {
            let plain: String = label
                .replace("<?php the_title(); ?>", "")
                .replace("<?php the_content(); ?>", "");
            format!(
                "{pad}<input{class} type=\"submit\" value=\"{}\" />",
                escape_attr(plain.trim())
            )
        }
        _ => {
            let permalink = node
                .props
                .get("wp_dynamic")
                .map(|s| s == "permalink")
                .unwrap_or(false);
            let href = if permalink {
                "<?php the_permalink(); ?>".to_string()
            } else {
                escape_attr(node.props.get("href").map(|s| s.as_str()).unwrap_or("#"))
            };
            format!("{pad}<a{class} href=\"{href}\">{label}</a>")
        }
    }
}

fn render_input(node: &Node, pad: &str, css: &mut CssGen) -> String {
    let class = class_attr(node, css);
    let input_type = node.props.get("type").map(|s| s.as_str()).unwrap_or("text");
    let placeholder = node
        .props
        .get("placeholder")
        .cloned()
        .or_else(|| node.children.first().and_then(|c| c.text.clone()))
        .unwrap_or_default();
    format!(
        "{pad}<input{class} type=\"{}\" placeholder=\"{}\" />",
        escape_attr(input_type),
        escape_attr(&placeholder)
    )
}

/// Replace dynamic markers in a text with real template tags,
/// escaping the literal text around them.
pub fn markers_to_php(text: &str) -> String {
    const MARKERS: &[(&str, &str)] = &[
        ("{{post_title}}", "<?php the_title(); ?>"),
        ("{{post_content}}", "<?php the_content(); ?>"),
        ("{{post_excerpt}}", "<?php the_excerpt(); ?>"),
        ("{{post_date}}", "<?php echo esc_html( get_the_date() ); ?>"),
        ("{{post_author}}", "<?php the_author(); ?>"),
        ("{{post_terms}}", "<?php the_category( ', ' ); ?>"),
        ("{{site_title}}", "<?php bloginfo( 'name' ); ?>"),
        ("{{site_tagline}}", "<?php bloginfo( 'description' ); ?>"),
        ("{{site_url}}", "<?php echo esc_url( home_url( '/' ) ); ?>"),
        ("{{permalink}}", "<?php the_permalink(); ?>"),
        (
            "{{theme_uri}}",
            "<?php echo esc_url( get_template_directory_uri() ); ?>",
        ),
    ];
    let mut out = String::new();
    let mut rest = text;
    while !rest.is_empty() {
        // Find the earliest marker occurrence.
        let mut best: Option<(usize, &str, &str)> = None;
        for (marker, php) in MARKERS {
            if let Some(pos) = rest.find(marker) {
                if best.map(|(p, _, _)| pos < p).unwrap_or(true) {
                    best = Some((pos, marker, php));
                }
            }
        }
        match best {
            Some((pos, marker, php)) => {
                out.push_str(&escape_html(&rest[..pos]));
                out.push_str(php);
                rest = &rest[pos + marker.len()..];
            }
            None => {
                out.push_str(&escape_html(rest));
                break;
            }
        }
    }
    out
}

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn escape_attr(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// The generated `class="..."` attribute for a node, or empty.
fn class_attr(node: &Node, css: &mut CssGen) -> String {
    match css.class_for(node) {
        Some(c) => format!(" class=\"{c}\""),
        None => String::new(),
    }
}

/// Accumulates the generated CSS classes while pages render.
#[derive(Default)]
struct CssGen {
    seen: BTreeMap<String, String>,
    rules: Vec<String>,
    next: usize,
}

impl CssGen {
    fn class_for(&mut self, node: &Node) -> Option<String> {
        let decls = node_decls(node);
        if decls.is_empty() {
            return None;
        }
        let signature = decls
            .iter()
            .map(|(k, v)| format!("{k}:{v}"))
            .collect::<Vec<_>>()
            .join(";");
        if let Some(existing) = self.seen.get(&signature) {
            return Some(existing.clone());
        }
        self.next += 1;
        let name = format!("cdx-{}", self.next);
        let body = decls
            .iter()
            .map(|(k, v)| format!("  {k}: {v};"))
            .collect::<Vec<_>>()
            .join("\n");
        self.rules.push(format!(".{name} {{\n{body}\n}}"));
        self.seen.insert(signature, name.clone());
        Some(name)
    }
}

/// The CSS declarations one node's own styles produce.
fn node_decls(node: &Node) -> Vec<(String, String)> {
    let mut d: Vec<(String, String)> = Vec::new();
    if let Some(g) = &node.fill_gradient {
        d.push(("background".into(), g.css()));
    } else if let Some(fill) = node.fill {
        if fill != chida_core::Color::TRANSPARENT {
            d.push(("background".into(), fill.to_hex()));
        }
    }
    if let Some(c) = node.text_color {
        // For containers the colour is inherited by children in
        // CSS, so it belongs in the generated class either way.
        d.push(("color".into(), c.to_hex()));
    }
    if matches!(
        node.kind,
        NodeKind::Text | NodeKind::Button | NodeKind::Input
    ) || node.text.is_some()
    {
        if node.font_size > 0.0 && node.font_size != 16.0 {
            d.push(("font-size".into(), format!("{}px", fmt_num(node.font_size))));
        }
        let weight = node.effective_weight();
        if weight != 400 {
            d.push(("font-weight".into(), weight.to_string()));
        }
        if node.line_height > 0.0 {
            d.push(("line-height".into(), fmt_num(node.line_height)));
        }
        if node.letter_spacing != 0.0 {
            d.push((
                "letter-spacing".into(),
                format!("{}px", fmt_num(node.letter_spacing)),
            ));
        }
    }
    if let Some(align) = node.props.get("align") {
        if align != "left" {
            d.push(("text-align".into(), align.clone()));
        }
    }
    if node.corner_radius > 0.0 {
        d.push((
            "border-radius".into(),
            format!("{}px", fmt_num(node.corner_radius)),
        ));
    }
    if let Some(stroke) = node.stroke {
        if node.stroke_width > 0.0 {
            d.push((
                "border".into(),
                format!("{}px solid {}", fmt_num(node.stroke_width), stroke.to_hex()),
            ));
        }
    }
    if node.opacity < 1.0 {
        d.push(("opacity".into(), fmt_num(node.opacity)));
    }
    match node.layout_direction {
        LayoutDirection::Row => {
            d.push(("display".into(), "flex".into()));
            d.push(("flex-direction".into(), "row".into()));
        }
        LayoutDirection::Column => {
            d.push(("display".into(), "flex".into()));
            d.push(("flex-direction".into(), "column".into()));
        }
        LayoutDirection::None => {}
    }
    if node.has_layout() && node.layout_gap > 0.0 {
        d.push(("gap".into(), format!("{}px", fmt_num(node.layout_gap))));
    }
    let (pt, pr, pb, pl) = node.padding_values();
    if pt > 0.0 || pr > 0.0 || pb > 0.0 || pl > 0.0 {
        if pt == pr && pr == pb && pb == pl {
            d.push(("padding".into(), format!("{}px", fmt_num(pt))));
        } else {
            d.push((
                "padding".into(),
                format!(
                    "{}px {}px {}px {}px",
                    fmt_num(pt),
                    fmt_num(pr),
                    fmt_num(pb),
                    fmt_num(pl)
                ),
            ));
        }
    }
    if node.has_layout() && node.layout_wrap {
        d.push(("flex-wrap".into(), "wrap".into()));
    }
    if !node.shadows.is_empty() {
        let shadows = node
            .shadows
            .iter()
            .filter(|sh| sh.enabled)
            .map(|sh| {
                let a = (sh.color.a as f32 / 255.0) * sh.opacity.clamp(0.0, 1.0);
                format!(
                    "{}px {}px {}px rgba({},{},{},{:.3})",
                    fmt_num(sh.x),
                    fmt_num(sh.y),
                    fmt_num(sh.blur),
                    sh.color.r,
                    sh.color.g,
                    sh.color.b,
                    a
                )
            })
            .collect::<Vec<_>>()
            .join(", ");
        d.push(("box-shadow".into(), shadows));
    }
    if node.blur_radius > 0.0 {
        d.push((
            "filter".into(),
            format!("blur({}px)", fmt_num(node.blur_radius)),
        ));
    }
    if node.background_blur > 0.0 {
        d.push((
            "backdrop-filter".into(),
            format!("blur({}px)", fmt_num(node.background_blur)),
        ));
    }
    d
}

fn fmt_num(v: f32) -> String {
    if v == v.trunc() {
        format!("{}", v as i64)
    } else {
        format!("{v:.2}")
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_string()
    }
}

fn functions_php(project: &Project, slug: &str, domain: &str) -> String {
    let mut ident: String = slug
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect();
    if ident
        .chars()
        .next()
        .map(|c| !c.is_ascii_alphabetic())
        .unwrap_or(true)
    {
        ident = format!("chida_{ident}");
    }
    format!(
        "<?php\n/**\n * Functions for the {name} theme, exported from Chida.\n */\n\nfunction {ident}_setup() {{\n    add_theme_support( 'title-tag' );\n    add_theme_support( 'post-thumbnails' );\n    register_nav_menus( array(\n        'primary' => __( 'Primary Menu', '{domain}' ),\n    ) );\n}}\nadd_action( 'after_setup_theme', '{ident}_setup' );\n\nfunction {ident}_scripts() {{\n    wp_enqueue_style( '{slug}-style', get_stylesheet_uri(), array(), '1.0.0' );\n}}\nadd_action( 'wp_enqueue_scripts', '{ident}_scripts' );\n",
        name = project.name
    )
}

fn style_css(project: &Project, domain: &str, css: &CssGen) -> String {
    let theme = &project.theme;
    let mut out = format!(
        "/*\nTheme Name: {name}\nAuthor: Chida\nDescription: {desc}\nVersion: 1.0.0\nText Domain: {domain}\n*/\n\nbody {{\n  margin: 0;\n  background: {bg};\n  color: {fg};\n  font-family: {font};\n}}\n\nimg {{\n  max-width: 100%;\n  height: auto;\n}}\n",
        name = project.name,
        desc = if project.description.is_empty() {
            "Exported from a Chida project."
        } else {
            &project.description
        },
        bg = theme.background.to_hex(),
        fg = theme.text.to_hex(),
        font = theme.font_family,
    );
    if !css.rules.is_empty() {
        out.push('\n');
        out.push_str(&css.rules.join("\n\n"));
        out.push('\n');
    }
    out
}
