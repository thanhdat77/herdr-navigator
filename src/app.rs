use std::{
    collections::{HashMap, HashSet},
    path::Path,
};

use crate::{
    config::Config,
    herdr::{herdr_json, run_herdr},
    matcher::match_score,
    model::{Entry, Source},
    paths::{canonical_str, herdr_plus_quick_actions_dir},
    sources::{
        bootstrap_project_tabs, collect_agents, collect_projects, collect_roots,
        collect_workspaces, collect_zoxide, quick_actions_entry,
    },
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
    pub(crate) path_to_workspace: HashMap<String, String>,
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
            path_to_workspace: HashMap::new(),
        }
    }

    pub(crate) fn refresh(&mut self) {
        let mut entries = Vec::new();
        let mut seen = HashSet::new();
        let (workspace_entries, path_to_workspace) = collect_workspaces();
        self.path_to_workspace = path_to_workspace;

        if self.config.sources.open_workspaces {
            push_unique(&mut entries, &mut seen, workspace_entries);
        }
        if self.config.sources.herdr_plus_projects {
            push_unique(&mut entries, &mut seen, collect_projects());
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
            entries.push(quick_actions_entry());
        }

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
        match e.source {
            Source::Agent => {
                let target = e.agent_target.as_ref().ok_or("agent has no target")?;
                run_herdr(["agent", "focus", target])
            }
            Source::Workspace => {
                let id = e.workspace_id.as_ref().ok_or("workspace has no id")?;
                run_herdr(["workspace", "focus", id])
            }
            Source::Project => self.open_project(e),
            Source::QuickAction => run_herdr([
                "plugin",
                "action",
                "invoke",
                "cloudmanic.herdr-plus.quick-actions",
            ]),
            Source::Zoxide | Source::Root => self.focus_or_create(&e.path, &e.title),
        }
    }

    pub(crate) fn open_project(&self, e: &Entry) -> Result<(), String> {
        if self.config.picker.reuse_existing {
            if let Some(id) = self.path_to_workspace.get(&e.key()) {
                return run_herdr(["workspace", "focus", id]);
            }
        }
        if !self.config.picker.create_missing {
            return Err("create_missing=false and no workspace exists".into());
        }
        let project = e.project.as_ref();
        let label = project.map(|p| p.name.as_str()).unwrap_or(e.title.as_str());
        let json = herdr_json([
            "workspace",
            "create",
            "--cwd",
            &e.path.display().to_string(),
            "--label",
            label,
            "--focus",
        ])?;
        if let Some(p) = project {
            bootstrap_project_tabs(p, &json, &e.path)?;
        }
        Ok(())
    }

    pub(crate) fn focus_or_create(&self, path: &Path, label: &str) -> Result<(), String> {
        let key = canonical_str(path).unwrap_or_else(|| path.display().to_string());
        if self.config.picker.reuse_existing {
            if let Some(id) = self.path_to_workspace.get(&key) {
                return run_herdr(["workspace", "focus", id]);
            }
        }
        if !self.config.picker.create_missing {
            return Err("create_missing=false and no workspace exists".into());
        }
        run_herdr([
            "workspace",
            "create",
            "--cwd",
            &path.display().to_string(),
            "--label",
            label,
            "--focus",
        ])
    }
}

fn push_unique(entries: &mut Vec<Entry>, seen: &mut HashSet<String>, incoming: Vec<Entry>) {
    for e in incoming {
        let key = format!("{}:{}", e.source.label(), e.key());
        if seen.insert(key) {
            entries.push(e);
        }
    }
}
