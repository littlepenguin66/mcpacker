use crate::models::{LoaderType, ModInfo, ServerContext, SideType};
use crate::parsers::filter;
use anyhow::{Context, Result, bail};
use serde::Deserialize;
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;
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

/// Parse CurseForge modpack
pub fn parse_curseforge(
    path: &PathBuf,
    filter_client: bool,
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

    for file in manifest.files {
        let (name, temp_file_name, download_urls) =
            resolve_cf_file_multi_mirror(file.project_id, file.file_id);

        let display_name = name.clone();

        let mut side = SideType::Both;

        if filter_client && filter::is_client_only_mod(&name) {
            side = SideType::Client;
        }

        if side == SideType::Client {
            continue;
        }

        mods.push(ModInfo {
            name: display_name,
            file_name: temp_file_name,
            download_urls,
            hash: "".to_string(),
            hash_algo: "none".to_string(),
            side,
            is_required: file.required,
        });
    }

    Ok((server_context, mods))
}

/// Resolve CurseForge file URLs
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
