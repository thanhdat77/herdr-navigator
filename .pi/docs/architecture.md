# Architecture Notes

## Code layout

Modules now split by responsibility: `main` CLI, `app` state/dispatch, `tui` rendering/input, `config`, `model`, `sources`, `herdr`, `theme`, `matcher`, `paths`. New source integrations usually start in `sources.rs`; move out only if they grow.

## Shape

Single-file Rust TUI plugin:

```text
herdr-plugin.toml
  action open -> target/release/herdr-picker-plus open
  pane picker -> target/release/herdr-picker-plus ui

src/main.rs
  Config -> Theme -> App -> collect sources -> filter/rank -> open selected
```

## Entry modes

- `open`: asks Herdr to open the plugin overlay pane.
- `ui`: runs the interactive TUI inside the overlay pane.
- `list`: prints collected entries for debugging.

## Data flow

1. Load plugin config from Herdr plugin config dir.
2. Load theme from Herdr config if enabled.
3. Collect entries from enabled sources.
4. Score entries by matcher + source priority bonus.
5. Enter dispatches by source.

## Source model

Each picker row is an `Entry`:
- `source`: workspace/project/server/zoxide/root/agent/quick/plugin
- `title`, `subtitle`, `path`
- optional `workspace_id`
- optional `agent_target`
- optional Herdr Plus `Project`
- action enum for focus/create/server/integration behavior

Duplicate workspace/project/root/zoxide paths are collapsed by canonical path where applicable. Agent entries are appended separately because multiple agents can share cwd. Server entries dedupe by title + remote target.

## Open behavior

- Workspace: `herdr workspace focus <id>`
- Agent: `herdr agent focus <target>`
- Server: focus `server: NAME` workspace if open; else create one, rename first tab to `remote`, and run `herdr --remote <target>` in root pane
- Project: focus existing path if open; else create workspace and apply tabs
- Zoxide/root: focus existing path if open; else create workspace
- Quick: invoke Herdr Plus Quick Actions

## Herdr Plus project integration

Reads TOML files from:

```text
~/.config/herdr/plugins/config/cloudmanic.herdr-plus/projects/*.toml
```

Project fields used:
- `name`
- `description`
- `working_dir`
- `[[tabs]] name`
- `[[tabs]] command`

When creating a new project workspace:
1. `herdr workspace create --cwd <working_dir> --label <name> --focus`
2. Rename first tab.
3. Run first tab command in root pane if present.
4. Create remaining tabs and run commands.

Keep this simple. Do not clone Herdr Plus internals unless needed.

## Theme flow

Herdr plugin API does not expose the active palette. Plugin reads Herdr config:

```text
~/.config/herdr/config.toml
```

Then:
1. choose local palette from supported `theme.name`
2. apply `[theme.custom]` overrides last
3. fallback to One Light if unavailable

Supported names now: `one-light`, `catppuccin`, `rose-pine`, `rose-pine-dawn`, `terminal`.

## Integration layer

Picker core now dispatches through `EntryAction`, not only `Source`. `Source` is display/filter metadata; `EntryAction` owns Enter behavior.

Built-in adapters live under `src/integrations/`:

- `herdr_plus.rs`: Herdr Plus projects, Quick Actions, and project tab bootstrap.
- `command.rs`: generic command/JSON integrations from `[[integrations]]` config.

External plugin contract is intentionally small: `collect` prints JSON items, `open` runs with template vars. Picker Plus owns success/error notifications.
