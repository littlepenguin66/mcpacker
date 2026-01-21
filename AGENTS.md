
# PROJECT KNOWLEDGE BASE
**Generated:** 2026-01-21T07:14:25Z  
**Branch:** (not captured)  
**Commit:** (not captured)  

## OVERVIEW
Rust CLI that parses Modrinth (`.mrpack`) and CurseForge (`.zip`) packs, downloads server mods with hash checks, installs loader, and emits start scripts.

## STRUCTURE
```mcpacker/AGENTS.md#L10-22
./
├── src/
│   ├── main.rs         # CLI args, orchestration
│   ├── ops/            # download/install/generate pipeline
│   ├── parsers/        # modpack parsing + client-only cache
│   ├── models/         # shared types, jar metadata
│   ├── ui/             # console styling helpers
│   └── utils.rs        # filename sanitization
├── modpacks/           # sample inputs
├── Cargo.toml          # deps, release profile
└── Cargo.lock
```

## WHERE TO LOOK
| Task | Location | Notes |
|------|----------|-------|
| CLI surface & flow | `src/main.rs` | Clap config; parse → download → install → generate |
| Download pipeline | `src/ops/downloader.rs` | Parallel fetch, content-disposition rename, hash verify, backoff |
| Loader install | `src/ops/installer.rs` | Fetch/verify server jar; respects proxy + hash flags |
| Server files | `src/ops/generator.rs` | Emits start scripts using memory/java args; sets EULA flag |
| Modpack parsing | `src/parsers/{modrinth,curseforge}.rs` | Builds `ServerContext` + `Vec<ModInfo>`; extension-gated |
| Client-only filter cache | `src/parsers/filter.rs` | Fetches keyword list to OS cache (directories crate); proxy aware |
| Jar metadata extraction | `src/models/mod.rs` | Reads `fabric.mod.json` / `mods.toml` / `mcmod.info` |
| UX output helpers | `src/ui/mod.rs` | `print_step/info/success/warn`, logo, emojis |

## CODE MAP (KEY SYMBOLS)
| Symbol | Kind | Location | Role |
|--------|------|----------|------|
| `Args` | struct | `src/main.rs` | CLI flags: input/output/memory/java/proxy/hash/install toggles |
| `main` | async fn | `src/main.rs` | Drives parse → download → install → generate; updates cache when requested |
| `download_all` | async fn | `src/ops/downloader.rs` | Streams downloads with progress bars; optional hash skip |
| `install_loader` | async fn | `src/ops/installer.rs` | Installs loader jar with optional expected hash |
| `generate_server_files` | async fn | `src/ops/generator.rs` | Writes start scripts/config; applies memory/java path; accepts EULA toggle |
| `parse_modrinth` / `parse_curseforge` | fns | `src/parsers/*.rs` | Return `ServerContext` + mod list per format |
| `ModMetadata::extract_from_jar` | fn | `src/models/mod.rs` | Derives name/version/id from jar internals |
| `sanitize_filename` | fn | `src/utils.rs` | Normalizes filenames before writes/renames |

## CONVENTIONS (PROJECT-SPECIFIC)
- Inputs: `.mrpack` → Modrinth; `.zip` → CurseForge; otherwise bail.
- Memory flag must start with digit and end with `M`/`G` (case-insensitive).
- Proxy string passed directly to HTTP client; invalid proxy should fail fast with context.
- Hash verification supports `sha1`/`sha512`; `--skip_hash` disables mod hash checks.
- Resource packs (`.zip` after resolution) are skipped from mods download.
- Progress output via indicatif; keep concise, no noisy logs.
- Cache for client-only keywords lives in OS cache dir via `ProjectDirs`.

## ANTI-PATTERNS (FORBIDDEN)
- Hardcoding cache paths; always use `ProjectDirs`.
- Silently skipping hashes without explicit `--skip_hash`.
- Accepting installer without verifying when `--installer_hash` is provided.
- Duplicating parent guidance in future subdirectory AGENTS (none needed given current size).

## UNIQUE STYLES
- All user-facing text styled through `console` helpers (`print_step/info/success/warn`); avoid raw `println!` for status.
- Downloader uses incremental backoff per alternate URL and multi-progress bars; preserve UX cues.
- Filenames sanitized aggressively before writes/renames to avoid platform issues.

## COMMANDS
```mcpacker/AGENTS.md#L69-72
cargo run --release -- <pack.zip|pack.mrpack> [--output PATH] [--memory 4G] [--parallel 10] [--proxy URL] [--skip_hash] [--skip_installer_verify] [--installer_hash HEX]
cargo run --release -- --update_list [--proxy URL]   # refresh client-only cache; exits if no input
```

## NOTES / GOTCHAS
- If `--update_list` is set and no input provided, process exits after cache refresh.
- When `--skip_installer_verify` is set, emit warning; otherwise verify installer if hash supplied.
- Loader type/version from parsers drives installer/generator; maintain alignment when adding formats.
- No subdirectory AGENTS warranted (20 files, depth 3). Keep this file authoritative and lean.
