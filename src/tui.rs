use std::io;

use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
    Frame, Terminal,
};

use crate::{
    app::App,
    model::{Entry, EntryAction, Source},
    theme::Theme,
};

// Key icons: plain Unicode only, no Nerd Font dependency.
const KEY_CTRL_S: &str = "⌃S";
const KEY_CTRL_A: &str = "⌃A";
const KEY_CTRL_X: &str = "⌃X";
const KEY_CTRL_O: &str = "⌃O";
const KEY_ENTER: &str = "↵";

pub(crate) fn tui_loop(app: &mut App) -> io::Result<()> {
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
                Action::CloseWorkspace => {
                    if let Err(e) = app.close_selected_workspace() {
                        crate::herdr::notify_error(&format!("Close failed: {e}"));
                    }
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
    CloseWorkspace,
}

fn handle_key(app: &mut App, key: KeyEvent) -> Action {
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        match key.code {
            KeyCode::Char('a') => app.set_filter(Some(Source::Agent)),
            KeyCode::Char('q') => app.set_filter(Some(Source::QuickAction)),
            KeyCode::Char('w') => app.set_filter(Some(Source::Workspace)),
            KeyCode::Char('p') => app.set_filter(Some(Source::Project)),
            KeyCode::Char('z') => app.set_filter(Some(Source::Zoxide)),
            KeyCode::Char('r') => app.set_filter(Some(Source::Root)),
            KeyCode::Char('s') => app.set_filter(Some(Source::Server)),
            KeyCode::Char('o') => app.preview = !app.preview,
            KeyCode::Char('u') => {
                app.query.clear();
                app.set_filter(None);
            }
            KeyCode::Char('x') => return Action::CloseWorkspace,
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

fn draw(f: &mut Frame, app: &App) {
    let area = f.area();
    f.render_widget(Clear, area);
    let outer = Block::default()
        .style(Style::default().bg(app.theme.panel_bg))
        .title(" Herdr Picker Plus ")
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

    draw_list(f, app, body[0]);
    if app.preview {
        draw_preview(f, app, body[1]);
    }

    let help = format!(
        "{KEY_CTRL_S} servers  {KEY_CTRL_A} agents  {KEY_CTRL_X} close  {KEY_CTRL_O} preview  {KEY_ENTER} open  Esc quit"
    );
    f.render_widget(
        Paragraph::new(help).style(
            Style::default()
                .fg(app.theme.overlay0)
                .bg(app.theme.panel_bg),
        ),
        rows[2],
    );
}

fn draw_list(f: &mut Frame, app: &App, area: Rect) {
    let show_scores = !app.query.trim().is_empty();
    let row_width = area.width.saturating_sub(3) as usize;
    let items: Vec<ListItem> = app
        .filtered
        .iter()
        .enumerate()
        .map(|(row, idx)| {
            let e = &app.entries[*idx];
            let color = source_color(&app.theme, &e.source);
            let source = format!("[{:<7}] ", truncate(e.source_name(), 7));
            let subtitle = if e.subtitle.is_empty() {
                String::new()
            } else {
                format!("  {}", e.subtitle)
            };
            let score = show_scores
                .then(|| app.filtered_scores.get(row).map(|s| format!("score {s}")))
                .flatten();
            let left_len =
                source.chars().count() + e.title.chars().count() + subtitle.chars().count();
            let spacer = score
                .as_ref()
                .map(|s| {
                    " ".repeat(
                        row_width
                            .saturating_sub(left_len + s.chars().count())
                            .max(2),
                    )
                })
                .unwrap_or_default();
            let mut spans = vec![
                Span::styled(source, Style::default().fg(color)),
                Span::styled(&e.title, Style::default().fg(app.theme.text)),
                Span::styled(subtitle, Style::default().fg(app.theme.subtext0)),
            ];
            if let Some(score) = score {
                spans.push(Span::raw(spacer));
                spans.push(Span::styled(score, Style::default().fg(app.theme.overlay0)));
            }
            ListItem::new(Line::from(spans))
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
        EntryAction::InvokePluginAction { .. } => "invoke Herdr plugin action",
        EntryAction::OpenServer { .. } if app.matching_server_workspace(e).is_some() => {
            "focus matching server workspace"
        }
        EntryAction::OpenServer { .. } => "create server workspace + connect tab",
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
    lines.join("\n")
}

fn truncate(value: &str, max: usize) -> String {
    value.chars().take(max).collect()
}

fn source_color(theme: &Theme, source: &Source) -> Color {
    match source {
        Source::Workspace => theme.green,
        Source::Project => theme.mauve,
        Source::Zoxide => theme.blue,
        Source::Root => theme.teal,
        Source::Agent => theme.yellow,
        Source::Server => theme.green,
        Source::QuickAction => theme.peach,
        Source::Integration => theme.red,
    }
}
