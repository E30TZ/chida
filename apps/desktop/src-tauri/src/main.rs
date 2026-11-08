
// Chida desktop entry point. The heavy lifting lives in the chida-* crates;
// this shell only wires Tauri commands to them.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde::Serialize;
use std::path::PathBuf;

#[derive(Serialize)]
struct ProjectInfo {
    name: String,
    pages: usize,
    nodes: usize,
    tables: usize,
}

#[tauri::command]
fn open_project(path: String) -> Result<ProjectInfo, String> {
    let project = chida_core::Project::load(&PathBuf::from(&path)).map_err(|e| e.to_string())?;
    Ok(ProjectInfo { name: project.name.clone(), pages: project.pages.len(), nodes: project.total_nodes(), tables: 0 })
}

#[tauri::command]
fn render_page(path: String, route: String, out_png: String) -> Result<String, String> {
    let project = chida_core::Project::load(&PathBuf::from(&path)).map_err(|e| e.to_string())?;
    let page = project.page(&route).ok_or_else(|| format!("no page at route {route}"))?;
    let canvas = chida_render::render_page(page, 1.0);
    canvas.save_png(&PathBuf::from(&out_png)).map_err(|e| e.to_string())?;
    Ok(out_png)
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![open_project, render_page])
        .run(tauri::generate_context!())
        .expect("error while running the Chida desktop app");
}
