use crate::models::{LoaderType, ServerContext};
use anyhow::{Context, Result};
use std::path::Path;
use tokio::fs::{File, OpenOptions};
use tokio::io::AsyncWriteExt;
use zip::ZipArchive;

use crate::ui::print_step;

/// Generate server files
pub async fn generate_server_files(
    context: &ServerContext,
    pack_path: &Path,
    output_dir: &Path,
    memory: &str,
    server_jar: &str,
    java_path: &str,
    accept_eula: bool,
) -> Result<String> {
    print_step("Extracting overrides");
    extract_overrides(pack_path, output_dir).await?;

    print_step("Generating eula.txt");
    let eula_path = output_dir.join("eula.txt");
    let mut eula_file = File::create(eula_path).await?;
    if accept_eula {
        eula_file.write_all(b"eula=true\n").await?;
    } else {
        eula_file.write_all(b"eula=false\n").await?;
    }

    print_step("Generating start scripts");
    let script_name =
        generate_start_scripts(context, output_dir, memory, server_jar, java_path).await?;

    Ok(script_name)
}

/// Extract overrides from pack
async fn extract_overrides(pack_path: &Path, output_dir: &Path) -> Result<()> {
    let pack_path = pack_path.to_path_buf();
    let output_dir = output_dir.to_path_buf();

    tokio::task::spawn_blocking(move || {
        let file = std::fs::File::open(&pack_path)
            .with_context(|| format!("Failed to open pack file: {:?}", pack_path))?;
        let mut archive = ZipArchive::new(file)?;

        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;
            let name = file.name().to_string();

            if name.starts_with("overrides/") && !name.ends_with('/') {
                let relative_path = name.trim_start_matches("overrides/");

                let path = std::path::Path::new(relative_path);
                if path
                    .components()
                    .any(|c| !matches!(c, std::path::Component::Normal(_)))
                {
                    anyhow::bail!("Malicious path detected in modpack: {}", name);
                }

                let dest_path = output_dir.join(relative_path);

                if let Some(parent) = dest_path.parent() {
                    std::fs::create_dir_all(parent)?;
                }

                let mut outfile = std::fs::File::create(&dest_path)?;
                std::io::copy(&mut file, &mut outfile)?;
            }
        }
        Ok::<(), anyhow::Error>(())
    })
    .await??;

    Ok(())
}

/// Generate start scripts
async fn generate_start_scripts(
    context: &ServerContext,
    output_dir: &Path,
    memory: &str,
    server_jar: &str,
    java_path: &str,
) -> Result<String> {
    let run_sh_path = output_dir.join("run.sh");
    if run_sh_path.exists() {
        #[cfg(unix)]
        {
            let rp = run_sh_path.clone();
            let _ = tokio::task::spawn_blocking(move || {
                let _ = crate::ops::utils::make_executable(&rp);
            })
            .await
            .ok();
        }

        match context.loader_type {
            LoaderType::Forge | LoaderType::NeoForge => {
                let args_path = output_dir.join("user_jvm_args.txt");
                let mut args_content = String::new();
                if args_path.exists() {
                    args_content = tokio::fs::read_to_string(&args_path)
                        .await
                        .unwrap_or_default();
                }
                if !args_content.contains("-Xmx") {
                    let memory_arg = format!("\n# McPacker Memory Setting\n-Xmx{}\n", memory);
                    let mut args_file = OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open(&args_path)
                        .await?;
                    args_file.write_all(memory_arg.as_bytes()).await?;
                }
            }
            _ => {}
        }

        if cfg!(windows) {
            return Ok("run.bat".to_string());
        } else {
            return Ok("run.sh".to_string());
        }
    }

    let bat_content = match context.loader_type {
        LoaderType::Forge | LoaderType::NeoForge => {
            "@echo off\ncd /d \"%~dp0\"\nREM Forge/NeoForge uses user_jvm_args.txt for JVM arguments.\nREM Run the installer-generated script to start.\ncall run.bat\npause\n".to_string()
        }
        _ => {
            format!(
                "@echo off\ncd /d \"%~dp0\"\n{} -Xmx{} -jar {} nogui\npause\n",
                java_path, memory, server_jar
            )
        }
    };

    let bat_path = output_dir.join("start.bat");
    let mut bat_file = File::create(&bat_path).await?;
    bat_file.write_all(bat_content.as_bytes()).await?;

    let sh_content = match context.loader_type {
        LoaderType::Forge | LoaderType::NeoForge => {
            "#!/bin/sh\ncd \"$(dirname \"$0\")\"\n# Forge/NeoForge uses user_jvm_args.txt for JVM arguments.\n# Run the installer-generated script to start.\n./run.sh\n".to_string()
        }
        _ => {
            format!(
                "#!/bin/sh\ncd \"$(dirname \"$0\")\"\n{} -Xmx{} -jar {} nogui\n",
                java_path, memory, server_jar
            )
        }
    };

    let sh_path = output_dir.join("start.sh");
    let mut sh_file = File::create(&sh_path).await?;
    sh_file.write_all(sh_content.as_bytes()).await?;

    let sp = sh_path.clone();
    let _ = tokio::task::spawn_blocking(move || {
        let _ = crate::ops::utils::make_executable(&sp);
    })
    .await
    .ok();

    match context.loader_type {
        LoaderType::Forge | LoaderType::NeoForge => {
            let args_path = output_dir.join("user_jvm_args.txt");
            let mut args_content = String::new();

            if args_path.exists() {
                args_content = tokio::fs::read_to_string(&args_path)
                    .await
                    .unwrap_or_default();
            }

            if !args_content.contains("-Xmx") {
                let memory_arg = format!("\n# McPacker Memory Setting\n-Xmx{}\n", memory);
                let mut args_file = OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&args_path)
                    .await?;
                args_file.write_all(memory_arg.as_bytes()).await?;
            }
        }
        _ => {}
    }

    if cfg!(windows) {
        Ok("start.bat".to_string())
    } else {
        Ok("start.sh".to_string())
    }
}
