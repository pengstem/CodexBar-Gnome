use std::env;
use std::fs;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::util;

#[derive(Debug, Clone)]
pub struct ResolvedConfig {
    pub active_path: PathBuf,
    file: ConfigFile,
}

impl ResolvedConfig {
    pub fn load() -> Result<Self, String> {
        let home = util::home_dir()?;
        let active_path = active_config_path(&home)?;
        let legacy_path = legacy_config_path(&home);
        migrate_legacy_config_if_needed(&legacy_path, &active_path)?;

        let file = if active_path.exists() {
            load_config_file(&active_path)?
        } else {
            ConfigFile::default()
        };

        Ok(Self {
            active_path,
            file,
        })
    }

    pub fn provider(&self, provider_id: &str) -> Option<&ProviderConfig> {
        self.file.providers.iter().find(|provider| provider.id == provider_id)
    }

    pub fn provider_enabled(&self, provider_id: &str) -> bool {
        self.provider(provider_id)
            .and_then(|provider| provider.enabled)
            .unwrap_or(true)
    }

    pub fn provider_source(&self, provider_id: &str) -> Option<&str> {
        self.provider(provider_id).and_then(|provider| provider.source.as_deref())
    }
}

#[derive(Debug, Clone, Deserialize, Default)]
struct ConfigFile {
    #[serde(default)]
    providers: Vec<ProviderConfig>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ProviderConfig {
    pub id: String,
    pub enabled: Option<bool>,
    pub source: Option<String>,
}

fn load_config_file(path: &Path) -> Result<ConfigFile, String> {
    let contents = fs::read_to_string(path)
        .map_err(|error| format!("failed to read config at {}: {error}", path.display()))?;
    serde_json::from_str(&contents)
        .map_err(|error| format!("failed to parse config at {}: {error}", path.display()))
}

fn active_config_path(home: &Path) -> Result<PathBuf, String> {
    let config_root = match env::var_os("XDG_CONFIG_HOME") {
        Some(value) if !value.is_empty() => PathBuf::from(value),
        _ => home.join(".config"),
    };

    Ok(config_root.join("codexbar-gnome").join("config.json"))
}

fn legacy_config_path(home: &Path) -> PathBuf {
    home.join(".codexbar").join("config.json")
}

fn migrate_legacy_config_if_needed(legacy_path: &Path, active_path: &Path) -> Result<bool, String> {
    if active_path.exists() || !legacy_path.exists() {
        return Ok(false);
    }

    let parent = active_path
        .parent()
        .ok_or_else(|| format!("missing parent directory for {}", active_path.display()))?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;

    let contents = fs::read(legacy_path)
        .map_err(|error| format!("failed to read legacy config {}: {error}", legacy_path.display()))?;

    let mut file = fs::File::create(active_path)
        .map_err(|error| format!("failed to create {}: {error}", active_path.display()))?;
    file.write_all(&contents)
        .map_err(|error| format!("failed to write {}: {error}", active_path.display()))?;

    let permissions = fs::Permissions::from_mode(0o600);
    fs::set_permissions(active_path, permissions)
        .map_err(|error| format!("failed to set permissions on {}: {error}", active_path.display()))?;

    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_defaults_to_enabled() {
        let config = ResolvedConfig {
            active_path: PathBuf::from("/tmp/config.json"),
            file: ConfigFile::default(),
        };

        assert!(config.provider_enabled("codex"));
        assert!(config.provider_enabled("claude"));
    }

    #[test]
    fn provider_enabled_reads_explicit_value() {
        let config = ResolvedConfig {
            active_path: PathBuf::from("/tmp/config.json"),
            file: ConfigFile {
                providers: vec![ProviderConfig {
                    id: "codex".to_string(),
                    enabled: Some(false),
                    source: Some("oauth".to_string()),
                }],
            },
        };

        assert!(!config.provider_enabled("codex"));
        assert_eq!(config.provider_source("codex"), Some("oauth"));
    }
}
