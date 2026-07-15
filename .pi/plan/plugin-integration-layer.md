# Plan: Plugin Integration Layer

## Goal

Make Herdr Navigator a real picker center with a clean integration layer:

- Herdr Plus becomes a built-in integration adapter.
- Future plugins can integrate without editing picker core code.
- Core picker stays focused on collection, filtering, rendering, and dispatch.

## Guiding constraints

- Keep it simple first; no SDK/framework until real integrations need it.
- Missing optional plugins must degrade quietly.
- Herdr Plus should be the reference integration, not special-case logic scattered across core.
- Public docs should explain the integration contract for plugin authors.


## Integration contract v1

A plugin that wants to appear in Navigator only needs a command API:

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

`collect` returns JSON array:

```json
[
  {
    "id": "stable-id",
    "title": "Display name",
    "subtitle": "Optional details",
    "path": "/optional/path",
    "kind": "free-form-type"
  }
]
```

Minimum item fields: `id`, `title`.

Template vars for `open` and notifications:

```text
{{id}} {{title}} {{subtitle}} {{path}} {{kind}}
```

Navigator owns default notifications. Plugins do not need to implement a notification API.

Notification behavior:

- `collect` failure: skip quietly by default, optional debug log later.
- `open` success: show Herdr notification so user knows the action completed.
- `open` failure: show Herdr notification with short stderr/body.
- Built-in actions should use the same notification helper.

Herdr command:

```bash
herdr notification show "Herdr Navigator" --body "Opened Dotfiles" --position top-right --sound done
```

Error command:

```bash
herdr notification show "Herdr Navigator" --body "Failed: <short error>" --position top-right --sound request
```

## Phase 1 — Extract Herdr Plus adapter ✅

Create:

```text
src/integrations/
  mod.rs
  herdr_plus.rs
```

Move from `sources.rs` into `integrations/herdr_plus.rs`:

- Herdr Plus project config paths
- project TOML loading
- quick actions entry
- project workspace creation/tab bootstrap

Core should call:

```rust
integrations::herdr_plus::collect_projects()
integrations::herdr_plus::quick_actions_entry()
integrations::herdr_plus::open_project(...)
```

Done when:

- `sources.rs` no longer knows Herdr Plus internals.
- Herdr Plus can be understood as one adapter module.
- Behavior is unchanged.

## Phase 2 — Separate entry action from source ✅

Current dispatch is mostly based on `Source`. For plugin integrations, entries need their own action.

Target shape:

```rust
Entry {
    source,
    title,
    subtitle,
    path,
    action,
}
```

Possible action enum:

```rust
EntryAction::FocusWorkspace { id }
EntryAction::FocusAgent { target }
EntryAction::OpenProject { project }
EntryAction::FocusOrCreate { path, label }
EntryAction::RunCommand { command }
EntryAction::InvokePluginAction { action }
```

Done when:

- `Source` is mostly display/filter metadata.
- Enter behavior comes from `EntryAction`.
- Existing workspace/project/zoxide/root/agent/quick behavior still works.

## Phase 3 — Generic command/JSON integrations ✅

Add config support:

```toml
[[integrations]]
id = "bookmarks"
label = "my plugin"
enabled = true
collect = "bookmarks list --json"
open = "bookmarks open {{id}}"
```

`collect` command returns JSON:

```json
[
  {
    "id": "abc",
    "title": "Item",
    "subtitle": "Info",
    "path": "/some/path"
  }
]
```

Template vars for `open`:

```text
{{id}}
{{title}}
{{subtitle}}
{{path}}
```

Done when:

- picker can load configured external integrations
- collect command failures are skipped quietly
- Enter runs the configured open command
- no plugin-specific Rust code is needed for simple integrations

## Phase 4 — Public docs and examples ✅

Add:

```text
docs/plugin-integrations.md
examples/integrations/basic-command.toml
examples/integrations/advanced-command.toml
```

Docs should cover:

- integration contract
- JSON schema
- template vars
- quiet failure behavior
- when to use built-in adapter vs generic command integration

Positioning:

```text
Herdr Plus = built-in reference adapter
Other plugins = command/JSON contract first
```

## Phase 5 — Tests and verification ✅

Add small tests for:

- integration config parsing
- collect JSON parsing
- command template rendering
- missing collect command does not fail picker

Always run:

```bash
cargo fmt --check
cargo clippy -- -D warnings
cargo test
cargo build --release
./target/release/herdr-navigator list
```

## Minimal first implementation

Do Phase 1 first.

Reason: it improves contributor structure immediately and reduces risk. Generic integrations should come after the current Herdr Plus behavior is isolated and stable.


## Implementation status

Implemented in the working tree:

- Herdr Plus adapter under `src/integrations/herdr_plus.rs`.
- Generic command/JSON adapter under `src/integrations/command.rs`.
- `EntryAction` dispatch model.
- User-defined integration source labels via `label`; Navigator does not hardcode external source names.
- Success/error notifications around selected actions.
- Public docs and examples for integration authors.
