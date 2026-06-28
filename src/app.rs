use std::{
    collections::{HashMap, HashSet},
    path::Path,
};

use crate::{
    config::Config,
    herdr::{herdr_json, notify_done, notify_error, run_herdr},
    integrations::{command, herdr_plus},
    matcher::match_score,
    model::{Entry, EntryAction, Source, WorkspaceKind, WorkspaceRef},
    paths::{canonical_str, herdr_plus_quick_actions_dir},
    sources::{collect_agents, collect_roots, collect_workspaces, collect_zoxide},
    theme::Theme,
};

pub(crate) struct App {
    pub(crate) config: Config,
    pub(crate) theme: Theme,
    pub(crate) entries: Vec<Entry>,
    pub(crate) filtered: Vec<usize>,
    pub(crate) selected: usize,
    pub(crate) query: String,
    pub(crate) source_filter: Option<Source>,
    pub(crate) preview: bool,
    pub(crate) path_to_workspaces: HashMap<String, Vec<WorkspaceRef>>,
}

impl App {
    pub(crate) fn new(config: Config, theme: Theme) -> Self {
        Self {
            config,
            theme,
            entries: vec![],
            filtered: vec![],
            selected: 0,
            query: String::new(),
            source_filter: None,
            preview: true,
            path_to_workspaces: HashMap::new(),
        }
    }

    pub(crate) fn refresh(&mut self) {
        let mut entries = Vec::new();
        let mut seen = HashSet::new();
        let (workspace_entries, path_to_workspaces) = collect_workspaces();
        self.path_to_workspaces = path_to_workspaces;

        if self.config.sources.open_workspaces {
            push_unique(&mut entries, &mut seen, workspace_entries);
        }
        if self.config.sources.herdr_plus_projects {
            push_unique(&mut entries, &mut seen, herdr_plus::collect_projects());
        }
        if self.config.sources.zoxide {
            push_unique(&mut entries, &mut seen, collect_zoxide());
        }
        if self.config.sources.roots {
            push_unique(&mut entries, &mut seen, collect_roots(&self.config));
        }
        if self.config.sources.agents {
            entries.extend(collect_agents());
        }
        if self.config.sources.herdr_plus_quick_actions && herdr_plus_quick_actions_dir().is_dir() {
            entries.push(herdr_plus::quick_actions_entry());
        }
        push_unique(
            &mut entries,
            &mut seen,
            command::collect(&self.config.integrations),
        );

        self.entries = entries;
        self.apply_filter();
    }

    pub(crate) fn apply_filter(&mut self) {
        let q = self.query.to_lowercase();
        let mut scored = Vec::new();
        for (idx, e) in self.entries.iter().enumerate() {
            if let Some(sf) = &self.source_filter {
                if &e.source != sf {
                    continue;
                }
            }
            let hay = e.haystack();
            let source_bonus = self.config.picker.source_bonus(&e.source);
            if q.is_empty() {
                scored.push((source_bonus, idx));
            } else if let Some(score) = match_score(&self.config.picker.engine, &hay, &q) {
                scored.push((score + source_bonus, idx));
            }
        }
        scored.sort_by(|(score_a, idx_a), (score_b, idx_b)| {
            score_b
                .cmp(score_a)
                .then_with(|| {
                    self.config
                        .picker
                        .source_rank(&self.entries[*idx_a].source)
                        .cmp(&self.config.picker.source_rank(&self.entries[*idx_b].source))
                })
                .then_with(|| self.entries[*idx_a].title.cmp(&self.entries[*idx_b].title))
        });
        self.filtered = scored.into_iter().map(|(_, idx)| idx).collect();
        self.selected = 0;
    }

    pub(crate) fn set_filter(&mut self, source: Option<Source>) {
        self.source_filter = if self.source_filter == source {
            None
        } else {
            source
        };
        self.selected = 0;
    }

    pub(crate) fn cycle_filter(&mut self) {
        self.source_filter = match &self.source_filter {
            None => Some(Source::Workspace),
            Some(cur) => {
                let all = Source::all();
                let pos = all.iter().position(|s| s == cur).unwrap_or(0);
                all.get(pos + 1).cloned()
            }
        };
        self.selected = 0;
        self.apply_filter();
    }

    pub(crate) fn next(&mut self) {
        if !self.filtered.is_empty() {
            self.selected = (self.selected + 1).min(self.filtered.len() - 1);
        }
    }
    pub(crate) fn prev(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }
    pub(crate) fn selected_entry(&self) -> Option<&Entry> {
        self.filtered
            .get(self.selected)
            .and_then(|idx| self.entries.get(*idx))
    }

    pub(crate) fn open_selected(&self) -> Result<(), String> {
        let e = self.selected_entry().ok_or("nothing selected")?;
        let (result, notify_success, notify_failure) = match &e.action {
            EntryAction::FocusAgent { target } => {
                (run_herdr(["agent", "focus", target]), true, true)
            }
            EntryAction::FocusWorkspace { id } => {
                (run_herdr(["workspace", "focus", id]), true, true)
            }
            EntryAction::OpenProject => (self.open_project(e), true, true),
            EntryAction::InvokePluginAction { action } => (
                run_herdr(["plugin", "action", "invoke", action]),
                true,
                true,
            ),
            EntryAction::FocusOrCreateDir => {
                (self.focus_or_create_dir(&e.path, &e.title), true, true)
            }
            EntryAction::RunCommand {
                command,
                notify_success,
                notify_error,
            } => (
                command::run_command(command),
                *notify_success,
                *notify_error,
            ),
        };

        match result {
            Ok(()) => {
                if notify_success {
                    notify_done(&format!("Opened {}", e.title));
                }
                Ok(())
            }
            Err(err) => {
                if notify_failure {
                    notify_error(&format!("Failed {}: {}", e.title, err.trim()));
                }
                Err(err)
            }
        }
    }

    pub(crate) fn open_project(&self, e: &Entry) -> Result<(), String> {
        if self.config.picker.reuse_existing {
            if let Some(ws) = self.matching_project_workspace(e) {
                return run_herdr(["workspace", "focus", &ws.id]);
            }
        }
        if !self.config.picker.create_missing {
            return Err("create_missing=false and no workspace exists".into());
        }
        let project = e.project.as_ref();
        let label = project
            .map(|p| format!("project: {}", p.name))
            .unwrap_or_else(|| format!("project: {}", e.title));
        let json = herdr_json([
            "workspace",
            "create",
            "--cwd",
            &e.path.display().to_string(),
            "--label",
            &label,
            "--focus",
        ])?;
        if let Some(p) = project {
            herdr_plus::bootstrap_project_tabs(p, &json, &e.path)?;
        }
        Ok(())
    }

    pub(crate) fn focus_or_create_dir(&self, path: &Path, label: &str) -> Result<(), String> {
        let key = canonical_str(path).unwrap_or_else(|| path.display().to_string());
        if self.config.picker.reuse_existing {
            if let Some(ws) = self.matching_dir_workspace_by_key(&key) {
                return run_herdr(["workspace", "focus", &ws.id]);
            }
        }
        if !self.config.picker.create_missing {
            return Err("create_missing=false and no workspace exists".into());
        }
        let label = format!("dir: {label}");
        run_herdr([
            "workspace",
            "create",
            "--cwd",
            &path.display().to_string(),
            "--label",
            &label,
            "--focus",
        ])
    }

    pub(crate) fn workspaces_for_entry(&self, e: &Entry) -> &[WorkspaceRef] {
        self.path_to_workspaces
            .get(&e.key())
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    pub(crate) fn matching_project_workspace(&self, e: &Entry) -> Option<&WorkspaceRef> {
        self.workspaces_for_entry(e)
            .iter()
            .find(|ws| ws.kind == WorkspaceKind::Project)
    }

    pub(crate) fn matching_dir_workspace(&self, e: &Entry) -> Option<&WorkspaceRef> {
        self.matching_dir_workspace_by_key(&e.key())
    }

    fn matching_dir_workspace_by_key(&self, key: &str) -> Option<&WorkspaceRef> {
        self.path_to_workspaces
            .get(key)?
            .iter()
            .find(|ws| ws.kind == WorkspaceKind::Dir)
    }
}

fn push_unique(entries: &mut Vec<Entry>, seen: &mut HashSet<String>, incoming: Vec<Entry>) {
    for e in incoming {
        let key = match &e.action {
            EntryAction::FocusWorkspace { id } => format!("open:{id}"),
            EntryAction::RunCommand { command, .. } => format!("{}:{command}", e.source_name()),
            _ => format!("{}:{}", e.source_name(), e.key()),
        };
        if seen.insert(key) {
            entries.push(e);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::{config::Config, model::Project, theme::Theme};

    fn entry(source: Source, path: &str, title: &str) -> Entry {
        Entry {
            source,
            title: title.into(),
            subtitle: String::new(),
            path: PathBuf::from(path),
            workspace_id: None,
            agent_target: None,
            project: None,
            action: EntryAction::FocusOrCreateDir,
            source_label: None,
        }
    }

    fn workspace(id: &str, label: &str, kind: WorkspaceKind, path: &str) -> WorkspaceRef {
        WorkspaceRef {
            id: id.into(),
            label: label.into(),
            kind,
            path: PathBuf::from(path),
            tab_count: 1,
            pane_count: 1,
        }
    }

    #[test]
    fn source_specific_reuse_distinguishes_same_path_workspaces() {
        let mut app = App::new(Config::default(), Theme::load(false));
        app.path_to_workspaces.insert(
            "/tmp".into(),
            vec![
                workspace("w1", "project: tmp", WorkspaceKind::Project, "/tmp"),
                workspace("w2", "dir: tmp", WorkspaceKind::Dir, "/tmp"),
            ],
        );

        let mut project = entry(Source::Project, "/tmp", "tmp");
        project.project = Some(Project {
            name: "tmp".into(),
            description: String::new(),
            working_dir: "/tmp".into(),
            tabs: vec![],
        });
        let dir = entry(Source::Zoxide, "/tmp", "tmp");

        assert_eq!(app.matching_project_workspace(&project).unwrap().id, "w1");
        assert_eq!(app.matching_dir_workspace(&dir).unwrap().id, "w2");
    }

    #[test]
    fn workspace_rows_are_not_deduped_by_path() {
        let mut entries = Vec::new();
        let mut seen = HashSet::new();
        push_unique(
            &mut entries,
            &mut seen,
            vec![
                Entry {
                    workspace_id: Some("w1".into()),
                    action: EntryAction::FocusWorkspace { id: "w1".into() },
                    ..entry(Source::Workspace, "/tmp", "project: tmp")
                },
                Entry {
                    workspace_id: Some("w2".into()),
                    action: EntryAction::FocusWorkspace { id: "w2".into() },
                    ..entry(Source::Workspace, "/tmp", "dir: tmp")
                },
            ],
        );

        assert_eq!(entries.len(), 2);
    }
}
