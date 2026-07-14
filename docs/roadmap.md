# Roadmap

Picker Plus stays a small Herdr-native navigation center. New features should reduce the cost of switching context rather than duplicate Herdr session management or existing picker filters.

## Now

- [x] **Jump Back** — configurable transition history and `herdr-picker-plus.jump-back` action. The previous workspace can be pinned first in the initial unfiltered picker view; closed workspaces invalidate the saved target cleanly.

## Next

- [ ] **Pinned + Recent** — boost pinned and recently opened entries when the query is empty without changing fuzzy ranking for typed queries.
- [ ] **Selected-entry utilities** — start with copying the selected path; add an action menu only if several secondary actions prove useful.

## Conditional

- [ ] **Fast-first refresh** — add integration timeouts and last-good caching only if startup measurements show slow external collectors.

## Explore later

- [ ] **Tree view** — optional server → session → workspace → agent browsing while keeping flat fuzzy search as the default.
- [ ] **Contextual side pane** — show the current workspace, previous workspace, relevant agents, and quick actions until the user starts searching.

## Not planned

- A separate attention inbox: `Ctrl-A` already opens agents sorted by the configured status order.
- Process, pane, or session restoration: Herdr owns that lifecycle.
- A plugin manager or general tmux object manager.
