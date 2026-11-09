//! chida-wp: WordPress theme import and export for Chida.
//!
//! A theme arrives as a zip (or a plain file map): the importer
//! reads `style.css` and the PHP templates and builds a Chida
//! project whose dynamic blocks (the Loop, menus, template tags)
//! are flagged nodes. The exporter turns any project back into an
//! installable theme, with the flags becoming real WordPress PHP.
//!
//! Zip support uses the `zip` crate with only the pure Rust
//! deflate backend (flate2 on zlib-rs), chosen because it compiles
//! for wasm32: the same crate powers the CLI natively and the web
//! editor in the browser.

mod css;
mod exporter;
mod html;
mod importer;

pub use exporter::export_theme;
pub use importer::{import_theme, slugify, ImportSummary, ImportedTheme, ThemeFiles};

use std::collections::BTreeMap;
use std::io::{Cursor, Read, Write};

/// Read a theme `.zip` into the flat file map the importer takes.
/// Directories are skipped and nested paths are kept as-is (the
/// importer strips a single wrapping theme folder itself).
pub fn read_zip(bytes: &[u8]) -> Result<ThemeFiles, String> {
    let mut archive =
        zip::ZipArchive::new(Cursor::new(bytes.to_vec())).map_err(|e| e.to_string())?;
    let mut out = ThemeFiles::new();
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(|e| e.to_string())?;
        if entry.is_dir() {
            continue;
        }
        let name = entry.name().map_err(|e| e.to_string())?.replace('\\', "/");
        let mut data = Vec::new();
        entry.read_to_end(&mut data).map_err(|e| e.to_string())?;
        out.insert(name, data);
    }
    Ok(out)
}

/// Write text theme files as a deflated `.zip`.
pub fn write_zip(files: &BTreeMap<String, String>) -> Result<Vec<u8>, String> {
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    for (name, text) in files {
        writer
            .start_file(name, options)
            .map_err(|e| e.to_string())?;
        writer
            .write_all(text.as_bytes())
            .map_err(|e| e.to_string())?;
    }
    let cursor = writer.finish().map_err(|e| e.to_string())?;
    Ok(cursor.into_inner())
}
