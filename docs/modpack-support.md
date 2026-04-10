# Modpack Support

## Supported Input Formats

McPacker currently accepts exactly two input types:

- Modrinth packs: `.mrpack`
- CurseForge packs: `.zip`

Any other file extension is rejected at CLI validation time.

## Supported Loader Families

The current code supports these server loader families:

- Fabric
- Quilt
- Forge
- NeoForge

The loader type and version are extracted from the modpack metadata, then passed into the installer and startup-file generator.

## Modrinth Support

### Source of Truth

Modrinth support is based on `modrinth.index.json` inside the `.mrpack` archive.

### What Is Read

McPacker reads:

- Minecraft version
- loader dependency
- file download URLs
- per-file hashes
- side environment flags

### Current Behavior

- Chooses `sha512` when present, otherwise `sha1`.
- Filters client-only mods by default.
- Keeps client-only mods only when `--keep-client` is set.
- Skips entries with no download URLs.

### Side Handling

Side detection comes from `env.client` and `env.server`.
Files marked as unsupported on the server side are treated as client-only and removed unless `--keep-client` is enabled.

## CurseForge Support

### Source of Truth

CurseForge support is based on `manifest.json` inside the `.zip` archive.

### What Is Read

McPacker reads:

- Minecraft version
- loader list from `minecraft.modLoaders`
- file entries with `projectID`, `fileID`, and `required`

### Current Behavior

- Uses the primary loader when one is marked, otherwise the first loader entry.
- Builds download URLs from CurseForge file IDs.
- Does not receive package-provided per-mod hashes from the manifest, so normal CurseForge mod downloads do not have manifest-level hash verification.
- Can apply client-only filtering only when `--filter-client` is enabled.

### Client-Only Filtering

CurseForge filtering is heuristic, not authoritative.
It depends on:

- the cached keyword list in `filter.rs`
- best-effort metadata lookups that resolve real file names

If metadata lookup fails for some files, McPacker keeps processing the pack and emits a warning that filtering may be incomplete.

## Download Behavior Across Formats

For both formats, the downloader:

- writes mods into `mods/`
- retries alternate URLs when available
- sanitizes filenames before writing
- skips resolved `.zip` artifacts that are resource packs

If a file already exists and its hash matches the expected hash, it is reused.
This reuse path mainly applies to Modrinth packs because they provide per-file hashes directly.

## Overrides Support

McPacker extracts files from the `overrides/` directory in the archive and writes them into the output directory.

That means the current implementation expects override content to be stored under `overrides/`.
It does not currently document or normalize alternate override directory names.

## Generated Output

A successful run produces:

- downloaded mods
- loader runtime files
- `eula.txt`
- startup scripts

For Forge and NeoForge, the generated output may also include:

- `installer.jar`
- installer scripts
- `installer.log`
- `user_jvm_args.txt`

## Known Limitations

- Only `.mrpack` and CurseForge `.zip` files are supported.
- CurseForge client-only filtering is best-effort and keyword-based.
- CurseForge manifests do not provide the same per-file hash data that Modrinth packs provide.
- Resource pack skipping depends on the resolved artifact name ending in `.zip`.
- The current extractor only copies files under `overrides/`.

## Recommended Usage

### Modrinth

Use the default flow unless you explicitly want to keep client-side mods:

```bash
mcpacker pack.mrpack
mcpacker pack.mrpack --keep-client
```

### CurseForge

Enable filtering if you want client-only heuristics:

```bash
mcpacker pack.zip --filter-client
```

If filtering matters for your server, watch for warnings about incomplete metadata resolution and review the resulting `mods/` directory before deployment.
