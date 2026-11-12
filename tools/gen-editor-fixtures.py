#!/usr/bin/env python3
"""Generate the editor's bundled fixtures for generated templates.

For every pack in crates/chida-cli/src/gen/packs/*.json, runs the real
Chida CLI (`chida new --template <id>`) and stores the minified bundle
under apps/editor/fixtures/gen/<id>.json, then writes
apps/editor/src/gen_registry.rs (template metadata for the start-screen
gallery). The fixtures are NOT embedded in the wasm: trunk copies the
fixtures directory into dist (see index.html) and the editor fetches a
fixture on demand when its card is clicked, so 197 templates cost the
wasm nothing. The 8 classic fixtures are untouched (see
apps/editor/generate-fixture.sh).

Usage: python3 tools/gen-editor-fixtures.py [--skip-build]
"""
import json
import os
import subprocess
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
PACKS_DIR = os.path.join(ROOT, "crates", "chida-cli", "src", "gen", "packs")
FIX_DIR = os.path.join(ROOT, "apps", "editor", "fixtures", "gen")
REGISTRY = os.path.join(ROOT, "apps", "editor", "src", "gen_registry.rs")

CATEGORIES = {
    "shop": ("فروشگاه", "Shop"),
    "food": ("غذا و رستوران", "Food"),
    "health": ("سلامت و درمان", "Health"),
    "beauty": ("زیبایی", "Beauty"),
    "fitness": ("ورزش و تناسب اندام", "Fitness"),
    "education": ("آموزش", "Education"),
    "kids": ("کودک", "Kids"),
    "coaching": ("مربیگری", "Coaching"),
    "portfolio": ("نمونه‌کار", "Portfolio"),
    "services": ("خدمات", "Services"),
    "home": ("خانه و دکور", "Home"),
    "auto": ("خودرو", "Auto"),
    "agriculture": ("کشاورزی و گیاه", "Plants"),
    "corporate": ("شرکتی", "Corporate"),
    "saas": ("نرم‌افزار و استارتاپ", "Software"),
    "finance": ("مالی", "Finance"),
    "tech": ("فناوری", "Technology"),
    "business": ("کسب‌وکار", "Business"),
    "travel": ("سفر و گردشگری", "Travel"),
    "events": ("رویداد", "Events"),
    "realestate": ("املاک", "Real estate"),
    "blog": ("بلاگ و رسانه", "Blog and media"),
    "charity": ("خیریه", "Charity"),
    "pets": ("حیوانات", "Pets"),
    "sports": ("باشگاه ورزشی", "Sports"),
    "music": ("موسیقی", "Music"),
    "photo": ("عکاسی", "Photography"),
    "pages": ("صفحه‌های کاربردی", "Ready pages"),
    "misc": ("متفرقه", "More"),
}

# Primary hex per palette (must match engine.rs resolve_palette).
PALETTE_PRIMARY = {
    "emerald": "#00825a", "forest": "#156534", "pine": "#065f46",
    "teal": "#0f766e", "turquoise": "#0d9488", "ocean": "#0d6e9e",
    "sky": "#0284c7", "navy": "#1e40af", "royal": "#4338ca",
    "violet": "#7c3aed", "grape": "#6d28d9", "plum": "#9d174d",
    "berry": "#a21caf", "rose": "#e11d48", "cherry": "#be123c",
    "crimson": "#991b1b", "terracotta": "#c2410c", "coral": "#ea580c",
    "amber": "#b45309", "gold": "#a16207", "sand": "#92400e",
    "coffee": "#78350f", "olive": "#4d7c0f", "mint": "#059669",
    "charcoal": "#1f2937", "slate": "#334155", "steel": "#475569",
    "midnight": "#6366f1",
}


def rs_str(s):
    return '"' + s.replace("\\", "\\\\").replace('"', '\\"') + '"'


def main():
    skip_build = "--skip-build" in sys.argv
    env = dict(os.environ)
    env["PATH"] = os.path.expanduser("~/.cargo/bin") + ":" + env.get("PATH", "")
    # The CLI embeds the pack registry at compile time, so regenerate it
    # from the packs on disk before building.
    subprocess.run(["python3", os.path.join(HERE, "gen-registry.py")],
                   cwd=ROOT, check=True)
    chida = os.path.join(ROOT, "target", "release", "chida")
    if not skip_build or not os.path.exists(chida):
        print("building the CLI (release)...")
        subprocess.run(["cargo", "build", "--release", "-p", "chida-cli"],
                       cwd=ROOT, env=env, check=True)
    pack_files = sorted(f for f in os.listdir(PACKS_DIR) if f.endswith(".json"))
    if not pack_files:
        print("no packs found", file=sys.stderr)
        return 1
    os.makedirs(FIX_DIR, exist_ok=True)
    # Drop stale generated fixtures.
    for old in os.listdir(FIX_DIR):
        if old.endswith(".json"):
            os.remove(os.path.join(FIX_DIR, old))
    metas = []
    total_bytes = 0
    with tempfile.TemporaryDirectory() as tmp:
        for fname in pack_files:
            with open(os.path.join(PACKS_DIR, fname), encoding="utf-8") as f:
                pack = json.load(f)
            pid = pack["id"]
            out = subprocess.run(
                [chida, "new", f"tpl-{pid}", "--template", pid, "--out", tmp],
                capture_output=True, text=True)
            if out.returncode != 0 and "unknown template" in out.stderr:
                # A pack landed after the registry was generated (pack
                # writers still running): regenerate, rebuild once, retry.
                print(f"retrying {pid} after a registry refresh...")
                subprocess.run(["python3", os.path.join(HERE, "gen-registry.py")],
                               cwd=ROOT, check=True)
                subprocess.run(["cargo", "build", "--release", "-p", "chida-cli"],
                               cwd=ROOT, env=env, check=True)
                out = subprocess.run(
                    [chida, "new", f"tpl-{pid}", "--template", pid, "--out", tmp],
                    capture_output=True, text=True)
            if out.returncode != 0:
                print(f"FAILED to build {pid}: {out.stderr.strip()}", file=sys.stderr)
                return 1
            with open(os.path.join(tmp, f"tpl-{pid}.chida.json"), encoding="utf-8") as f:
                bundle = json.load(f)
            dest = os.path.join(FIX_DIR, f"{pid}.json")
            with open(dest, "w", encoding="utf-8") as f:
                json.dump(bundle, f, ensure_ascii=False, separators=(",", ":"))
            total_bytes += os.path.getsize(dest)
            cat_fa, cat_en = CATEGORIES[pack["category"]]
            metas.append((
                pid, pack["name_fa"], pack["name_en"], pack["category"],
                cat_fa, cat_en,
                PALETTE_PRIMARY.get(pack["palette"], "#107a57"),
            ))
    cats = sorted({m[3] for m in metas})
    lines = [
        "//! Generated by tools/gen-editor-fixtures.py. Do not edit by hand.",
        "",
        "/// One generated template: key, Persian name, English name,",
        "/// category id, category names, and the tile color (palette",
        "/// primary). The tile is a colored placeholder; generated",
        "/// templates share no hand-drawn thumbnails.",
        "pub struct GenTemplate {",
        "    pub key: &'static str,",
        "    pub fa: &'static str,",
        "    pub en: &'static str,",
        "    pub category: &'static str,",
        "    pub cat_fa: &'static str,",
        "    pub cat_en: &'static str,",
        "    pub color: &'static str,",
        "}",
        "",
        "pub const GEN_TEMPLATES: &[GenTemplate] = &[",
    ]
    for (pid, fa, en, cat, cat_fa, cat_en, color) in metas:
        lines.append(
            "    GenTemplate { key: %s, fa: %s, en: %s, category: %s, cat_fa: %s, cat_en: %s, color: %s },"
            % (rs_str(pid), rs_str(fa), rs_str(en), rs_str(cat),
               rs_str(cat_fa), rs_str(cat_en), rs_str(color)))
    lines.append("];")
    lines.append("")
    lines.append("/// Categories that have generated templates: id, FA, EN.")
    lines.append("pub const GEN_CATEGORIES: &[(&str, &str, &str)] = &[")
    for cat in cats:
        cat_fa, cat_en = CATEGORIES[cat]
        lines.append("    (%s, %s, %s)," % (rs_str(cat), rs_str(cat_fa), rs_str(cat_en)))
    lines.append("];")
    lines.append("")
    with open(REGISTRY, "w", encoding="utf-8") as f:
        f.write("\n".join(lines))
    print(f"wrote {len(metas)} fixtures ({total_bytes / 1e6:.1f} MB) + {REGISTRY}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
