use crate::models::{LoaderType, ModInfo, ServerContext, SideType};
use crate::parsers::filter;
use crate::ui::print_warn;
use anyhow::{Context, Result, bail};
use futures::StreamExt;
use reqwest::{
    Client,
    cookie::Jar,
    header::{HeaderMap, HeaderValue},
};
use serde::Deserialize;
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use zip::ZipArchive;

#[derive(Debug, Deserialize)]
struct Manifest {
    minecraft: MinecraftInfo,
    files: Vec<ManifestFile>,
    #[allow(dead_code)]
    overrides: String,
}

#[derive(Debug, Deserialize)]
struct MinecraftInfo {
    version: String,
    #[serde(rename = "modLoaders")]
    mod_loaders: Vec<ModLoader>,
}

#[derive(Debug, Deserialize)]
struct ModLoader {
    id: String,
    primary: bool,
}

#[derive(Debug, Deserialize)]
struct ManifestFile {
    #[serde(rename = "projectID")]
    project_id: u32,
    #[serde(rename = "fileID")]
    file_id: u32,
    required: bool,
}

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

pub async fn parse_curseforge(
    path: &Path,
    filter_client: bool,
    proxy: Option<&str>,
) -> Result<(ServerContext, Vec<ModInfo>)> {
    let file = File::open(path).with_context(|| format!("Failed to open file: {:?}", path))?;
    let mut archive = ZipArchive::new(file).with_context(|| "Failed to open zip archive")?;

    let mut manifest_file = archive
        .by_name("manifest.json")
        .with_context(|| "manifest.json not found in archive")?;

    let mut json_content = String::new();
    manifest_file.read_to_string(&mut json_content)?;

    let manifest: Manifest =
        serde_json::from_str(&json_content).with_context(|| "Failed to parse manifest.json")?;

    let mc_version = manifest.minecraft.version;

    let primary_loader = manifest
        .minecraft
        .mod_loaders
        .iter()
        .find(|ml| ml.primary)
        .or_else(|| manifest.minecraft.mod_loaders.first());

    let (loader_type, loader_version) = if let Some(loader) = primary_loader {
        if loader.id.starts_with("forge-") {
            (
                LoaderType::Forge,
                loader.id.trim_start_matches("forge-").to_string(),
            )
        } else if loader.id.starts_with("fabric-") {
            (
                LoaderType::Fabric,
                loader.id.trim_start_matches("fabric-").to_string(),
            )
        } else if loader.id.starts_with("quilt-") {
            (
                LoaderType::Quilt,
                loader.id.trim_start_matches("quilt-").to_string(),
            )
        } else if loader.id.starts_with("neoforge-") {
            (
                LoaderType::NeoForge,
                loader.id.trim_start_matches("neoforge-").to_string(),
            )
        } else {
            (LoaderType::Forge, loader.id.clone())
        }
    } else {
        bail!("No mod loader found in manifest");
    };

    let server_context = ServerContext {
        minecraft_version: mc_version,
        loader_type,
        loader_version,
    };

    let mut mods = Vec::new();
    let keywords = if filter_client {
        filter::client_only_keywords()
    } else {
        Vec::new()
    };

    if filter_client && keywords.is_empty() {
        print_warn("Client-only filter list is empty; CurseForge filtering may be incomplete.");
    }

    let resolution = if filter_client && !keywords.is_empty() {
        resolve_cf_file_names(&manifest.files, proxy).await?
    } else {
        ResolutionState::default()
    };

    if let Some(message) = partial_filter_warning(filter_client, resolution.failed) {
        print_warn(&message);
    }

    for file in manifest.files {
        let resolved_file_name = resolution
            .file_names
            .get(&(file.project_id, file.file_id))
            .map(String::as_str);

        let Some(mod_info) = build_mod_info(&file, resolved_file_name, &keywords, filter_client)
        else {
            continue;
        };

        mods.push(mod_info);
    }

    Ok((server_context, mods))
}

#[derive(Default)]
struct ResolutionState {
    file_names: HashMap<(u32, u32), String>,
    failed: usize,
}

async fn resolve_cf_file_names(
    files: &[ManifestFile],
    proxy: Option<&str>,
) -> Result<ResolutionState> {
    let client = build_metadata_client(proxy)?;
    let _ = client.get("https://www.curseforge.com").send().await;

    let resolved = futures::stream::iter(files.iter().map(|file| {
        let client = client.clone();
        async move {
            Ok::<_, anyhow::Error>(
                fetch_cf_file_name(&client, file.project_id, file.file_id)
                    .await
                    .map(|file_name| ((file.project_id, file.file_id), file_name)),
            )
        }
    }))
    .buffer_unordered(10)
    .collect::<Vec<_>>()
    .await;

    let mut state = ResolutionState {
        file_names: HashMap::with_capacity(files.len()),
        failed: 0,
    };
    for entry in resolved {
        match entry? {
            Ok(((project_id, file_id), file_name)) => {
                state.file_names.insert((project_id, file_id), file_name);
            }
            Err(_) => state.failed += 1,
        }
    }

    Ok(state)
}

fn build_metadata_client(proxy: Option<&str>) -> Result<Client> {
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
        .cookie_provider(jar)
        .user_agent(USER_AGENT)
        .tcp_nodelay(true)
        .pool_max_idle_per_host(10)
        .pool_idle_timeout(Duration::from_secs(15))
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(20))
        .default_headers(headers);

    if let Some(proxy_url) = proxy {
        let proxy = reqwest::Proxy::all(proxy_url)
            .with_context(|| format!("Invalid proxy URL: {}", proxy_url))?;
        client_builder = client_builder.proxy(proxy);
    }

    client_builder.build().map_err(Into::into)
}

async fn fetch_cf_file_name(client: &Client, project_id: u32, file_id: u32) -> Result<String> {
    let meta_url = format!(
        "https://www.curseforge.com/api/v1/mods/{}/files/{}",
        project_id, file_id
    );

    let response = client
        .get(&meta_url)
        .header("Accept", "application/json")
        .send()
        .await
        .with_context(|| {
            format!(
                "Failed to resolve CurseForge metadata for project {} file {}",
                project_id, file_id
            )
        })?;

    if !response.status().is_success() {
        bail!(
            "Failed to resolve CurseForge metadata for project {} file {}: {}",
            project_id,
            file_id,
            response.status()
        );
    }

    let json = response.json::<CfFileResponse>().await.with_context(|| {
        format!(
            "Failed to decode CurseForge metadata for project {} file {}",
            project_id, file_id
        )
    })?;

    Ok(json.data.file_name)
}

fn build_mod_info(
    file: &ManifestFile,
    resolved_file_name: Option<&str>,
    keywords: &[String],
    filter_client: bool,
) -> Option<ModInfo> {
    let (default_name, temp_file_name, download_urls) =
        resolve_cf_file_multi_mirror(file.project_id, file.file_id);
    let file_name = resolved_file_name.unwrap_or(&temp_file_name).to_string();
    let display_name = resolved_file_name
        .map(display_name_from_file_name)
        .unwrap_or(default_name);
    let side = if filter_client
        && resolved_file_name.is_some_and(|name| filter::is_client_only_name(name, keywords))
    {
        SideType::Client
    } else {
        SideType::Both
    };

    if side == SideType::Client {
        return None;
    }

    Some(ModInfo {
        name: display_name,
        file_name,
        download_urls,
        hash: String::new(),
        hash_algo: "none".to_string(),
        side,
        is_required: file.required,
    })
}

fn display_name_from_file_name(file_name: &str) -> String {
    Path::new(file_name)
        .file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_else(|| file_name.to_string())
}

fn partial_filter_warning(filter_client: bool, failed: usize) -> Option<String> {
    if filter_client && failed > 0 {
        Some(format!(
            "Failed to resolve metadata for {} CurseForge files; client-only filtering may be incomplete.",
            failed
        ))
    } else {
        None
    }
}

fn resolve_cf_file_multi_mirror(project_id: u32, file_id: u32) -> (String, String, Vec<String>) {
    let name = format!("CF-{}", project_id);
    let temp_file_name = format!("{}.jar", file_id);

    let api_url = format!(
        "https://www.curseforge.com/api/v1/mods/{}/files/{}/download",
        project_id, file_id
    );

    let urls = vec![api_url];

    (name, temp_file_name, urls)
}

#[cfg(test)]
mod tests {
    use super::{
        ManifestFile, build_mod_info, display_name_from_file_name, partial_filter_warning,
    };

    #[test]
    fn strips_extension_from_resolved_file_name() {
        assert_eq!(
            display_name_from_file_name("Sodium-Fabric-0.5.0+mc1.20.1.jar"),
            "Sodium-Fabric-0.5.0+mc1.20.1"
        );
    }

    #[test]
    fn filters_client_only_curseforge_mods_when_name_is_resolved() {
        let file = ManifestFile {
            project_id: 394468,
            file_id: 1234567,
            required: true,
        };
        let keywords = vec!["sodium".to_string()];

        assert!(
            build_mod_info(
                &file,
                Some("Sodium-Fabric-0.5.0+mc1.20.1.jar"),
                &keywords,
                true
            )
            .is_none()
        );
    }

    #[test]
    fn keeps_unresolved_entries_even_when_filtering_is_enabled() {
        let file = ManifestFile {
            project_id: 394468,
            file_id: 1234567,
            required: true,
        };
        let keywords = vec!["sodium".to_string()];

        let mod_info = build_mod_info(&file, None, &keywords, true).unwrap();

        assert_eq!(mod_info.name, "CF-394468");
        assert_eq!(mod_info.file_name, "1234567.jar");
    }

    #[test]
    fn reports_partial_filter_failures() {
        assert!(partial_filter_warning(true, 2).is_some());
        assert!(partial_filter_warning(true, 0).is_none());
        assert!(partial_filter_warning(false, 2).is_none());
    }
}
