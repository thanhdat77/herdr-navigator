use std::{
    io,
    sync::mpsc::{Receiver, TryRecvError},
    time::Duration,
};

use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyEventKind,
        MouseButton, MouseEvent, MouseEventKind,
    },
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
    Frame, Terminal,
};

use crate::{
    app::{App, InputMode},
    keymap::{keybindings, Command},
    model::{Entry, EntryAction, Source},
    paths::home,
    sources::status_icon_at,
    theme::Theme,
};

pub(crate) fn tui_loop(
    app: &mut App,
    persist: bool,
    mut update_check: Option<Receiver<Option<String>>>,
) -> io::Result<()> {
    enable_raw_mode()?;
    let mut out = io::stdout();
    execute!(out, EnterAlternateScreen, EnableMouseCapture)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(out))?;
    let mut list_hits = ListHits::default();
    let result = loop {
        if let Some(result) = update_check.as_ref().map(Receiver::try_recv) {
            match result {
                Ok(version) => {
                    app.update_available = version;
                    update_check = None;
                }
                Err(TryRecvError::Disconnected) => update_check = None,
                Err(TryRecvError::Empty) => {}
            }
        }
        terminal.draw(|f| list_hits = draw(f, app))?;
        let animate = has_working_entry(app);
        if (animate || update_check.is_some()) && !event::poll(Duration::from_millis(125))? {
            if animate {
                app.spinner_tick = app.spinner_tick.wrapping_add(1);
            }
            continue;
        }
        let action = match event::read()? {
            Event::Key(key) if key.kind == KeyEventKind::Press => handle_key(app, key),
            Event::Mouse(mouse) => handle_mouse(app, mouse, &list_hits),
            _ => Action::Continue,
        };
        match action {
            Action::Continue => {}
            Action::Quit => break Ok(()),
            action @ (Action::Open | Action::OpenTemplate) => {
                // leave the TUI while the action runs: herdr CLI output goes to
                // the normal screen instead of corrupting the alternate screen
                cleanup_terminal(&mut terminal)?;
                let outcome = app.open_selected(matches!(action, Action::OpenTemplate));
                if let Err(e) = outcome {
                    eprintln!("{e}");
                    wait_for_key();
                }
                if !persist {
                    return Ok(());
                }
                app.refresh();
                enable_raw_mode()?;
                execute!(
                    terminal.backend_mut(),
                    EnterAlternateScreen,
                    EnableMouseCapture
                )?;
                terminal.clear()?;
            }
            Action::Update => {
                cleanup_terminal(&mut terminal)?;
                if let Some(version) = app.update_available.clone() {
                    if confirm_update(&version)? {
                        match crate::update::install(&version) {
                            Ok(()) => eprintln!("Updated Herdr Navigator to v{version}."),
                            Err(error) => eprintln!("Update failed: {error}"),
                        }
                        wait_for_key();
                        return Ok(());
                    }
                }
                enable_raw_mode()?;
                execute!(
                    terminal.backend_mut(),
                    EnterAlternateScreen,
                    EnableMouseCapture
                )?;
                terminal.clear()?;
            }
            Action::CloseWorkspace => {
                if let Err(e) = app.close_selected_workspace() {
                    crate::herdr::notify_error(
                        &format!("Close failed: {e}"),
                        &app.config.notifications,
                    );
                }
            }
        }
    };
    cleanup_terminal(&mut terminal)?;
    result
}

fn cleanup_terminal(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> io::Result<()> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), DisableMouseCapture, LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}

fn confirm_update(version: &str) -> io::Result<bool> {
    eprintln!("Update Herdr Navigator to v{version}? [y/N]");
    let mut answer = String::new();
    io::stdin().read_line(&mut answer)?;
    Ok(matches!(
        answer.trim().to_ascii_lowercase().as_str(),
        "y" | "yes"
    ))
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
    OpenTemplate,
    Update,
    CloseWorkspace,
}

#[derive(Default)]
struct ListHits {
    area: Rect,
    rows: Vec<(std::ops::Range<u16>, usize)>,
}

fn handle_mouse(app: &mut App, mouse: MouseEvent, hits: &ListHits) -> Action {
    if app.input_mode == InputMode::Help || !hits.area.contains((mouse.column, mouse.row).into()) {
        return Action::Continue;
    }

    match mouse.kind {
        MouseEventKind::ScrollUp => app.prev(),
        MouseEventKind::ScrollDown => app.next(),
        MouseEventKind::Down(MouseButton::Left) => {
            let Some((_, row)) = hits.rows.iter().find(|(range, _)| range.contains(&mouse.row)) else {
                return Action::Continue;
            };
            if app.selected == *row {
                return Action::Open;
            }
            app.selected = *row;
        }
        _ => {}
    }
    Action::Continue
}

fn handle_key(app: &mut App, key: KeyEvent) -> Action {
    let command = keybindings(app)
        .into_iter()
        .find(|binding| binding.matches(app, key))
        .map(|binding| binding.command);

    if app.input_mode == InputMode::Help {
        if matches!(command, Some(Command::Back | Command::ToggleHelp)) {
            app.input_mode = InputMode::Normal;
        }
        return Action::Continue;
    }

    if let Some(command) = command {
        return execute_command(app, command, key);
    }

    if app.config.picker.vim_mode && app.input_mode == InputMode::Normal {
        return Action::Continue;
    }

    if let KeyCode::Char(c) = key.code {
        app.query.push(c);
        app.apply_filter();
    }
    Action::Continue
}

fn execute_command(app: &mut App, command: Command, key: KeyEvent) -> Action {
    match command {
        Command::Back => {
            if key.code == KeyCode::Esc && app.input_mode == InputMode::Search {
                app.input_mode = InputMode::Normal;
                Action::Continue
            } else {
                Action::Quit
            }
        }
        Command::Open => Action::Open,
        Command::OpenTemplate => Action::OpenTemplate,
        Command::Update => Action::Update,
        Command::MoveUp => {
            app.prev();
            Action::Continue
        }
        Command::MoveDown => {
            app.next();
            Action::Continue
        }
        Command::StartSearch => {
            app.query.clear();
            app.apply_filter();
            app.input_mode = InputMode::Search;
            Action::Continue
        }
        Command::CycleFilter => {
            app.cycle_filter();
            Action::Continue
        }
        Command::DeleteChar => {
            app.query.pop();
            app.apply_filter();
            Action::Continue
        }
        Command::Clear => {
            app.query.clear();
            app.set_filter(None);
            app.input_mode = InputMode::Normal;
            app.apply_filter();
            Action::Continue
        }
        Command::CloseWorkspace => Action::CloseWorkspace,
        Command::ToggleMark => {
            if let Err(error) = app.toggle_selected_pin() {
                crate::herdr::notify_error(
                    &format!("Mark failed: {error}"),
                    &app.config.notifications,
                );
            }
            Action::Continue
        }
        Command::TogglePreview => {
            app.preview = !app.preview;
            Action::Continue
        }
        Command::ToggleHelp => {
            app.input_mode = InputMode::Help;
            Action::Continue
        }
        Command::Filter(source) => {
            if !key
                .modifiers
                .contains(crossterm::event::KeyModifiers::CONTROL)
            {
                app.query.clear();
                app.input_mode = if app.config.picker.vim_filter_search {
                    InputMode::Search
                } else {
                    InputMode::Normal
                };
            }
            app.set_filter(Some(source));
            app.apply_filter();
            Action::Continue
        }
    }
}

fn draw(f: &mut Frame, app: &App) -> ListHits {
    let area = f.area();
    f.render_widget(Clear, area);
    let mut outer = Block::default()
        .style(Style::default().bg(app.theme.panel_bg))
        .title(" Herdr Navigator ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(app.theme.accent));
    if let Some(version) = &app.update_available {
        outer = outer.title_top(
            Line::from(Span::styled(
                format!(" ↑ v{version} available · F5 update "),
                Style::default()
                    .fg(app.theme.yellow)
                    .add_modifier(Modifier::BOLD),
            ))
            .right_aligned(),
        );
    }
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
    .block(
        Block::default()
            .style(Style::default().bg(app.theme.panel_bg))
            .borders(Borders::BOTTOM),
    );
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

    let list_hits = draw_list(f, app, body[0]);
    if app.preview {
        draw_preview(f, app, body[1]);
    }

    draw_key_hints(f, app, rows[2]);
    if app.input_mode == InputMode::Help {
        draw_keybindings_help(f, app, area);
    }
    list_hits
}

fn draw_key_hints(f: &mut Frame, app: &App, area: Rect) {
    let mut command_spans = Vec::new();
    let mut filter_spans = Vec::new();
    for binding in keybindings(app) {
        let Some((key, label)) = binding.compact_hint(app) else {
            continue;
        };
        if key.is_empty() {
            continue;
        }
        let spans = if binding.group == "Filters" {
            &mut filter_spans
        } else {
            &mut command_spans
        };
        let active = binding.is_active(app);
        let key_style = if active {
            Style::default()
                .fg(app.theme.panel_bg)
                .bg(app.theme.accent)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
                .fg(app.theme.accent)
                .add_modifier(Modifier::BOLD)
        };
        spans.push(Span::styled(format!(" {key} "), key_style));
        spans.push(Span::styled(
            format!("{label}  "),
            Style::default().fg(if active {
                app.theme.text
            } else {
                app.theme.overlay0
            }),
        ));
    }
    f.render_widget(
        Paragraph::new(Text::from(vec![
            Line::from(command_spans),
            Line::from(filter_spans),
        ]))
        .style(Style::default().bg(app.theme.panel_bg)),
        area,
    );
}

fn draw_keybindings_help(f: &mut Frame, app: &App, area: Rect) {
    let bindings = keybindings(app);
    let mut lines = Vec::new();
    for group in ["Navigation", "Actions", "View", "Filters"] {
        let start = lines.len();
        lines.push(Line::styled(
            format!(" {group}"),
            Style::default()
                .fg(app.theme.accent)
                .add_modifier(Modifier::BOLD),
        ));
        for binding in bindings.iter().filter(|binding| binding.group == group) {
            let key = binding.key_label(app);
            if key.is_empty() {
                continue;
            }
            let active = binding.is_active(app);
            let key_style = if active {
                Style::default()
                    .fg(app.theme.panel_bg)
                    .bg(app.theme.accent)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(app.theme.accent)
            };
            lines.push(Line::from(vec![
                Span::raw("   "),
                Span::styled(format!("{key:<12}"), key_style),
                Span::styled(&binding.label, Style::default().fg(app.theme.text)),
            ]));
        }
        if lines.len() == start + 1 {
            lines.pop();
        } else {
            lines.push(Line::default());
        }
    }
    lines.pop();

    let height = (lines.len() as u16 + 2).min(area.height.saturating_sub(2).max(1));
    let popup = area.centered(Constraint::Percentage(72), Constraint::Length(height));
    f.render_widget(Clear, popup);
    f.render_widget(
        Paragraph::new(Text::from(lines))
            .style(Style::default().bg(app.theme.panel_bg))
            .block(
                Block::default()
                    .title(" Keybindings ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(app.theme.accent)),
            ),
        popup,
    );
}

fn has_working_entry(app: &App) -> bool {
    app.entries.iter().filter_map(entry_status).any(|status| {
        let status = status.to_lowercase();
        status.contains("work") || status.contains("run")
    })
}

fn agent_status_color(theme: &Theme, status: &str) -> Color {
    let status = status.to_lowercase();
    if status.contains("block")
        || status.contains("error")
        || status.contains("fail")
        || status.contains("attention")
        || status.contains("request")
        || status.contains("wait")
    {
        theme.red
    } else if status.contains("work") || status.contains("run") {
        theme.yellow
    } else if status.contains("done") || status.contains("complete") {
        theme.teal
    } else if status.contains("idle") {
        theme.green
    } else {
        theme.overlay0
    }
}

fn display_title(entry: &Entry) -> &str {
    if entry.source == Source::Workspace {
        entry
            .title
            .strip_prefix("dir: ")
            .or_else(|| entry.title.strip_prefix("project: "))
            .unwrap_or(&entry.title)
    } else {
        &entry.title
    }
}

fn entry_status(entry: &Entry) -> Option<&str> {
    match entry.source {
        Source::Agent => Some(
            entry
                .subtitle
                .split_once(" · ")
                .map(|(status, _)| status)
                .filter(|status| !status.is_empty())
                .unwrap_or("unknown"),
        ),
        Source::Workspace => entry.subtitle.strip_prefix("agent:").map(|rest| {
            rest.split_once(" · ")
                .map(|(status, _)| status)
                .unwrap_or(rest)
        }),
        _ => None,
    }
}

fn entry_metadata(entry: &Entry) -> String {
    match entry.source {
        Source::Agent => {
            let metadata = entry
                .subtitle
                .split_once(" · ")
                .map(|(_, metadata)| metadata)
                .unwrap_or("");
            metadata
                .split_once(" · ")
                .map(|(pane, tab)| format!("{tab} · {pane}"))
                .unwrap_or_else(|| metadata.to_string())
        }
        Source::Workspace => {
            let metadata = entry
                .subtitle
                .strip_prefix("agent:")
                .and_then(|rest| rest.split_once(" · ").map(|(_, metadata)| metadata))
                .unwrap_or(&entry.subtitle);
            let mut parts = metadata.split_whitespace();
            match (parts.next(), parts.next(), parts.next()) {
                (Some(_), Some(tabs), Some(panes)) => {
                    match (tabs.strip_prefix("tabs:"), panes.strip_prefix("panes:")) {
                        (Some(tabs), Some(panes)) => format!("{tabs} tabs · {panes} panes"),
                        _ => metadata.to_string(),
                    }
                }
                _ => metadata.to_string(),
            }
        }
        _ => entry.subtitle.clone(),
    }
}

fn display_path(entry: &Entry) -> String {
    entry
        .path
        .strip_prefix(home())
        .ok()
        .map(|path| {
            if path.as_os_str().is_empty() {
                "~".into()
            } else {
                format!("~/{}", path.display())
            }
        })
        .unwrap_or_else(|| entry.path.display().to_string())
}

fn metadata_width(width: u16) -> usize {
    if width >= 90 {
        28
    } else if width >= 68 {
        20
    } else if width >= 52 {
        14
    } else {
        0
    }
}

fn truncate_end(value: &str, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        return value.into();
    }
    if max_chars == 0 {
        return String::new();
    }
    value
        .chars()
        .take(max_chars.saturating_sub(1))
        .chain(std::iter::once('…'))
        .collect()
}

fn entry_branch(app: &App, entry: &Entry, group_end: bool) -> (&'static str, Color) {
    let is_workspace = entry.source == Source::Workspace;
    let is_current = is_workspace && entry.search_terms.iter().any(|term| term == "focused");
    let is_previous = is_workspace
        && app.config.jump_back.pin_previous
        && app.query.trim().is_empty()
        && app.source_filter.is_none()
        && entry.workspace_id.is_some()
        && entry.workspace_id == app.previous_workspace_id;
    if app.is_pinned(entry) {
        ("  ◆  ", app.theme.yellow)
    } else if is_current {
        ("  ◆  ", app.theme.accent)
    } else if is_previous {
        ("  ◆  ", app.theme.red)
    } else if group_end {
        ("  └─ ", app.theme.overlay0)
    } else {
        ("  ├─ ", app.theme.overlay0)
    }
}

fn draw_list(f: &mut Frame, app: &App, area: Rect) -> ListHits {
    let show_scores = !app.query.trim().is_empty();
    let row_width = area.width.saturating_sub(3) as usize;
    let mut items = Vec::new();
    let mut item_entries = Vec::new();
    let mut selected_row = None;
    for (row, idx) in app.filtered.iter().enumerate() {
        let e = &app.entries[*idx];
        let color = source_color(&app.theme, &e.source);
        let group_start =
            row == 0 || app.entries[app.filtered[row - 1]].source_name() != e.source_name();
        let group_end = row + 1 == app.filtered.len()
            || app.entries[app.filtered[row + 1]].source_name() != e.source_name();
        if group_start {
            items.push(ListItem::new(Line::from(Span::styled(
                format!(" ▾ {} ", e.source_name()),
                Style::default().fg(color).add_modifier(Modifier::BOLD),
            ))));
            item_entries.push(None);
        }

        if row == app.selected {
            selected_row = Some(items.len());
        }
        let (branch, branch_color) = entry_branch(app, e, group_end);
        let score = show_scores
            .then(|| app.filtered_scores.get(row).map(|s| format!("score {s}")))
            .flatten();

        if app.config.picker.detailed_rows {
            let status = entry_status(e);
            let icon = status
                .map(|status| format!("{} ", status_icon_at(&e.source, status, app.spinner_tick)))
                .unwrap_or_default();
            let status_label = status.filter(|status| *status != "unknown");
            let raw_path = e.path.display().to_string();
            let raw_metadata = entry_metadata(e);
            let meta_width = metadata_width(area.width);
            let show_metadata = !matches!(e.source, Source::Zoxide | Source::Root)
                && meta_width > 0
                && !raw_metadata.is_empty()
                && raw_metadata != raw_path;
            let separator_width = usize::from(show_metadata && status_label.is_some()) * 3;
            let metadata_budget = meta_width
                .saturating_sub(
                    status_label
                        .map(str::chars)
                        .map(Iterator::count)
                        .unwrap_or(0),
                )
                .saturating_sub(separator_width);
            let metadata = if show_metadata {
                truncate_end(&raw_metadata, metadata_budget)
            } else {
                String::new()
            };
            let right_width = metadata.chars().count()
                + separator_width
                + status_label
                    .map(str::chars)
                    .map(Iterator::count)
                    .unwrap_or(0);
            let fixed_width = branch.chars().count() + icon.chars().count();
            let right_column_width = if right_width > 0 {
                meta_width.max(right_width)
            } else {
                0
            };
            let title_budget = row_width
                .saturating_sub(fixed_width)
                .saturating_sub(right_column_width)
                .saturating_sub(usize::from(right_width > 0));
            let title = truncate_end(display_title(e), title_budget);
            let spacer = if right_width == 0 {
                String::new()
            } else {
                " ".repeat(
                    row_width
                        .saturating_sub(fixed_width)
                        .saturating_sub(title.chars().count())
                        .saturating_sub(right_column_width),
                )
            };
            let status_color = status
                .map(|status| agent_status_color(&app.theme, status))
                .unwrap_or(color);
            let mut title_spans = vec![
                Span::styled(branch, Style::default().fg(branch_color)),
                Span::styled(icon, Style::default().fg(status_color)),
                Span::styled(title, Style::default().fg(app.theme.text)),
            ];
            if right_width > 0 {
                title_spans.push(Span::raw(spacer));
                if let Some(status_label) = status_label {
                    title_spans.push(Span::styled(
                        status_label.to_string(),
                        Style::default().fg(status_color),
                    ));
                    if !metadata.is_empty() {
                        title_spans
                            .push(Span::styled(" · ", Style::default().fg(app.theme.overlay0)));
                    }
                }
                if !metadata.is_empty() {
                    title_spans.push(Span::styled(
                        metadata,
                        Style::default().fg(app.theme.overlay0),
                    ));
                }
            }

            if matches!(e.source, Source::Zoxide | Source::Root) {
                let detail_branch = if group_end { "     " } else { "  │  " };
                let path_budget = row_width.saturating_sub(detail_branch.chars().count());
                let path = truncate_end(&display_path(e), path_budget);
                items.push(ListItem::new(vec![
                    Line::from(title_spans),
                    Line::from(vec![
                        Span::styled(detail_branch, Style::default().fg(app.theme.overlay0)),
                        Span::styled(path, Style::default().fg(app.theme.subtext0)),
                    ]),
                ]));
            } else {
                items.push(ListItem::new(Line::from(title_spans)));
            }
        } else {
            let status_text = entry_status(e);
            let status = status_text
                .map(|status| format!("{} ", status_icon_at(&e.source, status, app.spinner_tick)))
                .unwrap_or_default();
            let subtitle = if e.subtitle.is_empty() {
                String::new()
            } else {
                format!("  {}", e.subtitle)
            };
            let left_len = branch.chars().count()
                + status.chars().count()
                + e.title.chars().count()
                + subtitle.chars().count();
            let spacer = score
                .as_ref()
                .map(|score| {
                    " ".repeat(
                        row_width
                            .saturating_sub(left_len + score.chars().count())
                            .max(2),
                    )
                })
                .unwrap_or_default();
            let mut spans = vec![
                Span::styled(branch, Style::default().fg(branch_color)),
                Span::styled(
                    status,
                    Style::default().fg(status_text
                        .map(|status| agent_status_color(&app.theme, status))
                        .unwrap_or(color)),
                ),
                Span::styled(e.title.clone(), Style::default().fg(app.theme.text)),
                Span::styled(subtitle, Style::default().fg(app.theme.subtext0)),
            ];
            if let Some(score) = score {
                spans.push(Span::raw(spacer));
                spans.push(Span::styled(score, Style::default().fg(app.theme.overlay0)));
            }
            items.push(ListItem::new(Line::from(spans)));
        }
        item_entries.push(Some(row));
    }
    let item_heights: Vec<u16> = items.iter().map(|item| item.height() as u16).collect();
    let mut state = ListState::default();
    state.select(selected_row);
    let list = List::new(items)
        .block(Block::default().title(" Results ").borders(Borders::RIGHT))
        .highlight_style(
            Style::default()
                .bg(app.theme.surface0)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("→ ");
    f.render_stateful_widget(list, area, &mut state);

    let mut rows = Vec::new();
    let mut y = area.y;
    for (entry, height) in item_entries
        .into_iter()
        .zip(item_heights)
        .skip(state.offset())
    {
        if y.saturating_add(height) > area.bottom() {
            break;
        }
        if let Some(entry) = entry {
            rows.push((y..y + height, entry));
        }
        y += height;
    }
    ListHits { area, rows }
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
        format!("type: {}", e.source_name()),
        format!("title: {}", e.title),
        format!("path: {}", e.path.display()),
    ];
    if !e.subtitle.is_empty() {
        lines.push(format!("info: {}", e.subtitle));
    }
    if let Some(label) = &e.workspace_label {
        lines.push(format!("workspace: {label}"));
    }
    if let Some(id) = &e.workspace_id {
        lines.push(format!("workspace_id: {id}"));
    }
    if let Some(target) = &e.agent_target {
        lines.push(format!("agent target: {target}"));
    }
    if e.source == Source::Agent {
        lines.push(
            "agent filters: @ all agents (configured sort), !agent, @workspace/status, /path"
                .into(),
        );
    }
    if !e.search_terms.is_empty() {
        lines.push(format!("search terms: {}", e.search_terms.join(", ")));
    }
    let workspaces = app.workspaces_for_entry(e);
    if !workspaces.is_empty() {
        lines.push("existing workspaces:".into());
        for ws in workspaces {
            lines.push(format!(
                "  - {} [{}] tabs:{} panes:{} {}",
                ws.id,
                ws.label,
                ws.tab_count,
                ws.pane_count,
                ws.path.display()
            ));
        }
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
    let action: &str = match &e.action {
        EntryAction::FocusWorkspace { .. } => "focus existing workspace",
        EntryAction::FocusAgent { .. } => "focus agent pane",
        EntryAction::OpenRemote { .. } => "open remote Herdr",
        EntryAction::AttachSession { .. } => "attach Herdr session",
        EntryAction::InvokePluginAction { .. } => "invoke Herdr plugin action",
        EntryAction::RunCommand { .. } if e.source == Source::Session => "open session via plugin",
        EntryAction::RunCommand { .. } => "run integration command",
        EntryAction::OpenProject if app.matching_project_workspace(e).is_some() => {
            "focus matching project workspace"
        }
        EntryAction::OpenProject => "create project workspace + tabs",
        EntryAction::FocusOrCreateDir if app.matching_dir_workspace(e).is_some() => {
            "focus matching dir workspace"
        }
        EntryAction::FocusOrCreateDir => "create dir workspace",
    };
    lines.push(format!("enter: {action}"));
    if let Some(template) = app.directory_template_for_selected() {
        lines.push(format!(
            "{}: apply template {template}",
            app.config.picker.directory_template_key
        ));
    }
    lines.join("\n")
}

fn source_color(theme: &Theme, source: &Source) -> Color {
    match source {
        Source::Workspace => theme.green,
        Source::Project => theme.mauve,
        Source::Zoxide => theme.blue,
        Source::Root => theme.teal,
        Source::Agent => theme.yellow,
        Source::Server => theme.green,
        Source::Session => theme.green,
        Source::QuickAction => theme.mauve,
        Source::Integration => theme.red,
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crossterm::event::KeyModifiers;
    use ratatui::backend::TestBackend;

    use super::*;
    use crate::{config::Config, theme::Theme};

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn entry(source: Source, title: &str) -> Entry {
        Entry {
            source,
            title: title.into(),
            subtitle: String::new(),
            path: PathBuf::from(title),
            workspace_id: None,
            workspace_label: None,
            agent_target: None,
            project: None,
            action: EntryAction::FocusOrCreateDir,
            source_label: None,
            search_terms: vec![],
        }
    }

    fn buffer_text(terminal: &Terminal<TestBackend>) -> String {
        let buffer = terminal.backend().buffer();
        (0..buffer.area.height)
            .map(|y| {
                (0..buffer.area.width)
                    .map(|x| buffer[(x, y)].symbol())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn herdr_plus_sources_share_color() {
        let theme = Theme::load(false);
        assert_eq!(
            source_color(&theme, &Source::Project),
            source_color(&theme, &Source::QuickAction)
        );
    }

    #[test]
    fn alt_enter_opens_selected_directory_with_configured_template() {
        let mut config = Config::default();
        config.picker.directory_template = Some("default.toml".into());
        let mut app = App::new(config, Theme::load(false));
        app.entries = vec![entry(Source::Zoxide, "/tmp")];
        app.apply_filter();

        assert!(matches!(
            handle_key(&mut app, key(KeyCode::Enter)),
            Action::Open
        ));
        assert!(matches!(
            handle_key(&mut app, KeyEvent::new(KeyCode::Enter, KeyModifiers::ALT)),
            Action::OpenTemplate
        ));

        app.config.picker.directory_template_key = "ctrl-g".into();
        assert!(matches!(
            handle_key(
                &mut app,
                KeyEvent::new(KeyCode::Char('g'), KeyModifiers::CONTROL)
            ),
            Action::OpenTemplate
        ));
    }

    #[test]
    fn update_badge_renders_action_and_f5_triggers_it() {
        let mut app = App::new(Config::default(), Theme::load(false));
        assert!(matches!(
            handle_key(
                &mut app,
                KeyEvent::new(KeyCode::F(5), crossterm::event::KeyModifiers::NONE)
            ),
            Action::Continue
        ));
        app.update_available = Some("0.3.2".into());

        let backend = TestBackend::new(70, 8);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| { draw(f, &app); }).unwrap();

        assert!(buffer_text(&terminal).contains("↑ v0.3.2 available · F5 update"));
        assert!(matches!(
            handle_key(
                &mut app,
                KeyEvent::new(KeyCode::F(5), crossterm::event::KeyModifiers::NONE)
            ),
            Action::Update
        ));
    }

    #[test]
    fn status_colors_match_herdr() {
        let theme = Theme::load(false);

        assert_eq!(agent_status_color(&theme, "blocked"), theme.red);
        assert_eq!(agent_status_color(&theme, "working"), theme.yellow);
        assert_eq!(agent_status_color(&theme, "done"), theme.teal);
        assert_eq!(agent_status_color(&theme, "idle"), theme.green);
        assert_eq!(agent_status_color(&theme, "unknown"), theme.overlay0);
    }

    #[test]
    fn open_and_pinned_workspaces_replace_tree_branches_with_diamonds() {
        let mut app = App::new(Config::default(), Theme::load(false));
        let mut current = entry(Source::Workspace, "Current");
        current.workspace_id = Some("w1".into());
        current.search_terms.push("focused".into());
        let mut previous = entry(Source::Workspace, "Previous");
        previous.workspace_id = Some("w2".into());
        app.entries = vec![current, previous];
        app.filtered = vec![0, 1];
        app.filtered_scores = vec![0; 2];
        app.previous_workspace_id = Some("w2".into());
        app.selected = 1;

        let backend = TestBackend::new(40, 10);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| { draw_list(f, &app, f.area()); }).unwrap();
        let text = buffer_text(&terminal);
        let buffer = terminal.backend().buffer();

        assert!(text.contains("  ◆  Current"));
        assert!(text.contains("  ◆  Previous"));
        assert!(!text.contains("├─ ◆"));
        assert!(buffer
            .content()
            .iter()
            .any(|cell| cell.symbol() == "◆" && cell.fg == app.theme.accent));
        assert!(buffer
            .content()
            .iter()
            .any(|cell| cell.symbol() == "◆" && cell.fg == app.theme.red));
    }

    #[test]
    fn current_workspace_marker_wins_over_stale_pin() {
        let mut app = App::new(Config::default(), Theme::load(false));
        let mut current = entry(Source::Workspace, "Current");
        current.workspace_id = Some("w1".into());
        current.search_terms.push("focused".into());
        app.previous_workspace_id = Some("w1".into());

        assert_eq!(
            entry_branch(&app, &current, false),
            ("  ◆  ", app.theme.accent)
        );
    }

    #[test]
    fn marked_entry_uses_a_yellow_diamond() {
        let mut app = App::new(Config::default(), Theme::load(false));
        let marked = entry(Source::Root, "Marked");
        app.pinned_entries.insert("root:Marked".into());

        assert_eq!(
            entry_branch(&app, &marked, false),
            ("  ◆  ", app.theme.yellow)
        );
    }

    #[test]
    fn mark_marker_wins_over_previous_workspace() {
        let mut app = App::new(Config::default(), Theme::load(false));
        let mut previous = entry(Source::Workspace, "Previous");
        previous.workspace_id = Some("w2".into());
        previous.action = EntryAction::FocusWorkspace { id: "w2".into() };
        app.previous_workspace_id = Some("w2".into());
        app.pinned_entries.insert("workspace:w2".into());

        assert_eq!(
            entry_branch(&app, &previous, false),
            ("  ◆  ", app.theme.yellow)
        );
    }

    #[test]
    fn mark_marker_wins_over_current_workspace() {
        let mut app = App::new(Config::default(), Theme::load(false));
        let mut current = entry(Source::Workspace, "Current");
        current.workspace_id = Some("w1".into());
        current.action = EntryAction::FocusWorkspace { id: "w1".into() };
        current.search_terms.push("focused".into());
        app.pinned_entries.insert("workspace:w1".into());

        assert_eq!(
            entry_branch(&app, &current, false),
            ("  ◆  ", app.theme.yellow)
        );
    }

    #[test]
    fn detailed_rows_only_expand_directory_sources() {
        let mut app = App::new(Config::default(), Theme::load(false));
        let mut workspace = entry(Source::Workspace, "dir: demo");
        workspace.path = PathBuf::from("/work/demo");
        workspace.subtitle = "agent:blocked · w1 tabs:2 panes:3".into();
        let mut agent = entry(Source::Agent, "claude · demo");
        agent.subtitle = "working · w1:p2 · w1:t1".into();
        let mut root = entry(Source::Root, "root-demo");
        root.path = PathBuf::from("/projects/root-demo");
        root.subtitle = "/projects/root-demo".into();
        app.entries = vec![workspace, agent, root];
        app.filtered = vec![0, 1, 2];
        app.filtered_scores = vec![0, 0, 0];

        let backend = TestBackend::new(90, 10);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| { draw_list(f, &app, f.area()); }).unwrap();
        let text = buffer_text(&terminal);
        let workspace_line = text.lines().find(|line| line.contains("● demo")).unwrap();
        let agent_line = text
            .lines()
            .find(|line| line.contains("⠋ claude · demo"))
            .unwrap();

        assert!(!text.contains("/work/demo"));
        assert!(!workspace_line.contains("demo  blocked"));
        assert!(workspace_line.find("blocked · 2 tabs · 3 panes").unwrap() > 50);
        assert!(agent_line.find("working · w1:t1 · w1:p2").unwrap() > 50);
        assert!(text.contains("/projects/root-demo"));
    }

    #[test]
    fn list_renders_source_groups_as_a_tree() {
        let mut app = App::new(Config::default(), Theme::load(false));
        app.entries = vec![
            entry(Source::Agent, "Claude"),
            entry(Source::Agent, "Codex"),
            entry(Source::Root, "Dotfiles"),
        ];
        app.filtered = vec![0, 1, 2];
        app.filtered_scores = vec![0; 3];

        let backend = TestBackend::new(40, 12);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| { draw_list(f, &app, f.area()); }).unwrap();
        let text = buffer_text(&terminal);

        assert!(text.contains(" ▾ agent "));
        assert!(text.contains(&format!(
            "  ├─ {} Claude",
            status_icon_at(&Source::Agent, "", 0)
        )));
        assert!(text.contains(&format!(
            "  └─ {} Codex",
            status_icon_at(&Source::Agent, "", 0)
        )));
        assert!(text.contains(" ▾ root "));
        assert!(text.contains("  └─ Dotfiles"));
    }

    #[test]
    fn vim_mode_uses_normal_keys_then_searches_with_slash() {
        let mut app = App::new(Config::default(), Theme::load(false));
        app.config.picker.vim_mode = true;
        handle_key(&mut app, key(KeyCode::Char('j')));
        assert!(app.query.is_empty());

        handle_key(&mut app, key(KeyCode::Char('a')));
        assert_eq!(app.source_filter, Some(Source::Agent));

        handle_key(&mut app, key(KeyCode::Char('/')));
        assert_eq!(app.input_mode, InputMode::Search);
        assert_eq!(app.source_filter, Some(Source::Agent));

        handle_key(&mut app, key(KeyCode::Char('j')));
        assert_eq!(app.query, "j");

        handle_key(&mut app, key(KeyCode::Esc));
        assert_eq!(app.input_mode, InputMode::Normal);
    }

    #[test]
    fn vim_filter_search_starts_search_after_source_key() {
        let mut app = App::new(Config::default(), Theme::load(false));
        app.config.picker.vim_mode = true;
        app.config.picker.vim_filter_search = true;

        handle_key(&mut app, key(KeyCode::Char('a')));
        assert_eq!(app.source_filter, Some(Source::Agent));
        assert_eq!(app.input_mode, InputMode::Search);

        handle_key(&mut app, key(KeyCode::Char('c')));
        assert_eq!(app.query, "c");
    }

    #[test]
    fn question_mark_toggles_registry_help_overlay() {
        let mut app = App::new(Config::default(), Theme::load(false));
        handle_key(&mut app, key(KeyCode::Char('?')));
        assert_eq!(app.input_mode, InputMode::Help);

        let backend = TestBackend::new(80, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| { draw(f, &app); }).unwrap();
        let text = buffer_text(&terminal);
        assert!(text.contains(" Keybindings "));
        assert!(text.contains("toggle preview"));
        assert!(text.contains("agents"));
        assert!(!text.contains("?/?"));

        handle_key(&mut app, key(KeyCode::Char('?')));
        assert_eq!(app.input_mode, InputMode::Normal);
    }

    #[test]
    fn registry_reports_active_toggle_state() {
        let mut app = App::new(Config::default(), Theme::load(false));
        app.preview = true;
        let preview = keybindings(&app)
            .into_iter()
            .find(|binding| binding.command == Command::TogglePreview)
            .unwrap();

        assert!(preview.is_active(&app));
    }

    #[test]
    fn registry_maps_ctrl_b_to_mark_without_stealing_enter() {
        let app = App::new(Config::default(), Theme::load(false));
        let mark = keybindings(&app)
            .into_iter()
            .find(|binding| binding.command == Command::ToggleMark)
            .unwrap();

        assert!(mark.label.contains("mark"));
        assert!(mark.matches(
            &app,
            KeyEvent::new(KeyCode::Char('b'), KeyModifiers::CONTROL)
        ));
        assert!(!mark.matches(&app, KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)));
        assert!(keybindings(&app)
            .into_iter()
            .find(|binding| binding.command == Command::Open)
            .unwrap()
            .matches(&app, KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)));
    }

    #[test]
    fn compact_footer_groups_movement_and_lists_filters() {
        let mut app = App::new(Config::default(), Theme::load(false));
        app.config.picker.vim_mode = true;
        let backend = TestBackend::new(110, 20);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| { draw(f, &app); }).unwrap();
        let text = buffer_text(&terminal);

        assert!(text.contains("j/k up/down"));
        assert!(text.contains("a agent"));
        assert!(text.contains("z zoxide"));
        assert!(!text.contains("k move up"));
    }

    #[test]
    fn mouse_scroll_moves_selection_inside_results() {
        let mut app = App::new(Config::default(), Theme::load(false));
        app.entries = vec![entry(Source::Workspace, "one"), entry(Source::Workspace, "two")];
        app.filtered = vec![0, 1];
        let hits = ListHits {
            area: Rect::new(0, 3, 40, 10),
            rows: vec![(4..5, 0), (5..6, 1)],
        };

        handle_mouse(
            &mut app,
            MouseEvent { kind: MouseEventKind::ScrollDown, column: 1, row: 4, modifiers: KeyModifiers::NONE },
            &hits,
        );
        assert_eq!(app.selected, 1);

        handle_mouse(
            &mut app,
            MouseEvent { kind: MouseEventKind::ScrollUp, column: 1, row: 4, modifiers: KeyModifiers::NONE },
            &hits,
        );
        assert_eq!(app.selected, 0);
    }

    #[test]
    fn mouse_click_selects_then_opens_result() {
        let mut app = App::new(Config::default(), Theme::load(false));
        app.entries = vec![entry(Source::Workspace, "one"), entry(Source::Workspace, "two")];
        app.filtered = vec![0, 1];
        let hits = ListHits {
            area: Rect::new(0, 3, 40, 10),
            rows: vec![(4..5, 0), (5..6, 1)],
        };
        let click = MouseEvent { kind: MouseEventKind::Down(MouseButton::Left), column: 1, row: 5, modifiers: KeyModifiers::NONE };

        assert!(matches!(handle_mouse(&mut app, click, &hits), Action::Continue));
        assert_eq!(app.selected, 1);
        assert!(matches!(handle_mouse(&mut app, click, &hits), Action::Open));
    }

    #[test]
    fn mouse_ignores_input_outside_results() {
        let mut app = App::new(Config::default(), Theme::load(false));
        app.entries = vec![entry(Source::Workspace, "one"), entry(Source::Workspace, "two")];
        app.filtered = vec![0, 1];
        let hits = ListHits {
            area: Rect::new(0, 3, 40, 10),
            rows: vec![(4..5, 0), (5..6, 1)],
        };

        handle_mouse(
            &mut app,
            MouseEvent { kind: MouseEventKind::ScrollDown, column: 50, row: 4, modifiers: KeyModifiers::NONE },
            &hits,
        );
        assert_eq!(app.selected, 0);
    }

    #[test]
    fn input_modes_transition_exclusively() {
        let mut app = App::new(Config::default(), Theme::load(false));
        app.config.picker.vim_mode = true;
        app.config.picker.vim_filter_search = true;
        assert_eq!(app.input_mode, InputMode::Normal);

        handle_key(&mut app, key(KeyCode::Char('a')));
        assert_eq!(app.input_mode, InputMode::Search);

        handle_key(&mut app, key(KeyCode::Char('?')));
        assert_eq!(app.input_mode, InputMode::Help);

        handle_key(&mut app, key(KeyCode::Esc));
        assert_eq!(app.input_mode, InputMode::Normal);
    }
}
