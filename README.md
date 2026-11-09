<p align="center">
  <img src="assets/logo/logo-horizontal.svg" width="340" alt="Chida logo">
</p>

<h1 align="center">Chida</h1>

<p align="center">
  <strong>Design the page. Chida builds the app.</strong><br>
  A visual full-stack builder written in Rust. Design pages, data tables and
  logic flows in one project, then export a render of the design or a complete
  Rust application you own: Axum backend, Leptos frontend, SQL schema and
  OpenAPI description.
</p>

<p align="center">
  <a href="https://github.com/E30TZ/chida/actions/workflows/ci.yml"><img src="https://github.com/E30TZ/chida/actions/workflows/ci.yml/badge.svg" alt="CI status"></a>
  <a href="https://github.com/E30TZ/chida/releases/latest"><img src="https://img.shields.io/github/v/release/E30TZ/chida" alt="Latest release"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-green.svg" alt="MIT license"></a>
</p>

<p align="center">
  <strong>English</strong> · <a href="README.fa.md">فارسی</a>
</p>

<p align="center">
  <img src="docs/shots/editor.png" width="880" alt="The Chida web editor with a Persian marketplace project open">
</p>

<p align="center"><em>The Chida web editor: a real design tool that runs in the browser, shown here with the Persian RTL marketplace template.</em></p>

Figma gives you a picture. A site builder gives you a hosted page. Chida
gives you the app: the design, the database schema, the API and the source
code, generated as a Rust workspace you can build, run and ship yourself.
Persian and Arabic are first class: text is shaped with HarfBuzz and
rendered right-to-left in PNG, SVG and the generated web output, with the
Vazirmatn font bundled.

## Download

Prebuilt files are attached to every release on the
[releases page](https://github.com/E30TZ/chida/releases). The table lists
the exact names for the current version, 1.0.1. Every file is built by CI
from the release tag, and a `SHA256SUMS` file is attached so downloads can
be verified.

| Platform | File | What it is |
| --- | --- | --- |
| Linux, any distro (x86_64) | `chida-1.0.1-linux-x86_64.tar.gz` | Portable tarball: the `chida` binary, a desktop entry and `install.sh`, which installs into `~/.local/bin` |
| Linux, any distro (x86_64) | `chida-1.0.1-linux-x86_64.AppImage` | Single file build of the CLI; no install, just make it executable and run it |
| Debian, Ubuntu and derivatives | `chida_1.0.1_amd64.deb` | Debian package: installs `/usr/bin/chida`, the desktop entry and the icon |
| Fedora, RHEL and derivatives | `chida-1.0.1-1.fc45.x86_64.rpm` | RPM built in a Fedora 45 container from `packaging/fedora/chida.spec` |
| Linux, any distro (aarch64) | `chida-1.0.1-linux-aarch64.tar.gz` | Portable tarball for ARM64 machines, same layout as the x86_64 tarball |
| Windows (x86_64) | `chida-1.0.1-windows-x86_64.zip` | Zip with `chida.exe`, the README and the license |
| macOS, Apple Silicon | `chida-1.0.1-macos-aarch64.tar.gz` | Tarball with the `chida` binary, the README and the license |
| macOS, Intel | `chida-1.0.1-macos-x86_64.tar.gz` | Tarball with the `chida` binary, the README and the license |
| Any OS with a browser | `chida-editor-1.0.1.zip` | The web editor as static files (HTML, JS, WebAssembly); unzip and serve or open locally |

The Tauri desktop app under `apps/desktop` is not part of the downloads
yet; there are no desktop app installers in this release, only the command
line tool and the browser editor above.

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
`examples-output/generated-app/preview/index.html` for a static preview of
the generated app. The generated backend runs with `cargo run -p backend`
inside `generated-app` (it needs network access the first time, to fetch
Axum and Leptos from crates.io).

```text
chida new <name> [--template shop|blank|persian|digikala|liquid-glass|saas-dashboard|clinic|restaurant|portfolio] [--out <dir>]
chida templates
chida render <project.chida.json> [--out <dir>]
chida generate <project.chida.json> [--out <dir>]
chida info <project.chida.json>
```

## What works today

* **Document model** (`chida-core`): pages, node trees, components, design
  themes, JSON project files (`.chida.json`), validation, undo and redo
  history, and auto layout shared by the editor and the renderer.
* **Rendering** (`chida-render`): a CPU rasteriser that turns any page into
  a PNG (shapes, gradients, shadows, strokes, alpha blending and real
  TrueType text) plus an SVG export with real text elements. Long text
  wraps inside its node, and mixed-direction lines are segmented into bidi
  runs so embedded numbers and Latin words keep their correct order.
* **Data model** (`chida-db`): tables with typed columns, validation rules,
  relations, seed rows, an in-memory store, SQLite DDL, insert and select
  SQL, and simple migration statements.
* **Logic flows** (`chida-logic`): flows as data. An event, a condition
  tree (compare, and, or, not) and actions: navigate, show a message, set
  or increment values, save and delete records, call an API, log.
* **Code generation** (`chida-codegen`): from one project, a complete Rust
  workspace. An Axum backend with CRUD routes for every table plus register
  and login, a Leptos frontend with one component per page, the SQL
  schema, an OpenAPI 3 description, a dependency free static HTML preview,
  a Dockerfile, a systemd unit and a README.
* **Web editor** (`apps/editor`): a working design tool in the browser,
  not a mockup. Dark Figma-style interface, tool rail, pages and layers,
  rulers, snap guides, multi-select and marquee select, auto layout
  controls, named styles, image upload, a full-screen preview mode with
  working page navigation, tabbed Design, Data, Logic and Code panels,
  three levels (Simple, Builder, Pro) so beginners are not overwhelmed,
  and localStorage autosave.
* **Generated apps compile**: the generated Axum backends pass
  `cargo build` and were smoke tested live (`/health` answered ok and
  `/api/products` returned its seeded products). The generated Leptos
  frontend passes `cargo check` on the host target; a wasm32 Trunk build
  of the generated frontend was not verified in this environment.

## Templates

Nine built-in starters, each a real project with pages, a seeded database
and at least one logic flow. Run `chida templates` for the list, and see
`templates/README.md` for details.

| Template | What it builds |
| --- | --- |
| `shop` | Storefront and admin pages, products and orders schema, checkout flow |
| `persian` | Right-to-left Persian sample page with Persian digits and a call to action |
| `digikala` | Marketplace Pro: a Persian RTL marketplace with home, category, product and cart pages and Persian seed products. The sample brand is original |
| `liquid-glass` | A four page liquid glass family: landing, dashboard, shop and mobile |
| `saas-dashboard` | Dark admin dashboard with KPIs, a sales chart and orders and customers tables |
| `clinic` | Doctors, services and an appointment booking form and flow |
| `restaurant` | Featured dishes, a reservation form and a full menu page |
| `portfolio` | Works grid, services and a contact form |
| `blank` | One empty page with a title |

## Gallery

Real renders produced by the Chida renderer from the template projects.
Every picture below is the PNG output of `chida render`, not a mockup.

**Marketplace Pro, Persian RTL marketplace home** (`chida new shop2 --template digikala`)

![Marketplace Pro home](docs/shots/digikala-home.png)

**Persian page, shaped and right-to-left** (`chida new fa --template persian`)

![Persian render](docs/shots/persian.png)

**Demo shop storefront** (`chida new demo`)

![Demo shop storefront rendered by Chida](docs/shots/demo-shop.png)

**Liquid Glass landing** (`chida new glassy --template liquid-glass`)

![Liquid Glass home](docs/shots/liquid-glass-home.png)

**Liquid Glass mobile** (the `/mobile` page of the same project)

![Liquid Glass mobile](docs/shots/liquid-glass-mobile.png)

**SaaS Dashboard, dark admin panel** (`chida new panel --template saas-dashboard`)

![SaaS Dashboard](docs/shots/saas-dashboard.png)

## Desktop app

A Tauri 2 desktop shell with a Leptos UI lives in `apps/desktop`. The code
is complete, but it has not been compiled or packaged in this environment
(the system WebKitGTK and GTK development libraries are not installed
here), so there are no desktop installers yet. Build steps for Fedora 45
are in `apps/desktop/README.md`.

## Honest limitations

* Generated apps store data in memory and seed it at startup. The HTTP API
  and schema are complete; durable storage is a documented swap in the
  generated `state.rs`.
* Password hashing in generated code is salted SHA-256 and is labelled a
  stub in the generated source itself. Use Argon2id for production.
* Long mixed Persian and Latin paragraphs inside one node can order less
  perfectly than a full browser line breaker. Labels, headlines, prices
  and ordinary body lines render correctly in both scripts.
* The desktop shell is not compiled or packaged yet (see above).

## Roadmap

* Package the Tauri desktop app (installers for Linux, Windows, macOS).
* Durable storage (rusqlite or sqlx) in generated apps, Argon2id auth.
* Full paragraph layout in the rasteriser.
* WASM plugin system and installable template packs.
* Collaboration with CRDT sync.
* More templates: school, CRM, blog, real estate, fitness, corporate.

## Repository layout

```text
crates/chida-core     document and scene model, project files, history
crates/chida-render   PNG and SVG rendering
crates/chida-db       tables, validation, store, SQL generation
crates/chida-logic    visual flow engine
crates/chida-codegen  full-stack Rust app generation
crates/chida-cli      the chida command line tool
apps/editor           the web editor (Leptos, WebAssembly)
apps/desktop          Tauri 2 shell (not packaged yet)
templates             built-in template projects and their assets
docs                  screenshots and notes
packaging             Fedora spec and desktop entry
assets                logo and brand files
```

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md). Bug reports and template ideas are
welcome.

## License

MIT. See [LICENSE](LICENSE).

The bundled Vazirmatn font is licensed under the SIL Open Font License
1.1; DejaVu Sans is the fallback font.
