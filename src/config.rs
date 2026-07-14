use std::{collections::HashMap, fs};

use serde::Deserialize;

use crate::{model::Source, paths::plugin_config_dir};

const DEFAULT_CONFIG: &str = include_str!("../examples/default-config.toml");

#[derive(Clone, Deserialize)]
pub(crate) struct Config {
    #[serde(default)]
    pub(crate) picker: PickerConfig,
    #[serde(default)]
    pub(crate) jump_back: JumpBackConfig,
    #[serde(default)]
    pub(crate) sources: SourcesConfig,
    #[serde(default)]
    pub(crate) theme: ThemeConfig,
    #[serde(default)]
    pub(crate) roots: Vec<RootConfig>,
    #[serde(default)]
    pub(crate) sessions: SessionsConfig,
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
    #[serde(default = "yes")]
    pub(crate) preview: bool,
    #[serde(default)]
    pub(crate) vim_mode: bool,
    #[serde(default)]
    pub(crate) vim_filter_search: bool,
    #[serde(default)]
    pub(crate) filter_keys: HashMap<String, String>,
}

#[derive(Clone, Deserialize)]
pub(crate) struct JumpBackConfig {
    #[serde(default = "yes")]
    pub(crate) enabled: bool,
    #[serde(default = "yes")]
    pub(crate) pin_previous: bool,
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
    pub(crate) sessions: bool,
    #[serde(default = "yes")]
    pub(crate) herdr_plus_quick_actions: bool,
}

#[derive(Clone, Deserialize)]
pub(crate) struct SessionsConfig {
    #[serde(default = "yes")]
    pub(crate) local: bool,
    #[serde(default)]
    pub(crate) entries: Vec<SessionEntryConfig>,
}

#[derive(Clone, Deserialize)]
pub(crate) struct SessionEntryConfig {
    pub(crate) name: String,
    pub(crate) remote: Option<String>,
    pub(crate) session: Option<String>,
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
        "agent",
        "project",
        "session",
        "zoxide",
        "root",
        "server",
        "quick",
        "plugin",
    ]
    .into_iter()
    .map(String::from)
    .collect()
}
fn default_source_priority_boost() -> i64 {
    5
}
fn default_agent_sort() -> String {
    "herdr".into()
}
fn default_filter_key(source: &Source) -> Option<char> {
    match source {
        Source::Agent => Some('a'),
        Source::Server => Some('s'),
        Source::QuickAction => Some('q'),
        Source::Workspace => Some('w'),
        Source::Project => Some('p'),
        Source::Zoxide => Some('z'),
        Source::Root => Some('r'),
        Source::Session => Some('l'),
        Source::Integration => None,
    }
}

fn default_filter_keys() -> Vec<(Source, char)> {
    Source::all()
        .into_iter()
        .filter_map(|source| default_filter_key(&source).map(|key| (source, key)))
        .collect()
}

fn parse_filter_key(value: &str) -> Option<char> {
    let key = value
        .trim()
        .to_ascii_lowercase()
        .replace("ctrl+", "")
        .replace("ctrl-", "")
        .replace(['^', '⌃'], "");
    let mut chars = key.chars();
    let ch = chars.next()?;
    (chars.next().is_none() && ch.is_ascii_alphanumeric()).then_some(ch)
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
            preview: true,
            vim_mode: false,
            vim_filter_search: false,
            filter_keys: HashMap::new(),
        }
    }
}

impl PickerConfig {
    pub(crate) fn filter_key(&self, source: &Source) -> Option<char> {
        let key = self
            .custom_filter_keys()
            .into_iter()
            .find_map(|(custom_source, key)| (custom_source == *source).then_some(key))
            .or_else(|| default_filter_key(source))?;
        (self.filter_source_for_key(key).as_ref() == Some(source)).then_some(key)
    }

    pub(crate) fn filter_source_for_key(&self, key: char) -> Option<Source> {
        let key = key.to_ascii_lowercase();
        let custom = self.custom_filter_keys();
        custom
            .iter()
            .find(|(_, custom_key)| *custom_key == key)
            .map(|(source, _)| source.clone())
            .or_else(|| {
                default_filter_keys()
                    .into_iter()
                    .find_map(|(source, default_key)| {
                        (default_key == key && !custom.iter().any(|(s, _)| s == &source))
                            .then_some(source)
                    })
            })
    }

    fn custom_filter_keys(&self) -> Vec<(Source, char)> {
        self.filter_keys
            .iter()
            .filter_map(|(source, key)| {
                Some((Source::from_config(source)?, parse_filter_key(key)?))
            })
            .collect()
    }

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
impl Default for JumpBackConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            pin_previous: true,
        }
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
            sessions: true,
            herdr_plus_quick_actions: true,
        }
    }
}
impl Default for SessionsConfig {
    fn default() -> Self {
        Self {
            local: true,
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
            jump_back: JumpBackConfig::default(),
            sources: SourcesConfig::default(),
            theme: ThemeConfig::default(),
            sessions: SessionsConfig::default(),
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
    fn default_source_order_prioritizes_open_workspaces_then_agents() {
        let picker = PickerConfig::default();

        assert_eq!(picker.source_rank(&Source::Workspace), 0);
        assert_eq!(picker.source_rank(&Source::Agent), 1);
        assert_eq!(picker.source_rank(&Source::Session), 3);
        assert_eq!(picker.source_rank(&Source::Root), 5);
        assert_eq!(picker.source_rank(&Source::Server), 6);
        assert!(picker.source_bonus(&Source::Root) > picker.source_bonus(&Source::Server));
    }

    #[test]
    fn jump_back_defaults_to_enabled_and_pinned() {
        let config = Config::default();

        assert!(config.jump_back.enabled);
        assert!(config.jump_back.pin_previous);
    }

    #[test]
    fn parses_jump_back_config() {
        let config: Config = toml::from_str(
            r#"
            [jump_back]
            enabled = false
            pin_previous = false
            "#,
        )
        .unwrap();

        assert!(!config.jump_back.enabled);
        assert!(!config.jump_back.pin_previous);
    }

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
    fn custom_filter_key_overrides_default_source_key() {
        let config: Config = toml::from_str(
            r#"
            [picker.filter_keys]
            server = "ctrl-g"
            "#,
        )
        .unwrap();

        assert_eq!(
            config.picker.filter_source_for_key('g'),
            Some(Source::Server)
        );
        assert_eq!(config.picker.filter_source_for_key('s'), None);
        assert_eq!(
            config.picker.filter_source_for_key('a'),
            Some(Source::Agent)
        );
        assert_eq!(config.picker.filter_key(&Source::Server), Some('g'));
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

    #[test]
    fn parses_builtin_session_config() {
        let config: Config = toml::from_str(
            r#"
            [sessions]
            local = false

            [[sessions.entries]]
            name = "prod"
            remote = "prod-host"
            session = "default"
            tags = ["api"]
            "#,
        )
        .unwrap();

        assert!(config.sources.sessions);
        assert!(!config.sessions.local);
        assert_eq!(config.sessions.entries[0].name, "prod");
        assert_eq!(
            config.sessions.entries[0].remote.as_deref(),
            Some("prod-host")
        );
        assert_eq!(
            config.sessions.entries[0].session.as_deref(),
            Some("default")
        );
    }
}
