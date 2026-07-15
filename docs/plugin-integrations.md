# Plugin integrations

Herdr Navigator can show items from other tools/plugins through a small command/JSON contract.

## Contract v1

A plugin provides two commands:

1. `collect`: prints picker items as JSON.
2. `open`: opens the selected item.

Config (`label` is the source name shown in the picker):

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

## Collect JSON

Minimum item:

```json
[
  {
    "id": "stable-id",
    "title": "Display name"
  }
]
```

Recommended item:

```json
[
  {
    "id": "dotfiles",
    "title": "Dotfiles",
    "subtitle": "main config repo",
    "path": "/home/fenix/dotfiles",
    "kind": "repo"
  }
]
```

Fields:

| Field | Required | Meaning |
| --- | --- | --- |
| `id` | yes | stable identifier passed to `open` |
| `title` | yes | picker row title |
| `subtitle` | no | extra detail shown beside the title |
| `path` | no | optional path for matching/search/preview |
| `kind` | no | free-form type chosen by the integration; Navigator does not interpret it |

## Open command templates

Navigator replaces these variables in `open`:

```text
{{id}}
{{title}}
{{subtitle}}
{{path}}
{{kind}}
```

Values are shell-quoted before the command is run with `sh -c`.

Example:

```toml
open = "bookmarks open {{id}} --path {{path}}"
```

## Notifications

Navigator owns default notifications:

- collect failure: skip quietly
- open success: show success notification
- open failure: show error notification

This means plugin authors do not need to implement Herdr notifications.

Disable per integration if needed:

```toml
notify_success = false
notify_error = true
```

## Built-in vs command integration

Use command/JSON integration for simple list/open flows.

Use a built-in adapter when opening needs Herdr-specific behavior. Herdr Plus is built in because project entries may need workspace creation plus tab/bootstrap commands.
