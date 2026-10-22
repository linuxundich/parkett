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

## Current state (2026-10-22)

- `main` has all 0.3 work so far: PR #1 (`qa-fixes-0.3`: R-04…R-13, QA-11, QA-12, QA-14,
  QA-16…QA-18, L3, cargo audit, shortcut tests) and PR #2 (`small-features-0.3`: high contrast
  scheme, accent selection color, sorted imports, match count, QA-15, *Hoher Kontrast* colors,
  D3, D6, the `--` fix – GLib strips `--` on first start, see `cli::protect_separator`). Both
  merged with merge commits on 2026-10-22; not released yet (`[Unreleased]` in the changelog).
- Tested in the Fedora VM (smoke test 14/14, PR #2 checks, and on 2026-10-22/23 the VM part of
  *Testing gaps* on branch `vm-tests-0.3`); left for the host (Arch, GNOME 51): see
  *Testing gaps* in `BACKLOG.md`.
- VM test tips: QMP key codes follow the US layout on the German VM (`z`→y, `/` = `shift+7`);
  an Esc right before typing becomes Meta+key in bash; Ctrl+click on links needs the default
  app to start (TUXEDO Editor for text/plain fails in the VM, use a folder link).
- QA-16 is the least certain change (whether AdwPreferencesDialog re-selects the name).
- Christoph's own profile still has `exit_action = "hold"`; QA-11 only changes the default.
- Remaining 0.3 work (see `BACKLOG.md`): QA-20, L1/L2, rest of D2, small features. QA-08 and
  R-09 are done on branch `spawn-0.3` (2026-10-23): `Launch` (command + caller environment)
  goes Window → Tab → Pane → Terminal; `core::environment`, `core::startup`.
