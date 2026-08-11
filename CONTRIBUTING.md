# Contributing

Thanks for helping improve Herdr Navigator.

## Development

```bash
cargo fmt
cargo clippy -- -D warnings
cargo test
cargo build --release
```

Keep changes small and user-facing behavior documented in `README.md`.

## Where discussion belongs

- **Bug report:** open a GitHub issue with a reproduction and environment details.
- **Idea or feature proposal:** start an [Ideas discussion](https://github.com/thanhdat77/herdr-navigator/discussions/categories/ideas). An accepted, scoped change is promoted to an issue during triage.
- **Question or setup help:** use [Q&A](https://github.com/thanhdat77/herdr-navigator/discussions/categories/q-a).

Keep issues for confirmed defects and accepted implementation work; keep exploration and design feedback in Discussions.

## Pull requests

- Explain the user-visible change and link its accepted issue.
- Run `cargo fmt --check`, `cargo test`, and `cargo build --release` on the current `main` base.
- Add direct tests for at least 80% of the changed acceptance scenarios; line coverage alone is not sufficient.
- Update `CHANGELOG.md` under `Unreleased`.
