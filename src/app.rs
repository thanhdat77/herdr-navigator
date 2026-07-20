use std::{
    collections::{HashMap, HashSet},
    env, fs,
    path::Path,
};

use crate::{
    config::Config,
    herdr::{herdr_json, notify_done, notify_error, run_herdr},
    integrations::{command, herdr_plus, sessions},
    matcher::match_score,
    model::{Entry, EntryAction, Source, WorkspaceKind, WorkspaceRef},
    paths::{canonical_str, herdr_plus_quick_actions_dir, home, plugin_config_dir},
    sources::{collect_agents, collect_roots, collect_workspaces, collect_zoxide},
    theme::Theme,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum InputMode {
    Normal,
    Search,
    Help,
}

pub(crate) struct App {
    pub(crate) config: Config,
    pub(crate) theme: Theme,
    pub(crate) entries: Vec<Entry>,
    pub(crate) filtered: Vec<usize>,
    pub(crate) filtered_scores: Vec<i64>,
    pub(crate) selected: usize,
    pub(crate) query: String,
    pub(crate) input_mode: InputMode,
    pub(crate) source_filter: Option<Source>,
    pub(crate) preview: bool,
    pub(crate) path_to_workspaces: HashMap<String, Vec<WorkspaceRef>>,
    pub(crate) previous_workspace_id: Option<String>,
    pub(crate) pinned_entries: HashSet<String>,
    pub(crate) spinner_tick: u32,
    pub(crate) update_available: Option<String>,
}

impl App {
    pub(crate) fn new(config: Config, theme: Theme) -> Self {
        let preview = config.picker.preview;
        Self {
            config,
            theme,
            entries: vec![],
            filtered: vec![],
            filtered_scores: vec![],
            selected: 0,
            query: String::new(),
            input_mode: InputMode::Normal,
            source_filter: None,
            preview,
            path_to_workspaces: HashMap::new(),
            previous_workspace_id: None,
            pinned_entries: HashSet::new(),
            spinner_tick: 0,
            update_available: None,
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
        if self.config.sources.servers {
            push_unique(
                &mut entries,
                &mut seen,
                sessions::collect_remotes(&self.config),
            );
        }
        if self.config.sources.sessions {
            push_unique(
                &mut entries,
                &mut seen,
                sessions::collect_sessions(&self.config),
            );
        }
        if self.config.sources.agents {
            entries.extend(collect_agents(
                &workspace_entries,
                &self.config.agent_aliases,
            ));
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
        self.pinned_entries =
            read_pinned_entries(&plugin_config_dir().join(PINNED_ENTRIES_STATE_FILE))
                .unwrap_or_default();
        self.previous_workspace_id =
            if self.config.jump_back.enabled && self.config.jump_back.pin_previous {
                read_previous_workspace().ok()
            } else {
                None
            };
        self.apply_filter();
    }

    pub(crate) fn apply_filter(&mut self) {
        let query = Query::parse(&self.query);
        let empty_query = query.plain.is_empty();
        let agent_view =
            query.all_agents || (self.source_filter == Some(Source::Agent) && empty_query);
        let use_agent_priority = empty_query
            && (agent_view || self.source_filter.is_none())
            && agent_sort(&self.config.picker.agent_sort) == "priority";
        let pin_previous = self.config.jump_back.enabled
            && self.config.jump_back.pin_previous
            && self.query.trim().is_empty()
            && self.source_filter.is_none();
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
            let user_pinned_a = self.is_pinned(&self.entries[*idx_a]);
            let user_pinned_b = self.is_pinned(&self.entries[*idx_b]);
            let previous_pinned_a = pin_previous
                && self.entries[*idx_a].source == Source::Workspace
                && self.entries[*idx_a].workspace_id.as_deref()
                    == self.previous_workspace_id.as_deref();
            let previous_pinned_b = pin_previous
                && self.entries[*idx_b].source == Source::Workspace
                && self.entries[*idx_b].workspace_id.as_deref()
                    == self.previous_workspace_id.as_deref();
            previous_pinned_b
                .cmp(&previous_pinned_a)
                .then_with(|| user_pinned_b.cmp(&user_pinned_a))
                .then_with(|| score_b.cmp(score_a))
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
        let (scores, filtered): (Vec<_>, Vec<_>) = scored.into_iter().unzip();
        self.filtered = filtered;
        self.filtered_scores = scores;
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

    pub(crate) fn is_pinned(&self, entry: &Entry) -> bool {
        self.pinned_entries.contains(&pin_key(entry))
    }

    pub(crate) fn toggle_selected_pin(&mut self) -> Result<(), String> {
        let key = self
            .selected_entry()
            .map(pin_key)
            .ok_or("nothing selected")?;
        let mut pinned = self.pinned_entries.clone();
        if !pinned.remove(&key) {
            pinned.insert(key);
        }
        save_pinned_entries(
            &plugin_config_dir().join(PINNED_ENTRIES_STATE_FILE),
            &pinned,
        )?;
        self.pinned_entries = pinned;
        self.apply_filter();
        Ok(())
    }

    pub(crate) fn directory_template_for_selected(&self) -> Option<&str> {
        let template = self
            .config
            .picker
            .directory_template
            .as_deref()
            .map(str::trim)
            .filter(|template| !template.is_empty())?;
        matches!(
            &self.selected_entry()?.action,
            EntryAction::FocusOrCreateDir
        )
        .then_some(template)
    }

    pub(crate) fn open_selected(&self, use_directory_template: bool) -> Result<(), String> {
        let e = self.selected_entry().ok_or("nothing selected")?;
        let tracks_workspace_transition = self.config.jump_back.enabled
            && matches!(
                &e.action,
                EntryAction::FocusAgent { .. }
                    | EntryAction::FocusWorkspace { .. }
                    | EntryAction::OpenProject
                    | EntryAction::FocusOrCreateDir
            );
        let origin_workspace = if tracks_workspace_transition {
            launch_workspace_id().or_else(|| current_workspace_id().ok())
        } else {
            None
        };
        let (result, notify_success, notify_failure) = match &e.action {
            EntryAction::FocusAgent { target } => {
                (run_herdr(["agent", "focus", target]), true, true)
            }
            EntryAction::FocusWorkspace { id } => {
                (run_herdr(["workspace", "focus", id]), true, true)
            }
            EntryAction::OpenProject => (self.open_project(e), true, true),
            EntryAction::OpenRemote { target } => (sessions::open_remote(target), false, true),
            EntryAction::AttachSession { name, .. } => {
                (sessions::attach_session(name), false, true)
            }
            EntryAction::InvokePluginAction { action } => (
                run_herdr(["plugin", "action", "invoke", action]),
                true,
                true,
            ),
            EntryAction::FocusOrCreateDir => (
                self.focus_or_create_dir(&e.path, &e.title, use_directory_template),
                true,
                true,
            ),
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
                if tracks_workspace_transition {
                    let destination = current_workspace_id().ok();
                    if let Some(previous) = previous_workspace_to_record(
                        origin_workspace.as_deref(),
                        destination.as_deref(),
                    ) {
                        let _ = save_previous_workspace(previous);
                    }
                }
                if notify_success {
                    notify_done(&format!("Opened {}", e.title), &self.config.notifications);
                }
                Ok(())
            }
            Err(err) => {
                if notify_failure {
                    notify_error(
                        &format!("Failed {}: {}", e.title, err.trim()),
                        &self.config.notifications,
                    );
                }
                Err(err)
            }
        }
    }

    pub(crate) fn close_selected_workspace(&mut self) -> Result<(), String> {
        let (id, title) = {
            let e = self.selected_entry().ok_or("nothing selected")?;
            let id = self
                .workspace_to_close(e)
                .ok_or("no open workspace for selected item")?;
            (id, e.title.clone())
        };
        if let Some(err) = close_current_workspace_error(&id, launch_workspace_id().as_deref()) {
            return Err(err);
        }
        run_herdr(["workspace", "close", &id])?;
        notify_done(&format!("Closed {title}"), &self.config.notifications);
        self.refresh();
        Ok(())
    }

    fn workspace_to_close(&self, e: &Entry) -> Option<String> {
        match e.source {
            Source::Workspace | Source::Agent => e.workspace_id.clone(),
            Source::Project => self.matching_project_workspace(e).map(|ws| ws.id.clone()),
            Source::Zoxide | Source::Root => self.matching_dir_workspace(e).map(|ws| ws.id.clone()),
            Source::Server | Source::Session | Source::QuickAction | Source::Integration => None,
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

    pub(crate) fn focus_or_create_dir(
        &self,
        path: &Path,
        label: &str,
        use_directory_template: bool,
    ) -> Result<(), String> {
        let key = canonical_str(path).unwrap_or_else(|| path.display().to_string());
        if use_directory_template {
            let template_name = self
                .config
                .picker
                .directory_template
                .as_deref()
                .ok_or("no directory_template configured")?;
            let template = herdr_plus::load_project_template(template_name)?;
            if let Some(workspace_id) = self
                .matching_template_workspace_by_key(&key)
                .map(|workspace| workspace.id.clone())
            {
                run_herdr(["workspace", "focus", &workspace_id])?;
                return herdr_plus::append_project_tabs(&template, &workspace_id, path);
            }
            let json = herdr_json([
                "workspace",
                "create",
                "--cwd",
                &path.display().to_string(),
                "--label",
                &format!("dir: {label}"),
                "--focus",
            ])?;
            return herdr_plus::bootstrap_project_tabs(&template, &json, path);
        }

        if self.config.picker.reuse_existing {
            if let Some(ws) = self.matching_dir_workspace_by_key(&key) {
                return run_herdr(["workspace", "focus", &ws.id]);
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
            &format!("dir: {label}"),
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

    fn matching_template_workspace_by_key(&self, key: &str) -> Option<&WorkspaceRef> {
        let workspaces = self.path_to_workspaces.get(key)?;
        workspaces
            .iter()
            .find(|workspace| workspace.kind == WorkspaceKind::Dir)
            .or_else(|| workspaces.first())
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
    if ["block", "fail", "error"]
        .iter()
        .any(|needle| status.contains(needle))
    {
        4
    } else if ["need", "attention", "review", "request", "question", "wait"]
        .iter()
        .any(|needle| status.contains(needle))
    {
        3
    } else if status.contains("done") || status.contains("complete") {
        2
    } else if status.contains("work") || status.contains("run") {
        1
    } else {
        0
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

const JUMP_BACK_STATE_FILE: &str = "jump-back-workspace";
const PINNED_ENTRIES_STATE_FILE: &str = "pinned-entries.json";

pub(crate) fn jump_back(config: &Config) -> Result<String, String> {
    if !config.jump_back.enabled {
        return Err("jump back is disabled in config".into());
    }
    let json = herdr_json(["workspace", "list"])?;
    let current = focused_workspace_id(&json)
        .ok_or("can't determine the current workspace")?
        .to_string();
    let previous = read_previous_workspace()?;
    if previous == current {
        return Err("no previous workspace yet".into());
    }
    let Some(label) = workspace_label(&json, &previous).map(str::to_string) else {
        let _ = fs::remove_file(plugin_config_dir().join(JUMP_BACK_STATE_FILE));
        return Err("previous workspace no longer exists".into());
    };

    run_herdr(["workspace", "focus", &previous])?;
    save_previous_workspace(&current)?;
    Ok(label)
}

fn focused_workspace_id(json: &serde_json::Value) -> Option<&str> {
    json.pointer("/result/workspaces")
        .and_then(|v| v.as_array())?
        .iter()
        .find(|workspace| workspace.get("focused").and_then(|v| v.as_bool()) == Some(true))?
        .get("workspace_id")?
        .as_str()
}

fn workspace_label<'a>(json: &'a serde_json::Value, id: &str) -> Option<&'a str> {
    json.pointer("/result/workspaces")
        .and_then(|v| v.as_array())?
        .iter()
        .find(|workspace| workspace.get("workspace_id").and_then(|v| v.as_str()) == Some(id))?
        .get("label")?
        .as_str()
}

fn current_workspace_id() -> Result<String, String> {
    let json = herdr_json(["workspace", "list"])?;
    focused_workspace_id(&json)
        .map(str::to_string)
        .ok_or_else(|| "can't determine the current workspace".into())
}

fn previous_workspace_to_record<'a>(
    origin: Option<&'a str>,
    destination: Option<&str>,
) -> Option<&'a str> {
    match (origin, destination) {
        (Some(origin), Some(destination)) if origin != destination => Some(origin),
        _ => None,
    }
}

fn read_previous_workspace() -> Result<String, String> {
    let path = plugin_config_dir().join(JUMP_BACK_STATE_FILE);
    let value = fs::read_to_string(path).map_err(|error| {
        if error.kind() == std::io::ErrorKind::NotFound {
            "no previous workspace yet".into()
        } else {
            format!("failed to read jump-back state: {error}")
        }
    })?;
    let value = value.trim();
    if value.is_empty() {
        Err("no previous workspace yet".into())
    } else {
        Ok(value.into())
    }
}

fn save_previous_workspace(id: &str) -> Result<(), String> {
    let dir = plugin_config_dir();
    fs::create_dir_all(&dir).map_err(|e| format!("failed to create plugin config: {e}"))?;
    fs::write(dir.join(JUMP_BACK_STATE_FILE), id)
        .map_err(|e| format!("failed to save jump-back state: {e}"))
}

fn read_pinned_entries(path: &Path) -> Result<HashSet<String>, String> {
    let value =
        fs::read_to_string(path).map_err(|e| format!("failed to read pinned entries: {e}"))?;
    serde_json::from_str::<Vec<String>>(&value)
        .map(|entries| entries.into_iter().collect())
        .map_err(|e| format!("failed to parse pinned entries: {e}"))
}

fn save_pinned_entries(path: &Path, entries: &HashSet<String>) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("failed to create plugin config: {e}"))?;
    }
    let mut entries = entries.iter().collect::<Vec<_>>();
    entries.sort_unstable();
    let value = serde_json::to_string_pretty(&entries)
        .map_err(|e| format!("failed to encode pinned entries: {e}"))?;
    fs::write(path, value).map_err(|e| format!("failed to save pinned entries: {e}"))
}

fn launch_workspace_id() -> Option<String> {
    env::var("HERDR_PLUGIN_CONTEXT_JSON")
        .ok()
        .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
        .and_then(|v| v.get("workspace_id")?.as_str().map(str::to_string))
        .filter(|s| !s.is_empty())
}

fn close_current_workspace_error(id: &str, current: Option<&str>) -> Option<String> {
    (current == Some(id))
        .then(|| "can't close the workspace that owns this picker; switch away first".into())
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

fn pin_key(entry: &Entry) -> String {
    match &entry.action {
        EntryAction::FocusWorkspace { id } => format!("workspace:{id}"),
        EntryAction::FocusAgent { target } => format!("agent:{target}"),
        EntryAction::OpenProject => format!("project:{}", entry.key()),
        EntryAction::OpenRemote { target } => format!("remote:{target}"),
        EntryAction::AttachSession { name, remote } => {
            format!("session:{}:{name}", remote.as_deref().unwrap_or("local"))
        }
        EntryAction::InvokePluginAction { action } => {
            format!("plugin:{}:{action}", entry.source_name())
        }
        EntryAction::FocusOrCreateDir => format!("{}:{}", entry.source_name(), entry.key()),
        EntryAction::RunCommand { command, .. } => {
            format!("{}:{command}", entry.source_name())
        }
    }
}

fn push_unique(entries: &mut Vec<Entry>, seen: &mut HashSet<String>, incoming: Vec<Entry>) {
    for e in incoming {
        let key = match &e.action {
            EntryAction::FocusWorkspace { id } => format!("open:{id}"),
            EntryAction::OpenRemote { target } => format!("remote:{target}"),
            EntryAction::AttachSession { name, remote } => {
                format!("session:{}:{name}", remote.as_deref().unwrap_or("local"))
            }
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
    use std::{
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

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
        let working = agent_entry_with_status("working");
        let blocked = agent_entry_with_status("blocking");
        let attention = agent_entry_with_status("needs attention");
        let done = agent_entry_with_status("done");

        assert!(Query::parse("@").filters_match(&idle));
        assert!(Query::parse("@").filters_match(&blocked));
        assert!(Query::parse("@idle").filters_match(&idle));
        assert!(Query::parse("@Dotfiles").filters_match(&idle));
        assert_eq!(agent_status_bonus(&blocked), 4);
        assert_eq!(agent_status_bonus(&attention), 3);
        assert_eq!(agent_status_bonus(&done), 2);
        assert_eq!(agent_status_bonus(&working), 1);
        assert_eq!(agent_status_bonus(&idle), 0);
        assert_eq!(agent_sort("priority"), "priority");
        assert_eq!(agent_sort("spaces"), "spaces");
    }

    #[test]
    fn agent_aliases_are_searchable_plain_text() {
        assert!(agent_entry().haystack().contains("main ai dot"));
    }

    #[test]
    fn default_empty_picker_prioritizes_agent_status() {
        let mut app = App::new(Config::default(), Theme::load(false));
        app.config.picker.agent_sort = "priority".into();
        app.entries = vec![
            agent_entry_with_status("idle"),
            agent_entry_with_status("done"),
        ];
        app.apply_filter();

        let first = &app.entries[app.filtered[0]];
        assert!(first.subtitle.starts_with("done"));
    }

    #[test]
    fn previous_workspace_is_pinned_only_on_initial_unfiltered_view() {
        let mut app = App::new(Config::default(), Theme::load(false));
        let mut alpha = entry(Source::Workspace, "/alpha", "alpha");
        alpha.workspace_id = Some("w1".into());
        alpha.action = EntryAction::FocusWorkspace { id: "w1".into() };
        let mut zulu = entry(Source::Workspace, "/zulu", "zulu");
        zulu.workspace_id = Some("w2".into());
        zulu.action = EntryAction::FocusWorkspace { id: "w2".into() };
        app.entries = vec![alpha, zulu];
        app.previous_workspace_id = Some("w2".into());

        app.apply_filter();
        assert_eq!(
            app.selected_entry().unwrap().workspace_id.as_deref(),
            Some("w2")
        );

        app.source_filter = Some(Source::Workspace);
        app.apply_filter();
        assert_eq!(
            app.selected_entry().unwrap().workspace_id.as_deref(),
            Some("w1")
        );

        app.source_filter = None;
        app.config.jump_back.pin_previous = false;
        app.apply_filter();
        assert_eq!(
            app.selected_entry().unwrap().workspace_id.as_deref(),
            Some("w1")
        );
    }

    #[test]
    fn pinned_entries_sort_first_and_persist() {
        let mut app = App::new(Config::default(), Theme::load(false));
        app.entries = vec![
            entry(Source::Root, "/alpha", "alpha"),
            entry(Source::Root, "/zulu", "zulu"),
        ];
        app.pinned_entries.insert(pin_key(&app.entries[1]));

        app.apply_filter();

        assert_eq!(app.selected_entry().unwrap().title, "zulu");

        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = env::temp_dir().join(format!("herdr-pins-{suffix}.json"));
        save_pinned_entries(&path, &app.pinned_entries).unwrap();
        assert_eq!(read_pinned_entries(&path).unwrap(), app.pinned_entries);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn previous_workspace_sorts_before_marked_entries() {
        let mut app = App::new(Config::default(), Theme::load(false));
        let marked = entry(Source::Root, "/marked", "marked");
        let mut previous = entry(Source::Workspace, "/previous", "previous");
        previous.workspace_id = Some("w2".into());
        previous.action = EntryAction::FocusWorkspace { id: "w2".into() };
        app.entries = vec![marked, previous];
        app.pinned_entries.insert(pin_key(&app.entries[0]));
        app.previous_workspace_id = Some("w2".into());

        app.apply_filter();

        assert_eq!(
            app.selected_entry().unwrap().workspace_id.as_deref(),
            Some("w2")
        );
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
    fn offers_directory_template_for_new_and_existing_directory_workspaces() {
        let mut config = Config::default();
        config.picker.directory_template = Some("default.toml".into());
        let mut app = App::new(config, Theme::load(false));
        app.entries = vec![entry(Source::Zoxide, "/tmp", "tmp")];
        app.apply_filter();

        assert_eq!(app.directory_template_for_selected(), Some("default.toml"));

        app.path_to_workspaces.insert(
            "/tmp".into(),
            vec![workspace("w2", "dir: tmp", WorkspaceKind::Dir, "/tmp")],
        );
        assert_eq!(app.directory_template_for_selected(), Some("default.toml"));

        app.config.picker.create_missing = false;
        assert_eq!(app.directory_template_for_selected(), Some("default.toml"));

        app.path_to_workspaces.insert(
            "/tmp".into(),
            vec![workspace(
                "w1",
                "project: tmp",
                WorkspaceKind::Project,
                "/tmp",
            )],
        );
        assert_eq!(
            app.matching_template_workspace_by_key("/tmp").unwrap().id,
            "w1"
        );
    }

    #[test]
    fn close_target_matches_entry_kind() {
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
        let dir = entry(Source::Root, "/tmp", "tmp");

        assert_eq!(app.workspace_to_close(&project), Some("w1".into()));
        assert_eq!(app.workspace_to_close(&dir), Some("w2".into()));
    }

    #[test]
    fn refuses_to_close_picker_owning_workspace() {
        assert_eq!(
            close_current_workspace_error("w1", Some("w1")),
            Some("can't close the workspace that owns this picker; switch away first".into())
        );
        assert_eq!(close_current_workspace_error("w1", Some("w2")), None);
        assert_eq!(close_current_workspace_error("w1", None), None);
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

    #[test]
    fn jump_back_records_only_real_workspace_transitions() {
        assert_eq!(
            previous_workspace_to_record(Some("w1"), Some("w2")),
            Some("w1")
        );
        assert_eq!(previous_workspace_to_record(Some("w1"), Some("w1")), None);
        assert_eq!(previous_workspace_to_record(None, Some("w2")), None);
        assert_eq!(previous_workspace_to_record(Some("w1"), None), None);
    }

    #[test]
    fn jump_back_resolves_focused_and_previous_workspaces() {
        let json = serde_json::json!({
            "result": {"workspaces": [
                {"workspace_id": "w1", "label": "one", "focused": false},
                {"workspace_id": "w2", "label": "two", "focused": true}
            ]}
        });

        assert_eq!(focused_workspace_id(&json), Some("w2"));
        assert_eq!(workspace_label(&json, "w1"), Some("one"));
        assert_eq!(workspace_label(&json, "missing"), None);
    }
}
