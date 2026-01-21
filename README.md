<div align="center">

███╗   ███╗ ██████╗██████╗  █████╗  ██████╗██╗  ██╗███████╗██████╗
████╗ ████║██╔════╝██╔══██╗██╔══██╗██╔════╝██║ ██╔╝██╔════╝██╔══██╗
██╔████╔██║██║     ██████╔╝███████║██║     █████╔╝ █████╗  ██████╔╝
██║╚██╔╝██║██║     ██╔═══╝ ██╔══██║██║     ██╔═██╗ ██╔══╝  ██╔══██╗
██║ ╚═╝ ██║╚██████╗██║     ██║  ██║╚██████╗██║  ██╗███████╗██║  ██║
╚═╝     ╚═╝ ╚═════╝╚═╝     ╚═╝  ╚═╝ ╚═════╝╚═╝  ╚═╝╚══════╝╚═╝  ╚═╝

# McPacker

**A command-line tool to convert client modpacks into ready-to-run Minecraft servers**

[![Rust](https://img.shields.io/badge/Rust-2024-orange?style=flat-square&logo=rust)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/License-AGPL%20v3-blue?style=flat-square)](LICENSE)

Transform client-side Modrinth and CurseForge modpacks into fully configured Minecraft servers with a single command.

[Overview](#overview) • [Features](#features) • [Installation](#installation) • [Usage](#usage) • [Examples](#examples)

</div>

---

## Overview

McPacker is a fast, reliable command-line tool built with Rust that converts client modpacks into server installations automatically. Most modpacks are distributed for client use only, requiring manual setup to run as servers. McPacker handles the complete conversion pipeline: parsing modpack files, filtering client-only mods, downloading server mods with hash verification, installing the appropriate server loader, and generating start scripts with your custom configuration.

Whether you're setting up a server for friends or managing multiple modded instances, McPacker eliminates the manual work and ensures reliable, verifiable installations.

## Features

- **Multiple Format Support**: Parse both Modrinth (`.mrpack`) and CurseForge (`.zip`) modpack formats
- **Smart Mod Filtering**: Automatically excludes client-only mods using a cached keyword database
- **Parallel Downloads**: Fast concurrent downloads with configurable parallelism and progress tracking
- **Hash Verification**: Built-in SHA-1 and SHA-512 verification ensures file integrity
- **Loader Installation**: Automatic detection and installation of Fabric, Forge, Quilt, and NeoForge loaders
- **Proxy Support**: Full HTTP/HTTPS proxy support for downloads and API requests
- **Start Script Generation**: Creates platform-specific startup scripts with custom memory and Java settings
- **Jar Metadata Extraction**: Reads mod information from `fabric.mod.json`, `mods.toml`, and legacy `mcmod.info`
- **Incremental Backoff**: Smart retry logic with multiple download URLs when available
- **Content-Disposition Support**: Automatically renames files based on server headers

## Installation

### From Source

Ensure you have [Rust](https://www.rust-lang.org/tools/install) installed (1.75 or later), then build and install:

```bash
git clone https://github.com/littlepenguin66/mcpacker.git
cd mcpacker
cargo install --path .
```

### Pre-built Binaries

Download the latest release from the [releases page](https://github.com/littlepenguin66/mcpacker/releases) for your platform.

## Usage

### Basic Usage

Convert a modpack to a server installation:

```bash
mcpacker your-modpack.mrpack
```

This will create a folder named after your modpack containing all server files, mods, and a start script.

### Command Line Options

```
mcpacker [OPTIONS] <INPUT>

Arguments:
  <INPUT>  Path to modpack file (.mrpack or .zip)

Options:
  -o, --output <PATH>           Output directory [default: modpack name]
  -m, --memory <SIZE>           Server memory allocation [default: 4G]
  --java-path <PATH>            Java executable path [default: java]
  -p, --parallel <NUM>          Parallel downloads [default: 10]
  -u, --update-list             Update client-only mods cache and exit
  --keep-client                 Keep client-only mods (Modrinth only)
  --filter-client               Filter client-only mods (CurseForge)
  --accept-eula                 Automatically accept Minecraft EULA
  --skip-hash                   Skip mod hash verification
  --skip-installer-verify       Skip loader installer hash verification
  --installer-hash <HASH>       Expected hash for loader installer
  --proxy <URL>                 HTTP/HTTPS proxy URL
  -h, --help                    Print help
  -v, --version                 Print version
```

### Memory Format

Specify memory allocation using standard formats:
- `4G` - 4 gigabytes
- `4096M` - 4096 megabytes
- Case-insensitive: `4g` or `4G` both work

## Examples

### Basic Server Setup

Create a server from a Modrinth modpack with 6GB RAM:

```bash
mcpacker my-modpack.mrpack --memory 6G
```

### Custom Output Directory

Install to a specific location:

```bash
mcpacker my-modpack.mrpack --output /path/to/server
```

### Using a Proxy

Download through a corporate or privacy proxy:

```bash
mcpacker my-modpack.mrpack --proxy http://proxy.example.com:8080
```

### Accept EULA Automatically

Skip the manual EULA acceptance step:

```bash
mcpacker my-modpack.mrpack --accept-eula
```

### CurseForge with Client Filtering

Process a CurseForge modpack and filter out client-only mods:

```bash
mcpacker my-curseforge-pack.zip --filter-client
```

### Update Client-Only Mods Cache

Refresh the cached list of client-only mods (useful for offline usage):

```bash
mcpacker --update-list
```

### Advanced Setup

Combine multiple options for a production server:

```bash
mcpacker my-modpack.mrpack \
  --output /srv/minecraft \
  --memory 8G \
  --java-path /usr/lib/jvm/java-17-openjdk/bin/java \
  --parallel 20 \
  --accept-eula \
  --proxy http://proxy.internal:3128
```

## How It Works

McPacker follows a straightforward pipeline:

1. **Parse**: Reads the modpack file and extracts metadata (Minecraft version, loader type, mod list)
2. **Filter**: Identifies and optionally excludes client-only mods using cached keyword matching
3. **Download**: Fetches all server mods in parallel with progress bars and hash verification
4. **Install**: Downloads and installs the appropriate server loader (Fabric, Forge, etc.)
5. **Generate**: Creates platform-specific start scripts and configuration files

The tool validates each step and provides clear feedback throughout the process.

## Client-Only Mod Filtering

McPacker maintains a cached list of keywords commonly used by client-only mods to avoid downloading unnecessary files. This cache is stored in your system's standard cache directory:

- **Linux**: `~/.cache/mcpacker/`
- **macOS**: `~/Library/Caches/mcpacker/`
- **Windows**: `%LOCALAPPDATA%\mcpacker\cache\`

The cache is automatically created on first run. Use `--update-list` to refresh it manually.

## Hash Verification

By default, McPacker verifies downloaded files using SHA-1 or SHA-512 hashes provided by the modpack. This ensures:

- Files haven't been corrupted during download
- Files match exactly what the modpack author intended
- Protection against man-in-the-middle attacks

> [!WARNING]
> Using `--skip-hash` disables verification and should only be used for troubleshooting. Similarly, `--skip-installer-verify` bypasses loader installer verification.

## Supported Loaders

McPacker automatically detects and installs the following Minecraft server loaders:

- **Fabric** - Lightweight modding toolchain
- **Forge** - Traditional modding platform
- **Quilt** - Modern Fabric fork
- **NeoForge** - Next-generation Forge

The appropriate loader version is extracted from the modpack metadata.

## Troubleshooting

### Download Failures

If downloads fail, McPacker will automatically:
1. Try alternative URLs if provided by the modpack
2. Use incremental backoff before retrying
3. Display clear error messages for manual intervention

You can increase parallelism with `--parallel` or use `--proxy` if network access is restricted.

### Memory Format Errors

Ensure memory values start with a digit and end with `M` or `G`:
- ✅ Valid: `4G`, `4096M`, `8g`
- ❌ Invalid: `G4`, `4`, `4GB`

### Missing Java

If the loader installation fails, verify Java is available:

```bash
java -version  # Should show Java 17+ for modern modpacks
```

Use `--java-path` to specify a custom Java installation.

### Client-Only Mods

If client-only mods are included in your server:
- **Modrinth**: These are automatically filtered unless `--keep-client` is used
- **CurseForge**: Use `--filter-client` to enable filtering

## Performance Tips

- **Parallel Downloads**: Increase `--parallel` up to 20-30 for faster downloads on high-speed connections
- **Proxy Caching**: Set up a caching proxy to speed up repeated installations
- **Local Cache**: The client-only mods list is cached locally; use `--update-list` periodically to refresh

## Platform Support

McPacker generates appropriate start scripts for your platform:

- **Windows**: `start.bat`
- **Linux/macOS**: `start.sh` (with executable permissions)

## Requirements

- **Rust**: 1.75 or later (for building from source)
- **Java**: Required for running the Minecraft server (version depends on Minecraft version)
  - Minecraft 1.17+: Java 17 or later recommended
  - Minecraft 1.16.5 and earlier: Java 8 or 11

## Project Structure

```
mcpacker/
├── src/
│   ├── main.rs           # CLI interface and orchestration
│   ├── ops/              # Core operations (download, install, generate)
│   ├── parsers/          # Modpack format parsers
│   ├── models/           # Data structures and types
│   ├── ui/               # Terminal UI helpers
│   └── utils.rs          # Utility functions
├── Cargo.toml            # Rust dependencies
└── README.md             # This file
```

## FAQ

**Q: Can I use this for client installations?**  
A: McPacker is designed specifically for server installations. Use the native launchers (Modrinth App, CurseForge) for client setups.

**Q: What if a mod fails to download?**  
A: McPacker will report the failure and continue with other mods. Check the output for specific error messages.

**Q: Can I customize the start script after generation?**  
A: Yes! The generated scripts are standard shell/batch files that you can edit manually.

**Q: Does this work with private modpacks?**  
A: McPacker works with any local modpack file. Download the pack file first, then process it with McPacker.

## Acknowledgments

Built with Rust and powered by excellent crates including:
- [clap](https://github.com/clap-rs/clap) - Command line parsing
- [tokio](https://tokio.rs/) - Async runtime
- [reqwest](https://github.com/seanmonstar/reqwest) - HTTP client
- [indicatif](https://github.com/console-rs/indicatif) - Progress bars
- [serde](https://serde.rs/) - Serialization framework

## Inspiration

McPacker draws inspiration from two excellent tools in the Minecraft modding ecosystem:

- **PCL** - For its robust modpack handling and user-friendly approach to managing Minecraft instances
- **ServerPackCreator** - For its comprehensive server setup automation and modpack conversion capabilities

## Support

If you encounter issues or have questions:

1. Check the [troubleshooting section](#troubleshooting)
2. Search existing [GitHub issues](https://github.com/littlepenguin66/mcpacker/issues)
3. Open a new issue with detailed information about your setup and the error
