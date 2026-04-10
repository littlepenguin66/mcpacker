# Repository Guidelines

## Project Structure & Module Organization
`mcpacker` is a single Rust binary crate. `src/main.rs` owns CLI parsing and the top-level conversion flow. Keep core logic in focused modules: `src/ops/` handles downloading, installer execution, and script generation; `src/parsers/` handles Modrinth, CurseForge, and client-only filtering; `src/models/` defines shared types; `src/ui/` owns terminal output. Release notes live in `docs/releases/`. README changes should usually be mirrored in `README-zh.md`.

## Build, Test, and Development Commands
Use Cargo for all local work:

- `cargo run -- <pack.mrpack>` runs the CLI against a local modpack.
- `cargo build --release` builds the optimized binary.
- `cargo test` runs the inline unit tests.
- `cargo fmt` applies standard Rust formatting.
- `cargo clippy --all-targets --all-features` catches lint issues before review.
- `cargo install --path .` installs the current checkout locally.

## Coding Style & Naming Conventions
This crate uses Rust 2024 and standard `rustfmt` defaults (4-space indentation). Follow existing naming: modules and functions in `snake_case`, types and enums in `PascalCase`, constants in `SCREAMING_SNAKE_CASE`. Prefer small, composable functions and explicit types over clever shortcuts. Avoid comments; if code needs explanation, rename or split it until the intent is obvious.

## Testing Guidelines
Tests are colocated with the code they cover under `#[cfg(test)]`, not in a separate `tests/` tree. Add or update nearby unit tests for every parser, validation, hashing, or installer behavior change. Name tests as behavior statements such as `rejects_zero_parallel_downloads`. There is no numeric coverage gate today, but changed paths should keep `cargo test` green.

## Commit & Pull Request Guidelines
Recent history mixes styles, but contributors should use one-line conventional commits such as `fix(cli): reject zero parallel downloads` or `chore(release): bump version to 26.4.10`. Keep PRs small and focused. Include a short description, linked issue when available, exact verification commands run, and sample terminal output when CLI behavior changes. Update `README.md`, `README-zh.md`, and `docs/releases/` together when flags, output, or versioned behavior changes.
