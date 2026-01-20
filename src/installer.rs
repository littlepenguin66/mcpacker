use crate::models::{LoaderType, ServerContext};
use crate::ui::{print_success, print_warn, style};
use anyhow::{Context, Result};
use futures::StreamExt;
use indicatif::{ProgressBar, ProgressState, ProgressStyle};
use reqwest::Client;
use std::path::PathBuf;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;

pub async fn install_loader(context: &ServerContext, output_dir: &PathBuf) -> Result<String> {
    let client = Client::new();

    match context.loader_type {
        LoaderType::Fabric | LoaderType::Quilt => {
            install_fabric_like(&client, context, output_dir).await
        }
        LoaderType::Forge | LoaderType::NeoForge => {
            install_forge_like(&client, context, output_dir).await
        }
    }
}

async fn download_file_with_progress(
    client: &Client,
    url: &str,
    output_path: &PathBuf,
    label: &str,
) -> Result<()> {
    let response = client
        .get(url)
        .send()
        .await
        .context(format!("Failed to request {}", url))?;

    if !response.status().is_success() {
        anyhow::bail!("Download failed for {}: {}", label, response.status());
    }

    let total_size = response.content_length().unwrap_or(0);

    // Print label first to ensure progress bar is on next line if desired,
    // or just let progress bar handle it.
    // User requested progress bar on next line.
    println!("Downloading: {}", label);

    let pb = ProgressBar::new(total_size);
    pb.set_style(
        ProgressStyle::with_template(
            "{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({eta})",
        )?
        .with_key("eta", |state: &ProgressState, w: &mut dyn std::fmt::Write| {
            write!(w, "{:.1}s", state.eta().as_secs_f64()).unwrap()
        })
        .progress_chars("#>-"),
    );

    // pb.set_message(format!("Downloading {}", label));

    let mut stream = response.bytes_stream();
    let mut file = File::create(output_path).await?;

    while let Some(item) = stream.next().await {
        let chunk = item.context("Failed to read chunk")?;
        file.write_all(&chunk).await?;
        pb.inc(chunk.len() as u64);
    }

    pb.finish_with_message("Done");
    Ok(())
}

async fn install_fabric_like(
    client: &Client,
    context: &ServerContext,
    output_dir: &PathBuf,
) -> Result<String> {
    // print_info("Downloading", "Server Jar");

    let base_url = if context.loader_type == LoaderType::Quilt {
        format!(
            "https://meta.quiltmc.org/v3/versions/loader/{}/{}/server/jar",
            context.minecraft_version, context.loader_version
        )
    } else {
        format!(
            "https://meta.fabricmc.net/v2/versions/loader/{}/{}/1.0.1/server/jar",
            context.minecraft_version, context.loader_version
        )
    };

    let jar_path = output_dir.join("server.jar");
    download_file_with_progress(client, &base_url, &jar_path, "Server Jar").await?;

    Ok("server.jar".to_string())
}

async fn install_forge_like(
    client: &Client,
    context: &ServerContext,
    output_dir: &PathBuf,
) -> Result<String> {
    // print_info("Downloading", "Forge/NeoForge Installer");

    let version_str = if context.loader_type == LoaderType::NeoForge {
        format!("{}", context.loader_version)
    } else {
        format!("{}-{}", context.minecraft_version, context.loader_version)
    };

    let url = if context.loader_type == LoaderType::NeoForge {
        format!(
            "https://maven.neoforged.net/releases/net/neoforged/neoforge/{v}/neoforge-{v}-installer.jar",
            v = version_str
        )
    } else {
        format!(
            "https://maven.minecraftforge.net/net/minecraftforge/forge/{v}/forge-{v}-installer.jar",
            v = version_str
        )
    };

    let installer_name = "installer.jar";
    let installer_path = output_dir.join(installer_name);

    download_file_with_progress(client, &url, &installer_path, "Installer").await?;

    // Create install script
    #[cfg(target_os = "windows")]
    let script_name = "install_forge.bat";
    #[cfg(not(target_os = "windows"))]
    let script_name = "install_forge.sh";

    let script_content = format!("java -jar {} --installServer", installer_name);
    let script_path = output_dir.join(script_name);

    let mut script_file = File::create(&script_path).await?;
    script_file.write_all(script_content.as_bytes()).await?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = script_file.metadata().await?.permissions();
        perms.set_mode(0o755);
        script_file.set_permissions(perms).await?;
    }

    println!("   Created installer script: {}", style(script_name).cyan());
    println!(
        "   Please run {} if auto-install fails.",
        style(script_name).bold()
    );

    println!("   Action: Running installer automatically...");
    let status = tokio::process::Command::new("java")
        .arg("-jar")
        .arg(installer_name)
        .arg("--installServer")
        .current_dir(output_dir)
        .status()
        .await;

    match status {
        Ok(s) if s.success() => {
            print_success("Forge installation successful!");
        }
        _ => {
            print_warn("Automatic installation failed or Java not found.");
            println!("      Please run {} manually.", style(script_name).bold());
        }
    }

    Ok(installer_name.to_string())
}
