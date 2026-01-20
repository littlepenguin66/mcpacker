use crate::models::ModInfo;
use anyhow::{Context, Result};
use futures::StreamExt;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use reqwest::Client;
use sha2::{Digest, Sha512};
use std::path::PathBuf;
use tokio::fs::{self, File};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

pub async fn download_all(mods: Vec<ModInfo>, output_dir: PathBuf, parallel: usize) -> Result<()> {
    let mods_dir = output_dir.join("mods");
    fs::create_dir_all(&mods_dir)
        .await
        .context("Failed to create mods directory")?;

    let client = Client::new();
    let multi_pb = MultiProgress::new();

    let total_style = ProgressStyle::with_template(
        "[{elapsed_precise}] {bar:40.cyan/blue} {pos:>7}/{len:7} {msg}",
    )?
    .progress_chars("##-");

    let total_pb = multi_pb.add(ProgressBar::new(mods.len() as u64));
    total_pb.set_style(total_style);
    total_pb.set_message("Downloading mods...");

    // Global bandwidth bar
    let byte_style =
        ProgressStyle::with_template("{spinner:.green} [Total: {bytes}] {bytes_per_sec}")?
            .progress_chars("##-");
    let byte_pb = multi_pb.add(ProgressBar::new_spinner());
    byte_pb.set_style(byte_style);

    let bodies = futures::stream::iter(mods)
        .map(|mod_info| {
            let client = client.clone();
            let mods_dir = mods_dir.clone();
            let pb = multi_pb.add(ProgressBar::new_spinner());
            let byte_pb = byte_pb.clone();
            pb.set_style(ProgressStyle::default_spinner());

            async move {
                let res = download_single_mod(&client, &mod_info, &mods_dir, &pb, &byte_pb).await;
                pb.finish_and_clear();
                res
            }
        })
        .buffer_unordered(parallel);

    bodies
        .for_each(|res| async {
            total_pb.inc(1);
            if let Err(e) = res {
                total_pb.println(format!("Error: {:#}", e));
            }
        })
        .await;

    total_pb.finish_with_message("Done!");
    byte_pb.finish_and_clear();
    Ok(())
}

async fn download_single_mod(
    client: &Client,
    mod_info: &ModInfo,
    mods_dir: &PathBuf,
    pb: &ProgressBar,
    byte_pb: &ProgressBar,
) -> Result<()> {
    pb.set_message(format!("Checking {}", mod_info.name));
    let file_path = mods_dir.join(&mod_info.file_name);

    if file_path.exists() {
        if verify_hash(&file_path, &mod_info.hash, &mod_info.hash_algo).await? {
            pb.println(format!("Skipping {} (already exists)", mod_info.name));
            return Ok(());
        }
    }

    pb.set_message(format!("Downloading {}", mod_info.name));

    let response = client
        .get(&mod_info.download_url)
        .send()
        .await
        .context(format!("Failed to request {}", mod_info.download_url))?;

    if !response.status().is_success() {
        anyhow::bail!(
            "Download failed for {}: {}",
            mod_info.name,
            response.status()
        );
    }

    let mut stream = response.bytes_stream();
    let mut file = File::create(&file_path).await?;

    while let Some(item) = stream.next().await {
        let chunk = item.context("Failed to read chunk")?;
        file.write_all(&chunk).await?;
        byte_pb.inc(chunk.len() as u64);
    }

    if !verify_hash(&file_path, &mod_info.hash, &mod_info.hash_algo).await? {
        anyhow::bail!("Hash mismatch for {}", mod_info.name);
    }

    Ok(())
}

async fn verify_hash(path: &PathBuf, expected_hash: &str, algo: &str) -> Result<bool> {
    if expected_hash.is_empty() {
        return Ok(true);
    }

    let mut file = File::open(path).await?;

    match algo {
        "sha512" => {
            let mut hasher = Sha512::new();
            let mut buf = [0u8; 8192];
            loop {
                let n = file.read(&mut buf).await?;
                if n == 0 {
                    break;
                }
                hasher.update(&buf[..n]);
            }
            let computed = hex::encode(hasher.finalize());
            Ok(computed == expected_hash)
        }
        _ => Ok(true),
    }
}
