# Changelog

All notable changes to this project are documented here.

## [Unreleased]

### Fixed
- Preserve Herdr Plus `[[tabs.panes]]` splits, labels, and startup commands when opening projects from Navigator.

## [0.3.2] - 2026-07-15

### Added
- Notification audio modes: Herdr defaults, silent notifications, or an asynchronous custom audio file.
- A new full-width Navigator demo replaces the cropped preview.

### Changed
- **Breaking:** the plugin id, Cargo package/binary, action prefix, config directory, and release archives are now consistently named `herdr-navigator`. On first run, missing files are copied from the legacy `herdr-picker-plus` config directory without deleting the originals.

### Fixed
- Navigator notifications can now be disabled completely with `[notifications] enabled = false` ([#1](https://github.com/thanhdat77/herdr-navigator/issues/1)).

## [0.3.1] - 2026-07-15

### Added
- Non-blocking daily GitHub release checks show an `↑ vX.Y.Z available` header badge; `picker.check_updates = false` disables them and network failures stay silent.

## [0.3.0] - 2026-07-15

### Added
- Herdr-style source-aware result rows keep status/tab/pane metadata in a right column and expand only zoxide/root entries to a second full-path line; native `prefix+g` glyphs mark selection, focus, blocked, working, idle, done, and unknown states. `picker.detailed_rows = false` restores the compact list.
- Re-invoking the overlay `open` action focuses the existing Navigator in the current workspace instead of opening a duplicate pane.
- Herdr Navigator banner, social preview, and a shorter outcome-led README.
- Configurable Jump Back action (`herdr-picker-plus.jump-back`) toggles to the workspace left by the last successful local picker navigation and can pin that workspace first in the initial picker view.
- Persistent side pane mode: the `open-side` action opens the picker in a right split (like herdr-file-viewer). Launch-or-focus, toggles closed when already focused, and the picker stays open after `Enter`.
- `Ctrl-X` closes the selected/open matching workspace without closing the picker; the picker refuses to close its owning workspace.
- Built-in server/remote source from remote `[sessions.entries]`, using `herdr --remote TARGET --handoff`, plus local session entries from `herdr session list --json`.

### Changed
- User-facing name and GitHub repository are now Herdr Navigator; the stable `herdr-picker-plus` plugin id, binary, config path, and action prefix are unchanged.
- Agent source now uses the dedicated `herdr agent list` endpoint instead of filtering `herdr pane list`, and agents are searchable by their `agent_session` id.
- Workspace rows surface the workspace-level `agent_status` from `herdr workspace list` in the subtitle, and match `focused`/status search terms.
- Requires Herdr 0.7.3+ (`min_herdr_version` bumped for `agent list` and workspace `agent_status`).
- Empty default picker results now honor agent status priority when `agent_sort = "priority"` or Herdr `agent_panel_sort = "priority"` is active.
- Agent status priority is now: blocked/error/fail, attention/request/wait, done/complete, working/running, idle/unknown.
- Removed built-in SSH/server-terminal handling from Picker; `Ctrl-S` now hands off to Herdr remote servers.
- Picker footer uses compact Ctrl-style key hints.

## [0.2.0] - 2026-06-29

### Added
- Command/JSON plugin integration contract via `[[integrations]]`.
- Herdr success/error notifications for selected actions.
- Agent search by agent name, workspace/session label, cwd/path, status, ids, and aliases.
- Agent shortcut: `@` as a Ctrl-A-style full agent view with configurable sorting.
- Server source from `~/.ssh/config` and manual `[[servers.entries]]`, with `Ctrl-S` filtering and SSH connect inside a server workspace tab. This moved to the separate `herdr-server-aware` plugin after 0.2.0.

### Changed

- Agent rows stay tied to the pane start directory while still searching current foreground cwd.
- Herdr Plus logic now lives in a built-in integration adapter.
- Picker entries dispatch through actions instead of hardcoding behavior by source.

## [0.1.2] - 2026-06-28

### Fixed
- Show multiple open workspaces with the same cwd instead of deduping by path.
- Reuse project and directory workspaces by source kind so project and zoxide/root entries do not steal each other.


## [0.1.1] - 2026-06-28

### Added
- Herdr Plus Quick Actions launcher entry.

## [0.1.0] - 2026-06-28

### Added
- Herdr overlay picker plus.
- Sources for open workspaces, Herdr Plus projects, zoxide, configured roots, and agents.
- Configurable source order and search engine.
- Herdr theme-name inheritance with custom token overrides.
- Release and CI workflows for public builds.
