<p align="center">
  <a href="https://github.com/E30TZ/chida" target="_blank" rel="noopener noreferrer">
    <picture>
      <source media="(prefers-color-scheme: dark)" srcset="assets/logo/logo-horizontal-dark.svg">
      <img width="340" src="assets/logo/logo-horizontal.svg" alt="Chida logo">
    </picture>
  </a>
</p>

<h1 align="center">🧩 Chida</h1>

<p align="center">
    <strong>You draw a page. Chida turns it into a real app.</strong>
</p>

---

<br/>
<p align="center">
    <a href="https://github.com/E30TZ/chida/actions/workflows/ci.yml" target="_blank">
        <img src="https://img.shields.io/github/actions/workflow/status/E30TZ/chida/ci.yml?style=flat-square" />
    </a>
    <a href="https://github.com/E30TZ/chida/releases/latest" target="_blank">
        <img src="https://img.shields.io/github/v/release/E30TZ/chida?style=flat-square" />
    </a>
    <a href="https://github.com/E30TZ/chida/blob/main/LICENSE" target="_blank">
        <img src="https://img.shields.io/github/license/E30TZ/chida?style=flat-square" />
    </a>
    <a href="https://github.com/E30TZ/chida" target="_blank">
        <img src="https://img.shields.io/github/stars/E30TZ/chida?style=social" />
    </a>
</p>

<p align="center">
 <a href="./README-fa.md">
 🇮🇷 فارسی
 </a>
</p>

<p align="center">
  <a href="https://github.com/E30TZ/chida" target="_blank" rel="noopener noreferrer" >
    <img src="docs/shots/editor.png" alt="The Chida editor" width="880" height="auto">
  </a>
</p>

## 📋 Table of Contents

> **Quick Navigation** - Jump to any section below

-   [📖 Overview](#-overview)
    -   [🤔 Why Chida?](#-why-chida)
        -   [✨ Features](#-features)
-   [🚀 Installation guide](#-installation-guide)
-   [🖼️ Gallery](#-gallery)
-   [⚠️ What is not ready yet](#-what-is-not-ready-yet)
-   [📚 Documentation](#-documentation)
-   [📄 License](#-license)

---

# 📖 Overview

> **What is Chida?**

Think of a shop. You draw the shop page: a big title, some pictures, a price under each product. Then you tell Chida what a product is: a name, a price, a photo. Then you press one button. Chida gives you back the whole shop as a real app that runs on a computer: the pages you drew, a place that keeps the products, and all the code. The code is yours. You can open it, change it, and put the app on the internet yourself. Nothing is locked inside Chida. Chida is written in Rust, and Persian and Arabic text is shaped and drawn right-to-left in every output.

---

## 🤔 Why Chida?

> **You draw. You own the code.**

A design tool gives you a picture. A site builder gives you a page on their computer, and takes it away when you stop paying. Chida gives you the app itself, as plain source code you own. And you do not need to know how to code to start: Chida opens in a simple mode with big buttons and few choices. When you want more, open the next level. The third level shows everything, even the code.

---

### ✨ Features

<div align="left">

**🎨 A real editor, in your browser**
- A design tool with pages, layers, rulers and snap guides
- Drag, resize, multi-select and marquee select
- Auto layout, named styles and image upload
- A full-screen preview where the page links really work
- Three levels: **Simple**, **Builder**, **Pro** - power stays hidden until you ask for it
- Your work saves itself in the browser

**📄 Pages become apps**
- Any page becomes a **PNG picture** or an **SVG file**
- One command writes the **whole app**: an Axum backend, a Leptos frontend, the SQL schema and an OpenAPI file
- The apps Chida writes **build and run**; their backends were tested live

**🗂️ Data and logic, no code**
- Tables with typed columns, rules and relations
- Flows you build from boxes: *if this happens, do that*
- Actions: go to a page, show a message, save or delete a record, call an API

**🌍 Persian and Arabic first**
- Text is shaped with **HarfBuzz** and drawn right-to-left in PNG, SVG and web output
- The **Vazirmatn** font ships with Chida
- Mixed lines keep numbers and Latin words in the right order

**📦 Ready-made starting points**
- **shop**, **digikala** (a big Persian shop), **persian**, **liquid-glass**, **saas-dashboard**, **clinic**, **restaurant**, **portfolio**, **blank**
- Every one is a real project with pages, seed data and at least one flow

</div>

---

# 🚀 Installation guide

> **Quick Start** - Your first page in three steps

### Step 1: get Chida for your computer

Take the file for your system from the [download page](https://github.com/E30TZ/chida/releases/latest):

**Windows:**
```
chida-1.0.1-windows-x86_64.zip
```

**Mac (Apple Silicon):**
```
chida-1.0.1-macos-aarch64.tar.gz
```

**Mac (Intel):**
```
chida-1.0.1-macos-x86_64.tar.gz
```

**Linux:**
```
chida-1.0.1-linux-x86_64.tar.gz        (any distro, no install)
chida-1.0.1-linux-x86_64.AppImage      (one file, no install)
chida_1.0.1_amd64.deb                  (Debian, Ubuntu)
chida-1.0.1-1.fc45.x86_64.rpm          (Fedora, RHEL)
chida-1.0.1-linux-aarch64.tar.gz       (ARM64)
```

**Any computer with a browser (the editor only):**
```
chida-editor-1.0.1.zip
```

A `SHA256SUMS` file on the same page lets you check that your download arrived whole. Desktop app installers (the editor in its own window) are being built now and will be added to the same page.

---

### Step 2: make your first page

Open a terminal in the folder where you put Chida and copy these lines, one by one:

```bash
./chida new myshop --template shop
./chida render myshop/myshop.chida.json --out myshop-pictures
```

### 📋 After that:

<div align="left">

**🖼️ Open the folder** `myshop-pictures` - the pictures inside are your shop, drawn by Chida

**✏️ Rather click than type?** Unzip `chida-editor-1.0.1.zip` and open the editor in your browser (if the page stays white, serve the folder with any small local web server; browsers do not load the editor straight from a file)

**🗂️ Your project is one file:** `myshop/myshop.chida.json` holds the pages, the tables and the flows

</div>

### 🔧 Next Steps:

```bash
# See every ready-made starting point
./chida templates

# Get the whole app, with all its code
./chida generate myshop/myshop.chida.json --out myshop-app
```

---

# 🖼️ Gallery

Every picture below is real: Chida drew it from a ready project with `chida render`. None of them is a painting made for this page.

**A Persian shop that reads right to left** - `chida new shop2 --template digikala`

![Marketplace home](docs/shots/digikala-home.png)

**A Persian page** - `chida new fa --template persian`

![Persian page](docs/shots/persian.png)

**A small shop** - `chida new demo`

![Demo shop](docs/shots/demo-shop.png)

**A page that looks like frosted glass** - `chida new glassy --template liquid-glass`

![Liquid glass page](docs/shots/liquid-glass-home.png)

**The same page on a phone**

![Liquid glass phone page](docs/shots/liquid-glass-mobile.png)

**A control panel for a shop owner** - `chida new panel --template saas-dashboard`

![Dashboard](docs/shots/saas-dashboard.png)

---

# ⚠️ What is not ready yet

> **We say it plainly**

<div align="left">

-   The app Chida makes for you keeps its lists in memory. When the app stops, the lists start again from the seed data. Saving to a real database is the next big job.
-   Passwords in the apps Chida makes are locked with a simple method for now, and the code says so itself, in the open. Do not use it for real people's passwords yet.
-   Very long texts that mix Persian and English in one box can come out in a slightly wrong order. Titles, prices and normal lines are fine.
-   The desktop window works but is new; its ready installers are still being built.

</div>

---

# 🛠️ For developers

Chida itself is a Rust workspace. A project is one JSON file (`name.chida.json`). The renderer shapes text with HarfBuzz (rustybuzz) and rasterises with fontdue. The app Chida writes for you is an Axum backend with one CRUD route set per table plus sign-up and login, a Leptos frontend with one component per page, the SQL schema, an OpenAPI 3 file, a static HTML preview, a Dockerfile and a systemd unit.

```bash
cargo build --release   # build Chida itself
cargo test              # 73 tests
```

```text
crates/chida-core     pages, elements, project files, undo history
crates/chida-render   PNG and SVG drawing
crates/chida-db       tables, rules, SQL writing
crates/chida-logic    the flow engine (if this, then that)
crates/chida-codegen  writes the whole app for you
crates/chida-cli      the chida command
apps/editor           the editor in the picture above (Leptos, WebAssembly)
apps/desktop          the editor in a desktop window (Tauri 2)
templates             the ready-made starting points
```

---

# 📚 Documentation

<div align="left">

**📖 This page in other languages:**

🇮🇷 **[فارسی](./README-fa.md)**

**📂 More in this repo:**

-   [Changelog](./CHANGELOG.md) - what changed in each version
-   [Contributing](./CONTRIBUTING.md) - how to help
-   [Templates](./templates/README.md) - every starting point in detail
-   [Desktop app](./apps/desktop/README.md) - build the desktop window yourself

</div>

---

# 📄 License

MIT. See [LICENSE](./LICENSE). You can use Chida for free, also for paid work.

The Vazirmatn font that ships with Chida is under the SIL Open Font License 1.1. DejaVu Sans is the backup font.

---

<p align="center">
  Made with ❤️ and Rust
</p>
