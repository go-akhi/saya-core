pub mod registry;
pub mod marketplace;

use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct PluginColumn {
    pub name: String,
    pub display: String,
    #[serde(rename = "type")]
    pub col_type: String,
    pub dtype: String,
    pub sortable: bool,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct AiAction {
    pub id: String,
    pub label: String,
    pub context_columns: Vec<String>,
    pub result_mapping: ResultMapping,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ResultMapping {
    pub cognitive_axis: String,
    pub context_axis: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ProvidedAction {
    pub label: String,
    pub target_types: Vec<String>,
    pub handler: String,
    #[serde(default)]
    pub field_mapping: Option<FieldMapping>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct FieldMapping {
    pub action_title: String,
    pub cognitive_axis: String,
    pub context_axis: String,
    pub source_type: String,
    pub source_id: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct PluginManifest {
    pub name: String,
    pub display_name: String,
    pub icon: Option<String>,
    #[serde(default)]
    pub columns: Vec<PluginColumn>,
    #[serde(default)]
    pub ai_actions: Vec<AiAction>,
    #[serde(default)]
    pub provides_actions: Vec<ProvidedAction>,
}

impl PluginManifest {
    pub fn from_file(path: &Path) -> Result<Self, String> {
        let content =
            std::fs::read_to_string(path).map_err(|e| format!("Failed to read manifest: {}", e))?;
        serde_json::from_str(&content).map_err(|e| format!("Invalid manifest JSON: {}", e))
    }
}

pub fn discover_plugins(plugins_dir: &Path) -> Vec<Result<PluginManifest, String>> {
    if !plugins_dir.is_dir() {
        return vec![];
    }

    let mut results = vec![];

    let entries = match std::fs::read_dir(plugins_dir) {
        Ok(e) => e,
        Err(e) => return vec![Err(format!("Failed to read plugins dir: {}", e))],
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let manifest_path = path.join("manifest.json");
        if manifest_path.exists() {
            results.push(PluginManifest::from_file(&manifest_path));
        }
    }

    results
}

pub fn validate_manifest(manifest: &PluginManifest, registered_plugins: &[String]) -> Vec<String> {
    let mut errors = vec![];

    if manifest.name.is_empty() {
        errors.push("Plugin name is required.".into());
    }

    if manifest.display_name.is_empty() {
        errors.push("Plugin display_name is required.".into());
    }

    let has_cognitive = manifest.columns.iter().any(|c| c.name == "cognitive_axis");
    let has_context = manifest.columns.iter().any(|c| c.name == "context_axis");

    if !has_cognitive {
        errors.push(format!(
            "Plugin '{}' is missing the required 'cognitive_axis' column.",
            manifest.name
        ));
    }

    if !has_context {
        errors.push(format!(
            "Plugin '{}' is missing the required 'context_axis' column.",
            manifest.name
        ));
    }

    for action in &manifest.provides_actions {
        for target in &action.target_types {
            if target != &manifest.name && !registered_plugins.contains(target) {
                errors.push(format!(
                    "Plugin '{}' declares actions for '{}' which is not registered.",
                    manifest.name, target
                ));
            }
        }

        if let Some(ref mapping) = action.field_mapping {
            for target in &action.target_types {
                if target != &manifest.name && !registered_plugins.contains(target) {
                    errors.push(format!(
                        "Plugin '{}' field_mapping references missing plugin '{}'.",
                        manifest.name, target
                    ));
                }
            }

            if mapping.cognitive_axis.starts_with("source.")
                && !registered_plugins.contains(&mapping.source_type)
            {
                errors.push(format!(
                    "Plugin '{}' field_mapping references missing source plugin '{}'.",
                    manifest.name, mapping.source_type
                ));
            }
        }
    }

    errors
}

pub fn scan_for_network_calls(ui_dir: &Path) -> Vec<String> {
    let mut violations = vec![];

    if !ui_dir.is_dir() {
        return violations;
    }

    scan_directory(ui_dir, &mut violations);
    violations
}

fn scan_directory(dir: &Path, violations: &mut Vec<String>) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            scan_directory(&path, violations);
            continue;
        }

        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if !matches!(ext, "js" | "html" | "htm" | "ts" | "jsx" | "tsx") {
            continue;
        }

        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        let path_str = path.to_string_lossy();

        if content.contains("fetch(") && !content.contains("saya://") {
            violations.push(format!(
                "{}: contains fetch() call — plugins must not make direct network requests",
                path_str
            ));
        }

        if content.contains("XMLHttpRequest") {
            violations.push(format!(
                "{}: contains XMLHttpRequest — plugins must not make direct network requests",
                path_str
            ));
        }

        if content.contains("axios(") || content.contains("axios.") {
            violations.push(format!(
                "{}: contains axios usage — plugins must not make direct network requests",
                path_str
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn temp_dir(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("saya_test_{}", name));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    // --- Manifest parsing ---

    #[test]
    fn test_parse_valid_manifest() {
        let dir = temp_dir("parse_valid");
        let manifest_path = dir.join("manifest.json");
        fs::write(
            &manifest_path,
            r#"{
                "name": "email",
                "display_name": "Email",
                "icon": "📧",
                "columns": [
                    { "name": "subject", "display": "Subject", "type": "main", "dtype": "text", "sortable": true },
                    { "name": "cognitive_axis", "display": "Axis", "type": "filterable", "dtype": "enum", "sortable": true },
                    { "name": "context_axis", "display": "Context", "type": "filterable", "dtype": "text", "sortable": false }
                ],
                "ai_actions": [
                    {
                        "id": "classify",
                        "label": "Classify",
                        "context_columns": ["subject", "sender"],
                        "result_mapping": { "cognitive_axis": "cognitive_axis", "context_axis": "context_axis" }
                    }
                ],
                "provides_actions": [
                    {
                        "label": "Push to Notes",
                        "target_types": ["email"],
                        "handler": "pipeline:push_to_note"
                    }
                ]
            }"#,
        )
        .unwrap();

        let manifest = PluginManifest::from_file(&manifest_path).unwrap();
        assert_eq!(manifest.name, "email");
        assert_eq!(manifest.display_name, "Email");
        assert_eq!(manifest.icon.as_deref(), Some("📧"));
        assert_eq!(manifest.columns.len(), 3);
        assert_eq!(manifest.ai_actions.len(), 1);
        assert_eq!(manifest.provides_actions.len(), 1);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_parse_manifest_missing_required_fields() {
        let dir = temp_dir("parse_missing");
        let manifest_path = dir.join("manifest.json");
        fs::write(&manifest_path, r#"{ "name": "test" }"#).unwrap();

        let result = PluginManifest::from_file(&manifest_path);
        assert!(result.is_err());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_parse_manifest_invalid_json() {
        let dir = temp_dir("parse_invalid");
        let manifest_path = dir.join("manifest.json");
        fs::write(&manifest_path, "not json at all").unwrap();

        let result = PluginManifest::from_file(&manifest_path);
        assert!(result.is_err());
        let _ = fs::remove_dir_all(&dir);
    }

    // --- Plugin discovery ---

    #[test]
    fn test_discover_plugins_empty_dir() {
        let dir = temp_dir("discover_empty");
        fs::create_dir_all(&dir).unwrap();
        let results = discover_plugins(&dir);
        assert!(results.is_empty());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_discover_plugins_nonexistent_dir() {
        let dir = std::path::Path::new("/nonexistent/path/that/does/not/exist");
        let results = discover_plugins(dir);
        assert!(results.is_empty());
    }

    #[test]
    fn test_discover_plugins_finds_manifests() {
        let dir = temp_dir("discover_finds");
        let plugin_dir = dir.join("email");
        fs::create_dir_all(&plugin_dir).unwrap();
        fs::write(
            plugin_dir.join("manifest.json"),
            r#"{ "name": "email", "display_name": "Email" }"#,
        )
        .unwrap();

        let plugin2_dir = dir.join("tasks");
        fs::create_dir_all(&plugin2_dir).unwrap();
        fs::write(
            plugin2_dir.join("manifest.json"),
            r#"{ "name": "tasks", "display_name": "Tasks" }"#,
        )
        .unwrap();

        let results = discover_plugins(&dir);
        assert_eq!(results.len(), 2);
        assert!(results.iter().all(|r| r.is_ok()));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_discover_skips_files() {
        let dir = temp_dir("discover_skips");
        fs::write(
            dir.join("notadir.txt"),
            r#"{ "name": "fake", "display_name": "Fake" }"#,
        )
        .unwrap();
        fs::create_dir_all(dir.join("real")).unwrap();
        fs::write(
            dir.join("real/manifest.json"),
            r#"{ "name": "real", "display_name": "Real" }"#,
        )
        .unwrap();

        let results = discover_plugins(&dir);
        assert_eq!(results.len(), 1);
        let _ = fs::remove_dir_all(&dir);
    }

    // --- Manifest validation ---

    #[test]
    fn test_validate_valid_manifest() {
        let manifest = PluginManifest {
            name: "email".into(),
            display_name: "Email".into(),
            icon: None,
            columns: vec![
                PluginColumn {
                    name: "cognitive_axis".into(),
                    display: "Axis".into(),
                    col_type: "filterable".into(),
                    dtype: "enum".into(),
                    sortable: true,
                },
                PluginColumn {
                    name: "context_axis".into(),
                    display: "Context".into(),
                    col_type: "filterable".into(),
                    dtype: "text".into(),
                    sortable: false,
                },
            ],
            ai_actions: vec![],
            provides_actions: vec![],
        };
        let errors = validate_manifest(&manifest, &[]);
        assert!(errors.is_empty());
    }

    #[test]
    fn test_validate_missing_name() {
        let manifest = PluginManifest {
            name: "".into(),
            display_name: "Email".into(),
            icon: None,
            columns: vec![],
            ai_actions: vec![],
            provides_actions: vec![],
        };
        let errors = validate_manifest(&manifest, &[]);
        assert!(errors.iter().any(|e| e.contains("name is required")));
    }

    #[test]
    fn test_validate_missing_display_name() {
        let manifest = PluginManifest {
            name: "email".into(),
            display_name: "".into(),
            icon: None,
            columns: vec![],
            ai_actions: vec![],
            provides_actions: vec![],
        };
        let errors = validate_manifest(&manifest, &[]);
        assert!(errors
            .iter()
            .any(|e| e.contains("display_name is required")));
    }

    #[test]
    fn test_validate_missing_cognitive_axis_column() {
        let manifest = PluginManifest {
            name: "email".into(),
            display_name: "Email".into(),
            icon: None,
            columns: vec![PluginColumn {
                name: "context_axis".into(),
                display: "Context".into(),
                col_type: "filterable".into(),
                dtype: "text".into(),
                sortable: false,
            }],
            ai_actions: vec![],
            provides_actions: vec![],
        };
        let errors = validate_manifest(&manifest, &[]);
        assert!(errors.iter().any(|e| e.contains("cognitive_axis")));
    }

    #[test]
    fn test_validate_missing_context_axis_column() {
        let manifest = PluginManifest {
            name: "email".into(),
            display_name: "Email".into(),
            icon: None,
            columns: vec![PluginColumn {
                name: "cognitive_axis".into(),
                display: "Axis".into(),
                col_type: "filterable".into(),
                dtype: "enum".into(),
                sortable: true,
            }],
            ai_actions: vec![],
            provides_actions: vec![],
        };
        let errors = validate_manifest(&manifest, &[]);
        assert!(errors.iter().any(|e| e.contains("context_axis")));
    }

    #[test]
    fn test_validate_action_targeting_missing_plugin() {
        let manifest = PluginManifest {
            name: "tasks".into(),
            display_name: "Tasks".into(),
            icon: None,
            columns: vec![
                PluginColumn {
                    name: "cognitive_axis".into(),
                    display: "Axis".into(),
                    col_type: "filterable".into(),
                    dtype: "enum".into(),
                    sortable: true,
                },
                PluginColumn {
                    name: "context_axis".into(),
                    display: "Context".into(),
                    col_type: "filterable".into(),
                    dtype: "text".into(),
                    sortable: false,
                },
            ],
            ai_actions: vec![],
            provides_actions: vec![ProvidedAction {
                label: "From Email".into(),
                target_types: vec!["email".into()],
                handler: "create_from_email".into(),
                field_mapping: None,
            }],
        };
        let errors = validate_manifest(&manifest, &[]);
        assert!(errors.iter().any(|e| e.contains("not registered")));
    }

    #[test]
    fn test_validate_action_targeting_self_is_ok() {
        let manifest = PluginManifest {
            name: "email".into(),
            display_name: "Email".into(),
            icon: None,
            columns: vec![
                PluginColumn {
                    name: "cognitive_axis".into(),
                    display: "Axis".into(),
                    col_type: "filterable".into(),
                    dtype: "enum".into(),
                    sortable: true,
                },
                PluginColumn {
                    name: "context_axis".into(),
                    display: "Context".into(),
                    col_type: "filterable".into(),
                    dtype: "text".into(),
                    sortable: false,
                },
            ],
            ai_actions: vec![],
            provides_actions: vec![ProvidedAction {
                label: "Archive".into(),
                target_types: vec!["email".into()],
                handler: "archive".into(),
                field_mapping: None,
            }],
        };
        let errors = validate_manifest(&manifest, &[]);
        assert!(!errors.iter().any(|e| e.contains("not registered")));
    }

    // --- Network isolation scanner ---

    #[test]
    fn test_scan_clean_plugin() {
        let dir = temp_dir("scan_clean");
        let ui_dir = dir.join("ui");
        fs::create_dir_all(&ui_dir).unwrap();
        fs::write(
            ui_dir.join("index.html"),
            r#"<html><body><h1>Hello</h1><script>console.log('ok')</script></body></html>"#,
        )
        .unwrap();

        let violations = scan_for_network_calls(&ui_dir);
        assert!(violations.is_empty());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_scan_detects_fetch() {
        let dir = temp_dir("scan_fetch");
        let ui_dir = dir.join("ui");
        fs::create_dir_all(&ui_dir).unwrap();
        fs::write(
            ui_dir.join("app.js"),
            r#"fetch('https://api.example.com/data')"#,
        )
        .unwrap();

        let violations = scan_for_network_calls(&ui_dir);
        assert_eq!(violations.len(), 1);
        assert!(violations[0].contains("fetch()"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_scan_detects_xhr() {
        let dir = temp_dir("scan_xhr");
        let ui_dir = dir.join("ui");
        fs::create_dir_all(&ui_dir).unwrap();
        fs::write(ui_dir.join("app.js"), r#"var xhr = new XMLHttpRequest();"#).unwrap();

        let violations = scan_for_network_calls(&ui_dir);
        assert_eq!(violations.len(), 1);
        assert!(violations[0].contains("XMLHttpRequest"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_scan_detects_axios() {
        let dir = temp_dir("scan_axios");
        let ui_dir = dir.join("ui");
        fs::create_dir_all(&ui_dir).unwrap();
        fs::write(ui_dir.join("app.js"), r#"axios.get('/api/data')"#).unwrap();

        let violations = scan_for_network_calls(&ui_dir);
        assert_eq!(violations.len(), 1);
        assert!(violations[0].contains("axios"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_scan_nonexistent_dir() {
        let dir = std::path::Path::new("/nonexistent/ui");
        let violations = scan_for_network_calls(dir);
        assert!(violations.is_empty());
    }

    #[test]
    fn test_scan_only_checks_code_files() {
        let dir = temp_dir("scan_extensions");
        let ui_dir = dir.join("ui");
        fs::create_dir_all(&ui_dir).unwrap();
        fs::write(
            ui_dir.join("data.json"),
            r#"{ "fetch": "this is just json" }"#,
        )
        .unwrap();

        let violations = scan_for_network_calls(&ui_dir);
        assert!(violations.is_empty());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_scan_nested_directories() {
        let dir = temp_dir("scan_nested");
        let ui_dir = dir.join("ui/assets");
        fs::create_dir_all(&ui_dir).unwrap();
        fs::write(ui_dir.join("api.js"), r#"fetch('https://evil.com')"#).unwrap();

        let violations = scan_for_network_calls(&dir.join("ui"));
        assert_eq!(violations.len(), 1);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_scan_allows_saya_protocol() {
        let dir = temp_dir("scan_saya");
        let ui_dir = dir.join("ui");
        fs::create_dir_all(&ui_dir).unwrap();
        fs::write(ui_dir.join("app.js"), r#"fetch('saya://api/items')"#).unwrap();

        let violations = scan_for_network_calls(&ui_dir);
        assert!(violations.is_empty());
        let _ = fs::remove_dir_all(&dir);
    }
}
