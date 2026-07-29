use std::{path::PathBuf, sync::OnceLock};

use serde::{Deserialize, Serialize};

use crate::paths::canonical_str;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) enum Source {
    Workspace,
    Project,
    Zoxide,
    Root,
    Agent,
    Server,
    Session,
    QuickAction,
    Integration,
}

impl Source {
    pub(crate) fn label(&self) -> &'static str {
        match self {
            Source::Workspace => "open",
            Source::Project => "project",
            Source::Zoxide => "zoxide",
            Source::Root => "root",
            Source::Agent => "agent",
            Source::Server => "server",
            Source::Session => "session",
            Source::QuickAction => "quick",
            Source::Integration => "plugin",
        }
    }

    pub(crate) fn from_config(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "workspace" | "workspaces" | "open" | "open_workspaces" => Some(Source::Workspace),
            "project" | "projects" | "herdr_plus_projects" => Some(Source::Project),
            "zoxide" | "z" => Some(Source::Zoxide),
            "root" | "roots" | "scan" => Some(Source::Root),
            "agent" | "agents" => Some(Source::Agent),
            "server" | "servers" | "remote" | "remotes" | "ssh" => Some(Source::Server),
            "session" | "sessions" => Some(Source::Session),
            "quick" | "quick_action" | "quick_actions" | "herdr_plus_quick_actions" => {
                Some(Source::QuickAction)
            }
            "plugin" | "integration" | "integrations" => Some(Source::Integration),
            _ => None,
        }
    }

    pub(crate) fn all() -> [Source; 9] {
        [
            Source::Workspace,
            Source::Project,
            Source::Server,
            Source::Session,
            Source::Zoxide,
            Source::Root,
            Source::Agent,
            Source::QuickAction,
            Source::Integration,
        ]
    }
}

#[derive(Clone, Debug)]
pub(crate) enum EntryAction {
    FocusWorkspace {
        id: String,
    },
    FocusAgent {
        target: String,
    },
    OpenProject,
    OpenRemote {
        target: String,
    },
    AttachSession {
        name: String,
        remote: Option<String>,
    },
    InvokePluginAction {
        action: String,
    },
    FocusOrCreateDir,
    RunCommand {
        command: String,
        notify_success: bool,
        notify_error: bool,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum WorkspaceKind {
    Project,
    Dir,
    Unknown,
}

#[derive(Clone, Debug)]
pub(crate) struct WorkspaceRef {
    pub(crate) id: String,
    pub(crate) label: String,
    pub(crate) kind: WorkspaceKind,
    pub(crate) path: PathBuf,
    pub(crate) tab_count: i64,
    pub(crate) pane_count: i64,
}

#[derive(Clone, Debug)]
pub(crate) struct Entry {
    pub(crate) source: Source,
    pub(crate) title: String,
    pub(crate) subtitle: String,
    pub(crate) path: PathBuf,
    pub(crate) path_key: OnceLock<String>,
    pub(crate) workspace_id: Option<String>,
    pub(crate) workspace_label: Option<String>,
    pub(crate) agent_target: Option<String>,
    pub(crate) project: Option<Project>,
    pub(crate) action: EntryAction,
    pub(crate) source_label: Option<String>,
    pub(crate) search_terms: Vec<String>,
}

impl Entry {
    pub(crate) fn key(&self) -> &str {
        self.path_key.get_or_init(|| {
            canonical_str(&self.path).unwrap_or_else(|| self.path.display().to_string())
        })
    }

    pub(crate) fn source_name(&self) -> &str {
        self.source_label
            .as_deref()
            .unwrap_or_else(|| self.source.label())
    }

    pub(crate) fn haystack(&self) -> String {
        format!(
            "{} {} {} {} {} {}",
            self.source_name(),
            self.title,
            self.subtitle,
            self.workspace_label.as_deref().unwrap_or(""),
            self.path.display(),
            self.search_terms.join(" ")
        )
        .to_lowercase()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs,
        os::unix::fs::symlink,
        time::{SystemTime, UNIX_EPOCH},
    };

    fn test_path(name: &str) -> PathBuf {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("herdr-entry-{name}-{suffix}"))
    }

    fn entry(path: PathBuf) -> Entry {
        Entry {
            source: Source::Root,
            title: "cached".into(),
            subtitle: String::new(),
            path,
            path_key: OnceLock::new(),
            workspace_id: None,
            workspace_label: None,
            agent_target: None,
            project: None,
            action: EntryAction::FocusOrCreateDir,
            source_label: None,
            search_terms: vec![],
        }
    }

    #[test]
    fn path_key_is_cached_after_first_filesystem_lookup() {
        let path = test_path("key");
        fs::create_dir(&path).unwrap();
        let entry = entry(path.clone());

        let first = entry.key().to_string();
        fs::remove_dir(&path).unwrap();

        assert_eq!(entry.key(), first);
    }

    #[test]
    fn path_key_preserves_symlink_resolution() {
        let target = test_path("target");
        let link = test_path("link");
        fs::create_dir(&target).unwrap();
        symlink(&target, &link).unwrap();

        assert_eq!(entry(link.clone()).key(), entry(target.clone()).key());

        fs::remove_file(link).unwrap();
        fs::remove_dir(target).unwrap();
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct Project {
    pub(crate) name: String,
    #[serde(default)]
    pub(crate) description: String,
    pub(crate) working_dir: String,
    #[serde(default)]
    pub(crate) tabs: Vec<ProjectTab>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct ProjectTab {
    pub(crate) name: String,
    pub(crate) command: Option<String>,
    #[serde(default)]
    pub(crate) panes: Vec<ProjectPane>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct ProjectPane {
    pub(crate) command: Option<String>,
    pub(crate) split: Option<String>,
    pub(crate) label: Option<String>,
}

impl ProjectTab {
    pub(crate) fn effective_panes(&self) -> Vec<ProjectPane> {
        if self.panes.is_empty() {
            return vec![ProjectPane {
                command: self.command.clone(),
                split: None,
                label: None,
            }];
        }

        self.panes
            .iter()
            .cloned()
            .enumerate()
            .map(|(index, mut pane)| {
                pane.split = match (index, pane.split.as_deref()) {
                    (0, _) => None,
                    (_, Some(split)) if !split.is_empty() => Some(split.into()),
                    _ => Some("down".into()),
                };
                pane
            })
            .collect()
    }
}
