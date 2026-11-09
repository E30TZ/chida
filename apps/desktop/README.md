# Chida desktop app

Tauri 2 shell whose frontend is the real Chida web editor
(`apps/editor`, Leptos and WebAssembly). The desktop window loads the
editor's static build from `apps/editor/dist` (see `frontendDist` in
`src-tauri/tauri.conf.json`), and the Tauri commands in
`src-tauri/src/main.rs` call the real `chida-core` and `chida-render`
crates, so opening a project and rendering a page to PNG work natively.

The older `ui/` crate in this folder is a retired early skeleton. It is
not used by the desktop build anymore.

## Downloads

Desktop installers are built by the `Desktop` GitHub Actions workflow
and attached to releases: `.deb`, `.AppImage` and `.rpm` for Linux, an
NSIS installer (`.exe`) for Windows and `.dmg` for macOS (Apple Silicon
and Intel).

## Build it yourself

System packages, Fedora 45:

```sh
sudo dnf install webkit2gtk4.1-devel gtk3-devel libsoup3-devel \
    librsvg2-devel rust cargo
```

Debian and Ubuntu:

```sh
sudo apt install libwebkit2gtk-4.1-dev libgtk-3-dev libsoup-3.0-dev \
    librsvg2-dev libayatana-appindicator3-dev patchelf
```

Then:

```sh
cargo install trunk --locked
cargo install tauri-cli --version "^2" --locked
rustup target add wasm32-unknown-unknown

# 1. Build the editor frontend
cd apps/editor && trunk build --release && cd ../..

# 2. Generate the platform icons from the source PNG
cd apps/desktop/src-tauri && cargo tauri icon icons/icon.png

# 3. Build and bundle
cargo tauri build
```

Bundles land in `apps/desktop/src-tauri/target/release/bundle/`. The
default bundle targets on Linux are deb, AppImage and RPM; pass
`--bundles nsis` on Windows or `--bundles dmg` on macOS to build those
installers instead.
