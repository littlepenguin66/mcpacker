use anyhow::{Context, Result};
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

const FALLBACK_URL: &str = "https://raw.githubusercontent.com/Griefed/ServerPackCreator/main/serverpackcreator-api/src/main/resources/serverpackcreator.properties";

/// Get cache path
fn get_cache_path() -> Result<PathBuf> {
    let project_dirs = directories::ProjectDirs::from("com", "mcpacker", "mcpacker")
        .context("Failed to determine cache directory")?;
    let cache_dir = project_dirs.cache_dir();
    if !cache_dir.exists() {
        std::fs::create_dir_all(cache_dir)?;
    }
    Ok(cache_dir.join("fallback_mods.txt"))
}

/// Check if cache is present
pub fn is_cache_present() -> bool {
    get_cache_path().map(|p| p.exists()).unwrap_or(false)
}

/// Update fallback list
pub async fn update_fallback_list(proxy: Option<&str>) -> Result<()> {
    let mut client_builder = reqwest::Client::builder();

    if let Some(proxy_url) = proxy {
        let proxy = reqwest::Proxy::all(proxy_url)
            .with_context(|| format!("Invalid proxy URL: {}", proxy_url))?;
        client_builder = client_builder.proxy(proxy);
    }

    let client = client_builder.build()?;
    let response = client.get(FALLBACK_URL).send().await?.text().await?;
    let keywords = parse_keywords_from_properties(&response);
    let cache_path = get_cache_path()?;
    let mut file = File::create(cache_path)?;

    let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
    writeln!(file, "# Updated: {}", now)?;

    for kw in keywords {
        writeln!(file, "{}", kw)?;
    }
    Ok(())
}

/// Parse keywords from properties
fn parse_keywords_from_properties(content: &str) -> Vec<String> {
    let mut keywords = Vec::new();
    let mut in_list = false;

    for line in content.lines() {
        let line = line.trim();
        if line.starts_with("de.griefed.serverpackcreator.configuration.fallbackmodslist=") {
            in_list = true;
            let val = line
                .split('=')
                .nth(1)
                .unwrap_or("")
                .trim()
                .trim_end_matches('\\');
            if !val.is_empty() {
                let cleaned = val.trim_end_matches(',').trim().to_lowercase();
                if !cleaned.is_empty() {
                    keywords.push(cleaned);
                }
            }
            if !line.ends_with('\\') {
                in_list = false;
            }
            continue;
        }

        if in_list {
            let cleaned = line.trim_end_matches('\\').trim_end_matches(',').trim();
            if !cleaned.is_empty() {
                keywords.push(cleaned.to_lowercase());
            }
            if !line.ends_with('\\') {
                in_list = false;
            }
        }
    }
    keywords
}

/// Load keywords
fn load_keywords() -> Vec<String> {
    if let Ok(path) = get_cache_path()
        && path.exists()
        && let Ok(content) = std::fs::read_to_string(path)
    {
        return content
            .lines()
            .filter(|l| !l.starts_with('#') && !l.is_empty())
            .map(|s| s.to_string())
            .collect();
    }

    Vec::new()
}

/// Check if mod is client-only
pub fn is_client_only_mod(name: &str) -> bool {
    let keywords = load_keywords();
    if keywords.is_empty() {
        return false;
    }
    let name_lower = name.to_lowercase();
    keywords.iter().any(|k| name_lower.contains(k))
}
