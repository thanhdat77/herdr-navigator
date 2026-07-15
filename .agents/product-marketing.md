# Product Marketing Context

*Last updated: 2026-07-15*

## Product Overview

**Display name:** Herdr Navigator
**Technical plugin ID:** `herdr-navigator` (aligned with the display name, binary, actions, config path, and repository since v0.3.2)
**One-liner:** One fuzzy navigator for every place and action in Herdr.
**What it does:** Herdr Navigator searches workspaces, agents, projects, sessions, remotes, directories, Quick Actions, and external command integrations from one Herdr-managed picker. Selecting a result performs the correct Herdr action: focus, create, attach, hand off, invoke, or run.
**Product category:** Herdr plugin; terminal fuzzy navigator; workflow switcher.
**Product type:** Free, MIT-licensed open-source developer tool.

## Target Audience

**Primary users:** Keyboard-first Herdr users managing several repositories, workspaces, sessions, remotes, and AI-agent panes.
**Primary use case:** Reach the next piece of work without remembering which Herdr surface or command owns it.
**Jobs to be done:**
- Find an open workspace or agent from one shortcut.
- Open a project or directory without creating duplicate workspaces.
- Attach a local session or hand off to a remote Herdr server.
- Bring another CLI tool into the same picker without changing Rust code.

**Anti-persona:** Users who do not run Herdr, only need a basic path picker, or want process/session restoration.

## Problems & Pain Points

**Core problem:** Navigation is split across workspace lists, agent panes, session commands, remotes, zoxide, project templates, and custom scripts. Users often remember the destination but not the surface needed to reach it.
**Why alternatives fall short:** Herdr's focused pickers are narrower; generic fuzzy finders return paths but do not know whether to focus, create, attach, or hand off; custom scripts become fragmented and hard to share.
**Emotional tension:** Context switching breaks flow, especially while coordinating several agents or repositories.

## Competitive Landscape

**Direct:** Herdr's built-in workspace/session navigation — simpler for one entity type, but not a cross-source action surface.
**Secondary:** `fzf`, `tv`, zoxide, and custom shell scripts — flexible discovery, but no built-in Herdr action semantics or source-aware workspace reuse.
**Analogues:** tmux-sessionx and tmux-fzf — validate the unified switch-or-create workflow, but target tmux rather than Herdr.

## Differentiation

- One searchable index across Herdr objects, paths, agents, remotes, and actions.
- Enter performs the source-appropriate Herdr operation instead of printing a path.
- Reuse-first and source-aware identity avoid duplicate or incorrect workspace focus.
- Agent status, workspace, cwd, IDs, and aliases are searchable.
- External tools integrate through a small command/JSON `collect` + `open` contract.
- Built-in Rust/ratatui interface; no external picker UI dependency.

## Objections

| Objection | Response |
|-----------|----------|
| Herdr already has navigation | Navigator adds one cross-source surface and action-aware dispatch across workspaces, agents, projects, sessions, remotes, paths, and integrations. |
| I do not use every integration | Every source can be disabled; Herdr Plus and zoxide degrade quietly when absent. |
| The v0.3.2 technical rename disrupts existing installs | Keep the one-time migration explicit; old config and Jump Back state copy automatically on first run. |
| Will it create duplicate workspaces? | It focuses matching open workspaces first and distinguishes project workspaces from plain directory workspaces sharing the same cwd. |

## Customer Language

**Inferred problem language:**
- “I know where I want to work; just take me there.”
- “I do not want a separate picker for workspaces, agents, sessions, and remotes.”
- “Focus what is already open before creating another workspace.”

**Words to use:** jump, navigate, focus, reuse, create, attach, hand off, workspace, agent, one shortcut, type what you remember.
**Words to avoid:** blazing fast, zero friction, native custom UI, SSH server picker, AI-powered, revolutionary.

## Brand Voice

**Tone:** Direct, technical, calm, useful.
**Style:** Outcome first, exact commands second, implementation detail later.
**Personality:** Focused, capable, keyboard-first, honest, extensible.

## Proof Points

- MIT-licensed Rust project with Linux and macOS declared support.
- Herdr 0.7.3+ actions for workspace focus/create, agent focus, session attach, remote handoff, and managed plugin panes.
- Configurable source order, filters, preview, themes, fuzzy engines, roots, aliases, sessions, Jump Back, and command integrations.
- Automated Rust tests and CI/release workflows.
- Real TUI screenshot in the repository.

No benchmarks, testimonials, adoption claims, or quantified time savings are currently available.

## Goals

**Primary goal:** Increase qualified Herdr-user installs from the GitHub README.
**Primary conversion action:** Run `herdr plugin install thanhdat77/herdr-navigator --yes`.
**Secondary actions:** Star the repository, configure `prefix+t`, and contribute integrations.
