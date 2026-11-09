# Changelog

All notable changes to Chida are recorded here. The format is based on
Keep a Changelog, and this project uses semantic versioning.

## [1.0.1] - 2026-11-09

Release pipeline fix. No changes to the Chida tool itself; the 1.0.0
release never received its download files because two CI jobs blocked
the publish step, and this release ships the same code with a working
pipeline.

### Fixed

- The Intel macOS build moved from the retired `macos-13` runner to
  `macos-15-intel`; the old job waited in the queue forever.
- The Debian package and AppImage steps now run the packaging scripts
  through `sh`, because the copies of `scripts/build-deb.sh` and
  `scripts/build-appimage.sh` stored on GitHub have no executable bit
  and the steps failed with "Permission denied".

## [1.0.0] - 2026-11-08

First release. Chida is a visual full-stack builder written in Rust:
design pages, data tables and logic flows in one project, then export a
render of the design or a complete Rust application you own.

### Added

- Document model (`chida-core`): pages, node trees, components, design
  themes, JSON project files (`.chida.json`), structural validation, and
  undo and redo history.
- Renderer (`chida-render`): PNG and SVG export from any page, with solid
  and rounded shapes, ellipses, strokes, alpha blending, real photo
  images (JPEG and PNG, cover fit, clipped to the node corner radius),
  and anti-aliased TrueType text shaped with HarfBuzz. Persian and Arabic
  text shapes, joins and runs right-to-left with the bundled Vazirmatn
  font (OFL 1.1), mixed-direction lines are segmented into bidi runs so
  embedded numbers and Latin words keep their correct order, and long
  text wraps inside its node.
- Data model (`chida-db`): typed tables and columns, validation rules,
  relations, seed rows, an in-memory store with unique enforcement, and
  SQLite DDL, insert and select SQL generation plus migration statements.
- Logic engine (`chida-logic`): flows as data, with events, condition
  trees (compare, and, or, not) and actions: navigate, show a message,
  set or increment values, save and delete records, call an API, log.
- Code generation (`chida-codegen`): from one project, a complete Rust
  workspace with an Axum backend (CRUD routes per table plus register
  and login), a Leptos frontend with one component per page, the SQL
  schema, an OpenAPI 3 description, a dependency free static HTML
  preview, a Dockerfile and a systemd unit. The generated marketplace
  backend builds and was smoke tested live (`/health` and
  `/api/products`).
- CLI (`chida`): `new`, `templates`, `render`, `generate` and `info`
  commands.
- Web editor (`apps/editor`): a Leptos and WebAssembly design tool that
  runs in the browser, with a dark Figma grade interface: tool rail,
  pages and layers panel with visibility and lock toggles, a pan and
  zoom canvas with dot grid, drag and corner resize, tabbed Design,
  Logic, Data and Code panels, a Simple, Builder and Pro mode switcher,
  and localStorage autosave.
- Built-in templates, each a real project with a seeded database and at
  least one flow: `shop` (demo storefront and admin), `persian`
  (right-to-left Persian sample), `digikala` (Marketplace Pro, a Persian
  RTL marketplace with home, category, product and cart pages, rebuilt
  section by section from the live site structure, with 28 openly
  licensed photos credited in `templates/assets/digikala/SOURCES.md`),
  `liquid-glass` (landing, dashboard, shop and mobile in Apple liquid
  glass style), `saas-dashboard`, `clinic`, `restaurant` and
  `portfolio`, plus a `blank` starter.
- Packaging: a Fedora RPM spec and CI workflow, a desktop entry, and a
  Tauri 2 desktop shell with a Leptos UI skeleton under `apps/desktop`
  (code complete; building it needs the WebKitGTK and GTK development
  libraries listed in `apps/desktop/README.md`).
- Cross-platform release artifacts, built by CI from the release tag:
  Linux x86_64 and aarch64 tarballs, an AppImage, a Debian package
  (`.deb`), a Fedora RPM, a Windows x86_64 zip, macOS tarballs for Apple
  Silicon and Intel, and the browser editor zip, all attached to one
  GitHub release with a SHA256SUMS file.

### Known limitations

- Generated apps keep data in memory and seed it at startup; durable
  storage is a documented swap in the generated `state.rs`.
- Password hashing in generated code is salted SHA-256 and is labelled
  a stub in the generated source. Use Argon2id for production.
- The editor has no auto layout yet, and text layout in the rasteriser
  is per node line with wrapping, not a full browser line breaker.
