use crate::models::{ModInfo, ModMetadata};
use crate::utils::sanitize_filename;
use anyhow::{Context, Result};
use futures::StreamExt;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use reqwest::{
    Client, Url,
    cookie::Jar,
    header::{HeaderMap, HeaderValue},
};
use serde::Deserialize;
use sha1::Sha1;
use sha2::{Digest, Sha512};
use std::path::{Path, PathBuf};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use std::time::Duration;
use tokio::fs::{self, File};
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufWriter};
use tokio::time::sleep;

#[derive(Debug, Deserialize)]
struct CfFileResponse {
    data: CfFileData,
}

#[derive(Debug, Deserialize)]
struct CfFileData {
    #[serde(rename = "fileName")]
    file_name: String,
}

const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum HashAlgorithm {
    Sha1,
    Sha512,
}

pub async fn download_all(
    mods: Vec<ModInfo>,
    output_dir: PathBuf,
    parallel: usize,
    skip_hash: bool,
    proxy: Option<&str>,
) -> Result<()> {
    let mods_dir = output_dir.join("mods");
    fs::create_dir_all(&mods_dir)
        .await
        .context("Failed to create mods directory")?;

    let jar = Arc::new(Jar::default());

    let mut headers = HeaderMap::new();
    headers.insert(
        "Referer",
        HeaderValue::from_static("https://www.curseforge.com/"),
    );
    headers.insert(
        "Origin",
        HeaderValue::from_static("https://www.curseforge.com"),
    );

    let mut client_builder = Client::builder()
        .redirect(reqwest::redirect::Policy::limited(10))
        .cookie_provider(jar.clone())
        .user_agent(USER_AGENT)
        .tcp_nodelay(true)
        .pool_max_idle_per_host(50)
        .pool_idle_timeout(Duration::from_secs(15))
        .connect_timeout(Duration::from_secs(10))
        .default_headers(headers);

    if let Some(proxy_url) = proxy {
        let proxy =
            reqwest::Proxy::all(proxy_url).context(format!("Invalid proxy URL: {}", proxy_url))?;
        client_builder = client_builder.proxy(proxy);
    }

    let client = client_builder.build()?;
    let _ = client.get("https://www.curseforge.com").send().await;

    let multi_pb = MultiProgress::new();
    let total_pb = multi_pb.add(ProgressBar::new(mods.len() as u64));
    total_pb.set_style(
        ProgressStyle::with_template(
            "{spinner:.green} [{elapsed_precise}] {bar:40.cyan/blue} {pos:>3}/{len:3} {msg}",
        )?
        .progress_chars("━╸ "),
    );
    total_pb.set_message("Preparing to start...");

    let byte_pb = multi_pb.add(ProgressBar::new_spinner());
    byte_pb.set_style(ProgressStyle::with_template(
        "    {bytes_per_sec} [Downloaded: {total_bytes}]",
    )?);

    let bodies = futures::stream::iter(mods)
        .map(|mod_info| {
            let client = client.clone();
            let mods_dir = mods_dir.clone();
            let total_pb = total_pb.clone();
            let byte_pb = byte_pb.clone();
            async move {
                download_single_mod(
                    &client, &mod_info, &mods_dir, &total_pb, &byte_pb, skip_hash,
                )
                .await
            }
        })
        .buffer_unordered(parallel);

    let error_count = AtomicUsize::new(0);
    bodies
        .for_each(|res| async {
            total_pb.inc(1);
            if let Err(e) = res {
                error_count.fetch_add(1, Ordering::SeqCst);
                total_pb.suspend(|| {
                    eprintln!("Download error: {:#}", e);
                });
            }
        })
        .await;

    let failures = error_count.load(Ordering::SeqCst);
    if failures > 0 {
        total_pb.finish_with_message("Some mods failed to download");
        byte_pb.finish_and_clear();
        anyhow::bail!("{} mods failed to download", failures);
    }

    total_pb.finish_with_message("All mods downloaded!");
    byte_pb.finish_and_clear();
    Ok(())
}

async fn download_single_mod(
    client: &Client,
    mod_info: &ModInfo,
    mods_dir: &Path,
    main_pb: &ProgressBar,
    byte_pb: &ProgressBar,
    skip_hash: bool,
) -> Result<()> {
    main_pb.set_message(format!("Downloading: {}", mod_info.name));

    let mut target_filename = if mod_info.file_name.is_empty() {
        format!("{}.jar", mod_info.name)
    } else {
        sanitize_filename(&mod_info.file_name)
    };
    let mut resolved_real_name = false;
    let mut download_urls = mod_info.download_urls.clone();

    if let Some(first_url) = mod_info.download_urls.first()
        && first_url.contains("curseforge.com/api")
    {
        let meta_url = first_url.trim_end_matches("/download");
        if let Ok(resp) = client
            .get(meta_url)
            .header("Accept", "application/json")
            .send()
            .await
        {
            if resp.status().is_success() {
                if let Ok(json) = resp.json::<CfFileResponse>().await {
                    target_filename = sanitize_filename(&json.data.file_name);
                    resolved_real_name = true;
                }
            } else {
                let project_id = get_project_id_from_url(first_url);
                let file_id = get_file_id_from_url(first_url);
                if project_id != "0" && file_id != "0" {
                    let maven_url = format!(
                        "https://www.cursemaven.com/curse/maven/O-{}/{}/dummy.jar",
                        project_id, file_id
                    );
                    download_urls.insert(0, maven_url);
                }
            }
        }
    }

    if resolved_real_name && target_filename.ends_with(".zip") {
        main_pb.set_message(format!("Skipping resource pack: {}", target_filename));
        return Ok(());
    }

    let temp_file_path = mods_dir.join(format!("{}.part", target_filename));
    let expected_hash = if skip_hash || mod_info.hash.is_empty() {
        None
    } else {
        parse_hash_algorithm(&mod_info.hash_algo)?
            .map(|algorithm| (algorithm, mod_info.hash.as_str()))
    };

    let current_file_path = mods_dir.join(&target_filename);
    if current_file_path.exists()
        && (skip_hash
            || verify_hash(&current_file_path, expected_hash)
                .await
                .unwrap_or(false))
    {
        main_pb.set_message(format!("Already exists: {}", target_filename));
        return Ok(());
    }

    main_pb.set_message(format!("Downloading: {}", target_filename));

    let mut last_error = None;
    for (url_index, url) in download_urls.iter().enumerate() {
        if url_index > 0 {
            let wait = Duration::from_millis(500 * url_index as u64);
            sleep(wait).await;
            main_pb.set_message(format!(
                "Retrying: {} (node {})",
                target_filename,
                url_index + 1
            ));
        }

        match try_download_from_url(
            client,
            url,
            &mod_info.name,
            &temp_file_path,
            main_pb,
            byte_pb,
            expected_hash,
        )
        .await
        {
            Ok((final_url, hash_ok)) => {
                if !resolved_real_name
                    && let Some(real_name) = extract_filename_from_url(&final_url)
                {
                    target_filename = sanitize_filename(&real_name);
                    resolved_real_name = true;
                }

                if target_filename.ends_with(".zip") {
                    main_pb.set_message(format!("Skipping resource pack: {}", target_filename));
                    if temp_file_path.exists() {
                        let _ = fs::remove_file(&temp_file_path).await;
                    }
                    return Ok(());
                }

                main_pb.set_message(format!("Verifying: {}", target_filename));

                if skip_hash || hash_ok {
                    let final_path = mods_dir.join(&target_filename);
                    if final_path.exists() {
                        let _ = fs::remove_file(&final_path).await;
                    }
                    fs::rename(&temp_file_path, &final_path).await?;

                    if !resolved_real_name && target_filename.starts_with("CF-") {
                        let _ =
                            rename_with_metadata(&mod_info.name, &final_path, &final_path).await;
                    }
                    main_pb.set_message(format!("Completed: {}", target_filename));
                    return Ok(());
                } else {
                    last_error = Some(anyhow::anyhow!("Hash mismatch"));
                }
            }
            Err(e) => last_error = Some(e),
        }
    }

    if temp_file_path.exists() {
        let _ = fs::remove_file(&temp_file_path).await;
    }

    main_pb.set_message(format!("Failed: {}", mod_info.name));
    Err(last_error.unwrap_or_else(|| anyhow::anyhow!("All nodes failed to download")))
}

async fn try_download_from_url(
    client: &Client,
    url: &str,
    mod_name: &str,
    file_path: &Path,
    main_pb: &ProgressBar,
    byte_pb: &ProgressBar,
    expected_hash: Option<(HashAlgorithm, &str)>,
) -> Result<(String, bool)> {
    let response = client.get(url).header("Accept", "*/*").send().await?;

    if !response.status().is_success() {
        anyhow::bail!(
            "Download failed [{}]: HTTP {} ({})",
            mod_name,
            response.status(),
            url
        );
    }

    if let Some(disposition) = response.headers().get(reqwest::header::CONTENT_DISPOSITION)
        && let Ok(disp_str) = disposition.to_str()
        && let Some(real_name) = parse_content_disposition(disp_str)
    {
        main_pb.set_message(format!("Downloading: {}", real_name));
    }

    let content_length = response.content_length();
    let final_url = response.url().to_string();
    let file = File::create(file_path).await?;
    if let Some(len) = content_length {
        let _ = file.set_len(len).await;
    }
    let mut writer = BufWriter::new(file);
    let mut stream = response.bytes_stream();

    let mut sha1_hasher = Sha1::new();
    let mut sha512_hasher = Sha512::new();
    let do_hash = expected_hash.is_some();

    while let Some(item) = stream.next().await {
        let chunk = item?;
        writer.write_all(&chunk).await?;
        if let Some((algorithm, _)) = expected_hash {
            match algorithm {
                HashAlgorithm::Sha1 => sha1_hasher.update(&chunk),
                HashAlgorithm::Sha512 => sha512_hasher.update(&chunk),
            }
        }
        byte_pb.inc(chunk.len() as u64);
    }
    writer.flush().await?;

    let hash_ok = if do_hash {
        let (algorithm, expected) = expected_hash.expect("hash is present when hashing is enabled");
        let computed = match algorithm {
            HashAlgorithm::Sha1 => hex::encode(sha1_hasher.finalize()),
            HashAlgorithm::Sha512 => hex::encode(sha512_hasher.finalize()),
        };
        hashes_match(&computed, expected)
    } else {
        true
    };

    Ok((final_url, hash_ok))
}

fn parse_content_disposition(header: &str) -> Option<String> {
    header
        .split(';')
        .find(|p| p.trim().starts_with("filename="))
        .map(|p| {
            p.trim()
                .trim_start_matches("filename=")
                .trim_matches('"')
                .to_string()
        })
}

fn extract_filename_from_url(url_str: &str) -> Option<String> {
    let url = Url::parse(url_str).ok()?;
    let last_seg = url.path_segments()?.next_back()?;
    let decoded = urlencoding::decode(last_seg).ok()?.to_string();
    if decoded.ends_with(".jar") || decoded.ends_with(".zip") {
        Some(decoded)
    } else {
        None
    }
}

fn get_project_id_from_url(url: &str) -> String {
    let parts: Vec<&str> = url.split('/').collect();
    parts
        .iter()
        .position(|&x| x == "mods")
        .and_then(|pos| parts.get(pos + 1))
        .map(|s| s.to_string())
        .unwrap_or_else(|| "0".to_string())
}

fn get_file_id_from_url(url: &str) -> String {
    let parts: Vec<&str> = url.split('/').collect();
    parts
        .iter()
        .position(|&x| x == "files")
        .and_then(|pos| parts.get(pos + 1))
        .map(|s| s.to_string())
        .unwrap_or_else(|| "0".to_string())
}

async fn verify_hash(path: &Path, expected_hash: Option<(HashAlgorithm, &str)>) -> Result<bool> {
    let Some((algorithm, expected_hash)) = expected_hash else {
        return Ok(true);
    };

    let mut file = File::open(path).await?;
    let mut hasher_sha512 = Sha512::new();
    let mut hasher_sha1 = sha1::Sha1::new();
    let mut buf = [0u8; 8192];

    loop {
        let n = file.read(&mut buf).await?;
        if n == 0 {
            break;
        }
        match algorithm {
            HashAlgorithm::Sha512 => hasher_sha512.update(&buf[..n]),
            HashAlgorithm::Sha1 => hasher_sha1.update(&buf[..n]),
        }
    }

    let computed = match algorithm {
        HashAlgorithm::Sha512 => hex::encode(hasher_sha512.finalize()),
        HashAlgorithm::Sha1 => hex::encode(hasher_sha1.finalize()),
    };
    Ok(hashes_match(&computed, expected_hash))
}

async fn rename_with_metadata(mod_name: &str, temp_path: &Path, final_path: &Path) -> Result<()> {
    if let Ok(meta) = ModMetadata::extract_from_jar(temp_path) {
        let final_name = format!(
            "{}-{}.jar",
            meta.get_display_name(mod_name),
            meta.get_version()
        );
        let target = final_path.with_file_name(sanitize_filename(&final_name));
        if target.exists() {
            let _ = fs::remove_file(&target).await;
        }
        fs::rename(temp_path, &target).await?;
    }
    Ok(())
}

fn hashes_match(computed: &str, expected: &str) -> bool {
    computed.eq_ignore_ascii_case(expected)
}

fn parse_hash_algorithm(algo: &str) -> Result<Option<HashAlgorithm>> {
    match algo.to_ascii_lowercase().as_str() {
        "" | "none" => Ok(None),
        "sha1" => Ok(Some(HashAlgorithm::Sha1)),
        "sha512" => Ok(Some(HashAlgorithm::Sha512)),
        _ => anyhow::bail!("Unsupported hash algorithm: {}", algo),
    }
}

#[cfg(test)]
mod tests {
    use super::{HashAlgorithm, hashes_match, parse_hash_algorithm};

    #[test]
    fn compares_hashes_case_insensitively() {
        assert!(hashes_match("deadbeef", "DEADBEEF"));
        assert!(!hashes_match("deadbeef", "deadbeee"));
    }

    #[test]
    fn parses_hash_algorithm_case_insensitively() {
        assert_eq!(
            parse_hash_algorithm("SHA1").unwrap(),
            Some(HashAlgorithm::Sha1)
        );
        assert_eq!(
            parse_hash_algorithm("sha512").unwrap(),
            Some(HashAlgorithm::Sha512)
        );
        assert_eq!(parse_hash_algorithm("none").unwrap(), None);
        assert!(parse_hash_algorithm("md5").is_err());
    }
}
