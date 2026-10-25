// SPDX-License-Identifier: GPL-3.0-or-later
//! Core of Parkett without GTK: split layout, color schemes, profiles, templates and the
//! match scoring of the command palette. Everything here is testable with `cargo test -p parkett-core`.

pub mod bookmark;
pub mod cli;
pub mod context;
pub mod environment;
pub mod fuzzy;
pub mod i18n;
pub mod label;
pub mod layout;
pub mod link;
pub mod palette;
pub mod paste;
pub mod paths;
pub mod profile;
pub mod session;
pub mod startup;
pub mod template;
pub mod uri;
