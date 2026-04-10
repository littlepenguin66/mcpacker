use crate::models::{LoaderType, ServerContext};
use crate::ui::{print_success, print_warn, style};
use anyhow::{Context, Result, bail};
use futures::StreamExt;
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::Client;
use sha1::Sha1;
use sha2::{Digest, Sha256, Sha512};
use std::path::Path;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

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

async fn download_file_with_progress(
    client: &Client,
    url: &str,
    output_path: &Path,
    label: &str,
    expected_hash: Option<&str>,
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
        let algorithm = select_hash_algorithm(expected_hash, skip_verify)?;
        let computed_hash = compute_hash(output_path, algorithm).await?;
        match expected_hash {
            Some(expected) if skip_verify => {
                print_warn("Installer hash verification skipped by flag.");
                println!(
                    "   Installer hash (computed): {}",
                    style(&computed_hash).cyan()
                );
                println!("   Installer hash (expected): {}", style(expected).cyan());
            }
            Some(expected) => {
                println!("   Verifying installer hash: {}", style(expected).cyan());
                verify_installer_hash(&computed_hash, expected)?;
                print_success("Installer hash verified.");
            }
            None => {
                print_warn("Installer hash not provided; skipping verification.");
                println!(
                    "   Installer SHA-256 (computed): {}",
                    style(&computed_hash).cyan()
                );
            }
        }
    }

    Ok(())
}

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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum HashAlgorithm {
    Sha1,
    Sha256,
    Sha512,
}

async fn compute_hash(path: &Path, algorithm: HashAlgorithm) -> Result<String> {
    let mut file = File::open(path).await?;
    let mut sha1 = Sha1::new();
    let mut sha256 = Sha256::new();
    let mut sha512 = Sha512::new();
    let mut buf = [0u8; 8192];

    loop {
        let n = file.read(&mut buf).await?;
        if n == 0 {
            break;
        }
        match algorithm {
            HashAlgorithm::Sha1 => sha1.update(&buf[..n]),
            HashAlgorithm::Sha256 => sha256.update(&buf[..n]),
            HashAlgorithm::Sha512 => sha512.update(&buf[..n]),
        }
    }

    Ok(match algorithm {
        HashAlgorithm::Sha1 => hex::encode(sha1.finalize()),
        HashAlgorithm::Sha256 => hex::encode(sha256.finalize()),
        HashAlgorithm::Sha512 => hex::encode(sha512.finalize()),
    })
}

fn detect_hash_algorithm(expected_hash: Option<&str>) -> Result<HashAlgorithm> {
    match expected_hash.map(str::len) {
        Some(40) => Ok(HashAlgorithm::Sha1),
        Some(64) | None => Ok(HashAlgorithm::Sha256),
        Some(128) => Ok(HashAlgorithm::Sha512),
        Some(length) => bail!(
            "Unsupported installer hash length {}. Expected SHA-1 (40), SHA-256 (64), or SHA-512 (128) hex characters",
            length
        ),
    }
}

fn select_hash_algorithm(expected_hash: Option<&str>, skip_verify: bool) -> Result<HashAlgorithm> {
    if skip_verify {
        return Ok(match expected_hash.map(str::len) {
            Some(40) => HashAlgorithm::Sha1,
            Some(128) => HashAlgorithm::Sha512,
            _ => HashAlgorithm::Sha256,
        });
    }

    detect_hash_algorithm(expected_hash)
}

fn verify_installer_hash(computed_hex: &str, expected_hex: &str) -> Result<()> {
    if !computed_hex.eq_ignore_ascii_case(expected_hex) {
        bail!(
            "Installer hash mismatch: expected {}, got {}",
            expected_hex,
            computed_hex
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        HashAlgorithm, detect_hash_algorithm, select_hash_algorithm, verify_installer_hash,
    };

    #[test]
    fn detects_supported_installer_hash_lengths() {
        assert_eq!(
            detect_hash_algorithm(Some(&"a".repeat(40))).unwrap(),
            HashAlgorithm::Sha1
        );
        assert_eq!(
            detect_hash_algorithm(Some(&"b".repeat(64))).unwrap(),
            HashAlgorithm::Sha256
        );
        assert_eq!(
            detect_hash_algorithm(Some(&"c".repeat(128))).unwrap(),
            HashAlgorithm::Sha512
        );
    }

    #[test]
    fn rejects_invalid_installer_hash_lengths() {
        assert!(detect_hash_algorithm(Some("abc")).is_err());
    }

    #[test]
    fn compares_installer_hashes_case_insensitively() {
        assert!(verify_installer_hash("deadbeef", "DEADBEEF").is_ok());
        assert!(verify_installer_hash("deadbeef", "DEADBEEE").is_err());
    }

    #[test]
    fn skip_verify_falls_back_without_rejecting_unknown_hash_length() {
        assert_eq!(
            select_hash_algorithm(Some("abc"), true).unwrap(),
            HashAlgorithm::Sha256
        );
    }
}
