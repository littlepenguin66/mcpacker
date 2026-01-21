mod models;
mod ops;
mod parsers;
mod ui;
mod utils;

const LOGO: &str = r#"
===================================================================
███╗   ███╗ ██████╗██████╗  █████╗  ██████╗██╗  ██╗███████╗██████╗
████╗ ████║██╔════╝██╔══██╗██╔══██╗██╔════╝██║ ██╔╝██╔════╝██╔══██╗
██╔████╔██║██║     ██████╔╝███████║██║     █████╔╝ █████╗  ██████╔╝
██║╚██╔╝██║██║     ██╔═══╝ ██╔══██║██║     ██╔═██╗ ██╔══╝  ██╔══██╗
██║ ╚═╝ ██║╚██████╗██║     ██║  ██║╚██████╗██║  ██╗███████╗██║  ██║
╚═╝     ╚═╝ ╚═════╝╚═╝     ╚═╝  ╚═╝ ╚═════╝╚═╝  ╚═╝╚══════╝╚═╝  ╚═╝
==================================================================="#;

use ops::{downloader, generator, installer};

use anyhow::{Context, Result};
use clap::Parser;
use std::path::PathBuf;
use ui::{LOOKING_GLASS, SPARKLE, print_header, print_info, print_step, print_success, print_warn};

#[derive(Parser, Debug)]
#[command(author, version, about = LOGO, long_about = None)]
struct Args {
    /// Path to modpack (.mrpack for Modrinth or .zip for CurseForge)
    #[arg(
        index = 1,
        required_unless_present = "update_list",
        value_parser = verify_input_file
    )]
    input: Option<PathBuf>,
    /// Output directory for the generated server pack
    #[arg(short, long)]
    output: Option<PathBuf>,
    /// Server memory setting (e.g. 4G or 4096M)
    #[arg(
        short,
        long,
        default_value = "4G",
        value_parser = verify_memory_format
    )]
    memory: String,
    /// Path to java executable used for the server
    #[arg(long, default_value = "java")]
    java_path: String,
    /// Maximum concurrent mod downloads
    #[arg(short, long, default_value = "10")]
    parallel: usize,
    /// Refresh cached client-only keyword list and exit if no input
    #[arg(long, short = 'u')]
    update_list: bool,
    /// Keep client-only mods when parsing Modrinth packs
    #[arg(long)]
    keep_client: bool,
    /// Filter out client-only mods when parsing CurseForge packs
    #[arg(long)]
    filter_client: bool,
    /// Auto-accept Mojang EULA in generated server files
    #[arg(long)]
    accept_eula: bool,
    /// Skip hash verification for downloaded mods
    #[arg(long)]
    skip_hash: bool,
    /// Skip installer hash verification (not recommended)
    #[arg(long)]
    skip_installer_verify: bool,
    /// Expected installer SHA (supports sha1/sha512)
    #[arg(long)]
    installer_hash: Option<String>,
    /// HTTP proxy URL for downloads and cache refresh
    #[arg(long)]
    proxy: Option<String>,
}

/// Verify input file exists and has valid extension
fn verify_input_file(s: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(s);
    if !path.exists() {
        return Err(format!("File does not exist: {}", s));
    }
    if !path.is_file() {
        return Err(format!("Path is not a file: {}", s));
    }
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    if ext != "zip" && ext != "mrpack" {
        return Err(format!(
            "Unsupported file format: .{} (only .zip or .mrpack are supported)",
            ext
        ));
    }
    Ok(path)
}

/// Verify memory format is valid
fn verify_memory_format(s: &str) -> Result<String, String> {
    let valid_end = s.ends_with('M') || s.ends_with('G') || s.ends_with('m') || s.ends_with('g');
    let valid_start = s.chars().next().is_some_and(|c| c.is_ascii_digit());

    if valid_start && valid_end {
        Ok(s.to_string())
    } else {
        Err(format!(
            "Invalid memory format: '{}'. Please use formats like '4G' or '4096M'",
            s
        ))
    }
}

#[tokio::main]
/// Main entry point
async fn main() -> Result<()> {
    let args = Args::parse();

    ui::print_logo();

    let cache_exists = parsers::filter::is_cache_present();
    if args.update_list || !cache_exists {
        if !cache_exists {
            print_step("No mods list cache found. Performing initial update...");
        } else {
            print_step("Updating client-only mods list");
        }
        parsers::filter::update_fallback_list(args.proxy.as_deref()).await?;
        print_success("Client-only mods list updated and cached.");
        if args.update_list && args.input.is_none() {
            return Ok(());
        }
    }

    let input = args
        .input
        .context("Missing input file. Use --help for usage.")?;

    let output = match args.output {
        Some(path) => path,
        None => {
            let stem = input
                .file_stem()
                .context("Cannot derive output folder name from input file")?;
            PathBuf::from(stem)
        }
    };

    print_header("McPacker - ServerPack Generator");
    print_info("Input", &input.to_string_lossy());
    print_info("Output", &output.to_string_lossy());
    if args.skip_installer_verify {
        print_warn("Installer hash verification disabled; use with caution.");
    } else if let Some(expected) = args.installer_hash.as_deref() {
        print_info("Installer hash (expected)", expected);
    }

    let extension = input
        .extension()
        .and_then(|e| e.to_str())
        .context("Failed to determine file extension")?;

    let (context, mods) = match extension {
        "mrpack" => {
            print_step("Parsing Modrinth Modpack");
            let (ctx, mods) = parsers::modrinth::parse_mrpack(&input, args.keep_client)?;
            (ctx, mods)
        }
        "zip" => {
            print_step("Parsing CurseForge Modpack");
            let (ctx, mods) = parsers::curseforge::parse_curseforge(&input, args.filter_client)?;
            (ctx, mods)
        }
        ext => {
            anyhow::bail!("Unsupported file extension: .{}", ext);
        }
    };

    println!(
        "{} Server: {} | Loader: {:?} {}",
        LOOKING_GLASS,
        ui::style(&context.minecraft_version).bold().green(),
        context.loader_type,
        ui::style(&context.loader_version).bold()
    );
    print_info("Mods found", &mods.len().to_string());

    downloader::download_all(
        mods,
        output.clone(),
        args.parallel,
        args.skip_hash,
        args.proxy.as_deref(),
    )
    .await?;

    print_step("Installing Server Loader");
    let server_jar = installer::install_loader(
        &context,
        &output,
        &args.java_path,
        args.skip_installer_verify,
        args.installer_hash.as_deref(),
        args.proxy.as_deref(),
    )
    .await?;
    print_success(&format!("Loader installed: {}", server_jar));

    print_step("Generating Configuration");
    let script_name = generator::generate_server_files(
        &context,
        &input,
        &output,
        &args.memory,
        &server_jar,
        &args.java_path,
        args.accept_eula,
    )
    .await?;

    println!();
    print_success(&format!("{} Server is ready!", SPARKLE));

    println!(
        "   Run {} to start your server.",
        ui::style(script_name).cyan()
    );

    Ok(())
}
