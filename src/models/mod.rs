use serde::{Deserialize, Serialize};

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
    pub download_url: String,
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
