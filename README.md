# Herdr Picker Plus

Herdr Picker Plus is a Herdr-native **picker center**: one overlay for jumping to workspaces, projects, directories, agents, and Herdr Plus actions.

```text
prefix+t -> search -> Enter
```

It is similar in spirit to `tv`, but deeply integrated with Herdr. Instead of only selecting a path, it can focus existing Herdr state, create workspaces, apply Herdr Plus project layouts, focus agents, and launch Herdr Plus Quick Actions.

## Overview

### What makes it stand out

- **Picker center for Herdr**: one place to search workspaces, projects, directories, agents, and actions.
- **Reuse-first workflow**: focuses matching open workspaces without confusing project and directory workspaces that share the same path.
- **Herdr Plus integration**: opens Herdr Plus project templates and can jump into Herdr Plus Quick Actions.
- **Workspace creation**: zoxide/root results can create a Herdr workspace directly.
- **Agent-aware**: agent panes appear as searchable entries and can be focused from the picker.
- **Theme-aware**: maps supported Herdr themes locally and applies `[theme.custom]` overrides.
- **No external picker UI**: the TUI is built in Rust with `ratatui`; no `fzf`/`tv` runtime dependency.
- **Plugin integration contract**: other tools can appear in the picker with a simple command/JSON list-open API.

### Sources

| Source | Reads | Enter |
| --- | --- | --- |
| `workspace` | `herdr workspace list` + pane cwd | focus the exact selected workspace |
| `project` | Herdr Plus `projects/*.toml` | focus existing cwd or create workspace + project tabs |
| `quick` | Herdr Plus Quick Actions | open Quick Actions picker |
| `zoxide` | `zoxide query -l` | focus existing cwd or create workspace |
| `root` | configured filesystem roots | focus existing cwd or create workspace |
| `agent` | agent panes from `herdr pane list` | focus agent pane |
| `plugin` | configured `[[integrations]]` commands | run configured open command |

## Requirements

Required:

- Herdr `0.7.0` or newer

Required only when building from source:

- Rust stable
- Cargo

Optional integrations:

- `zoxide` for the `zoxide` source
- Herdr Plus for the `project` and `quick` sources

## Install

Choose one install path.

### Option A: install from release archive

1. Download the archive for your platform from the GitHub Release page.
2. Extract it somewhere stable, for example:

   ```bash
   mkdir -p ~/.local/share/herdr/plugins
   tar -xzf herdr-picker-plus-linux-x86_64.tar.gz -C ~/.local/share/herdr/plugins
   ```

3. Link the extracted plugin directory:

   ```bash
   herdr plugin link ~/.local/share/herdr/plugins/herdr-picker-plus
   ```

4. Verify Herdr sees the plugin:

   ```bash
   herdr plugin list
   herdr plugin action list --plugin herdr-picker-plus
   ```

### Option B: install from source

1. Clone the repo:

   ```bash
   git clone <repo-url>
   cd herdr-picker-plus
   ```

2. Build the binary:

   ```bash
   cargo build --release
   ```

3. Link the local plugin directory:

   ```bash
   herdr plugin link "$PWD"
   ```

4. Verify:

   ```bash
   herdr plugin list
   herdr plugin action list --plugin herdr-picker-plus
   ```

## First run

Run the picker once without a keybinding:

```bash
herdr plugin action invoke herdr-picker-plus.open
```

If the overlay opens, installation is working.

## Add a keybinding

Add this to `~/.config/herdr/config.toml`:

```toml
[[keys.command]]
key = "prefix+t"
type = "plugin_action"
command = "herdr-picker-plus.open"
description = "picker center"
```

Reload Herdr:

```bash
herdr server reload-config
```

Now use:

```text
prefix+t
```

## Usage

| Key | Action |
| --- | --- |
| type | fuzzy search |
| `Enter` | open selected item |
| `Esc` / `Ctrl-C` | close |
| `Up` / `Down` | move selection |
| `Tab` | cycle source filters |
| `Ctrl-W` | workspaces only |
| `Ctrl-P` | Herdr Plus projects only |
| `Ctrl-Q` | Herdr Plus Quick Actions only |
| `Ctrl-Z` | zoxide only |
| `Ctrl-R` | roots only |
| `Ctrl-A` | agents only |
| `Ctrl-O` | toggle preview |
| `Ctrl-U` | clear query and filter |

## Configuration

Find the plugin config directory:

```bash
herdr plugin config-dir herdr-picker-plus
```

On first run, the plugin creates `config.toml` from [`examples/default-config.toml`](examples/default-config.toml).

### Default config

```toml
[picker]
reuse_existing = true
create_missing = true
engine = "nucleo" # nucleo | skim | simple
source_order = ["workspace", "project", "zoxide", "root", "agent", "quick", "plugin"]
source_priority_boost = 25

[sources]
open_workspaces = true
herdr_plus_projects = true
herdr_plus_quick_actions = true
zoxide = true
roots = true
agents = true

[theme]
inherit_herdr = true

[[roots]]
path = "~/workspace"
max_depth = 3

[[roots]]
path = "~/projects"
max_depth = 3
```

## Customize

### Choose sources

Disable sources you do not use:

```toml
[sources]
open_workspaces = true
herdr_plus_projects = false
herdr_plus_quick_actions = false
zoxide = true
roots = true
agents = true
```

### Change source priority

Earlier sources get a ranking bonus and appear first on an empty query:

```toml
[picker]
source_order = ["workspace", "project", "zoxide", "root", "agent", "quick", "plugin"]
source_priority_boost = 25
```

Accepted names:

```text
workspace, open, project, zoxide, root, agent, quick, plugin
```

Set the boost to zero for pure matcher score:

```toml
source_priority_boost = 0
```

### Change search engine

```toml
[picker]
engine = "nucleo" # nucleo | skim | simple
```

| Engine | Use when |
| --- | --- |
| `nucleo` | default; fast, fzf-like ranking, good Unicode behavior |
| `skim` | compare against skim/fzf-style scoring |
| `simple` | tiny built-in ordered-character matcher for debugging |

### Add root directories

Use roots for broad directory scanning. Keep this list short; zoxide should cover frequent directories.

```toml
[[roots]]
path = "~/workspace"
max_depth = 3

[[roots]]
path = "~/projects"
max_depth = 2
```

A directory becomes a root result if it contains one of:

```text
.git
package.json
Cargo.toml
```

### Theme behavior

```toml
[theme]
inherit_herdr = true
```

When enabled, the picker:

1. reads `~/.config/herdr/config.toml`
2. maps supported `theme.name` values locally
3. applies `[theme.custom]` overrides last
4. falls back to One Light if Herdr config is unavailable

Supported built-in names:

```text
one-light, catppuccin, rose-pine, rose-pine-dawn, terminal
```

## Plugin integrations

Other tools can integrate without Rust code by exposing a list/open command pair. The `label` is shown as that integration's source name in the picker:

```toml
[[integrations]]
id = "bookmarks"
label = "Bookmarks"
enabled = true
collect = "bookmarks list --json"
open = "bookmarks open {{id}}"
notify_success = true
notify_error = true
```

`collect` prints JSON:

```json
[{"id":"abc","title":"Item","subtitle":"Info","path":"/tmp","kind":"bookmark"}]
```

When selected, Picker Plus runs `open` with `{{id}}`, `{{title}}`, `{{subtitle}}`, `{{path}}`, and `{{kind}}` shell-quoted. Success and failure are reported through Herdr notifications.

See [`docs/plugin-integrations.md`](docs/plugin-integrations.md).

## Herdr Plus integration

Herdr Plus is optional. If it is not installed, Picker Plus still works with workspaces, zoxide, roots, and agents.

When Herdr Plus is installed:

- `project` entries are loaded from:

  ```text
  ~/.config/herdr/plugins/config/cloudmanic.herdr-plus/projects/*.toml
  ```

- selecting a project:
  - focuses an existing `project:` workspace when the project path is already open
  - otherwise creates a new `project:` workspace
  - applies the project's tabs and startup commands

- `quick` opens the Herdr Plus Quick Actions picker.

## Debugging

List all candidates without opening the TUI:

```bash
./target/release/herdr-picker-plus list
```

Show plugin actions:

```bash
herdr plugin action list --plugin herdr-picker-plus
```

Show installed plugins:

```bash
herdr plugin list
```

Unlink local plugin:

```bash
herdr plugin unlink herdr-picker-plus
```

## Troubleshooting

### `prefix+t` does nothing

Check the keybinding command:

```bash
rg "herdr-picker-plus.open" ~/.config/herdr/config.toml
herdr server reload-config
```

Then verify the action exists:

```bash
herdr plugin action list --plugin herdr-picker-plus
```

### The old picker opens

You may still have an old plugin linked or an old keybinding command. Relink the current plugin:

```bash
herdr plugin link "$PWD"
herdr server reload-config
herdr plugin list
```

Make sure your keybinding uses:

```toml
command = "herdr-picker-plus.open"
```

### Project entries do not appear

Check Herdr Plus project files exist:

```bash
find ~/.config/herdr/plugins/config/cloudmanic.herdr-plus/projects -name '*.toml'
```

Also check config:

```toml
[sources]
herdr_plus_projects = true
```

### Zoxide entries do not appear

Check `zoxide` is installed and has data:

```bash
zoxide query -l
```

## Project docs

- [`docs/architecture.md`](docs/architecture.md): architecture and runtime flow
- [`docs/integrations.md`](docs/integrations.md): integration patterns for Herdr and other plugins
- [`docs/plugin-integrations.md`](docs/plugin-integrations.md): command/JSON integration contract
- [`RELEASE.md`](RELEASE.md): release process
- [`SECURITY.md`](SECURITY.md): security policy

## Design notes

Herdr plugin v1 does not expose the active theme palette directly to external plugins. The picker reads Herdr config and maps supported theme names locally, with `[theme.custom]` overrides applied last.

Herdr plugin v1 also does not expose a native non-terminal custom UI API. This plugin follows the current Herdr-native pattern: an action opens a managed overlay pane, and the interactive TUI runs inside that pane.
