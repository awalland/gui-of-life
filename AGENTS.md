# Repository Guidelines

## Project Structure & Module Organization
- `shared/` holds the reusable Game of Life logic (`grid` module, randomization, stepping). Keep new simulation code here so the GUI stays lean.
- `gui/` is the eframe front end (`src/main.rs`) that renders the grid and drives updates. Treat it as a thin layer over `shared`.
- `Cargo.toml` at the root defines the two-crate workspace; add new crates here so `cargo` commands cover everything.
- `target/` is generated output; never commit it. Temporary assets or datasets should live under a new `data/` folder and be git-ignored.

## Build, Test, and Development Commands
- `cargo check --workspace` performs a fast compilation pass; run it before committing iterative changes.
- `cargo fmt --all` applies the shared `rustfmt.toml` (150-character width); run after edits or add `cargo fmt --check` to CI.
- `cargo clippy --workspace --all-targets -- -D warnings` enforces idiomatic Rust; treat warnings as build failures.
- `cargo run -p gui` launches the desktop app. Use `RUST_LOG=debug cargo run -p gui` when you add tracing.
- `cargo test --workspace` runs unit tests across crates; prefer `cargo test -p shared` for focused iterations on core logic.

## Coding Style & Naming Conventions
Use Rust 2021 defaults with four-space indentation and snake_case module/function names. Types follow UpperCamelCase, constants SCREAMING_SNAKE_CASE (`GRID_WIDTH`). Let `cargo fmt` settle disagreements; avoid manual line breaks unless readability truly improves. GUI-specific helpers belong in `gui/src/` modules to keep `shared/` platform-agnostic.

## Testing Guidelines
Currently tests live alongside code under `#[cfg(test)]` modules in `shared/src/lib.rs`; expand coverage there when adding rules or RNG tweaks. Follow the pattern `fn updates_cell_state()` for descriptive test names. Favor deterministic seeds when checking RNG behavior (`rand::rng().seed_from_u64(...)`). Until GUI automation exists, document manual smoke checks (randomize, observe evolution for 30s) in pull requests.

## Commit & Pull Request Guidelines
Existing commits use short, imperative subjects (`make cell borders round`). Keep body text optional but add context for logic changes or new commands. Every pull request should: describe the change and motivation, link issues with `Fixes #id` when applicable, call out testing (`cargo test`, manual GUI steps), and include screenshots/gifs if the UI changes. Small, focused PRs are easier to review and keep the history meaningful.
