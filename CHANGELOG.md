# Changelog

All notable changes to this project are documented here.

## [Unreleased]

### Added
- Command/JSON plugin integration contract via `[[integrations]]`.
- Herdr success/error notifications for selected actions.
- Agent search by agent name, workspace/session label, cwd/path, status, ids, and aliases.
- Agent shortcut: `@` as a Ctrl-A-style full agent view with configurable sorting.
- Server source from `~/.ssh/config` and manual `[[servers.entries]]`, with `Ctrl-S` filtering and `herdr --remote` connect inside a server workspace tab.

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
