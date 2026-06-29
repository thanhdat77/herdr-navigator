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
    #[serde(default)]
    pub(crate) servers: ServersConfig,
    #[serde(default)]
    pub(crate) integrations: Vec<IntegrationConfig>,
    #[serde(default)]
    pub(crate) agent_aliases: Vec<AgentAliasConfig>,
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
    #[serde(default = "default_agent_sort")]
    pub(crate) agent_sort: String,
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
    pub(crate) servers: bool,
    #[serde(default = "yes")]
    pub(crate) herdr_plus_quick_actions: bool,
}

#[derive(Clone, Deserialize)]
pub(crate) struct ServersConfig {
    #[serde(default = "yes")]
    pub(crate) ssh_config: bool,
    #[serde(default)]
    pub(crate) entries: Vec<ServerEntryConfig>,
}

#[derive(Clone, Deserialize)]
pub(crate) struct ServerEntryConfig {
    pub(crate) name: String,
    pub(crate) host: Option<String>,
    pub(crate) user: Option<String>,
    pub(crate) target: Option<String>,
    #[serde(default)]
    pub(crate) tags: Vec<String>,
}

#[derive(Clone, Deserialize)]
pub(crate) struct IntegrationConfig {
    pub(crate) id: String,
    pub(crate) label: String,
    #[serde(default = "yes")]
    pub(crate) enabled: bool,
    pub(crate) collect: String,
    pub(crate) open: String,
    #[serde(default = "yes")]
    pub(crate) notify_success: bool,
    #[serde(default = "yes")]
    pub(crate) notify_error: bool,
}

#[derive(Clone, Deserialize)]
pub(crate) struct ThemeConfig {
    #[serde(default = "yes")]
    pub(crate) inherit_herdr: bool,
}

#[derive(Clone, Deserialize)]
pub(crate) struct AgentAliasConfig {
    pub(crate) alias: String,
    pub(crate) agent: Option<String>,
    pub(crate) workspace: Option<String>,
    pub(crate) path: Option<String>,
}

impl AgentAliasConfig {
    pub(crate) fn matches(&self, agent: &str, workspace: &str, path: &str) -> bool {
        opt_matches(self.agent.as_deref(), agent)
            && opt_matches(self.workspace.as_deref(), workspace)
            && opt_matches(self.path.as_deref(), path)
    }
}

fn opt_matches(needle: Option<&str>, haystack: &str) -> bool {
    needle
        .map(|value| haystack.to_lowercase().contains(&value.to_lowercase()))
        .unwrap_or(true)
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
    [
        "workspace",
        "project",
        "server",
        "zoxide",
        "root",
        "agent",
        "quick",
        "plugin",
    ]
    .into_iter()
    .map(String::from)
    .collect()
}
fn default_source_priority_boost() -> i64 {
    25
}
fn default_agent_sort() -> String {
    "herdr".into()
}

impl Default for PickerConfig {
    fn default() -> Self {
        Self {
            reuse_existing: true,
            create_missing: true,
            engine: default_engine(),
            source_order: default_source_order(),
            source_priority_boost: default_source_priority_boost(),
            agent_sort: default_agent_sort(),
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
            servers: true,
            herdr_plus_quick_actions: true,
        }
    }
}
impl Default for ServersConfig {
    fn default() -> Self {
        Self {
            ssh_config: true,
            entries: vec![],
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
            servers: ServersConfig::default(),
            integrations: vec![],
            agent_aliases: vec![],
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_command_integration_config() {
        let config: Config = toml::from_str(
            r#"
            [[integrations]]
            id = "bookmarks"
            label = "Bookmarks"
            collect = "bookmarks list --json"
            open = "bookmarks open {{id}}"
            notify_success = false
            "#,
        )
        .unwrap();

        assert_eq!(config.integrations.len(), 1);
        assert_eq!(config.integrations[0].id, "bookmarks");
        assert_eq!(config.integrations[0].label, "Bookmarks");
        assert!(!config.integrations[0].notify_success);
        assert!(config.integrations[0].notify_error);
    }

    #[test]
    fn parses_server_config() {
        let config: Config = toml::from_str(
            r#"
            [servers]
            ssh_config = false

            [[servers.entries]]
            name = "prod-api"
            host = "10.0.0.5"
            user = "ubuntu"
            tags = ["prod", "api"]
            "#,
        )
        .unwrap();

        assert!(!config.servers.ssh_config);
        assert_eq!(config.servers.entries.len(), 1);
        assert_eq!(config.servers.entries[0].name, "prod-api");
        assert_eq!(config.servers.entries[0].tags, ["prod", "api"]);
    }

    #[test]
    fn parses_agent_aliases() {
        let config: Config = toml::from_str(
            r#"
            [[agent_aliases]]
            alias = "main ai dot"
            agent = "claude"
            workspace = "Dotfiles"
            path = "dotfiles"
            "#,
        )
        .unwrap();

        assert_eq!(config.agent_aliases.len(), 1);
        assert!(config.agent_aliases[0].matches("claude", "Dotfiles", "/home/fenix/dotfiles"));
        assert!(!config.agent_aliases[0].matches("codex", "Dotfiles", "/home/fenix/dotfiles"));
    }
}
