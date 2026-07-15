use std::{
    env,
    process::{self, Command},
};

mod app;
mod config;
mod herdr;
mod integrations;
mod keymap;
mod matcher;
mod model;
mod paths;
mod sources;
mod theme;
mod tui;
mod update;

use app::App;
use config::Config;
use herdr::herdr_bin;
use theme::Theme;
use tui::tui_loop;

fn main() {
    match env::args().nth(1).as_deref() {
        Some("open") => open_picker(),
        Some("open-side") => open_side_picker(),
        Some("jump-back") => jump_back(),
        Some("ui") => run_ui(env::args().nth(2).as_deref() == Some("--side")),
        Some("list") => debug_list(),
        _ => {
            eprintln!("usage: herdr-picker-plus <open|open-side|jump-back|ui|list>");
            process::exit(2);
        }
    }
}

// Must match the pane `title` values in herdr-plugin.toml; Herdr exposes them
// as labels in `pane list`.
const PICKER_PANE_LABEL: &str = "Herdr Navigator";
const SIDE_PANE_LABEL: &str = "Navigator Side";

enum PickerDecision {
    Open,
    Focus(String),
}

enum SideDecision {
    Open,
    Focus(String),
    Close(String),
}

fn pane_in_focused_workspace<'a>(
    pane_json: &'a serde_json::Value,
    label: &str,
) -> Option<(&'a serde_json::Value, &'a serde_json::Value)> {
    let panes = pane_json
        .pointer("/result/panes")
        .and_then(|v| v.as_array())?;
    let focused = panes
        .iter()
        .find(|p| p.get("focused").and_then(|v| v.as_bool()) == Some(true))?;
    let workspace = focused.get("workspace_id").and_then(|v| v.as_str())?;
    let target = panes.iter().find(|p| {
        p.get("label").and_then(|v| v.as_str()) == Some(label)
            && p.get("workspace_id").and_then(|v| v.as_str()) == Some(workspace)
    })?;
    Some((focused, target))
}

fn picker_pane_decision(pane_json: &serde_json::Value) -> PickerDecision {
    pane_in_focused_workspace(pane_json, PICKER_PANE_LABEL)
        .and_then(|(_, picker)| picker.get("pane_id")?.as_str())
        .map(|id| PickerDecision::Focus(id.into()))
        .unwrap_or(PickerDecision::Open)
}

fn open_picker() -> ! {
    let json = herdr::herdr_json(["pane", "list"]).unwrap_or(serde_json::Value::Null);
    match picker_pane_decision(&json) {
        PickerDecision::Open => open_plugin_pane("picker", &["--focus"]),
        PickerDecision::Focus(id) => run_plugin_pane_cmd("focus", &id),
    }
}

// Launch-or-focus, toggle on repeat — same UX as herdr-file-viewer's side pane,
// scoped to the focused workspace. Any parse failure degrades to Open.
fn side_pane_decision(pane_json: &serde_json::Value) -> SideDecision {
    let Some((focused, side)) = pane_in_focused_workspace(pane_json, SIDE_PANE_LABEL) else {
        return SideDecision::Open;
    };
    let Some(id) = side.get("pane_id").and_then(|v| v.as_str()) else {
        return SideDecision::Open;
    };
    if focused.get("pane_id").and_then(|v| v.as_str()) == Some(id) {
        SideDecision::Close(id.into())
    } else {
        SideDecision::Focus(id.into())
    }
}

fn open_side_picker() -> ! {
    let json = herdr::herdr_json(["pane", "list"]).unwrap_or(serde_json::Value::Null);
    match side_pane_decision(&json) {
        SideDecision::Open => open_plugin_pane(
            "picker-side",
            &["--placement", "split", "--direction", "right", "--focus"],
        ),
        SideDecision::Focus(id) => run_plugin_pane_cmd("focus", &id),
        SideDecision::Close(id) => run_plugin_pane_cmd("close", &id),
    }
}

fn open_plugin_pane(entrypoint: &str, extra: &[&str]) -> ! {
    let plugin = env::var("HERDR_PLUGIN_ID").unwrap_or_else(|_| "herdr-picker-plus".into());
    let status = Command::new(herdr_bin())
        .args([
            "plugin",
            "pane",
            "open",
            "--plugin",
            &plugin,
            "--entrypoint",
            entrypoint,
        ])
        .args(extra)
        .status();
    match status {
        Ok(s) => process::exit(s.code().unwrap_or(0)),
        Err(e) => {
            eprintln!("failed to open picker pane: {e}");
            process::exit(1);
        }
    }
}

fn run_plugin_pane_cmd(cmd: &str, pane_id: &str) -> ! {
    let status = Command::new(herdr_bin())
        .args(["plugin", "pane", cmd, pane_id])
        .status();
    process::exit(status.ok().and_then(|s| s.code()).unwrap_or(1));
}

fn jump_back() -> ! {
    let config = Config::load();
    match app::jump_back(&config) {
        Ok(label) => {
            herdr::notify_done(&format!("Jumped back to {label}"));
            process::exit(0);
        }
        Err(error) => {
            herdr::notify_error(&format!("Jump back failed: {error}"));
            eprintln!("jump back failed: {error}");
            process::exit(1);
        }
    }
}

fn run_ui(persist: bool) -> ! {
    let config = Config::load();
    let check_updates = config.picker.check_updates;
    let theme = Theme::load(config.theme.inherit_herdr);
    let mut app = App::new(config, theme);
    app.refresh();
    let update_check = check_updates.then(update::check_in_background);

    if let Err(e) = tui_loop(&mut app, persist, update_check) {
        eprintln!("picker plus error: {e}");
        process::exit(1);
    }
    process::exit(0);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pane(id: &str, ws: &str, label: Option<&str>, focused: bool) -> serde_json::Value {
        let mut p = serde_json::json!({"pane_id": id, "workspace_id": ws, "focused": focused});
        if let Some(label) = label {
            p["label"] = label.into();
        }
        p
    }

    fn pane_list(panes: Vec<serde_json::Value>) -> serde_json::Value {
        serde_json::json!({"id": "cli:pane:list", "result": {"type": "pane_list", "panes": panes}})
    }

    #[test]
    fn overlay_picker_opens_once_then_focuses_existing() {
        let no_picker = pane_list(vec![pane("w1:p1", "w1", None, true)]);
        assert!(matches!(
            picker_pane_decision(&no_picker),
            PickerDecision::Open
        ));

        let existing_picker = pane_list(vec![
            pane("w1:p1", "w1", None, true),
            pane("w1:p2", "w1", Some(PICKER_PANE_LABEL), false),
        ]);
        assert!(
            matches!(picker_pane_decision(&existing_picker), PickerDecision::Focus(id) if id == "w1:p2")
        );

        let other_workspace = pane_list(vec![
            pane("w1:p1", "w1", None, true),
            pane("w2:p2", "w2", Some(PICKER_PANE_LABEL), false),
        ]);
        assert!(matches!(
            picker_pane_decision(&other_workspace),
            PickerDecision::Open
        ));
    }

    #[test]
    fn side_pane_toggles_open_focus_close() {
        let no_side = pane_list(vec![pane("w1:p1", "w1", None, true)]);
        assert!(matches!(side_pane_decision(&no_side), SideDecision::Open));

        let unfocused_side = pane_list(vec![
            pane("w1:p1", "w1", None, true),
            pane("w1:p2", "w1", Some(SIDE_PANE_LABEL), false),
        ]);
        assert!(
            matches!(side_pane_decision(&unfocused_side), SideDecision::Focus(id) if id == "w1:p2")
        );

        let focused_side = pane_list(vec![
            pane("w1:p1", "w1", None, false),
            pane("w1:p2", "w1", Some(SIDE_PANE_LABEL), true),
        ]);
        assert!(
            matches!(side_pane_decision(&focused_side), SideDecision::Close(id) if id == "w1:p2")
        );

        let other_workspace = pane_list(vec![
            pane("w1:p1", "w1", None, true),
            pane("w2:p2", "w2", Some(SIDE_PANE_LABEL), false),
        ]);
        assert!(matches!(
            side_pane_decision(&other_workspace),
            SideDecision::Open
        ));

        assert!(matches!(
            side_pane_decision(&serde_json::Value::Null),
            SideDecision::Open
        ));
    }
}

fn debug_list() {
    let mut app = App::new(Config::load(), Theme::load(true));
    app.refresh();
    for e in app.entries {
        println!("{}\t{}\t{}", e.source_name(), e.title, e.path.display());
    }
}
