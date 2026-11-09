//! End to end checks for WordPress theme import and export,
//! driven by the small real theme in `tests/fixture-theme`.

use chida_core::{Node, Project};
use chida_wp::{export_theme, import_theme, read_zip, write_zip, ThemeFiles};
use std::path::PathBuf;

fn fixture_files() -> ThemeFiles {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixture-theme");
    let mut files = ThemeFiles::new();
    for entry in std::fs::read_dir(&dir).unwrap() {
        let entry = entry.unwrap();
        let name = entry.file_name().to_string_lossy().into_owned();
        files.insert(name, std::fs::read(entry.path()).unwrap());
    }
    files
}

fn find_in<'a>(node: &'a Node, pred: &dyn Fn(&Node) -> bool) -> Option<&'a Node> {
    if pred(node) {
        return Some(node);
    }
    node.children.iter().find_map(|c| find_in(c, pred))
}

fn find_node<'a>(project: &'a Project, pred: &dyn Fn(&Node) -> bool) -> Option<&'a Node> {
    project.pages.iter().find_map(|p| find_in(&p.root, pred))
}

fn has_node(project: &Project, pred: &dyn Fn(&Node) -> bool) -> bool {
    find_node(project, pred).is_some()
}

#[test]
fn import_builds_pages_and_dynamic_nodes() {
    let imported = import_theme(&fixture_files()).expect("fixture imports");
    let project = &imported.project;

    assert_eq!(project.name, "Fixture Blog");
    assert_eq!(project.meta.get("wp_theme").map(|s| s.as_str()), Some("true"));
    assert_eq!(project.meta.get("wp_slug").map(|s| s.as_str()), Some("fixture-blog"));
    assert_eq!(
        project.meta.get("wp_text_domain").map(|s| s.as_str()),
        Some("fixture-blog")
    );

    let names: Vec<&str> = project.pages.iter().map(|p| p.name.as_str()).collect();
    assert_eq!(names, vec!["سربرگ", "خانه", "نوشته تکی", "برگه", "پاصفحه"]);

    // The menu in the header.
    assert!(has_node(project, &|n| {
        n.props.get("wp_dynamic").map(|s| s.as_str()) == Some("menu")
    }));
    // The site title marker in the header.
    assert!(has_node(project, &|n| {
        n.text.as_deref() == Some("{{site_title}}")
    }));
    // The Loop on the home page.
    let home = project.page("خانه").expect("home page exists");
    let mut loop_found = false;
    home.root.walk(&mut |n| {
        if n.props.get("wp_loop").map(|s| s.as_str()) == Some("posts") {
            loop_found = true;
        }
    });
    assert!(loop_found, "home page carries a loop container");
    // Dynamic post fields.
    for marker in ["{{post_title}}", "{{post_content}}", "{{post_date}}", "{{post_author}}"] {
        assert!(
            has_node(project, &|n| {
                n.text.as_deref().map(|t| t.contains(marker)).unwrap_or(false)
            }),
            "marker {marker} is present"
        );
    }
    // The featured image flag.
    assert!(has_node(project, &|n| {
        n.props.get("wp_dynamic").map(|s| s.as_str()) == Some("featured")
    }));
    // Styling landed: the header bar has its dark green fill.
    assert!(has_node(project, &|n| {
        n.fill == Some(chida_core::Color::from_hex("#14342b").unwrap())
    }));

    let summary = &imported.summary;
    assert_eq!(summary.pages, 5);
    assert!(summary.elements > 20, "elements: {}", summary.elements);
    assert!(summary.dynamic_blocks >= 6, "dynamic: {}", summary.dynamic_blocks);
    assert!(summary.dropped_php >= 1, "the do_shortcode block is counted");
}

#[test]
fn export_emits_a_real_theme() {
    let imported = import_theme(&fixture_files()).unwrap();
    let files = export_theme(&imported.project);
    for name in [
        "style.css",
        "index.php",
        "single.php",
        "page.php",
        "header.php",
        "footer.php",
        "functions.php",
    ] {
        assert!(files.contains_key(name), "missing {name}");
    }
    let style = &files["style.css"];
    assert!(style.contains("Theme Name: Fixture Blog"), "{style}");
    assert!(style.contains("Text Domain: fixture-blog"));
    assert!(files["header.php"].contains("wp_head()"));
    assert!(files["footer.php"].contains("wp_footer()"));
    let index = &files["index.php"];
    assert!(index.contains("have_posts()"), "{index}");
    assert!(index.contains("the_post()"), "{index}");
    assert!(index.contains("the_content()"), "{index}");
    assert!(index.contains("the_permalink()"), "{index}");
    assert!(index.contains("the_post_thumbnail("), "{index}");
    assert!(index.contains("get_header()") && index.contains("get_footer()"));
    let single = &files["single.php"];
    assert!(single.contains("the_title()"), "{single}");
    assert!(single.contains("get_the_date()"), "{single}");
    assert!(single.contains("the_author()"), "{single}");
    let functions = &files["functions.php"];
    assert!(functions.contains("add_theme_support( 'title-tag' )"));
    assert!(functions.contains("add_theme_support( 'post-thumbnails' )"));
    assert!(functions.contains("register_nav_menus"));
    assert!(functions.contains("wp_enqueue_style"));
    assert!(functions.contains("after_setup_theme"));
    // No marker is ever left as literal text in the PHP output.
    for (name, body) in &files {
        assert!(!body.contains("{{"), "{name} leaks a marker: {body}");
    }
}

#[test]
fn roundtrip_keeps_loop_and_title() {
    let imported = import_theme(&fixture_files()).unwrap();
    let files = export_theme(&imported.project);
    let back: ThemeFiles = files
        .into_iter()
        .map(|(k, v)| (k, v.into_bytes()))
        .collect();
    let again = import_theme(&back).expect("exported theme re-imports");
    let home = again.project.page("خانه").expect("home page exists");
    let mut loop_found = false;
    home.root.walk(&mut |n| {
        if n.props.get("wp_loop").map(|s| s.as_str()) == Some("posts") {
            loop_found = true;
        }
    });
    assert!(loop_found, "loop survives the roundtrip");
    assert!(has_node(&again.project, &|n| {
        n.text.as_deref() == Some("{{post_title}}")
    }));
    assert!(has_node(&again.project, &|n| {
        n.props.get("wp_dynamic").map(|s| s.as_str()) == Some("menu")
    }));
    assert_eq!(again.project.name, "Fixture Blog");
}

#[test]
fn zip_roundtrip() {
    let imported = import_theme(&fixture_files()).unwrap();
    let files = export_theme(&imported.project);
    let zip = write_zip(&files).expect("zip writes");
    let back = read_zip(&zip).expect("zip reads");
    assert!(back.contains_key("style.css"));
    assert!(back.contains_key("index.php"));
    let again = import_theme(&back).expect("zipped theme imports");
    assert_eq!(again.project.pages.len(), 5);
}

#[test]
fn zip_with_wrapping_folder_imports() {
    let mut nested = ThemeFiles::new();
    for (name, bytes) in fixture_files() {
        nested.insert(format!("fixture-blog/{name}"), bytes);
    }
    let imported = import_theme(&nested).expect("wrapped theme imports");
    assert_eq!(imported.project.name, "Fixture Blog");
    assert_eq!(
        imported.project.meta.get("wp_slug").map(|s| s.as_str()),
        Some("fixture-blog")
    );
}

#[test]
fn not_a_theme_is_an_error() {
    let mut files = ThemeFiles::new();
    files.insert("index.php".into(), b"<p>hello</p>".to_vec());
    assert!(import_theme(&files).is_err());
}
