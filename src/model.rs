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
    pub(crate) workspace_id: Option<String>,
    pub(crate) workspace_label: Option<String>,
    pub(crate) agent_target: Option<String>,
    pub(crate) project: Option<Project>,
    pub(crate) action: EntryAction,
    pub(crate) source_label: Option<String>,
    pub(crate) search_terms: Vec<String>,
    /// Agent kind (`claude`, `opencode`, …) kept separate from `title`, which
    /// leads with the operator-chosen name when Herdr reports one. The `!`
    /// filter needs the kind regardless of how the entry is displayed.
    pub(crate) agent_kind: Option<String>,
    pub(crate) agent_task: Option<String>,
    /// Lazily resolved `key()`. `canonicalize` is a syscall and `key()` sits on
    /// hot paths (filtering, sorting, pin lookups, rendering), so resolve once.
    pub(crate) canonical: OnceLock<String>,
}

impl Entry {
    pub(crate) fn key(&self) -> &str {
        self.canonical.get_or_init(|| {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::{env, fs};

    fn dir_entry(path: PathBuf) -> Entry {
        Entry {
            source: Source::Zoxide,
            title: String::new(),
            subtitle: String::new(),
            path,
            workspace_id: None,
            workspace_label: None,
            agent_target: None,
            project: None,
            action: EntryAction::FocusOrCreateDir,
            source_label: None,
            search_terms: vec![],
            agent_kind: None,
            agent_task: None,
            canonical: OnceLock::new(),
        }
    }

    #[test]
    fn key_resolves_once_and_is_stable() {
        let dir = env::temp_dir().join(format!("herdr-nav-key-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let entry = dir_entry(dir.clone());

        let first = entry.key();
        let second = entry.key();
        assert_eq!(first, second);
        // Same backing allocation: the second call reused the cache instead of
        // canonicalizing again.
        assert!(std::ptr::eq(first, second));

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn key_still_resolves_symlinks() {
        let dir = env::temp_dir().join(format!("herdr-nav-link-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let target = dir.join("target");
        fs::create_dir_all(&target).unwrap();
        let link = dir.join("link");

        #[cfg(unix)]
        {
            let _ = fs::remove_file(&link);
            std::os::unix::fs::symlink(&target, &link).unwrap();
            assert_eq!(dir_entry(link).key(), dir_entry(target).key());
        }

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn key_falls_back_to_display_for_missing_paths() {
        let missing = PathBuf::from("/herdr-navigator/does/not/exist");
        assert_eq!(
            dir_entry(missing.clone()).key(),
            missing.display().to_string()
        );
    }
}
