# Architecture

## Overview

McPacker is a single-binary Rust CLI that turns a client modpack into a runnable server directory.
The main flow is:

1. Parse CLI arguments.
2. Refresh the client-only keyword cache when the selected flow needs it.
3. Parse the input pack into a `ServerContext` and a list of `ModInfo` records.
4. Download server-side mods into `mods/`.
5. Install the selected server loader.
6. Extract pack overrides and generate startup files.

The binary entry point lives in `src/main.rs` and orchestrates the whole pipeline.

## Core Data Types

### `ServerContext`

`ServerContext` carries the server-level information needed after parsing:

- Minecraft version
- loader type
- loader version

It is produced by the parser layer and consumed by the installer and generator layers.

### `ModInfo`

`ModInfo` represents one downloadable item:

- display name
- target file name
- candidate download URLs
- hash value and hash algorithm
- side classification
- required flag

Parsers build `ModInfo` values, and the downloader consumes them.

## Module Layout

### `src/main.rs`

Defines the CLI surface with `clap`, validates input, chooses the parser by file extension, and runs the pipeline.

### `src/parsers/`

Responsible for understanding modpack formats and client-only filtering.

- `modrinth.rs`
  Parses `.mrpack` archives by reading `modrinth.index.json`.
- `curseforge.rs`
  Parses CurseForge `.zip` packs by reading `manifest.json`.
- `filter.rs`
  Manages the cached keyword list used for client-only matching in the CurseForge flow.

The parser layer returns normalized data so downstream code does not need to care which pack format was used.

### `src/ops/`

Responsible for the execution pipeline after parsing.

- `downloader.rs`
  Downloads mods, applies retries, verifies hashes when available, and skips resource packs.
- `installer.rs`
  Installs Fabric, Quilt, Forge, or NeoForge server runtime artifacts.
- `generator.rs`
  Extracts `overrides/`, writes `eula.txt`, and creates startup scripts.
- `utils.rs`
  Holds small operational helpers such as making scripts executable on Unix.

### `src/models/`

Shared enums and structs, plus jar metadata extraction helpers used when a downloaded file needs a better final name.

### `src/ui/`

Small console formatting helpers for consistent terminal output.

### `src/utils.rs`

Filename sanitization used before writing downloaded files to disk.

## Execution Flow

## 1. CLI Validation

The CLI only accepts `.mrpack` and `.zip` input files.
Memory strings must be numeric values ending in `M` or `G`.
Parallel download count must be at least `1`.

## 2. Cache Handling

The client-only keyword cache is refreshed only when needed:

- `--update-list`
- CurseForge parsing with `--filter-client`

This avoids forcing network access for flows that do not use the cache.

## 3. Parsing

The parser layer produces a unified pair:

- `ServerContext`
- `Vec<ModInfo>`

Modrinth parsing is mostly manifest-driven.
CurseForge parsing is manifest-driven, but may also perform best-effort metadata lookups to resolve real file names for better filtering and naming.

## 4. Downloading

The downloader creates `mods/`, builds a shared `reqwest::Client`, and processes downloads concurrently.
For each mod it:

- resolves the best target filename it can find
- skips files already present when hashes match
- streams the response to disk
- verifies SHA-1 or SHA-512 when the pack provides a hash
- skips `.zip` artifacts that resolve to resource packs

If any downloads fail, the command exits with an aggregate failure count.

## 5. Loader Installation

Loader installation splits into two branches:

- Fabric and Quilt:
  download a server jar directly.
- Forge and NeoForge:
  download an installer jar, optionally verify its hash, write an install script, and run the installer automatically.

Installer hash verification supports SHA-1, SHA-256, and SHA-512.

## 6. Output Generation

The generator:

- extracts files under `overrides/`
- writes `eula.txt`
- writes `start.bat` and `start.sh`, or reuses existing Forge-style `run.*` scripts when present

For Forge and NeoForge, memory settings are appended to `user_jvm_args.txt` when needed.

## Design Notes

The codebase uses a normalized pipeline:

- format-specific work stays in `parsers/`
- network and file execution stay in `ops/`
- shared value types stay in `models/`

That keeps the main flow simple and makes adding new pack formats or loader behaviors easier without rewriting the whole command.
