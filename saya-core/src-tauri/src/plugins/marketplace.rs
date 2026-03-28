use ed25519_dalek::{Signature, Verifier};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use zip::ZipArchive;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginEntry {
    pub name: String,
    pub display_name: String,
    pub icon: String,
    pub version: String,
    pub description: String,
    pub repo_url: String,
    pub verified: bool,
    pub manifest: PluginEntryManifest,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginEntryManifest {
    pub columns: Vec<PluginColumnEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginColumnEntry {
    pub name: String,
    pub display: String,
    #[serde(rename = "type")]
    pub col_type: String,
    pub dtype: String,
    pub sortable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Registry {
    pub registry_version: String,
    pub updated_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub public_key: Option<String>,
    pub plugins: Vec<PluginEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryVerifyResult {
    pub valid: bool,
    pub plugins: Vec<PluginEntry>,
    pub error: Option<String>,
}

pub fn verify_registry_signature(json_str: &str) -> Result<RegistryVerifyResult, String> {
    let registry: Registry = serde_json::from_str(json_str)
        .map_err(|e| format!("Failed to parse registry JSON: {}", e))?;

    let signature_hex = registry.signature.as_ref()
        .ok_or("Registry missing signature field")?;
    let public_key_hex = registry.public_key.as_ref()
        .ok_or("Registry missing public_key field")?;

    let signature_bytes = hex::decode(signature_hex)
        .map_err(|e| format!("Invalid signature hex: {}", e))?;
    let public_key_bytes = hex::decode(public_key_hex)
        .map_err(|e| format!("Invalid public_key hex: {}", e))?;

    if public_key_bytes.len() != 32 {
        return Ok(RegistryVerifyResult {
            valid: false,
            plugins: vec![],
            error: Some("Public key must be 32 bytes".to_string()),
        });
    }

    let public_key = ed25519_dalek::VerifyingKey::from_bytes(
        public_key_bytes.as_slice().try_into().map_err(|_| "Invalid public key length")?
    ).map_err(|e| format!("Invalid public key: {}", e))?;

    let canonical_json = strip_signature_fields(json_str);
    let payload = canonical_json.as_bytes();

    let signature = Signature::from_slice(&signature_bytes)
        .map_err(|e| format!("Invalid signature: {}", e))?;

    match public_key.verify(payload, &signature) {
        Ok(_) => Ok(RegistryVerifyResult {
            valid: true,
            plugins: registry.plugins,
            error: None,
        }),
        Err(_) => Ok(RegistryVerifyResult {
            valid: false,
            plugins: vec![],
            error: Some("Signature verification failed".to_string()),
        }),
    }
}

fn strip_signature_fields(json_str: &str) -> String {
    let value: serde_json::Value = serde_json::from_str(json_str)
        .unwrap_or(serde_json::Value::Object(serde_json::Map::new()));

    let mut obj = value.as_object()
        .cloned()
        .unwrap_or_default();

    obj.remove("signature");
    obj.remove("public_key");

    serde_json::to_string(&obj).unwrap_or_else(|_| "{}".to_string())
}

pub async fn fetch_registry(url: &str) -> Result<String, String> {
    let client = Client::builder()
        .user_agent("Saya-Core/0.1.0")
        .build()
        .map_err(|e| format!("HTTP client error: {}", e))?;

    let response = client.get(url)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch registry: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("Registry fetch failed with status: {}", response.status()));
    }

    response.text().await
        .map_err(|e| format!("Failed to read registry response: {}", e))
}

pub async fn fetch_readme(owner: &str, repo: &str) -> Result<String, String> {
    let client = Client::builder()
        .user_agent("Saya-Core/0.1.0")
        .build()
        .map_err(|e| format!("HTTP client error: {}", e))?;

    let url = format!("https://api.github.com/repos/{}/{}/readme", owner, repo);
    let response = client.get(&url)
        .header("Accept", "application/vnd.github.raw")
        .send()
        .await
        .map_err(|e| format!("Failed to fetch README: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("README fetch failed with status: {}", response.status()));
    }

    response.text().await
        .map_err(|e| format!("Failed to read README response: {}", e))
}

pub async fn install_plugin_from_repo(
    repo_url: &str,
    plugins_dir: &PathBuf,
) -> Result<bool, String> {
    let client = Client::builder()
        .user_agent("Saya-Core/0.1.0")
        .build()
        .map_err(|e| format!("HTTP client error: {}", e))?;

    let (owner, repo) = parse_repo_url(repo_url)?;

    let zip_url = format!("https://github.com/{}/{}/zipball", owner, repo);
    let response = client.get(&zip_url)
        .send()
        .await
        .map_err(|e| format!("Failed to download plugin: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("Download failed with status: {}", response.status()));
    }

    let bytes = response.bytes().await
        .map_err(|e| format!("Failed to read zip response: {}", e))?;

    let cursor = std::io::Cursor::new(bytes);
    let mut archive = ZipArchive::new(cursor)
        .map_err(|e| format!("Failed to read zip archive: {}", e))?;

    let temp_dir = std::env::temp_dir().join(format!("saya_plugin_install_{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&temp_dir)
        .map_err(|e| format!("Failed to create temp dir: {}", e))?;

    let mut root_dir_name: Option<String> = None;
    for i in 0..archive.len() {
        let name = archive.by_index_raw(i)
            .map_err(|e| format!("Failed to read zip entry: {}", e))?
            .name()
            .to_string();
        if name.ends_with('/') {
            let parts: Vec<&str> = name.split('/').collect();
            if let Some(dir) = parts.first() {
                if dir.starts_with(&format!("{}-", repo)) {
                    root_dir_name = Some(dir.to_string());
                    break;
                }
            }
        }
    }

    let _root = root_dir_name.ok_or("Could not find plugin root in zipball")?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)
            .map_err(|e| format!("Failed to read zip entry: {}", e))?;
        let outpath = match file.enclosed_name() {
            Some(path) => temp_dir.join(path),
            None => continue,
        };

        if file.name().ends_with('/') {
            std::fs::create_dir_all(&outpath)
                .map_err(|e| format!("Failed to create dir: {}", e))?;
        } else {
            if let Some(parent) = outpath.parent() {
                if !parent.exists() {
                    std::fs::create_dir_all(parent)
                        .map_err(|e| format!("Failed to create parent dir: {}", e))?;
                }
            }
            let mut outfile = std::fs::File::create(&outpath)
                .map_err(|e| format!("Failed to create file: {}", e))?;
            std::io::copy(&mut file, &mut outfile)
                .map_err(|e| format!("Failed to write file: {}", e))?;
        }
    }

    let inner_dir = temp_dir.read_dir()
        .map_err(|e| format!("Failed to read temp dir: {}", e))?
        .filter_map(|e| e.ok())
        .find(|e| e.path().is_dir());

    if let Some(entry) = inner_dir {
        let src = entry.path();
        let dest = plugins_dir.join(&repo);

        if dest.exists() {
            return Err(format!("Plugin '{}' is already installed", repo));
        }

        std::fs::rename(&src, &dest)
            .map_err(|e| format!("Failed to move plugin to plugins dir: {}", e))?;
    }

    std::fs::remove_dir_all(&temp_dir).ok();

    tracing::info!("Successfully installed plugin '{}' from {}", repo, repo_url);
    Ok(true)
}

fn parse_repo_url(url: &str) -> Result<(String, String), String> {
    let url = url.trim_end_matches('/');
    
    if let Some(path) = url.strip_prefix("https://github.com/") {
        let parts: Vec<&str> = path.split('/').collect();
        if parts.len() >= 2 {
            return Ok((parts[0].to_string(), parts[1].to_string()));
        }
    }

    Err("Invalid GitHub repo URL".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_repo_url() {
        assert_eq!(
            parse_repo_url("https://github.com/saya-org/plugin-email").unwrap(),
            ("saya-org".to_string(), "plugin-email".to_string())
        );
        assert_eq!(
            parse_repo_url("https://github.com/user/repo/").unwrap(),
            ("user".to_string(), "repo".to_string())
        );
    }

    #[test]
    fn test_parse_repo_url_invalid() {
        assert!(parse_repo_url("https://gitlab.com/user/repo").is_err());
        assert!(parse_repo_url("not-a-url").is_err());
    }

    #[test]
    fn test_strip_signature_fields() {
        let json = r#"{
            "registry_version": "1",
            "signature": "abc123",
            "public_key": "def456",
            "plugins": []
        }"#;
        let result = strip_signature_fields(json);
        let value: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert!(!value.as_object().unwrap().contains_key("signature"));
        assert!(!value.as_object().unwrap().contains_key("public_key"));
        assert!(value.as_object().unwrap().contains_key("registry_version"));
    }
}
