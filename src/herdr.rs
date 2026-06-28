use std::{env, process::Command};

use serde_json::Value;

pub(crate) fn herdr_bin() -> String {
    env::var("HERDR_BIN_PATH").unwrap_or_else(|_| "herdr".into())
}
pub(crate) fn herdr_json<const N: usize>(args: [&str; N]) -> Result<Value, String> {
    let out = Command::new(herdr_bin())
        .args(args)
        .output()
        .map_err(|e| e.to_string())?;
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).to_string());
    }
    serde_json::from_slice(&out.stdout).map_err(|e| e.to_string())
}
pub(crate) fn run_herdr<const N: usize>(args: [&str; N]) -> Result<(), String> {
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

pub(crate) fn notify_done(body: &str) {
    notify(body, "done");
}

pub(crate) fn notify_error(body: &str) {
    notify(body, "request");
}

fn notify(body: &str, sound: &str) {
    let body = truncate(body, 180);
    let _ = Command::new(herdr_bin())
        .args([
            "notification",
            "show",
            "Picker Plus",
            "--body",
            &body,
            "--position",
            "top-right",
            "--sound",
            sound,
        ])
        .status();
}

fn truncate(value: &str, max_chars: usize) -> String {
    let mut out: String = value.chars().take(max_chars).collect();
    if value.chars().count() > max_chars {
        out.push('…');
    }
    out
}
