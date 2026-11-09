//! The `chida` command line tool.
//!
//! Commands:
//!   chida new <name> [--template shop|blank] [--out <dir>]
//!   chida render <project.chida.json> [--out <dir>]
//!   chida generate <project.chida.json> [--out <dir>]
//!   chida info <project.chida.json>

mod demo;
mod project_file;
mod templates;

use project_file::ProjectFile;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

fn arg_value(args: &[String], flag: &str) -> Option<String> {
    args.windows(2).find(|w| w[0] == flag).map(|w| w[1].clone())
}
fn positional(args: &[String], idx: usize) -> Option<String> {
    args.iter()
        .filter(|a| !a.starts_with("--"))
        .nth(idx)
        .cloned()
}

fn cmd_new(args: &[String]) -> Result<(), String> {
    let name = positional(args, 0)
        .ok_or("usage: chida new <name> [--template shop|blank] [--out <dir>]")?;
    let template = arg_value(args, "--template").unwrap_or_else(|| {
        if name == "demo" {
            "shop".into()
        } else {
            "blank".into()
        }
    });
    let file = match template.as_str() {
        "shop" => demo::demo_shop(),
        "blank" => demo::blank_project(&name),
        "persian" => demo::persian_project(),
        "digikala" => templates::digikala_marketplace(),
        "liquid-glass" | "glass" => templates::liquid_glass(),
        "saas-dashboard" | "saas" => templates::saas_dashboard(),
        "clinic" => templates::clinic(),
        "restaurant" => templates::restaurant(),
        "portfolio" => templates::portfolio(),
        other => return Err(format!("unknown template: {other} (try: chida templates)")),
    };
    let out_dir = arg_value(args, "--out")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    std::fs::create_dir_all(&out_dir).map_err(|e| e.to_string())?;
    let path = out_dir.join(format!("{name}.chida.json"));
    file.save(&path).map_err(|e| e.to_string())?;
    if template == "digikala" {
        // The marketplace template references real photo assets; copy
        // them next to the project so renders resolve them.
        let assets_dir = out_dir.join("assets").join("digikala");
        std::fs::create_dir_all(&assets_dir).map_err(|e| e.to_string())?;
        for (file_name, bytes) in templates::digikala_assets() {
            std::fs::write(assets_dir.join(file_name), bytes).map_err(|e| e.to_string())?;
        }
        println!(
            "Copied {} photo assets to {}",
            templates::digikala_assets().len(),
            assets_dir.display()
        );
    }
    println!(
        "Created {} ({} pages, {} nodes, {} tables, {} flows)",
        path.display(),
        file.project.pages.len(),
        file.project.total_nodes(),
        file.database.tables.len(),
        file.flows.len()
    );
    Ok(())
}

fn cmd_render(args: &[String]) -> Result<(), String> {
    let input =
        positional(args, 0).ok_or("usage: chida render <project.chida.json> [--out <dir>]")?;
    let file = ProjectFile::load(Path::new(&input)).map_err(|e| e.to_string())?;
    let out_dir = arg_value(args, "--out")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("render-out"));
    std::fs::create_dir_all(&out_dir).map_err(|e| e.to_string())?;
    for page in &file.project.pages {
        let resolved_page = file.project.resolve_page(page);
        let page = &resolved_page;
        let slug = if page.route == "/" {
            "home".to_string()
        } else {
            page.route.trim_matches('/').replace('/', "-")
        };
        let base = std::path::Path::new(&input)
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_default();
        let canvas = chida_render::render_page_with_base(page, 1.0, &base);
        let png_path = out_dir.join(format!("{slug}.png"));
        canvas.save_png(&png_path).map_err(|e| e.to_string())?;
        let svg_path = out_dir.join(format!("{slug}.svg"));
        std::fs::write(&svg_path, chida_render::render_page_svg(page))
            .map_err(|e| e.to_string())?;
        println!(
            "Rendered {} -> {} and {}",
            page.name,
            png_path.display(),
            svg_path.display()
        );
    }
    Ok(())
}

fn cmd_generate(args: &[String]) -> Result<(), String> {
    let input =
        positional(args, 0).ok_or("usage: chida generate <project.chida.json> [--out <dir>]")?;
    let file = ProjectFile::load(Path::new(&input)).map_err(|e| e.to_string())?;
    let out_dir = arg_value(args, "--out")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("generated-app"));
    let written = chida_codegen::generate(&file.project, &file.database, &file.flows, &out_dir)
        .map_err(|e| e.to_string())?;
    println!(
        "Generated {} files into {}",
        written.len(),
        out_dir.display()
    );
    for p in &written {
        println!("  {}", p.display());
    }
    Ok(())
}

fn cmd_info(args: &[String]) -> Result<(), String> {
    let input = positional(args, 0).ok_or("usage: chida info <project.chida.json>")?;
    let file = ProjectFile::load(Path::new(&input)).map_err(|e| e.to_string())?;
    println!("Project: {} ({})", file.project.name, file.project.id);
    println!(
        "Theme: {} / {}",
        file.project.theme.name, file.project.theme.font_family
    );
    println!("Pages: {}", file.project.pages.len());
    for p in &file.project.pages {
        println!(
            "  {} at {} ({} nodes, {}x{})",
            p.name,
            p.route,
            p.node_count(),
            p.width,
            p.height
        );
    }
    println!("Components: {}", file.project.components.len());
    println!("Tables: {}", file.database.tables.len());
    for t in &file.database.tables {
        println!(
            "  {} ({} columns, {} seed rows)",
            t.name,
            t.columns.len(),
            t.seed_rows.len()
        );
    }
    println!("Flows: {}", file.flows.len());
    for f in &file.flows {
        println!(
            "  {} on {:?} from {:?}",
            f.name, f.trigger.kind, f.trigger.source_node
        );
    }
    let issues = file.validate();
    if issues.is_empty() {
        println!("Validation: ok");
    } else {
        println!("Validation issues:");
        for i in issues {
            println!("  - {i}");
        }
    }
    Ok(())
}

fn cmd_wp_import(args: &[String]) -> Result<(), String> {
    let input = positional(args, 0)
        .ok_or("usage: chida wp-import <theme.zip | theme-dir> <out.chida.json>")?;
    let output = positional(args, 1)
        .ok_or("usage: chida wp-import <theme.zip | theme-dir> <out.chida.json>")?;
    let input_path = Path::new(&input);
    let files: chida_wp::ThemeFiles = if input_path.is_dir() {
        let mut map = chida_wp::ThemeFiles::new();
        collect_dir(input_path, input_path, &mut map).map_err(|e| e.to_string())?;
        map
    } else {
        let bytes = std::fs::read(input_path).map_err(|e| e.to_string())?;
        chida_wp::read_zip(&bytes)?
    };
    let imported = chida_wp::import_theme(&files)?;
    let file = ProjectFile {
        project: imported.project,
        database: chida_db::Database::default(),
        flows: Vec::new(),
    };
    file.save(Path::new(&output)).map_err(|e| e.to_string())?;
    let s = &imported.summary;
    println!(
        "Imported WordPress theme into {} ({} pages, {} elements, {} dynamic blocks, {} PHP blocks dropped)",
        output, s.pages, s.elements, s.dynamic_blocks, s.dropped_php
    );
    Ok(())
}

/// Recursively collect a theme directory into the flat file map.
fn collect_dir(base: &Path, dir: &Path, out: &mut chida_wp::ThemeFiles) -> std::io::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_dir(base, &path, out)?;
        } else if let Ok(rel) = path.strip_prefix(base) {
            let key = rel.to_string_lossy().replace('\\', "/");
            out.insert(key, std::fs::read(&path)?);
        }
    }
    Ok(())
}

fn cmd_wp_export(args: &[String]) -> Result<(), String> {
    let input =
        positional(args, 0).ok_or("usage: chida wp-export <project.chida.json> <out-theme.zip>")?;
    let output =
        positional(args, 1).ok_or("usage: chida wp-export <project.chida.json> <out-theme.zip>")?;
    let file = ProjectFile::load(Path::new(&input)).map_err(|e| e.to_string())?;
    let theme_files = chida_wp::export_theme(&file.project);
    let zip = chida_wp::write_zip(&theme_files)?;
    std::fs::write(&output, &zip).map_err(|e| e.to_string())?;
    println!(
        "Exported WordPress theme to {} ({} files)",
        output,
        theme_files.len()
    );
    for name in theme_files.keys() {
        println!("  {name}");
    }
    Ok(())
}

fn cmd_templates() -> Result<(), String> {
    println!("Available templates:");
    println!("  shop          Starter plant shop: storefront, admin, products and orders, one checkout flow");
    println!("  persian       Small Persian RTL sample page proving shaped rendering");
    println!("  digikala      Marketplace Pro: Persian RTL marketplace, home, category, product and cart pages, products, categories, orders and users");
    println!(
        "  liquid-glass  Liquid Glass Family: glass landing, dashboard, shop and mobile pages"
    );
    println!("  saas-dashboard  Dark SaaS admin dashboard: sidebar, KPI cards, sales chart, orders and customers");
    println!("  clinic        Mehr Clinic: services, doctors, appointment booking form and page");
    println!("  restaurant    Taam Restaurant: hero, featured dishes, reservation form and full menu page");
    println!("  portfolio     Personal portfolio: hero, works grid, services and contact page");
    println!("  blank         One empty page with a title");
    Ok(())
}

const USAGE: &str = concat!(
    "Chida ",
    env!("CARGO_PKG_VERSION"),
    ", a visual full-stack builder\n\nUsage:\n  chida new <name> [--template shop|blank|persian|digikala|liquid-glass|saas-dashboard|clinic|restaurant|portfolio] [--out <dir>]\n  chida templates\n  chida render <project.chida.json> [--out <dir>]\n  chida generate <project.chida.json> [--out <dir>]\n  chida info <project.chida.json>\n  chida wp-import <theme.zip | theme-dir> <out.chida.json>\n  chida wp-export <project.chida.json> <out-theme.zip>\n  chida --help"
);

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some(cmd) = args.first() else {
        eprintln!("{USAGE}");
        return ExitCode::FAILURE;
    };
    let rest = &args[1..];
    let result = match cmd.as_str() {
        "new" => cmd_new(rest),
        "templates" => cmd_templates(),
        "render" => cmd_render(rest),
        "generate" => cmd_generate(rest),
        "info" => cmd_info(rest),
        "wp-import" => cmd_wp_import(rest),
        "wp-export" => cmd_wp_export(rest),
        "--help" | "-h" | "help" => {
            println!("{USAGE}");
            Ok(())
        }
        "--version" | "-V" => {
            println!("chida {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        other => Err(format!("unknown command: {other}\n\n{USAGE}")),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn demo_project_file_roundtrip() {
        let f = demo::demo_shop();
        let back = ProjectFile::from_json(&f.to_json().unwrap()).unwrap();
        assert_eq!(f, back);
    }
    #[test]
    fn arg_parsing() {
        let args: Vec<String> = vec!["demo".into(), "--out".into(), "x".into()];
        assert_eq!(positional(&args, 0).as_deref(), Some("demo"));
        assert_eq!(arg_value(&args, "--out").as_deref(), Some("x"));
    }
}
