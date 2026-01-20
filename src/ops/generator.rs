use crate::models::{LoaderType, ServerContext};
use anyhow::{Context, Result};
use std::path::PathBuf;
use tokio::fs::{File, OpenOptions};
use tokio::io::AsyncWriteExt;
use zip::ZipArchive;

use crate::ui::print_step;

pub async fn generate_server_files(
    context: &ServerContext,
    pack_path: &PathBuf,
    output_dir: &PathBuf,
    memory: &str,
    server_jar: &str,
) -> Result<String> {
    print_step("Extracting overrides");
    extract_overrides(pack_path, output_dir).await?;

    print_step("Generating eula.txt");
    let eula_path = output_dir.join("eula.txt");
    let mut eula_file = File::create(eula_path).await?;
    eula_file.write_all(b"eula=false\n").await?;

    print_step("Generating start scripts");
    let script_name = generate_start_scripts(context, output_dir, memory, server_jar).await?;

    Ok(script_name)
}

async fn extract_overrides(pack_path: &PathBuf, output_dir: &PathBuf) -> Result<()> {
    let pack_path = pack_path.clone();
    let output_dir = output_dir.clone();

    tokio::task::spawn_blocking(move || {
        let file = std::fs::File::open(&pack_path)
            .with_context(|| format!("Failed to open pack file: {:?}", pack_path))?;
        let mut archive = ZipArchive::new(file)?;

        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;
            let name = file.name().to_string();

            if name.starts_with("overrides/") && !name.ends_with('/') {
                let relative_path = name.trim_start_matches("overrides/");
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

async fn generate_start_scripts(
    context: &ServerContext,
    output_dir: &PathBuf,
    memory: &str,
    server_jar: &str,
) -> Result<String> {
    let run_sh_path = output_dir.join("run.sh");
    // If run.sh exists (from NeoForge/Forge installer), use it directly instead of creating wrappers
    if run_sh_path.exists() {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Ok(metadata) = tokio::fs::metadata(&run_sh_path).await {
                let mut perms = metadata.permissions();
                perms.set_mode(0o755);
                let _ = tokio::fs::set_permissions(&run_sh_path, perms).await;
            }
        }

        // Apply memory settings to user_jvm_args.txt if needed
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

    // Standard start script generation for Fabric/Quilt or older Forge
    let bat_content = match context.loader_type {
        LoaderType::Forge | LoaderType::NeoForge => {
            format!(
                "@echo off\ncd /d \"%~dp0\"\nREM Forge/NeoForge uses user_jvm_args.txt for JVM arguments.\nREM Run the installer-generated script to start.\ncall run.bat\npause\n"
            )
        }
        _ => {
            format!(
                "@echo off\ncd /d \"%~dp0\"\njava -Xmx{} -jar {} nogui\npause\n",
                memory, server_jar
            )
        }
    };

    let bat_path = output_dir.join("start.bat");
    let mut bat_file = File::create(&bat_path).await?;
    bat_file.write_all(bat_content.as_bytes()).await?;

    let sh_content = match context.loader_type {
        LoaderType::Forge | LoaderType::NeoForge => {
            format!(
                "#!/bin/sh\ncd \"$(dirname \"$0\")\"\n# Forge/NeoForge uses user_jvm_args.txt for JVM arguments.\n# Run the installer-generated script to start.\n./run.sh\n"
            )
        }
        _ => {
            format!(
                "#!/bin/sh\ncd \"$(dirname \"$0\")\"\njava -Xmx{} -jar {} nogui\n",
                memory, server_jar
            )
        }
    };

    let sh_path = output_dir.join("start.sh");
    let mut sh_file = File::create(&sh_path).await?;
    sh_file.write_all(sh_content.as_bytes()).await?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = sh_file.metadata().await?.permissions();
        perms.set_mode(0o755);
        sh_file.set_permissions(perms).await?;
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
        Ok("start.bat".to_string())
    } else {
        Ok("start.sh".to_string())
    }
}
