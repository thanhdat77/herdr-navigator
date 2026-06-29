use std::{
    collections::{HashMap, HashSet},
    fs,
    path::Path,
};

use crate::{
    config::Config,
    herdr::{herdr_json, notify_done, notify_error, run_herdr},
    integrations::{command, herdr_plus},
    matcher::match_score,
    model::{Entry, EntryAction, Source, WorkspaceKind, WorkspaceRef},
    paths::{canonical_str, herdr_plus_quick_actions_dir, home},
    sources::{collect_agents, collect_roots, collect_servers, collect_workspaces, collect_zoxide},
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
            push_unique(&mut entries, &mut seen, workspace_entries.clone());
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
            entries.extend(collect_agents(
                &workspace_entries,
                &self.config.agent_aliases,
            ));
        }
        if self.config.sources.servers {
            push_unique(&mut entries, &mut seen, collect_servers(&self.config));
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
        let query = Query::parse(&self.query);
        let agent_view = query.all_agents
            || (self.source_filter == Some(Source::Agent) && query.plain.is_empty());
        let use_agent_priority =
            agent_view && agent_sort(&self.config.picker.agent_sort) == "priority";
        let mut scored = Vec::new();
        for (idx, e) in self.entries.iter().enumerate() {
            if let Some(sf) = &self.source_filter {
                if &e.source != sf {
                    continue;
                }
            }
            if !query.filters_match(e) {
                continue;
            }
            let hay = e.haystack();
            let bonus = self.config.picker.source_bonus(&e.source)
                + query.score_bonus(e, use_agent_priority);
            if query.plain.is_empty() {
                scored.push((bonus, idx));
            } else if let Some(score) = match_score(&self.config.picker.engine, &hay, &query.plain)
            {
                scored.push((score + bonus, idx));
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
                .then_with(|| {
                    if agent_view {
                        idx_a.cmp(idx_b)
                    } else {
                        self.entries[*idx_a].title.cmp(&self.entries[*idx_b].title)
                    }
                })
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
            EntryAction::OpenServer { target } => (self.open_server(e, target), true, true),
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

    pub(crate) fn open_server(&self, e: &Entry, target: &str) -> Result<(), String> {
        if self.config.picker.reuse_existing {
            if let Some(ws) = self.matching_server_workspace(e) {
                return run_herdr(["workspace", "focus", &ws.id]);
            }
        }
        if !self.config.picker.create_missing {
            return Err("create_missing=false and no server workspace exists".into());
        }
        let label = format!("server: {}", e.title);
        let json = herdr_json([
            "workspace",
            "create",
            "--cwd",
            &e.path.display().to_string(),
            "--label",
            &label,
            "--focus",
        ])?;
        if let Some(workspace_id) = json
            .pointer("/result/workspace/workspace_id")
            .and_then(|v| v.as_str())
        {
            let _ = run_herdr(["tab", "rename", &format!("{workspace_id}:t1"), "remote"]);
        }
        if let Some(pane) = json
            .pointer("/result/root_pane/pane_id")
            .and_then(|v| v.as_str())
        {
            let command = format!("ssh {}", shell_quote(target));
            let _ = run_herdr(["pane", "run", pane, &command]);
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

    pub(crate) fn matching_server_workspace(&self, e: &Entry) -> Option<&WorkspaceRef> {
        let label = format!("server: {}", e.title).to_ascii_lowercase();
        self.path_to_workspaces
            .values()
            .flatten()
            .find(|ws| ws.kind == WorkspaceKind::Server && ws.label.to_ascii_lowercase() == label)
    }

    fn matching_dir_workspace_by_key(&self, key: &str) -> Option<&WorkspaceRef> {
        self.path_to_workspaces
            .get(key)?
            .iter()
            .find(|ws| ws.kind == WorkspaceKind::Dir)
    }
}

struct Query {
    plain: String,
    agent: Vec<String>,
    workspace_or_status: Vec<String>,
    path: Vec<String>,
    status: Vec<String>,
    all_agents: bool,
}

impl Query {
    fn parse(input: &str) -> Self {
        let mut query = Self {
            plain: String::new(),
            agent: vec![],
            workspace_or_status: vec![],
            path: vec![],
            status: vec![],
            all_agents: false,
        };
        let mut plain = Vec::new();
        for raw in input.split_whitespace() {
            let token = raw.to_lowercase();
            if let Some(rest) = token.strip_prefix('!') {
                push_token(&mut query.agent, rest);
            } else if let Some(rest) = token.strip_prefix('@') {
                if rest.is_empty() {
                    query.all_agents = true;
                } else {
                    push_token(&mut query.workspace_or_status, rest);
                }
            } else if let Some(rest) = token.strip_prefix('/') {
                push_token(&mut query.path, rest);
            } else if let Some(rest) = token.strip_prefix('#') {
                push_token(&mut query.status, rest);
            } else {
                plain.push(token);
            }
        }
        query.plain = plain.join(" ");
        query
    }

    fn filters_match(&self, entry: &Entry) -> bool {
        let agent_query = self.all_agents
            || !self.agent.is_empty()
            || !self.workspace_or_status.is_empty()
            || !self.status.is_empty();
        if agent_query && entry.source != Source::Agent {
            return false;
        }
        all_match(&self.agent, &agent_text(entry))
            && all_match_either(
                &self.workspace_or_status,
                &workspace_text(entry),
                &status_text(entry),
            )
            && all_match(&self.path, &entry.path.display().to_string())
            && all_match(&self.status, &status_text(entry))
    }

    fn score_bonus(&self, entry: &Entry, use_agent_priority: bool) -> i64 {
        if entry.source == Source::Agent && use_agent_priority {
            agent_status_bonus(entry)
        } else {
            0
        }
    }
}

fn push_token(tokens: &mut Vec<String>, value: &str) {
    if !value.is_empty() {
        tokens.push(value.into());
    }
}

fn all_match(tokens: &[String], haystack: &str) -> bool {
    let haystack = haystack.to_lowercase();
    tokens.iter().all(|token| haystack.contains(token))
}

fn all_match_either(tokens: &[String], left: &str, right: &str) -> bool {
    let left = left.to_lowercase();
    let right = right.to_lowercase();
    tokens
        .iter()
        .all(|token| left.contains(token) || right.contains(token))
}

fn agent_status_bonus(entry: &Entry) -> i64 {
    let status = status_text(entry);
    if status.contains("block") {
        10_000
    } else if status.contains("done") || status.contains("complete") {
        9_000
    } else if [
        "need",
        "attention",
        "review",
        "request",
        "question",
        "wait",
        "fail",
        "error",
    ]
    .iter()
    .any(|needle| status.contains(needle))
    {
        8_000
    } else {
        1_000
    }
}

fn status_text(entry: &Entry) -> String {
    entry
        .subtitle
        .split('·')
        .next()
        .unwrap_or(&entry.subtitle)
        .trim()
        .to_lowercase()
}

fn agent_text(entry: &Entry) -> String {
    entry
        .title
        .split('·')
        .next()
        .unwrap_or(&entry.title)
        .to_string()
}

fn workspace_text(entry: &Entry) -> String {
    format!(
        "{} {} {}",
        entry.workspace_id.as_deref().unwrap_or(""),
        entry.workspace_label.as_deref().unwrap_or(""),
        entry.title
    )
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

fn agent_sort(configured: &str) -> String {
    match configured.to_lowercase().as_str() {
        "priority" => "priority".into(),
        "spaces" => "spaces".into(),
        _ => herdr_agent_panel_sort(),
    }
}

fn herdr_agent_panel_sort() -> String {
    let path = std::env::var("XDG_CONFIG_HOME")
        .map(|xdg| Path::new(&xdg).join("herdr/config.toml"))
        .unwrap_or_else(|_| home().join(".config/herdr/config.toml"));
    fs::read_to_string(path)
        .ok()
        .and_then(|s| s.parse::<toml::Value>().ok())
        .and_then(|v| {
            v.get("ui")
                .and_then(|x| x.as_table())
                .and_then(|x| x.get("agent_panel_sort"))
                .or_else(|| v.get("agent_panel_sort"))
                .and_then(|x| x.as_str())
                .map(str::to_string)
        })
        .unwrap_or_else(|| "spaces".into())
}

fn push_unique(entries: &mut Vec<Entry>, seen: &mut HashSet<String>, incoming: Vec<Entry>) {
    for e in incoming {
        let key = match &e.action {
            EntryAction::FocusWorkspace { id } => format!("open:{id}"),
            EntryAction::OpenServer { target } => format!("server:{}:{target}", e.title),
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
            workspace_label: None,
            agent_target: None,
            project: None,
            action: EntryAction::FocusOrCreateDir,
            source_label: None,
            search_terms: vec![],
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

    fn agent_entry() -> Entry {
        agent_entry_with_status("idle")
    }

    fn agent_entry_with_status(status: &str) -> Entry {
        Entry {
            source: Source::Agent,
            title: "claude · Dotfiles · dotfiles".into(),
            subtitle: format!("{status} · wF:p2 · wF:t2"),
            path: PathBuf::from("/home/fenix/dotfiles"),
            workspace_id: Some("wF".into()),
            workspace_label: Some("Dotfiles".into()),
            agent_target: Some("term_1".into()),
            project: None,
            action: EntryAction::FocusAgent {
                target: "term_1".into(),
            },
            source_label: None,
            search_terms: vec!["main ai dot".into()],
        }
    }

    #[test]
    fn agent_token_filters_match_identity_parts() {
        let agent = agent_entry();

        assert!(Query::parse("!claude @dot /dot #idle").filters_match(&agent));
        assert!(Query::parse("@wF").filters_match(&agent));
        assert!(!Query::parse("!codex").filters_match(&agent));
        assert!(!Query::parse("!dotfiles").filters_match(&agent));
        assert!(!Query::parse("!claude").filters_match(&entry(Source::Project, "/tmp", "claude")));
    }

    #[test]
    fn agent_shortcut_shows_all_agents_and_priority_is_configurable() {
        let idle = agent_entry_with_status("idle");
        let blocked = agent_entry_with_status("blocking");
        let done = agent_entry_with_status("done");

        assert!(Query::parse("@").filters_match(&idle));
        assert!(Query::parse("@").filters_match(&blocked));
        assert!(Query::parse("@idle").filters_match(&idle));
        assert!(Query::parse("@Dotfiles").filters_match(&idle));
        assert!(agent_status_bonus(&blocked) > agent_status_bonus(&done));
        assert!(agent_status_bonus(&done) > agent_status_bonus(&idle));
        assert_eq!(agent_sort("priority"), "priority");
        assert_eq!(agent_sort("spaces"), "spaces");
    }

    #[test]
    fn agent_aliases_are_searchable_plain_text() {
        assert!(agent_entry().haystack().contains("main ai dot"));
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
