# Herdr Workdir Picker

Herdr-native picker for the workflow:

```text
search workdir -> focus existing workspace -> or create workspace
```

Built like a Herdr extension: Rust + `ratatui`/`crossterm`, Herdr theme tokens, plugin action + overlay pane. Search engine is configurable: `nucleo`, `skim`, or `simple`.

## Features

Sources:

- open Herdr workspaces
- Herdr Plus projects (`cloudmanic.herdr-plus` config)
- zoxide dirs
- configured root scans
- agents from open panes

Keys:

| Key | Action |
| --- | --- |
| type | fuzzy search |
| `ctrl+w` | workspaces only |
| `ctrl+p` | Herdr Plus projects only |
| `ctrl+z` | zoxide only |
| `ctrl+r` | root scan only |
| `ctrl+a` | agents only |
| `ctrl+o` | toggle preview |
| `ctrl+u` | clear query/filter |
| `tab` | cycle filters |
| `enter` | focus/create/open |
| `esc` | close |

## Local install

```bash
cd /home/fenix/workspace/herdr-workdir-picker
cargo build --release
herdr plugin link /home/fenix/workspace/herdr-workdir-picker
herdr plugin action invoke fenix.workdir-picker.open
```

## Config

```bash
herdr plugin config-dir fenix.workdir-picker
```

Edit `config.toml`:

```toml
[picker]
reuse_existing = true
create_missing = true
engine = "nucleo" # nucleo | skim | simple

[sources]
open_workspaces = true
herdr_plus_projects = true
zoxide = true
roots = true
agents = true

[theme]
inherit_herdr = true

[[roots]]
path = "~/workspace"
max_depth = 3
```

## Optional keybind

Add to `~/.config/herdr/config.toml`:

```toml
[[keys.command]]
key = "prefix+t"
type = "plugin_action"
command = "fenix.workdir-picker.open"
description = "workdir picker"
```

Reload:

```bash
herdr server reload-config
```

## Debug

```bash
./target/release/herdr-workdir-picker list
```
