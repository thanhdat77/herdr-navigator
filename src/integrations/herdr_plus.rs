use std::{
    env, fs,
    path::{Path, PathBuf},
};

use serde_json::Value;

use crate::{
    herdr::{herdr_json, run_herdr},
    model::{Entry, EntryAction, Project, Source},
    paths::{expand_path, herdr_plus_projects_dir, home},
};

pub(crate) fn collect_projects() -> Vec<Entry> {
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
        let Ok(project) = load_project_file(&path) else {
            continue;
        };
        let p = expand_path(&project.working_dir);
        out.push(Entry {
            source: Source::Project,
            title: project.name.clone(),
            subtitle: project.description.clone(),
            path: p,
            workspace_id: None,
            workspace_label: None,
            agent_target: None,
            project: Some(project),
            action: EntryAction::OpenProject,
            source_label: None,
            search_terms: vec![],
        });
    }
    out
}

pub(crate) fn load_project_template(name: &str) -> Result<Project, String> {
    load_project_file(&herdr_plus_projects_dir().join(template_filename(name)?))
}

fn template_filename(name: &str) -> Result<PathBuf, String> {
    let name = name.trim();
    if name.is_empty() || Path::new(name).file_name().and_then(|part| part.to_str()) != Some(name) {
        return Err("directory_template must be a Herdr Plus project filename".into());
    }
    let mut file = PathBuf::from(name);
    if file.extension().is_none() {
        file.set_extension("toml");
    }
    if file.extension().and_then(|extension| extension.to_str()) != Some("toml") {
        return Err("directory_template must use the .toml extension".into());
    }
    Ok(file)
}

fn load_project_file(path: &Path) -> Result<Project, String> {
    let source = fs::read_to_string(path)
        .map_err(|error| format!("failed to read template {}: {error}", path.display()))?;
    toml::from_str(&source).map_err(|error| format!("invalid template {}: {error}", path.display()))
}

pub(crate) fn quick_actions_entry() -> Entry {
    Entry {
        source: Source::QuickAction,
        title: "Herdr Plus Quick Actions".into(),
        subtitle: "open the Herdr Plus quick-action picker".into(),
        path: env::current_dir().unwrap_or_else(|_| home()),
        workspace_id: None,
        workspace_label: None,
        agent_target: None,
        project: None,
        action: EntryAction::InvokePluginAction {
            action: "cloudmanic.herdr-plus.quick-actions".into(),
        },
        source_label: None,
        search_terms: vec![],
    }
}

pub(crate) fn bootstrap_project_tabs(
    project: &Project,
    create_json: &Value,
    cwd: &Path,
) -> Result<(), String> {
    let workspace_id = create_json
        .pointer("/result/workspace/workspace_id")
        .and_then(|v| v.as_str())
        .ok_or("workspace create did not return workspace_id")?;
    if project.tabs.is_empty() {
        return Ok(());
    }
    let root_pane = create_json
        .pointer("/result/root_pane/pane_id")
        .and_then(|v| v.as_str())
        .ok_or("workspace create did not return root pane")?;
    build_project_tabs(project, workspace_id, Some(root_pane), cwd)
}

pub(crate) fn append_project_tabs(
    project: &Project,
    workspace_id: &str,
    cwd: &Path,
) -> Result<(), String> {
    build_project_tabs(project, workspace_id, None, cwd)
}

fn build_project_tabs(
    project: &Project,
    workspace_id: &str,
    first_root: Option<&str>,
    cwd: &Path,
) -> Result<(), String> {
    let mut runs = Vec::new();

    for (tab_index, tab) in project.tabs.iter().enumerate() {
        let tab_root = match (tab_index, first_root) {
            (0, Some(root_pane)) => {
                let _ = run_herdr(["tab", "rename", &format!("{workspace_id}:t1"), &tab.name]);
                root_pane.to_string()
            }
            _ => herdr_json([
                "tab",
                "create",
                "--workspace",
                workspace_id,
                "--cwd",
                &cwd.display().to_string(),
                "--label",
                &tab.name,
                "--no-focus",
            ])?
            .pointer("/result/root_pane/pane_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| format!("tab create did not return root pane for {}", tab.name))?
            .to_string(),
        };

        let mut previous_pane = tab_root.clone();
        for (pane_index, pane) in tab.effective_panes().iter().enumerate() {
            let pane_id = if pane_index == 0 {
                tab_root.clone()
            } else {
                let direction = pane.split.as_deref().unwrap_or("down");
                herdr_json([
                    "pane",
                    "split",
                    &previous_pane,
                    "--direction",
                    direction,
                    "--no-focus",
                ])?
                .pointer("/result/pane/pane_id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    format!(
                        "pane split did not return pane {} for {}",
                        pane_index + 1,
                        tab.name
                    )
                })?
                .to_string()
            };
            if let Some(label) = pane
                .label
                .as_deref()
                .filter(|label| !label.trim().is_empty())
            {
                let _ = run_herdr(["pane", "rename", &pane_id, label.trim()]);
            }
            if let Some(command) = pane
                .command
                .as_deref()
                .filter(|command| !command.trim().is_empty())
            {
                runs.push((pane_id.clone(), command.to_string()));
            }
            previous_pane = pane_id;
        }
    }

    for (pane, command) in runs {
        let _ = run_herdr(["pane", "run", &pane, &command]);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_reusable_template_by_safe_filename() {
        assert_eq!(
            template_filename("default"),
            Ok(std::path::PathBuf::from("default.toml"))
        );
        assert_eq!(
            template_filename("default.toml"),
            Ok(std::path::PathBuf::from("default.toml"))
        );
        assert!(template_filename("../default.toml").is_err());

        let path = env::temp_dir().join(format!(
            "herdr-navigator-template-{}.toml",
            std::process::id()
        ));
        fs::write(
            &path,
            r#"
            name = "Reusable"
            working_dir = "/ignored"

            [[tabs]]
            name = "agent"
            command = "claude"
            "#,
        )
        .unwrap();
        let project = load_project_file(&path).unwrap();
        let _ = fs::remove_file(path);

        assert_eq!(project.name, "Reusable");
        assert_eq!(project.tabs[0].name, "agent");
    }

    #[test]
    fn parses_split_pane_project_tabs() {
        let project: Project = toml::from_str(
            r#"
name = "demo"
working_dir = "~/"

[[tabs]]
name = "server"

[[tabs.panes]]
label = "Server"
command = "serve"

[[tabs.panes]]
label = "Terminal"
split = "right"
"#,
        )
        .unwrap();

        let panes = &project.tabs[0].panes;
        assert_eq!(panes.len(), 2);
        assert_eq!(panes[0].label.as_deref(), Some("Server"));
        assert_eq!(panes[0].command.as_deref(), Some("serve"));
        assert_eq!(panes[1].label.as_deref(), Some("Terminal"));
        assert_eq!(panes[1].split.as_deref(), Some("right"));
    }

    #[cfg(unix)]
    #[test]
    fn bootstraps_split_panes_before_running_commands() {
        use std::{
            env,
            os::unix::fs::PermissionsExt,
            sync::{Mutex, OnceLock},
            time::{SystemTime, UNIX_EPOCH},
        };

        static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        let _lock = ENV_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap();
        let dir = env::temp_dir().join(format!(
            "herdr-navigator-split-test-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).unwrap();
        let log = dir.join("calls");
        let fake = dir.join("herdr");
        fs::write(
            &fake,
            format!(
                r#"#!/bin/sh
printf '%s\n' "$*" >> '{}'
case "$1 $2 $3" in
  "tab create --workspace") printf '%s\n' '{{"result":{{"root_pane":{{"pane_id":"w1:p2"}}}}}}' ;;
  "pane split w1:p2") printf '%s\n' '{{"result":{{"pane":{{"pane_id":"w1:p3"}}}}}}' ;;
  "pane split w1:p3") printf '%s\n' '{{"result":{{"pane":{{"pane_id":"w1:p4"}}}}}}' ;;
esac
"#,
                log.display()
            ),
        )
        .unwrap();
        fs::set_permissions(&fake, fs::Permissions::from_mode(0o755)).unwrap();

        let project: Project = toml::from_str(
            r#"
name = "demo"
working_dir = "/tmp/project"

[[tabs]]
name = "agent"
command = "claude"

[[tabs]]
name = "server"

[[tabs.panes]]
label = "Server"
command = "serve"

[[tabs.panes]]
label = "Terminal"
split = "right"

[[tabs.panes]]
label = "Logs"
command = "tail"
"#,
        )
        .unwrap();
        let create_json = serde_json::json!({
            "result": {
                "workspace": { "workspace_id": "w1" },
                "root_pane": { "pane_id": "w1:p1" }
            }
        });
        let previous_bin = env::var_os("HERDR_BIN_PATH");
        env::set_var("HERDR_BIN_PATH", &fake);
        let bootstrap_result =
            bootstrap_project_tabs(&project, &create_json, Path::new("/tmp/project"));
        let bootstrap_calls = fs::read_to_string(&log).unwrap();
        fs::write(&log, "").unwrap();
        let append_result = append_project_tabs(&project, "w1", Path::new("/tmp/project"));
        let append_calls = fs::read_to_string(&log).unwrap();
        match previous_bin {
            Some(path) => env::set_var("HERDR_BIN_PATH", path),
            None => env::remove_var("HERDR_BIN_PATH"),
        }
        fs::remove_dir_all(&dir).unwrap();

        bootstrap_result.unwrap();
        assert_eq!(
            bootstrap_calls.lines().collect::<Vec<_>>(),
            vec![
                "tab rename w1:t1 agent",
                "tab create --workspace w1 --cwd /tmp/project --label server --no-focus",
                "pane rename w1:p2 Server",
                "pane split w1:p2 --direction right --no-focus",
                "pane rename w1:p3 Terminal",
                "pane split w1:p3 --direction down --no-focus",
                "pane rename w1:p4 Logs",
                "pane run w1:p1 claude",
                "pane run w1:p2 serve",
                "pane run w1:p4 tail",
            ]
        );
        append_result.unwrap();
        let append_calls = append_calls.lines().collect::<Vec<_>>();
        assert_eq!(
            &append_calls[..2],
            &[
                "tab create --workspace w1 --cwd /tmp/project --label agent --no-focus",
                "tab create --workspace w1 --cwd /tmp/project --label server --no-focus",
            ]
        );
        assert!(!append_calls
            .iter()
            .any(|call| call.starts_with("tab rename")));
    }
}
