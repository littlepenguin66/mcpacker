# Agentic Guidelines for McPacker

This document provides essential information for AI agents working in the `mcpacker` repository.

## Project Overview
`mcpacker` is a Rust-based CLI tool designed to generate Minecraft server packs from Modrinth (`.mrpack`) and CurseForge (`.zip`) modpack files. It handles mod downloads, server loader installation (Fabric, Quilt, Forge, NeoForge), and configuration generation.

## Development Environment

### Commands
- **Build**: `cargo build`
- **Run**: `cargo run -- --input <file> --output <dir>`
- **Test (All)**: `cargo test`
- **Test (Single)**: `cargo test <test_name_substring>`
- **Lint**: `cargo clippy`
- **Format**: `cargo fmt`
- **Check Diagnostics**: `cargo check`

### Core Stack
- **Language**: Rust (Edition 2021)
- **Async Runtime**: `tokio`
- **HTTP Client**: `reqwest`
- **CLI Framework**: `clap`
- **Error Handling**: `anyhow` (application logic) and `thiserror` (custom errors)
- **Serialization**: `serde` / `serde_json`
- **UI/Progress**: `console` and `indicatif`

## Code Style & Conventions

### 1. General Patterns
- **Standard Rust Conventions**: Use `snake_case` for functions/variables and `PascalCase` for structs/enums.
- **Async First**: Most operational logic should be `async` using `tokio`.
- **Modularity**: Logic is split into `models` (data), `ops` (actions), `parsers` (file handling), and `ui` (presentation).

### 2. Error Handling
- Use `anyhow::Result<T>` for high-level application logic and CLI entry points.
- Use `anyhow::Context` (via `.context()` or `.with_context()`) to provide meaningful error messages when operations fail.
- For library-like modules (e.g., parsers), consider defining specific error types with `thiserror` if the caller needs to handle different error cases programmatically.
- **Never** use `unwrap()` or `expect()` in production code unless it's a proven invariant. Prefer `?` or handling the `Option`/`Result`.

### 3. Imports & Modules
- Group imports:
    1. Standard library (`std::*`)
    2. External crates (`anyhow`, `tokio`, etc.)
    3. Crate modules (`crate::models::*`)
- Maintain the existing module structure. New operations should go into `src/ops/`, and new modpack format support into `src/parsers/`.

### 4. UI & Logging
- **Do not** use `println!` directly for operational steps. Use the `ui` module helpers:
    - `ui::print_step("Doing something")` for major phases.
    - `ui::print_success("Finished task")` for completions.
    - `ui::print_info("Key", "Value")` for displaying data.
    - `ui::print_warn("Warning message")` for non-fatal issues.
- Use `indicatif` for long-running processes like downloads.

### 5. Documentation
- Use doc comments `///` for public functions and structs.
- Explain the *intent* of complex async streams or buffer handling (especially in `downloader.rs`).

## Repository Rules
- No existing `.cursorrules` or `.github/copilot-instructions.md` detected.
- Maintain consistent formatting using `cargo fmt`.
- Ensure `cargo clippy` passes before submitting changes.
- If adding a new feature, include corresponding unit tests in a `#[cfg(test)]` module within the file or in the `tests/` directory.

## Testing Strategy
- Tests should be placed in `#[cfg(test)]` blocks at the bottom of the relevant file.
- Use `tokio::test` for async tests.
- Mock external network requests where possible or use small sample files from the `modpacks/` directory for integration tests.
