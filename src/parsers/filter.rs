use anyhow::{Context, Result};
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

const FALLBACK_URL: &str = "https://raw.githubusercontent.com/Griefed/ServerPackCreator/main/serverpackcreator-api/src/main/resources/serverpackcreator.properties";

fn get_cache_path() -> Result<PathBuf> {
    let project_dirs = directories::ProjectDirs::from("com", "mcpacker", "mcpacker")
        .context("Failed to determine cache directory")?;
    let cache_dir = project_dirs.cache_dir();
    if !cache_dir.exists() {
        std::fs::create_dir_all(cache_dir)?;
    }
    Ok(cache_dir.join("fallback_mods.txt"))
}

pub fn is_cache_present() -> bool {
    get_cache_path().map(|p| p.exists()).unwrap_or(false)
}

pub async fn update_fallback_list(proxy: Option<&str>) -> Result<()> {
    let mut client_builder = reqwest::Client::builder();

    if let Some(proxy_url) = proxy {
        let proxy = reqwest::Proxy::all(proxy_url)
            .with_context(|| format!("Invalid proxy URL: {}", proxy_url))?;
        client_builder = client_builder.proxy(proxy);
    }

    let client = client_builder.build()?;
    let response = client
        .get(FALLBACK_URL)
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;
    let keywords = validated_keywords(parse_keywords_from_properties(&response))?;
    let cache_path = get_cache_path()?;
    let mut file = File::create(cache_path)?;

    let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
    writeln!(file, "# Updated: {}", now)?;

    for kw in keywords {
        writeln!(file, "{}", kw)?;
    }
    Ok(())
}

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

pub(crate) fn client_only_keywords() -> Vec<String> {
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

pub(crate) fn is_client_only_name(name: &str, keywords: &[String]) -> bool {
    let name_lower = name.to_lowercase();
    keywords.iter().any(|keyword| name_lower.contains(keyword))
}

fn validated_keywords(keywords: Vec<String>) -> Result<Vec<String>> {
    if keywords.is_empty() {
        anyhow::bail!("Failed to parse any client-only mod keywords from fallback list")
    }

    Ok(keywords)
}

#[cfg(test)]
mod tests {
    use super::{is_client_only_name, parse_keywords_from_properties, validated_keywords};

    #[test]
    fn parses_multiline_keyword_properties() {
        let content = "\
de.griefed.serverpackcreator.configuration.fallbackmodslist=sodium,\\
iris,\\
entityculling
";

        assert_eq!(
            parse_keywords_from_properties(content),
            vec![
                "sodium".to_string(),
                "iris".to_string(),
                "entityculling".to_string()
            ]
        );
    }

    #[test]
    fn matches_keywords_case_insensitively() {
        let keywords = vec!["sodium".to_string(), "iris".to_string()];

        assert!(is_client_only_name(
            "Sodium-Fabric-0.5.0+mc1.20.1.jar",
            &keywords
        ));
        assert!(!is_client_only_name("lithium-fabric-0.12.0.jar", &keywords));
    }

    #[test]
    fn rejects_empty_keyword_sets() {
        assert!(validated_keywords(Vec::new()).is_err());
        assert!(validated_keywords(vec!["sodium".to_string()]).is_ok());
    }
}
