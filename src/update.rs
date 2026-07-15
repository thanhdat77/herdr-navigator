use std::{
    fs,
    process::Command,
    sync::mpsc::{self, Receiver},
    thread,
    time::{SystemTime, UNIX_EPOCH},
};

use crate::paths::plugin_config_dir;

const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");
const RELEASE_REPO: &str = "https://github.com/thanhdat77/herdr-navigator.git";
const CACHE_SECONDS: u64 = 86_400;

pub(crate) fn check_in_background() -> Receiver<Option<String>> {
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let _ = sender.send(check_for_update());
    });
    receiver
}

fn check_for_update() -> Option<String> {
    let now = SystemTime::now().duration_since(UNIX_EPOCH).ok()?.as_secs();
    let cache_path = plugin_config_dir().join("update-check");
    if let Ok(cache) = fs::read_to_string(&cache_path) {
        if let Some(latest) = fresh_cached_release(&cache, now) {
            return newer_version(CURRENT_VERSION, &latest);
        }
    }

    let latest = fetch_latest_release().unwrap_or_else(|| CURRENT_VERSION.to_string());
    let _ = fs::create_dir_all(plugin_config_dir());
    let _ = fs::write(cache_path, format!("{now}\n{latest}\n"));
    newer_version(CURRENT_VERSION, &latest)
}

fn fetch_latest_release() -> Option<String> {
    let output = Command::new("git")
        .args([
            "-c",
            "http.lowSpeedLimit=1",
            "-c",
            "http.lowSpeedTime=5",
            "ls-remote",
            "--tags",
            "--refs",
            RELEASE_REPO,
            "v*",
        ])
        .env("GIT_TERMINAL_PROMPT", "0")
        .output()
        .ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8(output.stdout).ok())
        .flatten()
        .and_then(|tags| latest_release(&tags))
}

fn parse_version(value: &str) -> Option<[u64; 3]> {
    let mut parts = value.trim().trim_start_matches('v').split('.');
    let version = [
        parts.next()?.parse().ok()?,
        parts.next()?.parse().ok()?,
        parts.next()?.parse().ok()?,
    ];
    parts.next().is_none().then_some(version)
}

fn latest_release(tags: &str) -> Option<String> {
    let latest = tags
        .lines()
        .filter_map(|line| line.split_whitespace().nth(1))
        .filter_map(|reference| reference.strip_prefix("refs/tags/"))
        .filter_map(parse_version)
        .max()?;
    Some(format!("{}.{}.{}", latest[0], latest[1], latest[2]))
}

fn newer_version(current: &str, latest: &str) -> Option<String> {
    (parse_version(latest)? > parse_version(current)?).then(|| latest.to_string())
}

fn fresh_cached_release(cache: &str, now: u64) -> Option<String> {
    let mut lines = cache.lines();
    let checked = lines.next()?.parse::<u64>().ok()?;
    if now.checked_sub(checked)? > CACHE_SECONDS {
        return None;
    }
    let version = parse_version(lines.next()?)?;
    Some(format!("{}.{}.{}", version[0], version[1], version[2]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn newer_release_uses_latest_stable_semver_tag() {
        let tags = "a refs/tags/v0.3.0\nb refs/tags/v0.4.0-rc.1\nc refs/tags/v0.3.2\nd refs/tags/not-a-version\n";

        let latest = latest_release(tags).unwrap();
        assert_eq!(latest, "0.3.2");
        assert_eq!(newer_version("0.3.0", &latest), Some("0.3.2".into()));
        assert_eq!(newer_version("0.3.2", &latest), None);
    }

    #[test]
    fn cached_release_expires_after_one_day() {
        assert_eq!(
            fresh_cached_release("100\n0.3.2\n", 100 + CACHE_SECONDS),
            Some("0.3.2".into())
        );
        assert_eq!(
            fresh_cached_release("100\n0.3.2\n", 100 + CACHE_SECONDS + 1),
            None
        );
        assert_eq!(fresh_cached_release("invalid", 100), None);
    }
}
