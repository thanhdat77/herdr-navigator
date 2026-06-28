use std::fs;

use serde::Deserialize;

use crate::{model::Source, paths::plugin_config_dir};

const DEFAULT_CONFIG: &str = include_str!("../examples/default-config.toml");

#[derive(Clone, Deserialize)]
pub(crate) struct Config {
    #[serde(default)]
    pub(crate) picker: PickerConfig,
    #[serde(default)]
    pub(crate) sources: SourcesConfig,
    #[serde(default)]
    pub(crate) theme: ThemeConfig,
    #[serde(default)]
    pub(crate) roots: Vec<RootConfig>,
}

#[derive(Clone, Deserialize)]
pub(crate) struct PickerConfig {
    #[serde(default = "yes")]
    pub(crate) reuse_existing: bool,
    #[serde(default = "yes")]
    pub(crate) create_missing: bool,
    #[serde(default = "default_engine")]
    pub(crate) engine: String,
    #[serde(default = "default_source_order")]
    pub(crate) source_order: Vec<String>,
    #[serde(default = "default_source_priority_boost")]
    pub(crate) source_priority_boost: i64,
}
#[derive(Clone, Deserialize)]
pub(crate) struct SourcesConfig {
    #[serde(default = "yes")]
    pub(crate) open_workspaces: bool,
    #[serde(default = "yes")]
    pub(crate) herdr_plus_projects: bool,
    #[serde(default = "yes")]
    pub(crate) zoxide: bool,
    #[serde(default = "yes")]
    pub(crate) roots: bool,
    #[serde(default = "yes")]
    pub(crate) agents: bool,
    #[serde(default = "yes")]
    pub(crate) herdr_plus_quick_actions: bool,
}
#[derive(Clone, Deserialize)]
pub(crate) struct ThemeConfig {
    #[serde(default = "yes")]
    pub(crate) inherit_herdr: bool,
}
#[derive(Clone, Deserialize)]
pub(crate) struct RootConfig {
    pub(crate) path: String,
    #[serde(default = "default_depth")]
    pub(crate) max_depth: usize,
}
fn yes() -> bool {
    true
}
fn default_depth() -> usize {
    3
}
fn default_engine() -> String {
    "nucleo".into()
}
fn default_source_order() -> Vec<String> {
    ["workspace", "project", "zoxide", "root", "agent", "quick"]
        .into_iter()
        .map(String::from)
        .collect()
}
fn default_source_priority_boost() -> i64 {
    25
}

impl Default for PickerConfig {
    fn default() -> Self {
        Self {
            reuse_existing: true,
            create_missing: true,
            engine: default_engine(),
            source_order: default_source_order(),
            source_priority_boost: default_source_priority_boost(),
        }
    }
}

impl PickerConfig {
    pub(crate) fn source_rank(&self, source: &Source) -> usize {
        self.source_order
            .iter()
            .filter_map(|name| Source::from_config(name))
            .position(|item| &item == source)
            .unwrap_or_else(|| Source::all().len())
    }

    pub(crate) fn source_bonus(&self, source: &Source) -> i64 {
        let rank = self.source_rank(source) as i64;
        let total = Source::all().len() as i64;
        (total - rank).max(0) * self.source_priority_boost
    }
}
impl Default for SourcesConfig {
    fn default() -> Self {
        Self {
            open_workspaces: true,
            herdr_plus_projects: true,
            zoxide: true,
            roots: true,
            agents: true,
            herdr_plus_quick_actions: true,
        }
    }
}
impl Default for ThemeConfig {
    fn default() -> Self {
        Self {
            inherit_herdr: true,
        }
    }
}
impl Default for Config {
    fn default() -> Self {
        Self {
            picker: PickerConfig::default(),
            sources: SourcesConfig::default(),
            theme: ThemeConfig::default(),
            roots: vec![
                RootConfig {
                    path: "~/workspace".into(),
                    max_depth: 3,
                },
                RootConfig {
                    path: "~/work".into(),
                    max_depth: 3,
                },
                RootConfig {
                    path: "~/projects".into(),
                    max_depth: 3,
                },
            ],
        }
    }
}

impl Config {
    pub(crate) fn load() -> Self {
        let dir = plugin_config_dir();
        let _ = fs::create_dir_all(&dir);
        let path = dir.join("config.toml");
        if !path.exists() {
            let _ = fs::write(&path, DEFAULT_CONFIG);
        }
        fs::read_to_string(path)
            .ok()
            .and_then(|s| toml::from_str(&s).ok())
            .unwrap_or_default()
    }
}
