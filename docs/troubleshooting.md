# Troubleshooting

## Overview

This guide covers the most common failure modes in the current McPacker flow:

- input validation
- client-only cache refresh
- CurseForge filtering warnings
- download failures
- hash verification failures
- loader installer failures
- startup file issues

## Input Validation

### Unsupported input format

Symptom:

- the CLI exits before parsing begins
- the error mentions an unsupported file extension

What to check:

- McPacker only accepts `.mrpack` and `.zip`
- the path must point to an existing file

What to do:

```bash
mcpacker pack.mrpack
mcpacker pack.zip
```

If your file has another extension, rename is not enough unless the archive content actually matches the expected format.

### Invalid memory format

Symptom:

- the CLI rejects `--memory`

Valid examples:

- `4G`
- `4096M`
- `8g`

Invalid examples:

- `4`
- `4GB`
- `G4`
- `4fooG`

### Invalid parallel value

Symptom:

- the CLI rejects `--parallel`

What to check:

- `--parallel` must be a positive integer
- `0` is rejected

Valid examples:

- `--parallel 1`
- `--parallel 10`

## Cache Refresh Problems

### `--update-list` fails

Symptom:

- the command fails while updating the client-only keyword cache

Possible causes:

- no network access
- bad proxy URL
- GitHub returned a non-success response
- the downloaded content did not contain valid keywords

What to do:

- verify the proxy value if you use `--proxy`
- retry without a proxy if possible
- rerun:

```bash
mcpacker --update-list
mcpacker --update-list --proxy http://proxy.example.com:8080
```

### CurseForge filtering warns that the keyword list is empty

Symptom:

- you see a warning saying the client-only filter list is empty

What it means:

- CurseForge filtering depends on the cached keyword list
- if the cache is missing or empty, filtering can be incomplete

What to do:

```bash
mcpacker --update-list
mcpacker pack.zip --filter-client
```

## CurseForge Filtering Warnings

### Filtering may be incomplete

Symptom:

- you see a warning like:
  `Failed to resolve metadata for X CurseForge files; client-only filtering may be incomplete.`

What it means:

- McPacker could not resolve real file names for some CurseForge entries
- the pack still continues
- unresolved entries are kept instead of aborting the whole run

What to do:

- rerun later in case the CurseForge endpoint was transiently failing
- use a working proxy if direct access is unstable
- review the generated `mods/` directory manually before deploying the server

## Download Failures

### One or more mods fail to download

Symptom:

- the run ends with a message like `N mods failed to download`

What McPacker already does:

- retries alternate URLs when available
- applies incremental backoff between retry attempts
- skips files already present when hashes match

What to do:

- rerun the command
- lower or raise `--parallel` depending on your network stability
- try a proxy if your network blocks some hosts

Examples:

```bash
mcpacker pack.mrpack --parallel 5
mcpacker pack.mrpack --parallel 20
mcpacker pack.mrpack --proxy http://proxy.example.com:8080
```

### A resource pack appears in the pack manifest

Current behavior:

- if the resolved artifact name ends with `.zip`, McPacker treats it as a resource pack and skips it

If a required server-side dependency is packaged unusually, inspect the output and upstream pack metadata.

## Hash Verification Failures

### Mod hash mismatch

Symptom:

- a download finishes but then fails verification

What it means:

- the downloaded bytes did not match the hash from the pack metadata

What to do:

- rerun first to rule out a transient or mirrored file issue
- verify that the upstream modpack is still valid
- only use `--skip-hash` if you are intentionally bypassing mod verification

```bash
mcpacker pack.mrpack --skip-hash
```

### Installer hash mismatch

Symptom:

- Forge or NeoForge installer download completes but verification fails

What to check:

- `--installer-hash` currently supports SHA-1, SHA-256, and SHA-512 hex strings
- the value must match the downloaded installer exactly

What to do:

- confirm the hash source
- retry once
- use `--skip-installer-verify` only if you intentionally want to bypass installer verification

## Loader Installation Failures

### Forge or NeoForge installer fails to run

Symptom:

- McPacker reports that automatic installation failed
- `installer.log` is created in the output directory

What McPacker does:

- downloads `installer.jar`
- writes an install script
- tries to run `java -jar installer.jar --installServer`

What to do:

1. inspect `installer.log`
2. verify Java is installed and matches the target Minecraft version
3. rerun with an explicit Java path if needed
4. run the generated install script manually if you need more control

Examples:

```bash
java -version
mcpacker pack.zip --java-path /path/to/java
```

### Fabric or Quilt server jar download fails

Symptom:

- installation fails before startup files are generated

What to do:

- check network and proxy settings
- retry the command
- confirm the pack declares a valid loader version

## Startup Problems

### Generated server does not start

What to check:

- `eula.txt`
- the generated startup script
- Java version
- Forge or NeoForge `user_jvm_args.txt`

Notes:

- `--accept-eula` writes `eula=true`
- without it, McPacker writes `eula=false`
- Forge and NeoForge may reuse existing `run.sh` or `run.bat` scripts from the installer output

### Wrong Java executable is used

Symptom:

- the generated scripts or installer use the wrong Java binary

What to do:

- set `--java-path`

```bash
mcpacker pack.mrpack --java-path /usr/lib/jvm/java-17-openjdk/bin/java
```

## Proxy Problems

### Proxy URL is rejected immediately

Symptom:

- the command fails with an invalid proxy URL error

What to do:

- use a full URL, for example:

```bash
http://proxy.example.com:8080
http://127.0.0.1:7890
```

### Downloads still fail behind a proxy

What to check:

- whether the proxy allows GitHub Raw, CurseForge, Modrinth, Fabric, Forge, and NeoForge endpoints
- whether HTTPS interception changes downloaded content and causes hash failures

## When To Retry And When To Inspect

Retry first when:

- a metadata lookup fails
- a download endpoint times out
- a mirror returns a transient HTTP error

Inspect output files and logs when:

- the same mod fails repeatedly
- installer verification keeps failing
- Forge or NeoForge installation fails
- the generated server directory starts but crashes immediately

## Useful Commands

```bash
mcpacker --help
mcpacker --update-list
mcpacker pack.mrpack
mcpacker pack.zip --filter-client
mcpacker pack.mrpack --proxy http://proxy.example.com:8080
mcpacker pack.zip --java-path /path/to/java
```
