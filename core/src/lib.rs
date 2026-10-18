// SPDX-License-Identifier: GPL-3.0-or-later
//! Kern von Parkett ohne GTK: Split-Layout, Farbschemata, Profile, Vorlagen und die
//! Trefferbewertung der Befehlspalette. Alles hier ist mit `cargo test -p parkett-core` prüfbar.

pub mod context;
pub mod fuzzy;
pub mod layout;
pub mod palette;
pub mod paths;
pub mod profile;
pub mod template;
