# Parkett – notes for Claude

GNOME terminal with Tilix-style splits: Rust, GTK 4.18+, libadwaita 1.8+, VTE 0.80+.
Christoph (maintainer) writes in German; answer in German.

## Layout

- `core/` (`parkett-core`): everything without GTK – split layout, profiles, templates,
  session, palettes, command line, context detection, tab labels. Unit-tested.
- `app/` (`parkett`): GTK/libadwaita/VTE interface. `window.rs` (header bar, tabs),
  `tab.rs` (split tree ⇄ GtkPaned), `pane.rs` (title, search), `terminal.rs` (VTE, spawn,
  exit bar, foreground poll), `prefs.rs`, `shortcuts.rs`, `session.rs`.
- `data/`: GSettings schema, `.desktop`, metainfo, palettes, Nautilus extension.
- `docs/`: USAGE, ARCHITECTURE, BUILDING, QA, PACKAGING; `qa/2026-10-19/QA-REPORT.md` is the
  audit the IDs QA-xx / R-xx refer to; L/D/G IDs come from the code review in `docs/QA.md`.
- `BACKLOG.md`: open work by priority, decisions, testing gaps, done list. Keep it current.
- `CHANGELOG.md`: Keep a Changelog, entries under `[Unreleased]`.

## Commands

```sh
cargo run -p parkett          # run from the source tree (schema is compiled by app/build.rs)
cargo test                    # core + app unit tests, no display needed
build-aux/ci.sh               # fmt, clippy -D warnings, tests, cargo audit, Meson build + tests
build-aux/vm/smoke.sh         # smoke test in the GNOME test VM (see docs/BUILDING.md)
```

Run `build-aux/ci.sh` before every commit. App unit tests must not call GTK functions that need
`gtk::init` (e.g. `gtk::accelerator_parse`); parse as text instead (see `shortcuts.rs` tests).

## Conventions

- Code comments and UI strings are German; docs, commit messages, changelog and backlog are
  English. UI terms: *Reiter* (tab), *Bereich* (pane).
- Comments explain why and name the finding (`// … (QA-11)`), matching the existing density.
- Pure logic goes into `core` with tests; the app only wires it to widgets.
- Files with possible secrets go through `parkett_core::paths::write_private` (mode 0600).
- New settings: key in `data/de.linuxundich.Parkett.gschema.xml`, accessor in
  `app/src/settings.rs`, row in `prefs.rs`, mention in `docs/USAGE.md`.
- Tooltips with shortcuts: `shortcuts::live_tooltip` (built from the current assignment).
- Format: `rustfmt.toml` (max width 120).

## Current state (2026-10-21)

- PR #1 (`qa-fixes-0.3` → `main`): 17 items for 0.3 (R-04…R-13, QA-11, QA-12, QA-14, QA-16,
  QA-17, QA-18, L3, cargo audit, shortcut tests). Built and tested without a display only
  (container with Ubuntu 24.04 libraries); the app has not been started with these changes.
- Next step: Christoph tests on the host (Arch, GNOME 51) along *Testing gaps* in
  `BACKLOG.md`, fixes go onto `qa-fixes-0.3`, then merge.
- QA-16 is the least certain change (whether AdwPreferencesDialog re-selects the name).
- Christoph's own profile still has `exit_action = "hold"`; QA-11 only changes the default.
- After the merge, remaining 0.3 work (see `BACKLOG.md`): QA-15, QA-20, contrast of
  *Hoher Kontrast*, L1/L2, rest of D2, D3, D6, small features; larger: QA-08 (environment of a
  second invocation) and R-09 (startup command after the first prompt) – both touch the spawn
  path, do them on a tested base.
