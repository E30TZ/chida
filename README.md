<p align="center">
  <img src="assets/logo/logo-horizontal.svg" width="340" alt="Chida logo">
</p>

<h1 align="center">Chida</h1>

<p align="center">
  <strong>You draw a page. Chida turns it into a real app.</strong>
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
  <img src="docs/shots/editor.png" width="880" alt="The Chida editor">
</p>

<p align="center"><em>This is Chida. You move boxes, text and pictures onto the page, like in a drawing program.</em></p>

## What is Chida?

Think of a shop.

You draw the shop page: a big title, some pictures, a price under each
product. Then you tell Chida what a product is: a name, a price, a photo.
Then you press one button.

Chida gives you back the whole shop as a real app that runs on a
computer: the pages you drew, a place that keeps the products, and all
the code. The code is yours. You can open it, change it, and put the app
on the internet yourself. Nothing is locked inside Chida.

That is the whole idea: **you draw, Chida builds.**

You do not need to know how to code. Chida starts in a simple mode with
big buttons and few choices. When you want more, you can open the next
level and do more. The third level shows everything, even the code.

## Start here: 3 steps

**Step 1: get Chida.**
Go to the [download page](https://github.com/E30TZ/chida/releases/latest)
and take the file for your computer:

- Windows: the file that ends in `.zip`
- Mac: the file with `macos` in its name
- Linux: the `.tar.gz` file, or the package for your system (`.deb`,
  `.rpm`, `.AppImage`)

**Step 2: make your first page.**
Open a terminal in the folder where you put Chida and copy these lines,
one by one:

```sh
./chida new myshop --template shop
./chida render myshop/myshop.chida.json --out myshop-pictures
```

Now open the folder `myshop-pictures`. The pictures inside are your
shop, drawn by Chida.

**Step 3: get the full app.**
Copy this line:

```sh
./chida generate myshop/myshop.chida.json --out myshop-app
```

The folder `myshop-app` is your whole shop app, with all its code.

**Rather click than type?** Download `chida-editor-1.0.1.zip` from the
same page, unzip it, and open the editor in your browser. (If the page
stays white, serve the folder with any small local web server; browsers
do not load the editor straight from a file.)

## What can you make?

A shop. A page for a restaurant, with a menu and a table booking form. A
clinic page where people book a visit. A page about you and your work.
The pictures below are real. Chida drew every one of them from a ready
project; they are not paintings made for this page.

**A Persian shop that reads right to left**

![Marketplace home](docs/shots/digikala-home.png)

**A Persian page**

![Persian page](docs/shots/persian.png)

**A small shop**

![Demo shop](docs/shots/demo-shop.png)

**A page that looks like frosted glass**

![Liquid glass page](docs/shots/liquid-glass-home.png)

**The same page on a phone**

![Liquid glass phone page](docs/shots/liquid-glass-mobile.png)

**A control panel for a shop owner**

![Dashboard](docs/shots/saas-dashboard.png)

## Ready-made starting points

You do not start from a blank page unless you want to. Each name below
is a ready project. Use it like in step 2, for example
`./chida new myfood --template restaurant`.

- `shop` : a shop with a home page and an owner page
- `digikala` : a big Persian shop, like the picture above
- `persian` : one simple Persian page
- `liquid-glass` : pages that look like frosted glass
- `saas-dashboard` : a control panel with numbers and lists
- `clinic` : doctors and booking a visit
- `restaurant` : dishes, a menu and booking a table
- `portfolio` : your work, shown in a grid
- `blank` : one empty page

Run `./chida templates` to see this list on your own computer.

## Download table

The same files as step 1, with exact names (version 1.0.1). The file
`SHA256SUMS` on the download page lets you check that a file arrived
whole.

| Your computer | File to take | What you get |
| --- | --- | --- |
| Linux (x86_64) | `chida-1.0.1-linux-x86_64.tar.gz` | Chida in a folder, no install |
| Linux (x86_64) | `chida-1.0.1-linux-x86_64.AppImage` | One file; make it runnable and open it |
| Debian, Ubuntu | `chida_1.0.1_amd64.deb` | A package your system can install |
| Fedora, RHEL | `chida-1.0.1-1.fc45.x86_64.rpm` | A package your system can install |
| Linux (ARM64) | `chida-1.0.1-linux-aarch64.tar.gz` | Chida in a folder, no install |
| Windows | `chida-1.0.1-windows-x86_64.zip` | Chida in a folder, no install |
| Mac, Apple Silicon | `chida-1.0.1-macos-aarch64.tar.gz` | Chida in a folder, no install |
| Mac, Intel | `chida-1.0.1-macos-x86_64.tar.gz` | Chida in a folder, no install |
| Any computer with a browser | `chida-editor-1.0.1.zip` | The editor, as web pages |

Desktop app installers (the editor in its own window) are being built
now and will be added to the same download page.

## What is not ready yet

We say it plainly:

- The app Chida makes for you keeps its products and lists in memory.
  When the app stops, the lists start again from the seed data. Saving
  to a real database is the next big job.
- Passwords in the apps Chida makes are locked with a simple method for
  now. The code says so itself, in the open. Do not use it for real
  people's passwords yet.
- Very long texts that mix Persian and English in one box can come out
  in a slightly wrong order. Titles, prices and normal lines are fine.
- The desktop window works but is new: the ready installers for it are
  still being built (see above).

## For developers

Chida is written in Rust. A project is one JSON file
(`name.chida.json`) that holds the pages, the data tables and the logic
flows. The renderer turns any page into PNG and SVG; the text is shaped
with HarfBuzz (rustybuzz) and drawn with fontdue, with the Vazirmatn font
bundled for Persian and Arabic.

The app that Chida writes for you is a Rust workspace: an Axum backend
with one CRUD route set per table plus sign-up and login, a Leptos
frontend with one component per page, the SQL schema, an OpenAPI 3 file,
a static HTML preview, a Dockerfile and a systemd unit. The generated
backends build with `cargo build` and were tested live: `/health`
answers and `/api/products` returns the seed products.

```sh
cargo build --release   # build Chida itself
cargo test              # 73 tests
./target/release/chida templates
```

```text
crates/chida-core     pages, elements, project files, undo history
crates/chida-render   PNG and SVG drawing
crates/chida-db       tables, rules, SQL writing
crates/chida-logic    the flow engine (if this, then that)
crates/chida-codegen  writes the whole app for you
crates/chida-cli      the chida command
apps/editor           the editor you see in the picture (Leptos, WebAssembly)
apps/desktop          the editor in a desktop window (Tauri 2)
templates             the ready-made starting points
```

See [CONTRIBUTING.md](CONTRIBUTING.md) if you want to help. Bug reports
and ideas for new starting points are welcome.

## License

MIT. See [LICENSE](LICENSE). You can use Chida for free, also for paid
work.

The Vazirmatn font that ships with Chida is under the SIL Open Font
License 1.1. DejaVu Sans is the backup font.
