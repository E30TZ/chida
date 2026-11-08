# Chida desktop app

Tauri 2 shell plus a Leptos frontend, for Fedora first.

Status: scaffold, code complete, not compiled on the machine that produced
this repository state. Building it needs system libraries that are not
installed there: WebKitGTK 4.1 and GTK 3 development packages.

On Fedora 45:

```sh
sudo dnf install webkit2gtk4.1-devel gtk3-devel libsoup3-devel rust cargo
cargo install tauri-cli --version "^2"
cd apps/desktop/ui && trunk build --release
cd ../src-tauri && cargo tauri build
```

Bundles configured in `src-tauri/tauri.conf.json`: RPM and AppImage
(the bundle targets list also keeps deb for other distributions).

The Tauri commands in `src-tauri/src/main.rs` call the real `chida-core`
and `chida-render` crates: opening a project and rendering a page to PNG
already work through them once the shell compiles.
