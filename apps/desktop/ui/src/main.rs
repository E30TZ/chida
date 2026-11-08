
//! Leptos UI skeleton for Chida.
//! Three plain-language levels hide the full-stack power until it is
//! wanted: Simple (templates and blocks), Builder (pages, data, logic),
//! Pro (API, roles, generated code). This skeleton renders the shell and
//! the level switcher; editor panels plug into the marked slots.
use leptos::prelude::*;

#[component]
fn LevelSwitcher() -> impl IntoView {
    let (level, set_level) = signal(1u8);
    view! {
        <nav class="levels">
            <button class:active=move || level.get() == 1 on:click=move |_| set_level.set(1)>"Simple"</button>
            <button class:active=move || level.get() == 2 on:click=move |_| set_level.set(2)>"Builder"</button>
            <button class:active=move || level.get() == 3 on:click=move |_| set_level.set(3)>"Pro"</button>
        </nav>
        <p class="level-note">{move || match level.get() { 1 => "Pick a template, drop blocks, publish.", 2 => "Pages, tables and logic flows.", _ => "API, roles, plugins and Rust source." }}</p>
    }
}

#[component]
fn BlockPalette() -> impl IntoView {
    let blocks = ["Header", "Hero", "Card", "Button", "Form", "Table", "Chart", "Gallery", "Footer"];
    view! { <aside class="palette"><h2>"Blocks"</h2>{blocks.into_iter().map(|b| view! { <div class="block">{b}</div> }).collect_view()}</aside> }
}

#[component]
fn App() -> impl IntoView {
    view! {
        <main class="shell">
            <header class="topbar"><strong>"Chida"</strong><LevelSwitcher /></header>
            <div class="workbench">
                <BlockPalette />
                <section class="canvas" id="chida-canvas">"Drop blocks here to build a page."</section>
                <aside class="inspector"><h2>"Settings"</h2><p>"Select anything on the canvas to change it here."</p></aside>
            </div>
        </main>
    }
}

fn main() { leptos::mount::mount_to_body(App); }
