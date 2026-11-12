//! Generated templates: 170+ data-driven templates built from JSON packs.
//!
//! Each pack is one JSON file under `packs/` (see
//! `~/workspace/chida-templates/PACK-SCHEMA.md`). `registry.rs` is
//! generated from those files by `tools/gen-registry.py` and embeds them
//! with `include_str!`, so the installed binary carries every template.
//! The engine (`engine.rs`) turns a pack into a real `ProjectFile`.

mod engine;
mod pack;
mod registry;

pub use pack::Pack;

use crate::project_file::ProjectFile;

/// Category id, Persian name, English name.
pub const CATEGORIES: &[(&str, &str, &str)] = &[
    ("shop", "فروشگاه", "Shop"),
    ("food", "غذا و رستوران", "Food"),
    ("health", "سلامت و درمان", "Health"),
    ("beauty", "زیبایی", "Beauty"),
    ("fitness", "ورزش و تناسب اندام", "Fitness"),
    ("education", "آموزش", "Education"),
    ("kids", "کودک", "Kids"),
    ("coaching", "مربیگری", "Coaching"),
    ("portfolio", "نمونه‌کار", "Portfolio"),
    ("services", "خدمات", "Services"),
    ("home", "خانه و دکور", "Home"),
    ("auto", "خودرو", "Auto"),
    ("agriculture", "کشاورزی و گیاه", "Plants"),
    ("corporate", "شرکتی", "Corporate"),
    ("saas", "نرم‌افزار و استارتاپ", "Software"),
    ("finance", "مالی", "Finance"),
    ("tech", "فناوری", "Technology"),
    ("business", "کسب‌وکار", "Business"),
    ("travel", "سفر و گردشگری", "Travel"),
    ("events", "رویداد", "Events"),
    ("realestate", "املاک", "Real estate"),
    ("blog", "بلاگ و رسانه", "Blog and media"),
    ("charity", "خیریه", "Charity"),
    ("pets", "حیوانات", "Pets"),
    ("sports", "باشگاه ورزشی", "Sports"),
    ("music", "موسیقی", "Music"),
    ("photo", "عکاسی", "Photography"),
    ("pages", "صفحه‌های کاربردی", "Ready pages"),
    ("misc", "متفرقه", "More"),
];

/// Parse every embedded pack. Packs that fail to parse are skipped here;
/// the test suite fails on them instead (see `all_packs_parse`).
pub fn all_packs() -> Vec<Pack> {
    registry::PACK_SOURCES
        .iter()
        .filter_map(|(_, src)| serde_json::from_str(src).ok())
        .collect()
}

/// Build one generated template by id.
pub fn build_template(id: &str) -> Option<ProjectFile> {
    let src = registry::PACK_SOURCES
        .iter()
        .find(|(pid, _)| *pid == id)
        .map(|(_, src)| *src)?;
    let pack: Pack = serde_json::from_str(src).ok()?;
    Some(engine::build_project(&pack))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_packs_parse() {
        for (id, src) in registry::PACK_SOURCES {
            let pack: Pack = serde_json::from_str(src)
                .unwrap_or_else(|e| panic!("pack {id} does not parse: {e}"));
            assert_eq!(pack.id, *id, "pack id mismatch in {id}");
            assert!(
                CATEGORIES.iter().any(|(c, _, _)| c == &pack.category),
                "pack {id} has unknown category {}",
                pack.category
            );
            assert!(!pack.pages.is_empty(), "pack {id} has no pages");
            assert_eq!(pack.pages[0].route, "/", "pack {id} home page must be /");
        }
    }

    #[test]
    fn all_templates_build_and_validate() {
        for pack in all_packs() {
            let file = engine::build_project(&pack);
            let issues = file.validate();
            assert!(issues.is_empty(), "pack {} invalid: {:?}", pack.id, issues);
            assert!(
                file.project.total_nodes() >= 20,
                "pack {} too small: {} nodes",
                pack.id,
                file.project.total_nodes()
            );
        }
    }
}
