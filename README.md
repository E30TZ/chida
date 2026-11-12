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
    <strong>Build your own site from zero, without writing a single line of code.</strong>
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
 <a href="./README.fa.md">
 🇮🇷 فارسی
 </a>
</p>

<p align="center">
  <a href="https://github.com/E30TZ/chida" target="_blank" rel="noopener noreferrer" >
    <img src="docs/shots/hero.png" alt="The Chida editor" width="880" height="auto">
  </a>
</p>

<p align="center"><i>The editor, doing its job. You draw here; Chida builds the rest.</i></p>

<p align="center">
    <a href="https://e30tz.github.io/chida/" target="_blank"><strong>▶ Try the editor live in your browser</strong></a> - no download, no install.
</p>

## 📋 Table of Contents

-   [📖 What is Chida?](#-what-is-chida)
-   [👀 See it](#-see-it)
-   [✨ What you can do](#-what-you-can-do)
-   [🚀 Try it now](#-try-it-now)
-   [🔌 WordPress themes](#-wordpress-themes)
-   [🛠️ For developers](#-for-developers)
-   [📚 Documentation](#-documentation)
-   [📄 License](#-license)

---

# 📖 What is Chida?

Suppose you want a small shop on the internet. With Chida you do three things:

**1. You draw the shop page.** A large title. Pictures of the products. A price under each one. You drag boxes with the mouse, as in a drawing program.

**2. You define what a product is.** A name, a price, a photo. Three entries you type once, in a simple table.

**3. You press one button.** Chida returns the complete shop to you: the pages you drew, a program that keeps the products and takes orders, and **all of its code**. The code belongs to you. You can open it, modify it, and publish it yourself. Nothing remains locked inside Chida.

That is the entire idea. Version 2 adds what a real design tool requires: a vector pen, components, groups, smart guides, stronger auto layout, richer color and effects, and clicks that perform actions. Chida is written in Rust, so it is fast and light. Persian and Arabic text is rendered right-to-left and correctly in every result.

---

# 👀 See it

These pictures are real. Chida drew each one from a ready-made project with a single command, and the command is written under each picture so you can produce the same one yourself.

|  |  |
| :---: | :---: |
| ![Big Persian shop](docs/shots/shot-shop.png) | ![Persian page](docs/shots/shot-fa.png) |
| **A big Persian shop, reading right to left** | **A Persian page, simple and clean** |
| `chida new shop2 --template digikala` | `chida new fa --template persian` |
| ![Liquid glass page](docs/shots/shot-glass.png) | ![The same page on a phone](docs/shots/shot-mobile.png) |
| **A page that looks like frosted glass** | **The same page, on a phone** |
| `chida new glassy --template liquid-glass` |  |
| ![Dashboard](docs/shots/shot-panel.png) | ![Demo shop](docs/shots/shot-demo.png) |
| **A control panel for the shop owner** | **A small shop** |
| `chida new panel --template saas-dashboard` | `chida new demo` |

---

# ✨ What you can do

<div align="left">

**🎨 Draw pages that look real**
- Pages, layers, rulers and snap guides, like a real design tool
- Drag, resize, select many items at once, and draw a box to select
- Draw your own shapes with the **pen**: click for corners, drag for curves, then move, add or delete points whenever you wish
- Ready shapes are available as well: **polygon, star and arrow**, all drawn as real vector paths
- Create a **component** once and place copies anywhere. Change the master and every copy follows. Change one copy's text or color and only that copy changes. Detach a copy when you want it on its own
- **Group** items and move or resize them as one; drag layers to reorder them and double-click a layer to rename it
- Pull guides out of the ruler and snap to them; smart guides snap items into line and show the gap between them
- Boxes arrange themselves in rows and columns: a box can **hug** its contents or **fill** its parent, padding can differ per side, and rows can wrap
- Colors can pass through as many **gradient stops** as you wish; **shadows can stack**; an item, or what lies behind it, can blur
- Define what a **click** does: go to another page or open a link. Hover works in the preview as well
- A complete shelf of ready **sections** to drop in: navbars, heroes, features, prices, customer words, galleries, forms, footers and calls to action
- Save a look once and reuse it everywhere (named styles)
- Bring your own pictures in
- Press preview and click around: the page links really work
- Start in **Simple** mode with large buttons. Open **Builder**, then **Pro**, only when you want more. The final level shows even the code

**📄 Turn a drawing into a working app**
- One press gives you a **PNG picture** or an **SVG file** of any page
- One command writes the **whole app**: the program behind the pages, the screens people see, the database plan and the API plan
- The apps Chida writes **build and run**. Their backends were started and tested for real

**🗂️ Keep lists and make rules, without code**
- Your shop keeps products, your clinic keeps patients: every list is a table, and you choose its columns (name = text, price = number, and so on)
- Rules are boxes you connect: *if this happens, then do that*
- A rule can open a page, show a message, save or delete a row, or call another program

**🌍 Type Persian and Arabic the right way**
- Letters join and shape correctly, text runs right-to-left, in pictures, in SVG and in web pages
- The **Vazirmatn** font comes inside Chida
- In a mixed line, numbers and English words stay in the right order

**🔌 WordPress themes, in and out**
- Already have a WordPress theme? Bring its **zip file** in: Chida opens it as pages you can edit (header, home, single post, page, footer)
- The living parts stay alive: the post loop, the menu, titles, dates and content come back out as real WordPress code
- Press one button and download the theme zip again, ready to install in WordPress

</div>

Every project starts from a ready-made one, so you never face an empty page: **shop**, **digikala** (the big Persian shop above), **persian**, **liquid-glass** (the glass page above), **saas-dashboard**, **clinic**, **restaurant**, **portfolio**, **blank**.

---

# 🚀 Try it now

Three steps. No account. No payment.

### Step 1: take the file for your computer

All files live on one page: the [download page](https://github.com/E30TZ/chida/releases/latest). Pick yours:

| Your computer | Take this file | Then what |
| --- | --- | --- |
| Windows | `chida-2.0.0-windows-x86_64.zip` | Unzip it, run `chida.exe` |
| Mac with Apple Silicon | `chida-2.0.0-macos-aarch64.tar.gz` | Unzip it, run `chida` |
| Mac with Intel | `chida-2.0.0-macos-x86_64.tar.gz` | Unzip it, run `chida` |
| Linux, any distro | `chida-2.0.0-linux-x86_64.tar.gz` | Unzip it, run `chida`. No install |
| Linux, one single file | `chida-2.0.0-linux-x86_64.AppImage` | Make it executable, double-click |
| Debian or Ubuntu | `chida_2.0.0_amd64.deb` | Install it like any program |
| Fedora or RHEL | `chida-2.0.0-1.fc45.x86_64.rpm` | Install it like any program |
| Linux on ARM64 | `chida-2.0.0-linux-aarch64.tar.gz` | Unzip it, run `chida` |
| Only the editor, in a browser | `chida-editor-2.0.0.zip` | Unzip it, open it (note below) |

The `SHA256SUMS` file on the same page lets you check your download arrived whole.

**You want the editor in its own window, no terminal?** Take a desktop installer from the same page: `Chida_2.0.0_x64-setup.exe` (Windows), `Chida_2.0.0_x64.dmg` (Mac Intel), `Chida_2.0.0_aarch64.dmg` (Mac Apple Silicon), `Chida-desktop_2.0.0_amd64.deb` (Debian, Ubuntu), `Chida-desktop_2.0.0_amd64.AppImage` (any Linux), `Chida-2.0.0-1.x86_64.rpm` (Fedora, RHEL).

### Step 2: copy two lines

Open a terminal in the folder where you put Chida and paste these two lines, one at a time:

```bash
./chida new myshop --template shop
./chida render myshop/myshop.chida.json --out myshop-pictures
```

The first line makes a small shop project. The second line draws its pages as pictures.

### 📋 Step 3: look at what you got

<div align="left">

**🖼️ Open the folder** `myshop-pictures`. The pictures inside are your shop, drawn by Chida. That is the entire idea, working in your hands.

**✏️ If you prefer clicking to typing:** unzip `chida-editor-2.0.0.zip` and open the editor in your browser. If the page stays white, serve the folder with any small local web server: browsers do not load the editor straight from a file.

**🗂️ One file is the whole project:** `myshop/myshop.chida.json` holds the pages, the tables and the rules. Move it, copy it, send it to a friend.

</div>

### 🔧 Next: get the app itself

```bash
# See every ready-made starting point
./chida templates

# Get the whole app, with all its code
./chida generate myshop/myshop.chida.json --out myshop-app
```

---

# 🔌 WordPress themes

Chida reads WordPress themes and writes them back.

**Bring a theme in.** On the editor start screen, pick **WordPress theme** and choose the theme zip file. The theme opens as pages: header, home, single post, page and footer. The colours, sizes and layout come from the theme's own style sheet.

**Edit it like any Chida page.** Move boxes, change colours, fix the text. The parts that print your posts (the loop, the menu, titles, dates, featured pictures) are marked blocks on the page. Leave them where they are or move them around; they stay alive either way.

**Take the theme back.** Press **WordPress export** in the top bar. You get a zip file holding a ready theme: `style.css`, `header.php`, `index.php`, `single.php`, `page.php`, `footer.php`, `functions.php`. Install it in WordPress like any other theme.

The same works from the command line:

```bash
./chida wp-import mytheme.zip mytheme.chida.json
./chida wp-export mytheme.chida.json mytheme-back.zip
```

---

# 🛠️ For developers

Chida itself is a Rust workspace. A project is one JSON file (`name.chida.json`). The renderer shapes text with HarfBuzz (rustybuzz) and rasterises with fontdue. The app Chida writes for you is an Axum backend with one CRUD route set per table plus sign-up and login, a Leptos frontend with one component per page, the SQL schema, an OpenAPI 3 file, a static HTML preview, a Dockerfile and a systemd unit.

```bash
cargo build --release   # build Chida itself
cargo test              # 89 tests
```

```text
crates/chida-core     pages, elements, project files, undo history
crates/chida-render   PNG and SVG drawing
crates/chida-db       tables, rules, SQL writing
crates/chida-logic    the flow engine (if this, then that)
crates/chida-codegen  writes the whole app for you
crates/chida-wp       WordPress themes in and out
crates/chida-cli      the chida command
apps/editor           the editor in the picture above (Leptos, WebAssembly)
apps/desktop          the editor in a desktop window (Tauri 2)
templates             the ready-made starting points
```

---

# 📚 Documentation

<div align="left">

**📖 This page in other languages:**

🇮🇷 **[فارسی](./README.fa.md)**

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
