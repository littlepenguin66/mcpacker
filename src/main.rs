mod models;
mod ops;
mod parsers;
mod ui;

use ops::{downloader, generator, installer};

use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;
use ui::{
    format_key_value, print_header, print_info, print_step, print_success, LOOKING_GLASS, PACKAGE,
    SPARKLE,
};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Input modpack file (.mrpack or .zip)
    #[arg(short, long)]
    input: PathBuf,

    /// Output directory for the server
    #[arg(short, long)]
    output: PathBuf,

    /// Max memory for the server (e.g. 4G, 4096M)
    #[arg(short, long, default_value = "4G")]
    memory: String,

    /// Number of parallel downloads
    #[arg(short, long, default_value = "10")]
    parallel: usize,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    ui::print_logo();
    print_header("McPacker - ServerPack Generator");
    println!(
        "{} {}",
        PACKAGE,
        format_key_value("Input", &args.input.to_string_lossy())
    );
    println!(
        "{} {}",
        PACKAGE,
        format_key_value("Output", &args.output.to_string_lossy())
    );

    if !args.input.exists() {
        anyhow::bail!("Input file does not exist: {:?}", args.input);
    }

    let extension = args
        .input
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");

    let (context, mods) = match extension {
        "mrpack" => {
            print_step("Parsing Modrinth Modpack");
            let (ctx, mods) = parsers::modrinth::parse_mrpack(&args.input)?;
            (ctx, mods)
        }
        "zip" => {
            print_step("Parsing CurseForge Modpack");
            let (ctx, mods) = parsers::curseforge::parse_curseforge(&args.input)?;
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

    downloader::download_all(mods, args.output.clone(), args.parallel).await?;

    print_step("Installing Server Loader");
    let server_jar = installer::install_loader(&context, &args.output).await?;
    print_success(&format!("Loader installed: {}", server_jar));

    print_step("Generating Configuration");
    let script_name = generator::generate_server_files(
        &context,
        &args.input,
        &args.output,
        &args.memory,
        &server_jar,
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
