use std::{
    collections::{HashMap, HashSet},
    env, fs, io,
    path::{Path, PathBuf},
    process::{self, Command},
};

use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};
use nucleo_matcher::{
    pattern::{CaseMatching, Normalization, Pattern},
    Config as NucleoConfig, Matcher as NucleoMatcher, Utf32Str,
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
    Frame, Terminal,
};
use serde::Deserialize;
use serde_json::Value;

const DEFAULT_CONFIG: &str = include_str!("../examples/default-config.toml");

fn main() {
    match env::args().nth(1).as_deref() {
        Some("open") => open_picker(),
        Some("ui") => run_ui(),
        Some("list") => debug_list(),
        _ => {
            eprintln!("usage: herdr-workdir-picker <open|ui|list>");
            process::exit(2);
        }
    }
}

fn open_picker() -> ! {
    let plugin = env::var("HERDR_PLUGIN_ID").unwrap_or_else(|_| "fenix.workdir-picker".into());
    let status = Command::new(herdr_bin())
        .args([
            "plugin",
            "pane",
            "open",
            "--plugin",
            &plugin,
            "--entrypoint",
            "picker",
            "--focus",
        ])
        .status();
    match status {
        Ok(s) => process::exit(s.code().unwrap_or(0)),
        Err(e) => {
            eprintln!("failed to open picker pane: {e}");
            process::exit(1);
        }
    }
}

fn run_ui() -> ! {
    let config = Config::load();
    let theme = Theme::load(config.theme.inherit_herdr);
    let mut app = App::new(config, theme);
    app.refresh();

    if let Err(e) = tui_loop(&mut app) {
        eprintln!("workdir picker error: {e}");
        process::exit(1);
    }
    process::exit(0);
}

fn debug_list() {
    let mut app = App::new(Config::load(), Theme::load(true));
    app.refresh();
    for e in app.entries {
        println!("{:?}\t{}\t{}", e.source, e.title, e.path.display());
    }
}

fn tui_loop(app: &mut App) -> io::Result<()> {
    enable_raw_mode()?;
    let mut out = io::stdout();
    execute!(out, EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(out))?;
    let result = loop {
        terminal.draw(|f| draw(f, app))?;
        match event::read()? {
            Event::Key(key) if key.kind == KeyEventKind::Press => match handle_key(app, key) {
                Action::Continue => {}
                Action::Quit => break Ok(()),
                Action::Open => {
                    cleanup_terminal(&mut terminal)?;
                    let outcome = app.open_selected();
                    if let Err(e) = outcome {
                        eprintln!("{e}");
                        wait_for_key();
                    }
                    return Ok(());
                }
            },
            _ => {}
        }
    };
    cleanup_terminal(&mut terminal)?;
    result
}

fn cleanup_terminal(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> io::Result<()> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}

fn wait_for_key() {
    eprintln!("press enter to close...");
    let mut s = String::new();
    let _ = io::stdin().read_line(&mut s);
}

enum Action {
    Continue,
    Quit,
    Open,
}

fn handle_key(app: &mut App, key: KeyEvent) -> Action {
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        match key.code {
            KeyCode::Char('a') => app.set_filter(Some(Source::Agent)),
            KeyCode::Char('w') => app.set_filter(Some(Source::Workspace)),
            KeyCode::Char('p') => app.set_filter(Some(Source::Project)),
            KeyCode::Char('z') => app.set_filter(Some(Source::Zoxide)),
            KeyCode::Char('r') => app.set_filter(Some(Source::Root)),
            KeyCode::Char('o') => app.preview = !app.preview,
            KeyCode::Char('u') => {
                app.query.clear();
                app.set_filter(None);
            }
            KeyCode::Char('c') => return Action::Quit,
            _ => {}
        }
        app.apply_filter();
        return Action::Continue;
    }

    match key.code {
        KeyCode::Esc => Action::Quit,
        KeyCode::Enter => Action::Open,
        KeyCode::Up => {
            app.prev();
            Action::Continue
        }
        KeyCode::Down => {
            app.next();
            Action::Continue
        }
        KeyCode::Tab => {
            app.cycle_filter();
            Action::Continue
        }
        KeyCode::Backspace => {
            app.query.pop();
            app.apply_filter();
            Action::Continue
        }
        KeyCode::Char(c) => {
            app.query.push(c);
            app.apply_filter();
            Action::Continue
        }
        _ => Action::Continue,
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum Source {
    Workspace,
    Project,
    Zoxide,
    Root,
    Agent,
}

impl Source {
    fn label(&self) -> &'static str {
        match self {
            Source::Workspace => "open",
            Source::Project => "project",
            Source::Zoxide => "zoxide",
            Source::Root => "root",
            Source::Agent => "agent",
        }
    }

    fn from_config(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "workspace" | "workspaces" | "open" | "open_workspaces" => Some(Source::Workspace),
            "project" | "projects" | "herdr_plus_projects" => Some(Source::Project),
            "zoxide" | "z" => Some(Source::Zoxide),
            "root" | "roots" | "scan" => Some(Source::Root),
            "agent" | "agents" => Some(Source::Agent),
            _ => None,
        }
    }

    fn all() -> [Source; 5] {
        [
            Source::Workspace,
            Source::Project,
            Source::Zoxide,
            Source::Root,
            Source::Agent,
        ]
    }
}

#[derive(Clone, Debug)]
struct Entry {
    source: Source,
    title: String,
    subtitle: String,
    path: PathBuf,
    workspace_id: Option<String>,
    agent_target: Option<String>,
    project: Option<Project>,
}

impl Entry {
    fn key(&self) -> String {
        canonical_str(&self.path).unwrap_or_else(|| self.path.display().to_string())
    }
    fn haystack(&self) -> String {
        format!(
            "{} {} {} {}",
            self.source.label(),
            self.title,
            self.subtitle,
            self.path.display()
        )
        .to_lowercase()
    }
}

struct App {
    config: Config,
    theme: Theme,
    entries: Vec<Entry>,
    filtered: Vec<usize>,
    selected: usize,
    query: String,
    source_filter: Option<Source>,
    preview: bool,
    path_to_workspace: HashMap<String, String>,
}

impl App {
    fn new(config: Config, theme: Theme) -> Self {
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

    fn refresh(&mut self) {
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

        self.entries = entries;
        self.apply_filter();
    }

    fn apply_filter(&mut self) {
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
        if self.selected >= self.filtered.len() {
            self.selected = self.filtered.len().saturating_sub(1);
        }
    }

    fn set_filter(&mut self, source: Option<Source>) {
        self.source_filter = if self.source_filter == source {
            None
        } else {
            source
        };
        self.selected = 0;
    }

    fn cycle_filter(&mut self) {
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

    fn next(&mut self) {
        if !self.filtered.is_empty() {
            self.selected = (self.selected + 1).min(self.filtered.len() - 1);
        }
    }
    fn prev(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }
    fn selected_entry(&self) -> Option<&Entry> {
        self.filtered
            .get(self.selected)
            .and_then(|idx| self.entries.get(*idx))
    }

    fn open_selected(&self) -> Result<(), String> {
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
            Source::Zoxide | Source::Root => self.focus_or_create(&e.path, &e.title),
        }
    }

    fn open_project(&self, e: &Entry) -> Result<(), String> {
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

    fn focus_or_create(&self, path: &Path, label: &str) -> Result<(), String> {
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

fn draw(f: &mut Frame, app: &App) {
    let area = f.area();
    f.render_widget(Clear, area);
    let outer = Block::default()
        .title(" Herdr Workdir Picker ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(app.theme.accent));
    let inner = outer.inner(area);
    f.render_widget(outer, area);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(3),
            Constraint::Length(2),
        ])
        .split(inner);

    let filter = app
        .source_filter
        .as_ref()
        .map(|s| s.label())
        .unwrap_or("all");
    let search = Paragraph::new(Line::from(vec![
        Span::styled("query ", Style::default().fg(app.theme.overlay0)),
        Span::styled(
            &app.query,
            Style::default()
                .fg(app.theme.text)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("   "),
        Span::styled(
            format!("filter:{filter}"),
            Style::default().fg(app.theme.accent),
        ),
    ]))
    .block(Block::default().borders(Borders::BOTTOM));
    f.render_widget(search, rows[0]);

    let body = if app.preview {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(48), Constraint::Percentage(52)])
            .split(rows[1])
    } else {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(100)])
            .split(rows[1])
    };

    draw_list(f, app, body[0]);
    if app.preview {
        draw_preview(f, app, body[1]);
    }

    let help = "Ctrl-W workspace  Ctrl-P project  Ctrl-Z zoxide  Ctrl-R roots  Ctrl-A agents  Ctrl-O preview  Ctrl-U clear  Tab cycle  Enter open  Esc quit";
    f.render_widget(
        Paragraph::new(help).style(Style::default().fg(app.theme.overlay0)),
        rows[2],
    );
}

fn draw_list(f: &mut Frame, app: &App, area: Rect) {
    let items: Vec<ListItem> = app
        .filtered
        .iter()
        .map(|idx| {
            let e = &app.entries[*idx];
            let color = source_color(&app.theme, &e.source);
            ListItem::new(Line::from(vec![
                Span::styled(
                    format!("[{:<7}] ", e.source.label()),
                    Style::default().fg(color),
                ),
                Span::styled(&e.title, Style::default().fg(app.theme.text)),
                Span::styled(
                    format!("  {}", e.subtitle),
                    Style::default().fg(app.theme.subtext0),
                ),
            ]))
        })
        .collect();
    let mut state = ListState::default();
    if !app.filtered.is_empty() {
        state.select(Some(app.selected));
    }
    let list = List::new(items)
        .block(Block::default().title(" Results ").borders(Borders::RIGHT))
        .highlight_style(
            Style::default()
                .bg(app.theme.surface0)
                .fg(app.theme.text)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("› ");
    f.render_stateful_widget(list, area, &mut state);
}

fn draw_preview(f: &mut Frame, app: &App, area: Rect) {
    let text = if let Some(e) = app.selected_entry() {
        preview_text(app, e)
    } else {
        "No results".into()
    };
    let p = Paragraph::new(text)
        .style(Style::default().fg(app.theme.text))
        .wrap(Wrap { trim: false })
        .block(
            Block::default()
                .title(" Preview ")
                .borders(Borders::LEFT)
                .border_style(Style::default().fg(app.theme.surface_dim)),
        );
    f.render_widget(p, area);
}

fn preview_text(app: &App, e: &Entry) -> String {
    let mut lines = vec![
        format!("type: {}", e.source.label()),
        format!("title: {}", e.title),
        format!("path: {}", e.path.display()),
    ];
    if !e.subtitle.is_empty() {
        lines.push(format!("info: {}", e.subtitle));
    }
    if let Some(id) = &e.workspace_id {
        lines.push(format!("workspace_id: {id}"));
    }
    if let Some(target) = &e.agent_target {
        lines.push(format!("agent target: {target}"));
    }
    let key = e.key();
    if let Some(id) = app.path_to_workspace.get(&key) {
        lines.push(format!("existing workspace: {id}"));
    }
    if let Some(p) = &e.project {
        lines.push("".into());
        lines.push("project tabs:".into());
        for tab in &p.tabs {
            let cmd = tab.command.as_deref().unwrap_or("shell");
            lines.push(format!("  - {}: {}", tab.name, cmd));
        }
    }
    lines.push("".into());
    let action: &str = match e.source {
        Source::Workspace => "focus existing workspace",
        Source::Agent => "focus agent pane",
        _ if app.path_to_workspace.contains_key(&key) => "focus existing workspace",
        Source::Project => "create workspace + project tabs",
        _ => "create workspace",
    };
    lines.push(format!("enter: {action}"));
    lines.join("\n")
}

fn source_color(theme: &Theme, source: &Source) -> Color {
    match source {
        Source::Workspace => theme.green,
        Source::Project => theme.mauve,
        Source::Zoxide => theme.blue,
        Source::Root => theme.teal,
        Source::Agent => theme.yellow,
    }
}

fn match_score(engine: &str, hay: &str, q: &str) -> Option<i64> {
    match engine {
        "skim" => SkimMatcherV2::default().fuzzy_match(hay, q),
        "simple" => simple_fuzzy_score(hay, q).map(|score| -score),
        _ => {
            let mut matcher = NucleoMatcher::new(NucleoConfig::DEFAULT.match_paths());
            let pattern = Pattern::parse(q, CaseMatching::Ignore, Normalization::Smart);
            let mut buf = Vec::new();
            pattern
                .score(Utf32Str::new(hay, &mut buf), &mut matcher)
                .map(|score| score as i64)
        }
    }
}

fn simple_fuzzy_score(hay: &str, q: &str) -> Option<i64> {
    let mut score = 0;
    let mut pos = 0;
    for qc in q.chars() {
        let rest = &hay[pos..];
        let found = rest.find(qc)?;
        score += found as i64;
        pos += found + qc.len_utf8();
    }
    Some(score)
}

fn push_unique(entries: &mut Vec<Entry>, seen: &mut HashSet<String>, incoming: Vec<Entry>) {
    for e in incoming {
        let key = format!("{}:{}", e.source.label(), e.key());
        if seen.insert(key) {
            entries.push(e);
        }
    }
}

#[derive(Clone, Deserialize)]
struct Config {
    #[serde(default)]
    picker: PickerConfig,
    #[serde(default)]
    sources: SourcesConfig,
    #[serde(default)]
    theme: ThemeConfig,
    #[serde(default)]
    roots: Vec<RootConfig>,
}

#[derive(Clone, Deserialize)]
struct PickerConfig {
    #[serde(default = "yes")]
    reuse_existing: bool,
    #[serde(default = "yes")]
    create_missing: bool,
    #[serde(default = "default_engine")]
    engine: String,
    #[serde(default = "default_source_order")]
    source_order: Vec<String>,
    #[serde(default = "default_source_priority_boost")]
    source_priority_boost: i64,
}
#[derive(Clone, Deserialize)]
struct SourcesConfig {
    #[serde(default = "yes")]
    open_workspaces: bool,
    #[serde(default = "yes")]
    herdr_plus_projects: bool,
    #[serde(default = "yes")]
    zoxide: bool,
    #[serde(default = "yes")]
    roots: bool,
    #[serde(default = "yes")]
    agents: bool,
}
#[derive(Clone, Deserialize)]
struct ThemeConfig {
    #[serde(default = "yes")]
    inherit_herdr: bool,
}
#[derive(Clone, Deserialize)]
struct RootConfig {
    path: String,
    #[serde(default = "default_depth")]
    max_depth: usize,
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
    ["workspace", "project", "zoxide", "root", "agent"]
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
    fn source_rank(&self, source: &Source) -> usize {
        self.source_order
            .iter()
            .filter_map(|name| Source::from_config(name))
            .position(|item| &item == source)
            .unwrap_or_else(|| Source::all().len())
    }

    fn source_bonus(&self, source: &Source) -> i64 {
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
    fn load() -> Self {
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

#[derive(Clone)]
struct Theme {
    accent: Color,
    panel_bg: Color,
    surface0: Color,
    surface_dim: Color,
    overlay0: Color,
    text: Color,
    subtext0: Color,
    green: Color,
    yellow: Color,
    red: Color,
    blue: Color,
    teal: Color,
    mauve: Color,
}

impl Theme {
    fn catppuccin() -> Self {
        Self {
            accent: rgb(137, 180, 250),
            panel_bg: rgb(24, 24, 37),
            surface0: rgb(49, 50, 68),
            surface_dim: rgb(30, 30, 46),
            overlay0: rgb(108, 112, 134),
            text: rgb(205, 214, 244),
            subtext0: rgb(166, 173, 200),
            green: rgb(166, 227, 161),
            yellow: rgb(249, 226, 175),
            red: rgb(243, 139, 168),
            blue: rgb(137, 180, 250),
            teal: rgb(148, 226, 213),
            mauve: rgb(203, 166, 247),
        }
    }
    fn load(inherit: bool) -> Self {
        let mut t = Theme::catppuccin();
        if !inherit {
            return t;
        }
        let path = home().join(".config/herdr/config.toml");
        let Ok(s) = fs::read_to_string(path) else {
            return t;
        };
        let Ok(v) = s.parse::<toml::Value>() else {
            return t;
        };
        if let Some(custom) = v
            .get("theme")
            .and_then(|x| x.get("custom"))
            .and_then(|x| x.as_table())
        {
            for (k, v) in custom {
                if let Some(c) = v.as_str().and_then(parse_color) {
                    t.set(k, c);
                }
            }
        }
        t
    }
    fn set(&mut self, key: &str, color: Color) {
        match key {
            "accent" => self.accent = color,
            "panel_bg" => self.panel_bg = color,
            "surface0" => self.surface0 = color,
            "surface_dim" => self.surface_dim = color,
            "overlay0" => self.overlay0 = color,
            "text" => self.text = color,
            "subtext0" => self.subtext0 = color,
            "green" => self.green = color,
            "yellow" => self.yellow = color,
            "red" => self.red = color,
            "blue" => self.blue = color,
            "teal" => self.teal = color,
            "mauve" => self.mauve = color,
            _ => {}
        }
    }
}
fn rgb(r: u8, g: u8, b: u8) -> Color {
    Color::Rgb(r, g, b)
}
fn parse_color(s: &str) -> Option<Color> {
    let s = s.trim();
    if let Some(hex) = s.strip_prefix('#') {
        if hex.len() == 6 {
            return Some(rgb(
                u8::from_str_radix(&hex[0..2], 16).ok()?,
                u8::from_str_radix(&hex[2..4], 16).ok()?,
                u8::from_str_radix(&hex[4..6], 16).ok()?,
            ));
        }
    }
    match s {
        "red" => Some(Color::Red),
        "green" => Some(Color::Green),
        "yellow" => Some(Color::Yellow),
        "blue" => Some(Color::Blue),
        "cyan" => Some(Color::Cyan),
        "magenta" => Some(Color::Magenta),
        "white" => Some(Color::White),
        "black" => Some(Color::Black),
        _ => None,
    }
}

#[derive(Clone, Debug, Deserialize)]
struct Project {
    name: String,
    #[serde(default)]
    description: String,
    working_dir: String,
    #[serde(default)]
    tabs: Vec<ProjectTab>,
}
#[derive(Clone, Debug, Deserialize)]
struct ProjectTab {
    name: String,
    command: Option<String>,
}

fn collect_workspaces() -> (Vec<Entry>, HashMap<String, String>) {
    let ws_json = herdr_json(["workspace", "list"]).unwrap_or(Value::Null);
    let pane_json = herdr_json(["pane", "list"]).unwrap_or(Value::Null);
    let mut cwd_by_ws: HashMap<String, String> = HashMap::new();
    if let Some(panes) = pane_json
        .pointer("/result/panes")
        .and_then(|v| v.as_array())
    {
        for p in panes {
            let Some(ws) = p.get("workspace_id").and_then(|v| v.as_str()) else {
                continue;
            };
            let cwd = p
                .get("foreground_cwd")
                .or_else(|| p.get("cwd"))
                .and_then(|v| v.as_str())
                .unwrap_or("");
            if !cwd.is_empty() {
                cwd_by_ws.entry(ws.into()).or_insert(cwd.into());
            }
        }
    }
    let mut entries = Vec::new();
    let mut map = HashMap::new();
    if let Some(workspaces) = ws_json
        .pointer("/result/workspaces")
        .and_then(|v| v.as_array())
    {
        for w in workspaces {
            let id = w.get("workspace_id").and_then(|v| v.as_str()).unwrap_or("");
            let label = w.get("label").and_then(|v| v.as_str()).unwrap_or(id);
            let cwd = cwd_by_ws
                .get(id)
                .cloned()
                .unwrap_or_else(|| home().display().to_string());
            let path = PathBuf::from(&cwd);
            if let Some(key) = canonical_str(&path) {
                map.insert(key, id.into());
            }
            entries.push(Entry {
                source: Source::Workspace,
                title: label.into(),
                subtitle: format!(
                    "{} tabs:{} panes:{}",
                    id,
                    w.get("tab_count").and_then(|v| v.as_i64()).unwrap_or(0),
                    w.get("pane_count").and_then(|v| v.as_i64()).unwrap_or(0)
                ),
                path,
                workspace_id: Some(id.into()),
                agent_target: None,
                project: None,
            });
        }
    }
    (entries, map)
}

fn collect_agents() -> Vec<Entry> {
    let pane_json = herdr_json(["pane", "list"]).unwrap_or(Value::Null);
    let mut entries = Vec::new();
    if let Some(panes) = pane_json
        .pointer("/result/panes")
        .and_then(|v| v.as_array())
    {
        for p in panes {
            let Some(agent) = p.get("agent").and_then(|v| v.as_str()) else {
                continue;
            };
            let pane = p.get("pane_id").and_then(|v| v.as_str()).unwrap_or("");
            let term = p
                .get("terminal_id")
                .and_then(|v| v.as_str())
                .unwrap_or(pane);
            let cwd = p
                .get("foreground_cwd")
                .or_else(|| p.get("cwd"))
                .and_then(|v| v.as_str())
                .unwrap_or("/");
            let status = p
                .get("agent_status")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            entries.push(Entry {
                source: Source::Agent,
                title: agent.into(),
                subtitle: format!("{status} {pane}"),
                path: PathBuf::from(cwd),
                workspace_id: p
                    .get("workspace_id")
                    .and_then(|v| v.as_str())
                    .map(|s| s.into()),
                agent_target: Some(term.into()),
                project: None,
            });
        }
    }
    entries
}

fn collect_projects() -> Vec<Entry> {
    let mut out = Vec::new();
    let dir = herdr_plus_projects_dir();
    let Ok(read) = fs::read_dir(dir) else {
        return out;
    };
    for file in read.flatten() {
        let path = file.path();
        if path.extension().and_then(|s| s.to_str()) != Some("toml") {
            continue;
        }
        let Ok(s) = fs::read_to_string(&path) else {
            continue;
        };
        let Ok(project) = toml::from_str::<Project>(&s) else {
            continue;
        };
        let p = expand_path(&project.working_dir);
        out.push(Entry {
            source: Source::Project,
            title: project.name.clone(),
            subtitle: project.description.clone(),
            path: p,
            workspace_id: None,
            agent_target: None,
            project: Some(project),
        });
    }
    out
}

fn collect_zoxide() -> Vec<Entry> {
    let Ok(out) = Command::new("zoxide").args(["query", "-l"]).output() else {
        return vec![];
    };
    if !out.status.success() {
        return vec![];
    }
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|line| {
            let path = PathBuf::from(line);
            Entry {
                source: Source::Zoxide,
                title: basename(&path),
                subtitle: line.into(),
                path,
                workspace_id: None,
                agent_target: None,
                project: None,
            }
        })
        .collect()
}

fn collect_roots(config: &Config) -> Vec<Entry> {
    let mut out = Vec::new();
    for root in &config.roots {
        walk_dirs(&expand_path(&root.path), root.max_depth, &mut out);
    }
    out
}
fn walk_dirs(path: &Path, depth: usize, out: &mut Vec<Entry>) {
    if depth == 0 || !path.is_dir() {
        return;
    }
    if path.join(".git").exists()
        || path.join("package.json").exists()
        || path.join("Cargo.toml").exists()
    {
        out.push(Entry {
            source: Source::Root,
            title: basename(path),
            subtitle: path.display().to_string(),
            path: path.to_path_buf(),
            workspace_id: None,
            agent_target: None,
            project: None,
        });
    }
    if let Ok(read) = fs::read_dir(path) {
        for e in read.flatten() {
            let p = e.path();
            if p.is_dir() && !basename(&p).starts_with('.') {
                walk_dirs(&p, depth - 1, out);
            }
        }
    }
}

fn bootstrap_project_tabs(
    project: &Project,
    create_json: &Value,
    cwd: &Path,
) -> Result<(), String> {
    let workspace_id = create_json
        .pointer("/result/workspace/workspace_id")
        .and_then(|v| v.as_str())
        .ok_or("workspace create did not return workspace_id")?;
    let root_pane = create_json
        .pointer("/result/root_pane/pane_id")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if let Some(first) = project.tabs.first() {
        let _ = run_herdr(["tab", "rename", &format!("{workspace_id}:t1"), &first.name]);
        if let Some(cmd) = &first.command {
            if !root_pane.is_empty() {
                let _ = run_herdr(["pane", "run", root_pane, cmd]);
            }
        }
    }
    for tab in project.tabs.iter().skip(1) {
        let json = herdr_json([
            "tab",
            "create",
            "--workspace",
            workspace_id,
            "--cwd",
            &cwd.display().to_string(),
            "--label",
            &tab.name,
            "--no-focus",
        ])?;
        if let Some(cmd) = &tab.command {
            if let Some(pane) = json
                .pointer("/result/root_pane/pane_id")
                .and_then(|v| v.as_str())
            {
                let _ = run_herdr(["pane", "run", pane, cmd]);
            }
        }
    }
    Ok(())
}

fn herdr_bin() -> String {
    env::var("HERDR_BIN_PATH").unwrap_or_else(|_| "herdr".into())
}
fn plugin_config_dir() -> PathBuf {
    env::var("HERDR_PLUGIN_CONFIG_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| home().join(".config/herdr/plugins/config/fenix.workdir-picker"))
}
fn herdr_plus_projects_dir() -> PathBuf {
    home().join(".config/herdr/plugins/config/cloudmanic.herdr-plus/projects")
}
fn home() -> PathBuf {
    env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/"))
}
fn expand_path(s: &str) -> PathBuf {
    if let Some(rest) = s.strip_prefix("~/") {
        home().join(rest)
    } else if s == "~" {
        home()
    } else {
        PathBuf::from(s.replace("$HOME", &home().display().to_string()))
    }
}
fn basename(path: &Path) -> String {
    path.file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("workspace")
        .to_string()
}
fn canonical_str(path: &Path) -> Option<String> {
    fs::canonicalize(path).ok().map(|p| p.display().to_string())
}

fn herdr_json<const N: usize>(args: [&str; N]) -> Result<Value, String> {
    let out = Command::new(herdr_bin())
        .args(args)
        .output()
        .map_err(|e| e.to_string())?;
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).to_string());
    }
    serde_json::from_slice(&out.stdout).map_err(|e| e.to_string())
}
fn run_herdr<const N: usize>(args: [&str; N]) -> Result<(), String> {
    let status = Command::new(herdr_bin())
        .args(args)
        .status()
        .map_err(|e| e.to_string())?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("herdr exited with {status}"))
    }
}
