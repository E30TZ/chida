<p align="center"><img src="assets/logo/logo-horizontal.svg" width="320" alt="Chida logo"></p>

# Chida

A visual full-stack builder written in Rust. Design pages, data tables and
logic flows in one project, then export a picture of the design or a
complete Rust application you own.

Figma gives you a picture. A site builder gives you a hosted page. Chida
gives you the app: the design, the database schema, the API and the source
code.

![Demo shop storefront rendered by Chida](examples-output/render/home.png)

The screenshot above is real output: it was produced by running
`chida new demo` and `chida render` with the code in this repository.

## Download

Release files are on the
[releases page](https://github.com/E30TZ/chida/releases). Every file is
built by CI from the release tag; the table lists the exact names for
version 1.0.0.

| Platform | File | What it is |
| --- | --- | --- |
| Linux, any distro (x86_64) | `chida-1.0.0-linux-x86_64.tar.gz` | Portable tarball with the `chida` binary, a desktop entry and `install.sh`, which installs into `~/.local/bin` |
| Linux, any distro (x86_64) | `chida-1.0.0-linux-x86_64.AppImage` | Single file build of the CLI; no install, just make it executable and run it |
| Debian, Ubuntu and derivatives | `chida_1.0.0_amd64.deb` | Debian package: installs `/usr/bin/chida`, the desktop entry and the icon |
| Fedora, RHEL and derivatives | `chida-1.0.0-1.fc45.x86_64.rpm` | RPM built in a Fedora 45 container from `packaging/fedora/chida.spec` |
| Linux, any distro (aarch64) | `chida-1.0.0-linux-aarch64.tar.gz` | Portable tarball for ARM64 machines, same layout as the x86_64 tarball |
| Windows (x86_64) | `chida-1.0.0-windows-x86_64.zip` | Zip with `chida.exe`, the README and the license |
| macOS, Apple Silicon | `chida-1.0.0-macos-aarch64.tar.gz` | Tarball with the `chida` binary, the README and the license |
| macOS, Intel | `chida-1.0.0-macos-x86_64.tar.gz` | Tarball with the `chida` binary, the README and the license |
| Any OS with a browser | `chida-editor-1.0.0.zip` | The web editor as static files (HTML, JS, WebAssembly); unzip and serve or open locally |

A `SHA256SUMS` file is attached to each release so downloads can be
verified. The Tauri desktop app under `apps/desktop` is not part of the
1.0.0 downloads yet; there are no desktop app installers in this release,
only the command line tool and the browser editor above.

## What works today

* **Document model** (`chida-core`): pages, node trees, components, design
  themes, JSON project files (`.chida.json`), validation, undo and redo
  history.
* **Rendering** (`chida-render`): a CPU rasteriser that turns any page into
  a PNG (solid and rounded shapes, ellipses, strokes, alpha blending and
  real TrueType text) plus an SVG export with real text elements. Text is
  shaped with HarfBuzz (rustybuzz) and rasterised anti-aliased with
  fontdue, using the bundled Vazirmatn font (OFL 1.1) with DejaVu Sans as
  fallback, so Persian and Arabic shape, join and run right-to-left in PNG
  exactly as they do in SVG. Try it with
  `chida new persian-demo --template persian` and look at
  `examples-output/render/persian.png`.
* **Data model** (`chida-db`): tables with typed columns, validation rules,
  relations, seed rows, an in-memory store, SQLite DDL, insert and select
  SQL, and simple migration statements.
* **Logic flows** (`chida-logic`): flows as data. An event, a condition
  tree (compare, and, or, not) and actions: navigate, show a message, set
  or increment values, save and delete records, call an API, log.
* **Code generation** (`chida-codegen`): from one project, a complete
  Rust workspace. An Axum backend with CRUD routes for every table plus
  register and login, a Leptos frontend with one component per page, the
  SQL schema, an OpenAPI 3 description, a dependency free static HTML
  preview, a Dockerfile, a systemd unit and a README.
* **CLI** (`chida`): four commands, shown below.
* **Shop template**: `chida new demo` builds a working sample with two
  pages (storefront and admin), a products and orders schema with a
  relation, seed products and a checkout flow.
* **Persian template**: `chida new fa --template persian` builds a
  right-to-left Persian sample page (headline, body, price in Persian
  digits and a call to action) that renders shaped in PNG and SVG.
* **Marketplace template**: `chida new shop2 --template digikala`
  builds Marketplace Pro, a Persian RTL marketplace with home,
  category, product and cart pages, four tables with Persian seed
  products, and add to cart plus search flows. The sample brand is
  original; see `templates/README.md`.
* **Liquid glass templates**: `chida new glassy --template
  liquid-glass` builds a four page liquid glass family (landing,
  dashboard, shop, mobile). Renders approximate the glass with
  translucency and glow; generated web output uses a real
  backdrop-filter. See `templates/README.md`.
* **More full templates**: `saas-dashboard` (dark admin dashboard
  with KPIs, a sales chart and orders and customers tables),
  `clinic` (doctors, services and an appointment booking form and
  flow), `restaurant` (featured dishes, a reservation form and a
  full menu page) and `portfolio` (works grid, services and a
  contact form). Each is a real project with a seeded database and
  at least one logic flow; see `templates/README.md`.
* **Template list**: `chida templates` prints every built-in template
  with a short description.
* **Generated apps compile**: the generated Axum backend in
  `examples-output/generated-app` passes `cargo check` and `cargo build`
  and serves its seeded `/api/products` data when run; the generated
  Leptos frontend passes `cargo check` on the host target (the wasm32
  target and a Trunk build were not verified in this environment). The
  backends generated from the richer `digikala` and `liquid-glass`
  templates also build with `cargo build`, and the marketplace
  backend was smoke tested live: `/health` answered ok and
  `/api/products` returned its seeded Persian products.

## Templates gallery

Real renders produced by the Chida renderer from the built-in
template projects. Every picture below is the PNG output of the
command under it, not a mockup.

**Marketplace Pro, Persian RTL marketplace home**
![Marketplace Pro home](docs/shots/digikala-home.png)
Create it with `chida new shop2 --template digikala`, then render with `chida render`.

**Liquid Glass landing**
![Liquid Glass home](docs/shots/liquid-glass-home.png)
Create it with `chida new glassy --template liquid-glass`, then render with `chida render`.

**Liquid Glass mobile**
![Liquid Glass mobile](docs/shots/liquid-glass-mobile.png)
The `/mobile` page of the same `liquid-glass` project above.

**SaaS Dashboard, dark admin panel**
![SaaS Dashboard](docs/shots/saas-dashboard.png)
Create it with `chida new panel --template saas-dashboard`, then render with `chida render`.

## Editor

The Chida web editor, running in the browser. It is a real design tool,
not a mockup: a dark interface with a tool rail, a pages and layers
panel, a pan and zoom canvas, drag and corner resize, tabbed Design,
Logic, Data and Code panels, three levels (Simple, Builder, Pro) and
localStorage autosave. It is built with Leptos and WebAssembly from
`apps/editor`; see that folder's README for build steps.

![Chida web editor](docs/shots/editor.png)

## Quick start

You need a Rust toolchain (rustup is fine). No other system dependencies
are needed for the CLI and the libraries.

```sh
cargo build --release
./target/release/chida new demo --out examples-output
./target/release/chida info examples-output/demo.chida.json
./target/release/chida render examples-output/demo.chida.json --out examples-output/render
./target/release/chida generate examples-output/demo.chida.json --out examples-output/generated-app
cargo test
```

Then open `examples-output/render/home.png` for the design,
`examples-output/render/home.svg` for the vector version, and
`examples-output/generated-app/preview/index.html` for a static preview
of the generated app. The generated backend runs with
`cargo run -p backend` inside `generated-app` (it needs network access
the first time, to fetch Axum and Leptos from crates.io).

## Commands

```text
chida new <name> [--template shop|blank|persian|digikala|liquid-glass|saas-dashboard|clinic|restaurant|portfolio] [--out <dir>]
chida templates
chida render <project.chida.json> [--out <dir>]
chida generate <project.chida.json> [--out <dir>]
chida info <project.chida.json>
```

## Desktop app

A Tauri 2 desktop shell with a Leptos UI skeleton lives in `apps/desktop`,
targeting Fedora RPM and AppImage bundles. It is code complete but is not
compiled in the environment where this foundation was built, because the
system WebKitGTK 4.1 and GTK 3 development libraries are not installed
there. Build steps for Fedora 45 are in `apps/desktop/README.md`.

## Honest limitations of this foundation

* Text layout is single line per node line: there is no automatic word
  wrapping, and one paragraph is shaped as one direction run, so long
  mixed Persian and Latin sentences inside one node can order less
  perfectly than a full browser line breaker would. Short labels,
  headlines, prices and body lines render correctly in both scripts.
* Generated apps store data in memory and seed it at startup. The HTTP API
  and schema are complete; durable storage is a documented swap in the
  generated `state.rs`.
* Password hashing in generated code is salted SHA-256 and is labelled a
  stub in the generated source itself. Use Argon2id for production.
* The web editor (apps/editor) is a working design tool: tool rail,
  pages and layers with eye and lock toggles, pan and zoom canvas,
  drag, corner resize, tabbed Design, Logic, Data and Code panels,
  and localStorage autosave. The Tauri desktop shell that will host
  the same UI is still not compiled in this environment (see the
  Desktop app section), and the editor has no auto layout yet.

## Roadmap

* Auto layout in the editor, and hosting the web editor UI inside
  the Tauri desktop shell.
* Full paragraph layout: word wrapping and multi run mixed direction
  lines in the rasteriser.
* Durable storage (rusqlite or sqlx) in generated apps, Argon2id auth.
* WASM plugin system and installable template packs.
* Collaboration with CRDT sync.
* Fedora RPM and AppImage attached to GitHub releases.

## Repository layout

```text
crates/chida-core     document and scene model, project files, history
crates/chida-render   PNG and SVG rendering
crates/chida-db       tables, validation, store, SQL generation
crates/chida-logic    visual flow engine
crates/chida-codegen  full-stack Rust app generation
crates/chida-cli      the chida command line tool
apps/desktop          Tauri 2 shell and Leptos UI skeleton (Fedora)
examples-output       demo project, renders and a generated app
packaging             Fedora spec and desktop entry
```

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md). Bug reports and template ideas are
welcome.

## License

MIT. See [LICENSE](LICENSE).
