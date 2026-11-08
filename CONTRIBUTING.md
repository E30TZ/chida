# Contributing to Chida

Thanks for your interest. Chida is a Rust workspace; the CLI and libraries
build with no system dependencies beyond a Rust toolchain.

## Setup

```sh
rustup update stable
cargo build
cargo test
```

All builds in this project are expected to work offline once the crates in
`Cargo.lock` are in your local cargo cache.

## Ground rules

* Keep every crate usable on its own, with tests next to the code it covers.
* A feature is not done until a test proves it and the demo still renders:
  run `cargo test`, then `chida new demo`, `chida render` and
  `chida generate` on the result.
* The README only claims what the code actually does. When you add a
  feature, update the "What works today" list; when you plan one, put it
  on the roadmap instead.
* Generated code must stay plain and owned: no callbacks into Chida, no
  licence traps, and security stubs must label themselves as stubs.
* Write commit messages in the imperative mood and keep commits focused.

## Pull requests

1. Fork and branch from `main`.
2. Add or update tests for the change.
3. Run `cargo test` and `cargo clippy` if you have it.
4. Describe what changed and how you verified it.

## Templates and blocks

New starter templates are very welcome. A template is a `ProjectFile` with
pages, tables, seed data and flows, and it must pass
`ProjectFile::validate` with no issues.
