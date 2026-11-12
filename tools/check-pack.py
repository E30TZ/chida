#!/usr/bin/env python3
"""Validate generated-template packs: tools/check-pack.py <file...>.

Checks JSON syntax, required fields, section types and variants, route
references, flow/table references, and the content rules from
PACK-SCHEMA.md (formal register, no em dash, no fake stats, generic
testimonial attribution). Exit 0 when every pack is clean.
"""
import json
import os
import re
import sys

CATEGORIES = {
    "shop", "food", "health", "beauty", "fitness", "education", "kids",
    "coaching", "portfolio", "services", "home", "auto", "agriculture",
    "corporate", "saas", "finance", "tech", "business", "travel",
    "events", "realestate", "blog", "charity", "pets", "sports", "music",
    "photo", "pages", "misc",
}
PALETTES = {
    "emerald", "forest", "pine", "teal", "turquoise", "ocean", "sky",
    "navy", "royal", "violet", "grape", "plum", "berry", "rose", "cherry",
    "crimson", "terracotta", "coral", "amber", "gold", "sand", "coffee",
    "olive", "mint", "charcoal", "slate", "steel", "midnight",
}
SECTIONS = {
    "hero": {"center", "split", "band", "minimal"},
    "features": {"grid-3", "grid-4", "rows", "list"},
    "services": set(),
    "products": set(),
    "menu": set(),
    "pricing": {"tiers-3", "simple"},
    "gallery": set(),
    "testimonials": {"cards-3", "quote"},
    "team": set(),
    "faq": set(),
    "cta": set(),
    "contact": set(),
    "booking": set(),
    "hours": set(),
    "text": set(),
    "schedule": set(),
    "courses": set(),
    "listings": set(),
    "events": set(),
    "steps": set(),
    "dashboard": set(),
    "table": set(),
}
COL_TYPES = {"text", "integer", "money", "date", "boolean"}
GENERIC_AUTHORS = {"مشتری نمونه", "یکی از مشتریان"}
# Every key the Rust schema (crates/chida-cli/src/gen/pack.rs) reads.
# Serde silently drops unknown keys, so a typo like "desc" instead of
# "text" would vanish without a trace; the checker rejects them here.
KEYS = {
    "pack": {"id", "category", "name_fa", "name_en", "brand", "tagline",
             "description", "palette", "dark", "nav_cta", "pages",
             "tables", "flow"},
    "nav_cta": {"label", "route"},
    "page": {"id", "name", "route", "chrome", "sections"},
    "section": {"type", "variant", "title", "subtitle", "badge", "text",
                "cta", "cta_route", "cta2", "cta2_route", "button",
                "button_route", "points", "items", "tiers", "kpis",
                "paragraphs", "fields", "submit", "phone", "email",
                "address", "hours", "table"},
    "item": {"title", "name", "text", "role", "author", "q", "a",
             "caption", "day", "time", "date", "meta", "tag", "price",
             "old_price"},
    "tier": {"name", "price", "period", "features", "featured"},
    "kpi": {"label", "value"},
    "table": {"name", "label", "columns", "rows"},
    "column": {"name", "type"},
    "flow": {"button", "kind", "table", "fields", "message", "navigate"},
    "flow_field": {"column", "value"},
}


def check_keys(path, issues, obj, kind, where):
    if not isinstance(obj, dict):
        return
    for key in obj:
        if key not in KEYS[kind]:
            issues.append(f"{path}: unknown {kind} key {key!r} in {where}")
# Colloquial words that must not appear in formal copy. Matched with
# word boundaries (a ZWNJ or letter next to the word means it is part of
# a longer formal word, e.g. «بگیرید» or «پایه», not the colloquial form).
COLLOQUIAL = ["می‌تونی", "می‌خوای", "بزن", "بده", "یه", "می‌شه", "همینه",
              "می‌مونه", "می‌کنی", "می‌گی", "بگیر", "بذار", "باشه"]
COLLOQUIAL_RE = [
    re.compile(r"(?<![\w‌])" + re.escape(w) + r"(?![\w‌])") for w in COLLOQUIAL
]
# Fake-statistic phrasing.
FAKE_STATS = ["٪ رضایت", "درصد رضایت", "مشتری راضی", "سال تجربه",
              "پروژه موفق", "بیش از ۱۰۰۰", "بیش از ۱۰۰", "هزاران مشتری"]


def check_text(path, issues, value, where, allow_sample_word=False):
    if not isinstance(value, str):
        return
    if "—" in value:
        issues.append(f"{path}: em dash in {where}")
    if not allow_sample_word and "متن نمونه" in value:
        issues.append(f"{path}: placeholder text in {where}")
    for word, pattern in zip(COLLOQUIAL, COLLOQUIAL_RE):
        if pattern.search(value):
            issues.append(f"{path}: colloquial {word!r} in {where}")
    for phrase in FAKE_STATS:
        if phrase in value:
            issues.append(f"{path}: possible fake statistic {phrase!r} in {where}")


def walk_strings(node, fn, where=""):
    if isinstance(node, str):
        fn(node, where)
    elif isinstance(node, list):
        for i, item in enumerate(node):
            walk_strings(item, fn, f"{where}[{i}]")
    elif isinstance(node, dict):
        for key, item in node.items():
            walk_strings(item, fn, f"{where}.{key}")


def check_pack(path):
    issues = []
    name = os.path.basename(path)
    try:
        with open(path, encoding="utf-8") as f:
            pack = json.load(f)
    except (OSError, json.JSONDecodeError) as exc:
        return [f"{path}: {exc}"]
    pid = pack.get("id", "")
    if name != f"{pid}.json":
        issues.append(f"{path}: file name {name} does not match id {pid!r}")
    check_keys(path, issues, pack, "pack", "pack")
    if isinstance(pack.get("nav_cta"), dict):
        check_keys(path, issues, pack["nav_cta"], "nav_cta", "nav_cta")
    for field in ("category", "name_fa", "name_en", "brand", "tagline",
                  "description", "palette", "pages"):
        if field not in pack or pack[field] in ("", [], None):
            issues.append(f"{path}: missing or empty {field}")
    if pack.get("category") not in CATEGORIES:
        issues.append(f"{path}: unknown category {pack.get('category')!r}")
    if pack.get("palette") not in PALETTES:
        issues.append(f"{path}: unknown palette {pack.get('palette')!r}")
    pages = pack.get("pages") or []
    routes = {p.get("route") for p in pages}
    if not pages or pages[0].get("route") != "/":
        issues.append(f"{path}: first page must be home /")
    if len(pages) > 3:
        issues.append(f"{path}: more than 3 pages")
    page_ids = {p.get("id") for p in pages}
    tables = {t.get("name") for t in pack.get("tables") or []}
    for page in pages:
        check_keys(path, issues, page, "page", f"page {page.get('id')}")
        for sec in page.get("sections") or []:
            check_keys(path, issues, sec, "section",
                       f"page {page.get('id')} section {sec.get('type')}")
            for item in sec.get("items") or []:
                check_keys(path, issues, item, "item",
                           f"page {page.get('id')} {sec.get('type')} item")
            for tier in sec.get("tiers") or []:
                check_keys(path, issues, tier, "tier",
                           f"page {page.get('id')} {sec.get('type')} tier")
            for kpi in sec.get("kpis") or []:
                check_keys(path, issues, kpi, "kpi",
                           f"page {page.get('id')} {sec.get('type')} kpi")
            kind = sec.get("type")
            if kind not in SECTIONS:
                issues.append(f"{path}: unknown section type {kind!r}")
                continue
            variants = SECTIONS[kind]
            if variants and sec.get("variant") not in variants:
                issues.append(
                    f"{path}: section {kind} needs variant in {sorted(variants)}")
            for key in ("cta_route", "cta2_route", "button_route"):
                route = sec.get(key)
                if route and route not in routes:
                    issues.append(f"{path}: {key} {route!r} is not a page route")
            if kind == "table" and sec.get("table") not in tables:
                issues.append(f"{path}: table section names unknown table {sec.get('table')!r}")
            if kind == "testimonials":
                for item in sec.get("items") or []:
                    author = item.get("author", "")
                    if author and author not in GENERIC_AUTHORS and \
                            not re.fullmatch(r"مشتری نمونه \d+", author):
                        issues.append(
                            f"{path}: testimonial author must be generic, got {author!r}")
    for table in pack.get("tables") or []:
        check_keys(path, issues, table, "table", f"table {table.get('name')}")
        for col in table.get("columns") or []:
            check_keys(path, issues, col, "column",
                       f"table {table.get('name')} column")
            if col.get("type") not in COL_TYPES:
                issues.append(f"{path}: bad column type {col.get('type')!r}")
        ncols = len(table.get("columns") or [])
        for row in table.get("rows") or []:
            if len(row) != ncols:
                issues.append(f"{path}: seed row {row} has wrong column count")
    flow = pack.get("flow")
    if flow:
        check_keys(path, issues, flow, "flow", "flow")
        for field in flow.get("fields") or []:
            check_keys(path, issues, field, "flow_field", "flow field")
        button = flow.get("button", "")
        prefix = button.rsplit("-", 1)[0] if "-" in button else ""
        if prefix not in page_ids:
            issues.append(f"{path}: flow button {button!r} has unknown page prefix")
        if flow.get("kind") not in ("message", "save"):
            issues.append(f"{path}: flow kind must be message or save")
        if flow.get("kind") == "save" and flow.get("table") not in tables:
            issues.append(f"{path}: flow save names unknown table {flow.get('table')!r}")
        if flow.get("navigate") and flow["navigate"] not in routes:
            issues.append(f"{path}: flow navigate {flow['navigate']!r} is not a page route")

    def text_rule(value, where):
        allow = where.endswith(".author") or ".tagline" in where
        check_text(path, issues, value, where, allow_sample_word=allow)

    walk_strings(pack, text_rule)
    return issues


def main():
    if len(sys.argv) < 2:
        print("usage: check-pack.py <pack.json...>", file=sys.stderr)
        return 2
    all_issues = []
    for path in sys.argv[1:]:
        all_issues.extend(check_pack(path))
    if all_issues:
        print("\n".join(all_issues))
        print(f"{len(all_issues)} issue(s)")
        return 1
    print(f"OK: {len(sys.argv) - 1} pack(s)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
