use anyhow::Context;
use serde::{Deserialize, Serialize};
use std::io::Read;

use crate::utils::sanitize_filename;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SideType {
    Server,
    Client,
    Both,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModInfo {
    pub name: String,
    pub file_name: String,
    pub download_urls: Vec<String>,
    pub hash: String,
    pub hash_algo: String,
    pub side: SideType,
    pub is_required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum LoaderType {
    Fabric,
    Forge,
    Quilt,
    NeoForge,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerContext {
    pub minecraft_version: String,
    pub loader_type: LoaderType,
    pub loader_version: String,
}

#[derive(Debug, Clone, Default)]
pub struct ModMetadata {
    pub name: Option<String>,
    pub version: Option<String>,
    pub mod_id: Option<String>,
}

#[derive(Debug, Deserialize)]
#[allow(non_snake_case)]
struct McModInfo {
    #[serde(default)]
    modid: Option<String>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    version: Option<String>,
}

#[derive(Debug, Deserialize)]
struct FabricModJson {
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    version: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ModsToml {
    #[serde(default)]
    mods: Vec<ModsTomlMod>,
}

#[derive(Debug, Deserialize)]
#[allow(non_snake_case)]
struct ModsTomlMod {
    #[serde(default)]
    modId: Option<String>,
    #[serde(default)]
    displayName: Option<String>,
    #[serde(default)]
    version: Option<String>,
}

impl ModMetadata {
    /// Extract metadata from jar file
    pub fn extract_from_jar(jar_path: &std::path::Path) -> anyhow::Result<Self> {
        use std::fs::File;
        use zip::ZipArchive;

        let file = File::open(jar_path)
            .with_context(|| format!("Failed to open jar file: {:?}", jar_path))?;
        let mut archive = ZipArchive::new(file)
            .with_context(|| format!("Failed to parse zip archive: {:?}", jar_path))?;

        let mut metadata = ModMetadata::default();

        if let Ok(fabric) = Self::try_read_fabric_mod_json(&mut archive) {
            metadata = fabric;
        } else if let Ok(toml) = Self::try_read_mods_toml(&mut archive) {
            metadata = toml;
        } else if let Ok(mcmod) = Self::try_read_mcmod_info(&mut archive) {
            metadata = mcmod;
        }

        Ok(metadata)
    }

    /// Try to read mcmod.info
    fn try_read_mcmod_info(archive: &mut zip::ZipArchive<std::fs::File>) -> anyhow::Result<Self> {
        let mut entry = archive.by_name("mcmod.info")?;
        let mut content = String::new();
        entry.read_to_string(&mut content)?;

        let json: serde_json::Value = json5::from_str(&content)?;

        let info = if json.is_array() {
            json.get(0)
        } else if let Some(mod_list) = json.get("modList") {
            mod_list.get(0)
        } else {
            Some(&json)
        };

        if let Some(info) = info {
            let parsed: McModInfo = serde_json::from_value(info.clone())?;

            Ok(ModMetadata {
                name: parsed.name,
                version: parsed.version,
                mod_id: parsed.modid,
            })
        } else {
            anyhow::bail!("Failed to parse mcmod.info")
        }
    }

    /// Try to read fabric.mod.json
    fn try_read_fabric_mod_json(
        archive: &mut zip::ZipArchive<std::fs::File>,
    ) -> anyhow::Result<Self> {
        let mut entry = archive.by_name("fabric.mod.json")?;
        let mut content = String::new();
        entry.read_to_string(&mut content)?;

        let parsed: FabricModJson = json5::from_str(&content)?;

        Ok(ModMetadata {
            name: parsed.name.or(parsed.id.clone()),
            version: parsed.version,
            mod_id: parsed.id,
        })
    }

    /// Try to read mods.toml
    fn try_read_mods_toml(archive: &mut zip::ZipArchive<std::fs::File>) -> anyhow::Result<Self> {
        let mut entry = archive.by_name("META-INF/mods.toml")?;
        let mut content = String::new();
        entry.read_to_string(&mut content)?;

        let parsed: ModsToml = toml::from_str(&content)?;

        if let Some(mod_info) = parsed.mods.first() {
            Ok(ModMetadata {
                name: mod_info.displayName.clone().or(mod_info.modId.clone()),
                version: mod_info.version.clone(),
                mod_id: mod_info.modId.clone(),
            })
        } else {
            anyhow::bail!("No mod info found in mods.toml")
        }
    }

    /// Get display name with fallback
    pub fn get_display_name(&self, fallback: &str) -> String {
        self.name
            .as_ref()
            .or(self.mod_id.as_ref())
            .map(|s| sanitize_filename(s))
            .unwrap_or_else(|| sanitize_filename(fallback))
    }

    /// Get version string
    pub fn get_version(&self) -> String {
        self.version
            .as_ref()
            .map(|s| sanitize_filename(s))
            .unwrap_or_else(|| "unknown".to_string())
    }
}
