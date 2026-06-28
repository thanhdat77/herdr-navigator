use std::{
    env,
    process::{self, Command},
};

mod app;
mod config;
mod herdr;
mod integrations;
mod matcher;
mod model;
mod paths;
mod sources;
mod theme;
mod tui;

use app::App;
use config::Config;
use herdr::herdr_bin;
use theme::Theme;
use tui::tui_loop;

fn main() {
    match env::args().nth(1).as_deref() {
        Some("open") => open_picker(),
        Some("ui") => run_ui(),
        Some("list") => debug_list(),
        _ => {
            eprintln!("usage: herdr-picker-plus <open|ui|list>");
            process::exit(2);
        }
    }
}

fn open_picker() -> ! {
    let plugin = env::var("HERDR_PLUGIN_ID").unwrap_or_else(|_| "herdr-picker-plus".into());
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
        eprintln!("picker plus error: {e}");
        process::exit(1);
    }
    process::exit(0);
}

fn debug_list() {
    let mut app = App::new(Config::load(), Theme::load(true));
    app.refresh();
    for e in app.entries {
        println!("{}\t{}\t{}", e.source_name(), e.title, e.path.display());
    }
}
