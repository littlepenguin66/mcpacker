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
    #[arg(
        index = 1,
        required_unless_present = "update_list",
        value_parser = verify_input_file
    )]
    input: Option<PathBuf>,
    #[arg(short, long)]
    output: Option<PathBuf>,
    #[arg(
        short,
        long,
        default_value = "4G",
        value_parser = verify_memory_format
    )]
    memory: String,
    #[arg(long, default_value = "java")]
    java_path: String,
    #[arg(short, long, default_value = "10", value_parser = verify_parallel_count)]
    parallel: usize,
    #[arg(long, short = 'u')]
    update_list: bool,
    #[arg(long)]
    keep_client: bool,
    #[arg(long)]
    filter_client: bool,
    #[arg(long)]
    accept_eula: bool,
    #[arg(long)]
    skip_hash: bool,
    #[arg(long)]
    skip_installer_verify: bool,
    #[arg(long)]
    installer_hash: Option<String>,
    #[arg(long)]
    proxy: Option<String>,
}

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

fn verify_memory_format(s: &str) -> Result<String, String> {
    let Some((index, _)) = s.char_indices().last() else {
        return Err(format!(
            "Invalid memory format: '{}'. Please use formats like '4G' or '4096M'",
            s
        ));
    };
    let (number, unit) = s.split_at(index);

    let valid_unit = matches!(unit, "M" | "G" | "m" | "g");
    let valid_number = !number.is_empty() && number.chars().all(|c| c.is_ascii_digit());

    if valid_number && valid_unit {
        Ok(s.to_string())
    } else {
        Err(format!(
            "Invalid memory format: '{}'. Please use formats like '4G' or '4096M'",
            s
        ))
    }
}

fn verify_parallel_count(s: &str) -> Result<usize, String> {
    let parallel = s.parse::<usize>().map_err(|_| {
        format!(
            "Invalid parallel value: '{}'. Please use a positive integer",
            s
        )
    })?;

    if parallel == 0 {
        Err("Parallel downloads must be at least 1".to_string())
    } else {
        Ok(parallel)
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    ui::print_logo();

    let input_extension = args
        .input
        .as_ref()
        .and_then(|path| path.extension())
        .and_then(|ext| ext.to_str());
    let needs_filter_cache =
        args.update_list || (args.filter_client && input_extension == Some("zip"));
    let cache_exists = parsers::filter::is_cache_present();
    if needs_filter_cache && (args.update_list || !cache_exists) {
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
            let (ctx, mods) = parsers::curseforge::parse_curseforge(
                &input,
                args.filter_client,
                args.proxy.as_deref(),
            )
            .await?;
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

#[cfg(test)]
mod tests {
    use super::{Args, verify_memory_format, verify_parallel_count};
    use clap::Parser;

    #[test]
    fn accepts_numeric_memory_values_with_supported_units() {
        assert_eq!(verify_memory_format("4G"), Ok("4G".to_string()));
        assert_eq!(verify_memory_format("4096m"), Ok("4096m".to_string()));
    }

    #[test]
    fn rejects_memory_values_with_non_numeric_prefixes() {
        assert!(verify_memory_format("4fooG").is_err());
        assert!(verify_memory_format("abcG").is_err());
        assert!(verify_memory_format("4").is_err());
    }

    #[test]
    fn rejects_zero_parallel_downloads() {
        assert!(Args::try_parse_from(["mcpacker", "--update-list", "--parallel", "0"]).is_err());
    }

    #[test]
    fn accepts_positive_parallel_downloads() {
        assert_eq!(verify_parallel_count("1"), Ok(1));
        assert_eq!(verify_parallel_count("12"), Ok(12));
    }
}
