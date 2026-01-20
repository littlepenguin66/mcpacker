use crate::models::{LoaderType, ModInfo, ServerContext, SideType};
use anyhow::{bail, Context, Result};
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
    // name is not strictly in the file object usually, it's just a path.
    // We can infer name from path or leave it as filename.
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

pub fn parse_mrpack(path: &PathBuf) -> Result<(ServerContext, Vec<ModInfo>)> {
    let file = File::open(path).with_context(|| format!("Failed to open file: {:?}", path))?;
    let mut archive = ZipArchive::new(file).with_context(|| "Failed to open zip archive")?;

    // 1. Read modrinth.index.json
    let mut index_file = archive
        .by_name("modrinth.index.json")
        .with_context(|| "modrinth.index.json not found in archive")?;

    let mut json_content = String::new();
    index_file.read_to_string(&mut json_content)?;

    let index: MrPackIndex = serde_json::from_str(&json_content)
        .with_context(|| "Failed to parse modrinth.index.json")?;

    // 2. Parse Loader and MC Version
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

    // 3. Extract Mods
    let mut mods = Vec::new();

    for file in index.files {
        // Determine Side
        // env.server: "required" | "optional" | "unsupported"
        // env.client: "required" | "optional" | "unsupported"

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
            (_, "unsupported") => SideType::Client, // Server doesn't support it -> Client only
            ("unsupported", _) => SideType::Server, // Client doesn't support it -> Server only
            _ => SideType::Both,
        };

        // Filter: We only care about things that go on the server
        if side == SideType::Client {
            continue;
        }

        let is_required = server_env == "required";

        // Get Name/Filename
        let file_path_in_pack = PathBuf::from(&file.path);
        let file_name = file_path_in_pack
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "unknown.jar".to_string());

        // Name usually isn't in the file object, use filename or path stem
        let name = file_path_in_pack
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| file_name.clone());

        // Get Hash (prefer sha512, fallback to sha1)
        let (hash, hash_algo) = if let Some(h) = file.hashes.sha512 {
            (h, "sha512".to_string())
        } else if let Some(h) = file.hashes.sha1 {
            (h, "sha1".to_string())
        } else {
            // If no hash, we might skip or error. Let's warn but keep if url exists.
            ("".to_string(), "none".to_string())
        };

        // Get URL
        let download_url = file.downloads.first().cloned().unwrap_or_default();
        if download_url.is_empty() {
            // Cannot download without URL
            continue;
        }

        mods.push(ModInfo {
            name,
            file_name,
            download_url,
            hash,
            hash_algo,
            side,
            is_required,
        });
    }

    Ok((server_context, mods))
}
