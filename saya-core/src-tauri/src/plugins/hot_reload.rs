use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Emitter};
use tracing::{debug, error, info};

pub struct HotReloadWatcher {
    _watcher: RecommendedWatcher,
    _plugins_dir: PathBuf,
}

#[derive(Clone, serde::Serialize)]
pub struct PluginFileChanged {
    pub plugin_name: String,
}

impl HotReloadWatcher {
    pub fn new(app_handle: AppHandle, plugins_dir: PathBuf) -> Result<Self, String> {
        let dir = plugins_dir.clone();

        if !dir.exists() {
            std::fs::create_dir_all(&dir)
                .map_err(|e| format!("Failed to create plugins dir: {}", e))?;
        }

        let plugins_dir_for_watcher = plugins_dir.clone();

        let watcher =
            notify::recommended_watcher(move |res: Result<Event, notify::Error>| match res {
                Ok(event) => {
                    if let Some(plugin_name) =
                        get_changed_plugin(&plugins_dir_for_watcher, &event.paths)
                    {
                        debug!("File changed in plugin: {}", plugin_name);
                        let _ = app_handle
                            .emit("plugin-file-changed", PluginFileChanged { plugin_name });
                    }
                }
                Err(e) => error!("File watch error: {}", e),
            })
            .map_err(|e| format!("Failed to create file watcher: {}", e))?;

        let mut w = watcher;
        w.watch(&dir, RecursiveMode::Recursive)
            .map_err(|e| format!("Failed to watch plugins dir: {}", e))?;

        info!("Hot reload watching: {}", dir.display());

        Ok(HotReloadWatcher {
            _watcher: w,
            _plugins_dir: plugins_dir,
        })
    }
}

fn get_changed_plugin(plugins_dir: &Path, paths: &[PathBuf]) -> Option<String> {
    for path in paths {
        if let Ok(relative) = path.strip_prefix(plugins_dir) {
            if let Some(first) = relative.components().next() {
                let name = first.as_os_str().to_string_lossy().to_string();
                if path.exists() && !name.starts_with('.') {
                    return Some(name);
                }
            }
        }
    }
    None
}
