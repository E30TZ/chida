# Chida Editor, Web Edition

The Chida editor as a Leptos CSR web app with a professional dark
design tool UI in the spirit of Figma and FigCraft, Persian first.
This is also the future desktop editor UI: the desktop shell will
host this same interface.

## Layout

- Top bar: Chida logo and project name, undo and redo buttons, the
  three level mode switcher (ساده، سازنده، حرفه‌ای), zoom percentage
  with zoom in, zoom out and fit buttons, and the actions رندر
  (downloads a quick SVG render of the current page), ساخت کد
  (opens the Code tab), انتشار (disabled in the web edition, the
  tooltip explains why) and خروجی JSON (downloads the project).
- Tool rail: انتخاب، دست، فریم، مستطیل، دایره، متن، تصویر، دکمه،
  کارت، ورودی with Persian tooltips. With a shape tool active,
  clicking the canvas adds that node at the click point.
- Left panel: pages list on top, layers tree below it in stacking
  order with a type icon, name, a working eye toggle (hidden nodes
  disappear from the canvas) and a working lock toggle (locked
  nodes cannot be dragged).
- Canvas: dark dot grid workspace, the page as a white artboard
  with a shadow and a name plus dimensions label, pan by dragging
  the workspace background or with the hand tool, zoom from 25 to
  200 percent, a blue selection outline with four corner handles,
  and corner drag resizing. A wide page fits to the workspace on
  load.
- Right panel tabs: طراحی (Design) with position and size, text
  (content, font size, bold, alignment), fill colour with hex,
  corner radius, stroke and opacity, every control live; منطق
  (Logic, Pro level) listing project flows as readable Persian
  lines; داده (Data, Builder level up) listing tables, columns
  and seed row counts; کد (Code, Pro level) with the full bundle
  JSON download, an SVG render download and a JSON preview.
- Bottom status strip: node count, selected node id and type,
  zoom, and the save state. Every edit is stored to localStorage
  with a timestamp and restored on reload; بازنشانی (reset)
  clears it and returns to the sample project.

## The simplicity layer

- ساده (Simple): layers show friendly block names only, and the
  Design tab shows just text, colour and size with big controls.
- سازنده (Builder): the full Design tab plus the Data tab.
- حرفه‌ای (Pro): adds the Logic and Code tabs.

## Data

The editor loads `fixtures/demo-project.json`, the full bundle
(design project, database and flows) generated from the real Chida
CLI shop template, not hand-written data. Regenerate it with
`./generate-fixture.sh`.

## Build

This crate targets `wasm32-unknown-unknown` only, so it is excluded
from the root workspace and declares its own workspace.

```sh
rustup target add wasm32-unknown-unknown
trunk build --release
python3 -m http.server --directory dist 8902
```

## Verification

Verified with scripted Playwright clicks against the served build
(selection and four handles, inspector fields, Pro tabs, the real
flow shown in the Logic tab, adding a text node from the tool
rail, drag moving a node by the exact mouse delta, corner handle
resizing, the eye toggle hiding a node, zoom stepping, and
localStorage persistence with the save timestamp), plus the
screenshot in `screenshot.png`, taken at 1600x1000 with the demo
shop artboard and the hero title selected.

Not verified: the publish action (disabled by design in the web
edition), and full code generation inside the browser (the Code
tab explains that `chida generate` runs on the project file).
