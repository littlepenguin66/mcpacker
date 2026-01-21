use crate::models::{LoaderType, ModInfo, ServerContext, SideType};
use anyhow::{Context, Result, bail};
use serde::Deserialize;
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;
use zip::ZipArchive;

#[derive(Debug, Deserialize)]
struct MrPackIndex {
    dependencies: std::collections::HashMap<String, String>,
    files: Vec<MrPackFile>,
}

#[derive(Debug, Deserialize)]
struct MrPackFile {
    path: String,
    hashes: MrPackHashes,
    env: Option<MrPackEnv>,
    downloads: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct MrPackHashes {
    sha1: Option<String>,
    sha512: Option<String>,
}

#[derive(Debug, Deserialize)]
struct MrPackEnv {
    client: Option<String>,
    server: Option<String>,
}

pub fn parse_mrpack(path: &PathBuf, keep_client: bool) -> Result<(ServerContext, Vec<ModInfo>)> {
    let file = File::open(path).with_context(|| format!("Failed to open file: {:?}", path))?;
    let mut archive = ZipArchive::new(file).with_context(|| "Failed to open zip archive")?;

    let mut index_file = archive
        .by_name("modrinth.index.json")
        .with_context(|| "modrinth.index.json not found in archive")?;

    let mut json_content = String::new();
    index_file.read_to_string(&mut json_content)?;

    let index: MrPackIndex = serde_json::from_str(&json_content)
        .with_context(|| "Failed to parse modrinth.index.json")?;

    let mc_version = index
        .dependencies
        .get("minecraft")
        .with_context(|| "Minecraft version not found in dependencies")?
        .clone();

    let (loader_type, loader_version) = if let Some(v) = index.dependencies.get("fabric-loader") {
        (LoaderType::Fabric, v.clone())
    } else if let Some(v) = index.dependencies.get("forge") {
        (LoaderType::Forge, v.clone())
    } else if let Some(v) = index.dependencies.get("neoforge") {
        (LoaderType::NeoForge, v.clone())
    } else if let Some(v) = index.dependencies.get("quilt-loader") {
        (LoaderType::Quilt, v.clone())
    } else {
        bail!(
            "Unsupported or missing loader in dependencies: {:?}",
            index.dependencies
        );
    };

    let server_context = ServerContext {
        minecraft_version: mc_version,
        loader_type,
        loader_version,
    };

    let mut mods = Vec::new();

    for file in index.files {
        let client_env = file
            .env
            .as_ref()
            .and_then(|e| e.client.as_deref())
            .unwrap_or("required");
        let server_env = file
            .env
            .as_ref()
            .and_then(|e| e.server.as_deref())
            .unwrap_or("required");

        let side = match (client_env, server_env) {
            (_, "unsupported") => SideType::Client,
            ("unsupported", _) => SideType::Server,
            _ => SideType::Both,
        };

        if side == SideType::Client && !keep_client {
            continue;
        }

        let is_required = server_env == "required";

        let file_path_in_pack = PathBuf::from(&file.path);
        let file_name = file_path_in_pack
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "unknown.jar".to_string());

        let name = file_path_in_pack
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| file_name.clone());

        let (hash, hash_algo) = if let Some(h) = file.hashes.sha512 {
            (h, "sha512".to_string())
        } else if let Some(h) = file.hashes.sha1 {
            (h, "sha1".to_string())
        } else {
            ("".to_string(), "none".to_string())
        };

        let download_urls = file.downloads.clone();
        if download_urls.is_empty() {
            continue;
        }

        mods.push(ModInfo {
            name,
            file_name,
            download_urls,
            hash,
            hash_algo,
            side,
            is_required,
        });
    }

    Ok((server_context, mods))
}
