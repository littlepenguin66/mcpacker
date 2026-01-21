use crate::models::{LoaderType, ServerContext};
use crate::ui::{print_success, print_warn, style};
use anyhow::{Context, Result, bail};
use futures::StreamExt;
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::Client;
use sha2::{Digest, Sha256};
use std::path::Path;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// Install server loader
pub async fn install_loader(
    context: &ServerContext,
    output_dir: &Path,
    java_path: &str,
    skip_installer_verify: bool,
    installer_hash: Option<&str>,
    proxy: Option<&str>,
) -> Result<String> {
    let mut client_builder = Client::builder();

    if let Some(proxy_url) = proxy {
        let proxy =
            reqwest::Proxy::all(proxy_url).context(format!("Invalid proxy URL: {}", proxy_url))?;
        client_builder = client_builder.proxy(proxy);
    }

    let client = client_builder.build()?;

    match context.loader_type {
        LoaderType::Fabric | LoaderType::Quilt => {
            install_fabric_like(
                &client,
                context,
                output_dir,
                skip_installer_verify,
                installer_hash,
            )
            .await
        }
        LoaderType::Forge | LoaderType::NeoForge => {
            install_forge_like(
                &client,
                context,
                output_dir,
                java_path,
                skip_installer_verify,
                installer_hash,
            )
            .await
        }
    }
}

/// Download file with progress
async fn download_file_with_progress(
    client: &Client,
    url: &str,
    output_path: &Path,
    label: &str,
    expected_sha256: Option<&str>,
    skip_verify: bool,
    is_installer: bool,
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

    let pb = ProgressBar::new(total_size);
    pb.set_style(ProgressStyle::with_template(
        "{spinner:.green} [{elapsed_precise}] {bar:40.cyan/blue} {bytes}/{total_bytes} ({bytes_per_sec}, {eta}) {msg}",
    )?.progress_chars("━╸ "));
    pb.set_message(format!("Downloading: {}", label));

    let mut stream = response.bytes_stream();
    let mut file = File::create(output_path).await?;

    while let Some(item) = stream.next().await {
        let chunk = item.context("Failed to read chunk")?;
        file.write_all(&chunk).await?;
        pb.inc(chunk.len() as u64);
    }

    pb.finish_with_message(format!("{} download complete", label));

    if is_installer {
        let computed_sha256 = compute_sha256(output_path).await?;
        match expected_sha256 {
            Some(expected) if skip_verify => {
                print_warn("Installer hash verification skipped by flag.");
                println!(
                    "   Installer SHA-256 (computed): {}",
                    style(&computed_sha256).cyan()
                );
                println!(
                    "   Installer SHA-256 (expected): {}",
                    style(expected).cyan()
                );
            }
            Some(expected) => {
                println!("   Verifying installer SHA-256: {}", style(expected).cyan());
                verify_sha256(&computed_sha256, expected)?;
                print_success("Installer hash verified.");
            }
            None => {
                print_warn("Installer hash not provided; skipping verification.");
                println!(
                    "   Installer SHA-256 (computed): {}",
                    style(&computed_sha256).cyan()
                );
            }
        }
    }

    Ok(())
}

/// Install Fabric or Quilt loader
async fn install_fabric_like(
    client: &Client,
    context: &ServerContext,
    output_dir: &Path,
    _skip_installer_verify: bool,
    _installer_hash: Option<&str>,
) -> Result<String> {
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
    download_file_with_progress(
        client,
        &base_url,
        &jar_path,
        "Server Jar",
        None,
        false,
        false,
    )
    .await?;

    Ok("server.jar".to_string())
}

/// Install Forge or NeoForge loader
async fn install_forge_like(
    client: &Client,
    context: &ServerContext,
    output_dir: &Path,
    java_path: &str,
    skip_installer_verify: bool,
    installer_hash: Option<&str>,
) -> Result<String> {
    let version_str = if context.loader_type == LoaderType::NeoForge {
        context.loader_version.to_string()
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

    download_file_with_progress(
        client,
        &url,
        &installer_path,
        "Installer",
        installer_hash,
        skip_installer_verify,
        true,
    )
    .await?;

    #[cfg(target_os = "windows")]
    let script_name = "install_forge.bat";
    #[cfg(not(target_os = "windows"))]
    let script_name = "install_forge.sh";

    let script_content = format!("{} -jar {} --installServer", java_path, installer_name);
    let script_path = output_dir.join(script_name);

    let mut script_file = File::create(&script_path).await?;
    script_file.write_all(script_content.as_bytes()).await?;

    let sp = script_path.clone();
    let _ = tokio::task::spawn_blocking(move || {
        let _ = crate::ops::utils::make_executable(&sp);
    })
    .await
    .ok();

    println!("   Created installer script: {}", style(script_name).cyan());
    println!(
        "   Please run {} if auto-install fails.",
        style(script_name).bold()
    );

    println!("   Action: Running installer automatically...");

    let log_path = output_dir.join("installer.log");
    let log_file = std::fs::File::create(&log_path).context("Failed to create installer.log")?;
    let stdout = std::process::Stdio::from(
        log_file
            .try_clone()
            .context("Failed to clone log file handle")?,
    );
    let stderr = std::process::Stdio::from(log_file);

    let status = tokio::process::Command::new(java_path)
        .arg("-jar")
        .arg(installer_name)
        .arg("--installServer")
        .current_dir(output_dir)
        .stdout(stdout)
        .stderr(stderr)
        .status()
        .await;

    match status {
        Ok(s) if s.success() => {
            print_success("Forge installation successful!");
        }
        _ => {
            print_warn("Automatic installation failed or Java not found.");
            println!(
                "      Log: {}",
                style(log_path.display().to_string()).yellow()
            );
            println!("      Please run {} manually.", style(script_name).bold());
            anyhow::bail!("Installer failed to run successfully");
        }
    }

    Ok(installer_name.to_string())
}

/// Compute SHA-256 hash
async fn compute_sha256(path: &Path) -> Result<String> {
    let mut file = File::open(path).await?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 8192];

    loop {
        let n = file.read(&mut buf).await?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }

    Ok(hex::encode(hasher.finalize()))
}

/// Verify SHA-256 hash
fn verify_sha256(computed_hex: &str, expected_hex: &str) -> Result<()> {
    if computed_hex.to_lowercase() != expected_hex.to_lowercase() {
        bail!(
            "Installer hash mismatch: expected {}, got {}",
            expected_hex,
            computed_hex
        );
    }
    Ok(())
}
