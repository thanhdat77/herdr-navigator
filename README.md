# Herdr Picker Plus

**Fuzzy then Move Anywhere in Your Herdr Workflow.**

Jump between remote Herdr servers, local sessions, open workspaces, project templates, directories, agent panes, quick actions, and external tools from one Herdr-native overlay. Type what you remember, press Enter, and let Herdr focus, create, hand off, attach, or launch the right thing.

```text
Ctrl-T / prefix+t -> type -> Enter
```

Why it matters:

- **Enhance your workflow everywhere**: move across servers, sessions, repos, agents, actions, and directories without leaving Herdr.
- **Deep Herdr integration**: hand off to remote Herdr servers, attach local sessions, reuse existing workspaces, focus agent panes, and launch Herdr Plus projects/actions.
- **Fully customizable**: source order, ranking bias, preview pane, search engine, roots, servers, sessions, agent aliases, theme behavior, and integrations all live in config.
- **Easily extendable**: any tool or plugin can join the picker through a tiny command/JSON `collect` + `open` contract.

It feels like a fuzzy finder, but it acts like Herdr: it can focus, create, attach, launch, reuse, and extend.

![Herdr Picker Plus screenshot](docs/assets/herdr-picker-plus.png)

## Overview

### What makes it stand out

- **Picker center for Herdr**: one place to search servers, sessions, workspaces, projects, directories, agents, and actions.
- **Reuse-first workflow**: focuses matching open workspaces without confusing project and directory workspaces that share the same path.
- **Herdr Plus integration**: opens Herdr Plus project templates and can jump into Herdr Plus Quick Actions.
- **Workspace creation**: zoxide/root results can create a Herdr workspace directly.
- **Agent-aware**: agent panes appear as searchable entries and can be focused from the picker.
- **Remote handoff built in**: `Ctrl-S` filters configured Herdr remote targets and runs `herdr --remote TARGET --handoff`; local sessions stay separate.
- **Theme-aware**: maps supported Herdr themes locally and applies `[theme.custom]` overrides.
- **No external picker UI**: the TUI is built in Rust with `ratatui`; no `fzf`/`tv` runtime dependency.
- **Plugin integration contract**: other tools can appear in the picker with a simple command/JSON list-open API.

### Sources

| Source | Reads | Enter |
| --- | --- | --- |
| `workspace` | `herdr workspace list` + pane cwd | focus the exact selected workspace |
| `project` | Herdr Plus `projects/*.toml` | focus existing cwd or create workspace + project tabs |
| `server` | remote entries from `[sessions.entries]` | hand off to `herdr --remote TARGET --handoff` |
| `session` | `herdr session list --json` + local `[sessions.entries]` | attach local session |
| `quick` | Herdr Plus Quick Actions | open Quick Actions picker |
| `zoxide` | `zoxide query -l` | focus existing cwd or create workspace |
| `root` | configured filesystem roots | focus existing cwd or create workspace |
| `agent` | agents from `herdr agent list` | focus agent pane |
| `plugin` | configured `[[integrations]]` commands | run configured open command |

### Fast remote handoff

Remote handoff is built in. Picker reads manual remote `[sessions.entries]`; selecting one runs `herdr --remote TARGET --handoff` so Herdr switches to the remote server instead of nesting a remote session inside the picker.

Local session switching still reads `herdr session list --json`; selecting one runs `herdr session attach NAME`.

## Requirements

Required:

- Herdr `0.7.3` or newer

Required only when building from source:

- Rust stable
- Cargo

Optional integrations:

- `zoxide` for the `zoxide` source
- Herdr Plus for the `project` and `quick` sources

## Install

Choose one install path.

### Option A: install with Herdr

This is the easiest path for public installs:

```bash
herdr plugin install thanhdat77/herdr-picker-plus --yes
```

Verify Herdr sees the plugin:

```bash
herdr plugin list
herdr plugin action list --plugin herdr-picker-plus
```

### Option B: install from release archive

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

### Option C: install from source

```bash
git clone https://github.com/thanhdat77/herdr-picker-plus.git
cd herdr-picker-plus
cargo build --release
herdr plugin link "$PWD"
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

## Side pane mode

Open the picker as a persistent split beside your work (like the file viewer):

```bash
herdr plugin action invoke herdr-picker-plus.open-side
```

In side mode the picker stays open after `Enter`. Re-invoking the action focuses the side pane, and closes it when it is already focused (launch-or-focus, toggle on repeat). Optional keybinding:

```toml
[[keys.command]]
key = "prefix+shift+t"
type = "plugin_action"
command = "herdr-picker-plus.open-side"
description = "picker side pane"
```

## Jump back

Successful local workspace navigation remembers the workspace you left. Invoke the dedicated action to toggle between the current and previous workspace:

```bash
herdr plugin action invoke herdr-picker-plus.jump-back
```

Optional tmux-style keybinding:

```toml
[[keys.command]]
key = "prefix+l"
type = "plugin_action"
command = "herdr-picker-plus.jump-back"
description = "jump to previous workspace"
```

If the previous workspace has been closed, Picker clears the stale target and reports the error.

Configure the behavior in the plugin config:

```toml
[jump_back]
# Record successful local workspace changes and enable the action.
enabled = true
# Pin the previous workspace first when the picker opens unfiltered.
pin_previous = true
```

The action key stays in Herdr's main config because Herdr owns global keybindings.

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
| `Ctrl-S` | servers/remotes only; configurable with `[picker.filter_keys]` |
| `Ctrl-A` | agents only; configurable with `[picker.filter_keys]` |
| `Ctrl-X` | close the selected/open matching workspace |
| `@` | same as `Ctrl-A`: show all agents, using configured agent sort |
| `!text` | match agent name, for example `!claude` |
| `@text` | agent-only match by workspace/session label/id or status, for example `@dotfiles` or `@idle` |
| `/text` | match cwd/path, for example `/chatbot` |
| `Ctrl-O` | toggle preview |
| `Ctrl-U` | clear query and filter |
| `?` | show all active keybindings |

Set `vim_mode = true` under `[picker]` for a normal mode where `j`/`k` move, source shortcut keys select a filter, and `/` starts searching within that filter. Add `vim_filter_search = true` to enter search immediately after a source key. `Esc` leaves search mode; press it again to close.

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
source_order = ["workspace", "agent", "project", "session", "zoxide", "root", "server", "quick", "plugin"]
source_priority_boost = 5
agent_sort = "herdr" # herdr | priority | spaces
preview = true
vim_mode = false
vim_filter_search = false

# Optional shortcut overrides. Values accept "ctrl-x", "ctrl+x", "^x", or "x".
# [picker.filter_keys]
# server = "ctrl-g"

[jump_back]
# Enable transition history and the jump-back action.
enabled = true
# Pin the previous workspace in the initial unfiltered picker view.
pin_previous = true

[sources]
open_workspaces = true
herdr_plus_projects = true
herdr_plus_quick_actions = true
zoxide = true
roots = true
agents = true
servers = true
sessions = true

[theme]
inherit_herdr = true

[[roots]]
path = "~/workspace"
max_depth = 3

[[roots]]
path = "~/projects"
max_depth = 3

# Optional: add human aliases to agent panes.
[[agent_aliases]]
alias = "main ai dot"
agent = "claude"
workspace = "Dotfiles"
path = "dotfiles"
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
servers = true
sessions = true
```

### Remote handoff and local sessions

Remote targets are built in and shown under `Ctrl-S` by default:

```toml
[sessions]
local = true

[[sessions.entries]]
name = "prod"
remote = "prod-api"
tags = ["prod", "api"]
```

Remote entries run `herdr --remote TARGET --handoff`. Local sessions from `herdr session list --json` run `herdr session attach NAME`.

### Agent search

Agent rows show a small status icon and include the agent name, workspace/session label, cwd, status, pane id, tab id, and terminal id in search. The `@` shortcut, `Ctrl-A`, and the empty default picker use `picker.agent_sort`; default `herdr` reads Herdr's `agent_panel_sort`. Set `priority` for blocked/error first, then attention/request, done, working, and idle; set `spaces` to keep Herdr/pane order.

```text
! blocked/error/fail
✓ done/complete
● working/running
○ idle/unknown
```

Useful queries:

```text
@                 # all agents, same as Ctrl-A
!claude @Dotfiles /dotfiles
!codex /chatbot
@idle
@wF
```

Add aliases when the real Herdr labels are not memorable enough:

```toml
[[agent_aliases]]
alias = "main ai dot"
agent = "claude"      # optional
workspace = "Dotfiles" # optional
path = "dotfiles"     # optional
```

All match fields are optional and use text-contains matching.

### Change filter shortcuts

Override source filter keys when `Ctrl-S` or another default conflicts with your terminal:

```toml
[picker.filter_keys]
session = "ctrl-g"
agent = "ctrl-a"
```

Accepted source names match `source_order`: `agent`, `server`, `workspace`, `project`, `session`, `zoxide`, `root`, `quick`. Values accept `ctrl-x`, `ctrl+x`, `^x`, or `x`.

### Change source priority

Earlier sources get a ranking bonus and appear first on an empty query:

```toml
[picker]
source_order = ["workspace", "agent", "project", "session", "zoxide", "root", "server", "quick", "plugin"]
source_priority_boost = 5
agent_sort = "herdr" # herdr | priority | spaces
preview = true
```

Accepted names:

```text
agent, server, remote, workspace, open, project, session, zoxide, root, quick, plugin
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

- [`docs/roadmap.md`](docs/roadmap.md): planned features and scope boundaries
- [`docs/architecture.md`](docs/architecture.md): architecture and runtime flow
- [`docs/integrations.md`](docs/integrations.md): integration patterns for Herdr and other plugins
- [`docs/plugin-integrations.md`](docs/plugin-integrations.md): command/JSON integration contract
- [`RELEASE.md`](RELEASE.md): release process
- [`SECURITY.md`](SECURITY.md): security policy

## Design notes

Picker Plus is intentionally small and Herdr-native:

- **Reuse first**: if the matching workspace already exists, focus it instead of creating another one.
- **Source-aware identity**: a project workspace and a plain directory workspace can share the same cwd without stealing each other.
- **Remote handoff stays Herdr-native**: server entries call `herdr --remote TARGET --handoff`; local sessions call `herdr session attach`; no SSH terminal wrapper lives in Picker.
- **Optional integrations**: Herdr Plus, zoxide, and command/JSON integrations are useful when present and quiet when missing.
- **Theme matching is pragmatic**: Herdr plugin v1 does not expose the active palette, so Picker Plus reads Herdr config and maps supported theme names locally, then applies `[theme.custom]` overrides.
- **UI follows plugin v1**: Herdr does not expose a native non-terminal custom UI API yet, so the action opens a managed overlay pane and the Rust TUI runs inside it.
