// SPDX-License-Identifier: GPL-3.0-or-later
//! Einstellungsdialog (AdwPreferencesDialog): Seiten „Allgemein“ und „Profile“.
//! Änderungen gelten sofort (GNOME-Stil); ein gewähltes Farbschema färbt laufende
//! Terminals des Profils gleich mit um (Live-Vorschau).

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use adw::prelude::*;
use gtk::{gio, glib, pango};
use parkett_core::palette::{Face, Palette};
use parkett_core::profile::{Bell, CursorBlink, CursorShape, ExitAction, Profile};

use crate::i18n::{gettext, ntrf, trf};
use crate::profiles::ProfileManager;

pub fn show(parent: Option<&gtk::Window>) {
    let dialog = adw::PreferencesDialog::builder().title(gettext("Preferences")).search_enabled(true).build();
    dialog.add(&general_page());
    dialog.add(&profiles_page(&dialog));
    dialog.add(&templates_page(&dialog));
    dialog.add(&bookmarks_page(&dialog));
    dialog.add(&shortcuts_page(&dialog));
    dialog.present(parent);
}

fn general_page() -> adw::PreferencesPage {
    let settings = crate::settings::settings();
    let page =
        adw::PreferencesPage::builder().title(gettext("General")).icon_name("preferences-system-symbolic").build();

    let look = adw::PreferencesGroup::builder().title(gettext("Appearance")).build();
    let dim = adw::SwitchRow::builder()
        .title(gettext("Dim inactive panes"))
        .subtitle(gettext("Shows which pane has focus in split tabs"))
        .build();
    settings.bind("dim-unfocused", &dim, "active").build();
    let labels = adw::SwitchRow::builder()
        .title(gettext("Short tab names"))
        .subtitle(gettext("Number, program and folder instead of the shell title"))
        .build();
    settings.bind("short-tab-labels", &labels, "active").build();
    let template = adw::EntryRow::builder()
        .title(gettext("Tab name template"))
        .tooltip_text(gettext("{n} number · {program} program · {dir} folder · {path} full path · {host} host over SSH · {title} shell title · {profile} profile"))
        .build();
    settings.bind("tab-label-template", &template, "text").build();
    settings.bind("short-tab-labels", &template, "sensitive").get_only().build();
    let tabbar = adw::SwitchRow::builder().title(gettext("Hide tab bar with a single tab")).build();
    settings.bind("autohide-tab-bar", &tabbar, "active").build();
    let context = adw::SwitchRow::builder()
        .title(gettext("Context color for root and SSH"))
        .subtitle(gettext("Line, chip and tinted background, colors from the color scheme"))
        .build();
    settings.bind("context-colors", &context, "active").build();
    let contrast = adw::SwitchRow::builder()
        .title(gettext("Follow system high contrast"))
        .subtitle(gettext("Color scheme “High Contrast” in all panes while it is on in GNOME"))
        .build();
    settings.bind("follow-high-contrast", &contrast, "active").build();
    let style = adw::ComboRow::builder()
        .title(gettext("Style"))
        .model(&gtk::StringList::new(&[
            gettext("Follow system style").as_str(),
            gettext("Light").as_str(),
            gettext("Dark").as_str(),
        ]))
        .build();
    style.set_selected(match settings.string("style").as_str() {
        "light" => 1,
        "dark" => 2,
        _ => 0,
    });
    style.connect_selected_notify(|row| {
        let v = ["system", "light", "dark"][row.selected().min(2) as usize];
        let _ = crate::settings::settings().set_string("style", v);
    });
    look.add(&style);
    look.add(&contrast);
    let title = adw::ComboRow::builder()
        .title(gettext("Pane titles in split tabs"))
        .model(&gtk::StringList::new(&[gettext("Floating").as_str(), gettext("As a row").as_str()]))
        .build();
    title.set_selected(if settings.string("pane-title") == "row" { 1 } else { 0 });
    title.connect_selected_notify(|row| {
        let _ =
            crate::settings::settings().set_string("pane-title", if row.selected() == 1 { "row" } else { "floating" });
    });
    look.add(&title);
    look.add(&dim);
    look.add(&context);
    look.add(&labels);
    look.add(&template);
    look.add(&tabbar);
    page.add(&look);

    let behaviour = adw::PreferencesGroup::builder().title(gettext("Behavior")).build();
    let dir = adw::ComboRow::builder()
        .title(gettext("Open new terminals in"))
        .model(&gtk::StringList::new(&[
            gettext("Directory of the active pane").as_str(),
            gettext("Home folder").as_str(),
        ]))
        .build();
    dir.set_selected(if settings.string("new-terminal-directory") == "home" { 1 } else { 0 });
    dir.connect_selected_notify(|row| {
        let _ = crate::settings::settings()
            .set_string("new-terminal-directory", if row.selected() == 1 { "home" } else { "current" });
    });
    let confirm = adw::SwitchRow::builder()
        .title(gettext("Ask before closing"))
        .subtitle(gettext("When a program is still running in a pane"))
        .build();
    settings.bind("confirm-close", &confirm, "active").build();
    let scrollback = adw::SpinRow::builder()
        .title(gettext("Scrollback lines"))
        .subtitle(gettext("−1 = unlimited"))
        .adjustment(&gtk::Adjustment::new(10000.0, -1.0, 1_000_000.0, 1000.0, 10000.0, 0.0))
        .build();
    settings.bind("scrollback-lines", &scrollback, "value").build();
    behaviour.add(&dir);
    behaviour.add(&confirm);
    let paste = adw::SwitchRow::builder()
        .title(gettext("Warn before multi-line paste"))
        .subtitle(gettext("Shows text with line breaks first – it would run immediately otherwise"))
        .build();
    settings.bind("warn-multiline-paste", &paste, "active").build();
    behaviour.add(&paste);
    let restore = adw::SwitchRow::builder()
        .title(gettext("Restore last session"))
        .subtitle(gettext("Windows, tabs, splits and directories – without the programs in them"))
        .build();
    settings.bind("restore-session", &restore, "active").build();
    behaviour.add(&restore);
    let notify = adw::SwitchRow::builder()
        .title(gettext("Notify when a command finishes"))
        .subtitle(gettext("After running for at least 10 seconds, when the pane is not visible"))
        .build();
    settings.bind("notify-command-finished", &notify, "active").build();
    behaviour.add(&notify);
    let sixel = adw::SwitchRow::builder().title(gettext("Show Sixel graphics")).build();
    if crate::terminal::sixel_supported() {
        sixel.set_subtitle(&gettext("Images in the terminal, e.g. from img2sixel or chafa"));
        settings.bind("enable-sixel", &sixel, "active").build();
    } else {
        sixel.set_subtitle(&gettext("The installed VTE library is built without Sixel"));
        sixel.set_sensitive(false);
    }
    behaviour.add(&sixel);
    behaviour.add(&scrollback);
    page.add(&behaviour);
    page
}

fn profiles_page(dialog: &adw::PreferencesDialog) -> adw::PreferencesPage {
    let page =
        adw::PreferencesPage::builder().title(gettext("Profiles")).icon_name("applications-utilities-symbolic").build();
    let add = gtk::Button::builder()
        .icon_name("list-add-symbolic")
        .tooltip_text(gettext("New profile"))
        .css_classes(["flat"])
        .valign(gtk::Align::Center)
        .build();
    let group = adw::PreferencesGroup::builder()
        .title(gettext("Profiles"))
        .description(gettext("A profile defines how a terminal starts and which color scheme it uses."))
        .header_suffix(&add)
        .build();
    page.add(&group);

    let rows: Rc<RefCell<Vec<adw::ActionRow>>> = Rc::default();
    let fill = {
        let group = group.downgrade();
        let rows = rows.clone();
        let dialog = dialog.downgrade();
        move || {
            let Some(group) = group.upgrade() else { return };
            for r in rows.borrow_mut().drain(..) {
                group.remove(&r);
            }
            let manager = ProfileManager::get();
            let default = manager.default_id();
            for p in manager.profiles() {
                let pal = manager.palette(&p.palette);
                let subtitle = if p.id == default {
                    trf("{scheme} · Default profile", &[("scheme", &pal.name)])
                } else {
                    pal.name.clone()
                };
                let row = adw::ActionRow::builder()
                    .title(glib::markup_escape_text(&p.name).as_str())
                    .subtitle(glib::markup_escape_text(&subtitle).as_str())
                    .activatable(true)
                    .build();
                row.add_suffix(&gtk::Image::from_icon_name("go-next-symbolic"));
                let id = p.id.clone();
                let dialog = dialog.clone();
                row.connect_activated(move |_| {
                    if let Some(d) = dialog.upgrade() {
                        d.push_subpage(&profile_editor(&d, &id, false));
                    }
                });
                group.add(&row);
                rows.borrow_mut().push(row);
            }
        }
    };
    fill();
    let release = release_on_destroy(&group, &ProfileManager::get(), ProfileManager::get().connect_changed(fill));
    dialog.connect_closed(move |_| release());

    let weak = dialog.downgrade();
    add.connect_clicked(move |_| {
        let Some(d) = weak.upgrade() else { return };
        match ProfileManager::get().create(&gettext("New profile")) {
            Ok(p) => d.push_subpage(&profile_editor(&d, &p.id, true)),
            Err(e) => d.add_toast(crate::window::plain_toast(&trf(
                "Profile not created: {error}",
                &[("error", &e.to_string())],
            ))),
        }
    });
    page
}

/// Ändert das gespeicherte Profil `id` und speichert sofort.
/// Trennt einen Handler an einem Objekt, das so lange lebt wie die App (ProfileManager,
/// GSettings), spätestens wenn `owner` zerstört wird; die zurückgegebene Funktion trennt
/// früher (bei „closed“ bzw. „hidden“), aber nur einmal. Endet der Dialog anders, etwa mit
/// dem Fenster, hielte der Handler sonst Gruppen und Zeilen fest (L1).
fn release_on_destroy(
    owner: &impl IsA<gtk::Widget>,
    source: &impl IsA<glib::Object>,
    id: glib::SignalHandlerId,
) -> impl Fn() + Clone + 'static {
    let slot = Rc::new(RefCell::new(Some((source.clone().upcast::<glib::Object>(), id))));
    let release = move || {
        if let Some((source, id)) = slot.borrow_mut().take() {
            source.disconnect(id);
        }
    };
    let r = release.clone();
    owner.connect_destroy(move |_| r());
    release
}

fn update(dialog: &adw::PreferencesDialog, id: &str, f: impl FnOnce(&mut Profile)) {
    let manager = ProfileManager::get();
    let mut p = manager.profile(id);
    if p.id != id {
        return; // inzwischen gelöscht
    }
    f(&mut p);
    if let Err(e) = manager.save(&p) {
        dialog.add_toast(crate::window::plain_toast(&trf("Not saved: {error}", &[("error", &e.to_string())])));
    }
}

/// `new`: frisch angelegt – dann bleibt der Name markiert, um ihn gleich zu überschreiben.
fn profile_editor(dialog: &adw::PreferencesDialog, id: &str, new: bool) -> adw::NavigationPage {
    let manager = ProfileManager::get();
    let profile = manager.profile(id);
    let id = profile.id.clone();
    let page = adw::PreferencesPage::new();

    // Allgemein
    let general = adw::PreferencesGroup::new();
    let name = adw::EntryRow::builder().title(gettext("Name")).text(&profile.name).build();
    let d = dialog.downgrade();
    let i = id.clone();
    name.connect_changed(move |e| {
        let text = e.text().trim().to_string();
        if let (false, Some(d)) = (text.is_empty(), d.upgrade()) {
            update(&d, &i, |p| p.name = text);
        }
    });
    let is_default = manager.default_id() == id;
    let default = adw::SwitchRow::builder()
        .title(gettext("Default profile"))
        .subtitle(gettext("For new tabs"))
        .active(is_default)
        .sensitive(!is_default)
        .build();
    let i = id.clone();
    default.connect_active_notify(move |row| {
        if row.is_active() {
            ProfileManager::get().set_default(&i);
            row.set_sensitive(false);
        }
    });
    general.add(&name);
    general.add(&default);
    page.add(&general);

    // Farbschema als Kacheln
    let import = gtk::Button::builder()
        .label(gettext("Import…"))
        .css_classes(["flat"])
        .valign(gtk::Align::Center)
        .tooltip_text(gettext("Ptyxis palette (.palette) or Tilix scheme (.json)"))
        .build();
    let schemes = adw::PreferencesGroup::builder()
        .title(gettext("Color scheme"))
        .description(gettext("Each scheme has a light and a dark variant (left/right); Parkett follows the system style. The selection applies immediately."))
        .header_suffix(&import)
        .build();
    let flow = gtk::FlowBox::builder()
        .selection_mode(gtk::SelectionMode::Single)
        .homogeneous(true)
        .min_children_per_line(2)
        .max_children_per_line(4)
        .column_spacing(12)
        .row_spacing(12)
        .css_classes(["parkett-schemes"])
        .build();
    schemes.add(&flow);
    page.add(&schemes);
    let filling = Rc::new(Cell::new(false));
    let fill = {
        let flow = flow.downgrade();
        let id = id.clone();
        let filling = filling.clone();
        move || {
            let Some(flow) = flow.upgrade() else { return };
            filling.set(true);
            flow.remove_all();
            let current = ProfileManager::get().profile(&id).palette;
            for pal in ProfileManager::get().palettes() {
                let tile = scheme_tile(&pal);
                flow.append(&tile);
                if pal.id == current {
                    if let Some(child) = tile.parent().and_downcast::<gtk::FlowBoxChild>() {
                        flow.select_child(&child);
                    }
                }
            }
            filling.set(false);
        }
    };
    fill();
    let d = dialog.downgrade();
    let i = id.clone();
    let f2 = filling.clone();
    flow.connect_selected_children_changed(move |flow| {
        if f2.get() {
            return;
        }
        let Some(child) = flow.selected_children().into_iter().next() else { return };
        let Some(pid) = child.child().map(|c| c.widget_name().to_string()) else { return };
        if let Some(d) = d.upgrade() {
            update(&d, &i, |p| p.palette = pid);
        }
    });
    let handler = ProfileManager::get().connect_changed({
        // nach Import neu füllen (Anzahl der Paletten geändert), sonst nur die Auswahl
        // an das Profil angleichen – Import und Profilwechsel kommen als zwei Änderungen
        let fill = fill.clone();
        let flow = flow.downgrade();
        let id = id.clone();
        let filling = filling.clone();
        move || {
            let Some(flow) = flow.upgrade() else { return };
            let mut n = 0;
            while flow.child_at_index(n).is_some() {
                n += 1;
            }
            if n as usize != ProfileManager::get().palettes().len() {
                fill();
                return;
            }
            let current = ProfileManager::get().profile(&id).palette;
            for i in 0..n {
                let child = flow.child_at_index(i).unwrap();
                let matches = child.child().is_some_and(|c| c.widget_name() == current.as_str());
                if matches && !child.is_selected() {
                    filling.set(true);
                    flow.select_child(&child);
                    filling.set(false);
                }
            }
        }
    });
    let release = release_on_destroy(&flow, &ProfileManager::get(), handler);
    let d = dialog.downgrade();
    let i = id.clone();
    import.connect_clicked(move |button| {
        let Some(d) = d.upgrade() else { return };
        let filters = gio::ListStore::new::<gtk::FileFilter>();
        let filter = gtk::FileFilter::new();
        filter.set_name(Some(&gettext("Color schemes (.palette, .json)")));
        filter.add_pattern("*.palette");
        filter.add_pattern("*.json");
        filters.append(&filter);
        let chooser =
            gtk::FileDialog::builder().title(gettext("Import color scheme")).filters(&filters).modal(true).build();
        let window = button.root().and_downcast::<gtk::Window>();
        let i = i.clone();
        chooser.open(window.as_ref(), gio::Cancellable::NONE, move |res| {
            let Ok(file) = res else { return };
            let Some(path) = file.path() else { return };
            match ProfileManager::get().import(&path) {
                Ok(pal) => {
                    update(&d, &i, |p| p.palette = pal.id.clone());
                    let text = if pal.has_variants() {
                        trf("“{name}” imported", &[("name", &pal.name)])
                    } else {
                        trf("“{name}” imported (only one variant, applies to light and dark)", &[("name", &pal.name)])
                    };
                    d.add_toast(crate::window::plain_toast(&text));
                }
                Err(e) => d.add_toast(crate::window::plain_toast(&trf(
                    "Import failed: {error}",
                    &[("error", &e.to_string())],
                ))),
            }
        });
    });

    // Schrift, Cursor, Glocke
    let look = adw::PreferencesGroup::builder().title(gettext("Font and cursor")).build();
    let system_font = adw::SwitchRow::builder()
        .title(gettext("Use system font"))
        .subtitle(gettext("Monospace font from the GNOME settings"))
        .active(profile.font.trim().is_empty())
        .build();
    let font_dialog = gtk::FontDialog::builder().title(gettext("Terminal font")).modal(true).build();
    // nur Festbreitenschriften anbieten
    font_dialog.set_filter(Some(&gtk::CustomFilter::new(|item| {
        item.downcast_ref::<pango::FontFamily>().is_none_or(|f| f.is_monospace())
    })));
    let initial = if profile.font.trim().is_empty() {
        adw::StyleManager::default().monospace_font_name().to_string()
    } else {
        profile.font.clone()
    };
    let font_button = gtk::FontDialogButton::builder()
        .dialog(&font_dialog)
        .font_desc(&pango::FontDescription::from_string(&initial))
        .valign(gtk::Align::Center)
        .build();
    let font_row = adw::ActionRow::builder().title(gettext("Font")).sensitive(!system_font.is_active()).build();
    font_row.add_suffix(&font_button);
    font_row.set_activatable_widget(Some(&font_button));
    let d = dialog.downgrade();
    let i = id.clone();
    let (fr, fb) = (font_row.downgrade(), font_button.downgrade());
    system_font.connect_active_notify(move |row| {
        let Some(d) = d.upgrade() else { return };
        let use_system = row.is_active();
        if let Some(fr) = fr.upgrade() {
            fr.set_sensitive(!use_system);
        }
        let font = if use_system {
            String::new()
        } else {
            fb.upgrade().and_then(|b| b.font_desc()).map(|f| f.to_string()).unwrap_or_default()
        };
        update(&d, &i, |p| p.font = font);
    });
    let d = dialog.downgrade();
    let i = id.clone();
    font_button.connect_font_desc_notify(move |b| {
        let (Some(d), Some(f)) = (d.upgrade(), b.font_desc()) else { return };
        update(&d, &i, |p| p.font = f.to_string());
    });
    look.add(&system_font);
    look.add(&font_row);
    let opacity = adw::SpinRow::builder()
        .title(gettext("Background opacity"))
        .subtitle(gettext("In percent; below 100 the desktop shows through"))
        .digits(0)
        .adjustment(&gtk::Adjustment::new(
            (profile.opacity() * 100.0).round(),
            parkett_core::profile::MIN_OPACITY * 100.0,
            100.0,
            5.0,
            10.0,
            0.0,
        ))
        .build();
    let d = dialog.downgrade();
    let i = id.clone();
    opacity.connect_value_notify(move |row| {
        if let Some(d) = d.upgrade() {
            let v = (row.value() / 100.0).clamp(parkett_core::profile::MIN_OPACITY, 1.0);
            update(&d, &i, |p| p.opacity = v);
        }
    });
    look.add(&opacity);
    let shapes = [CursorShape::Block, CursorShape::Ibeam, CursorShape::Underline];
    look.add(&choice_row(
        dialog,
        &id,
        &gettext("Cursor shape"),
        &[gettext("Block").as_str(), gettext("I-beam").as_str(), gettext("Underline").as_str()],
        shapes.iter().position(|c| *c == profile.cursor_shape).unwrap_or(0),
        move |p, n| p.cursor_shape = shapes[n],
    ));
    let blinks = [CursorBlink::System, CursorBlink::On, CursorBlink::Off];
    look.add(&choice_row(
        dialog,
        &id,
        &gettext("Cursor blinks"),
        &[gettext("Like the system").as_str(), gettext("Always").as_str(), gettext("Never").as_str()],
        blinks.iter().position(|c| *c == profile.cursor_blink).unwrap_or(0),
        move |p, n| p.cursor_blink = blinks[n],
    ));
    let bells = [Bell::Sound, Bell::Mark, Bell::None];
    look.add(&choice_row(
        dialog,
        &id,
        &gettext("Bell"),
        &[gettext("Sound and mark tab").as_str(), gettext("Mark tab only").as_str(), gettext("Off").as_str()],
        bells.iter().position(|c| *c == profile.bell).unwrap_or(0),
        move |p, n| p.bell = bells[n],
    ));
    page.add(&look);

    // Start
    let start = adw::PreferencesGroup::builder().title(gettext("Startup")).build();
    let login = adw::SwitchRow::builder().title(gettext("Run as login shell")).active(profile.login_shell).build();
    let d = dialog.downgrade();
    let i = id.clone();
    login.connect_active_notify(move |row| {
        if let Some(d) = d.upgrade() {
            let v = row.is_active();
            update(&d, &i, |p| p.login_shell = v);
        }
    });
    let command = adw::EntryRow::builder()
        .title(gettext("Command instead of shell (e.g. ssh bender)"))
        .text(&profile.command)
        .build();
    let directory = adw::EntryRow::builder()
        .title(gettext("Start directory (empty = preferences)"))
        .text(&profile.directory)
        .build();
    let env = adw::EntryRow::builder()
        .title(gettext("Environment variables (NAME=value, separated by spaces)"))
        .text(profile.environment_line())
        .build();
    for (row, which) in [(&command, 0u8), (&directory, 1), (&env, 2)] {
        let d = dialog.downgrade();
        let i = id.clone();
        row.connect_changed(move |e| {
            let text = e.text().to_string();
            let Some(d) = d.upgrade() else { return };
            // Umgebung: halb getippte Anführungszeichen nicht als „keine Variablen“ speichern
            let env = if which == 2 && !text.trim().is_empty() {
                match glib::shell_parse_argv(text.trim()) {
                    Ok(v) => v.into_iter().map(|s| s.to_string_lossy().into_owned()).collect(),
                    Err(_) => {
                        e.add_css_class("error");
                        return;
                    }
                }
            } else {
                Vec::new()
            };
            e.remove_css_class("error");
            update(&d, &i, |p| match which {
                0 => p.command = text,
                1 => p.directory = text,
                _ => p.environment = env,
            });
        });
    }
    start.add(&login);
    start.add(&command);
    start.add(&directory);
    start.add(&env);
    page.add(&start);

    // Automatischer Wechsel: Aussehen dieses Profils für bestimmte Rechner und Ordner
    let auto = adw::PreferencesGroup::builder()
        .title(gettext("Use automatically"))
        .description(gettext("Panes show the color scheme, font and opacity of this profile while a rule matches – for example while “ssh bender” is running. Separate several entries with commas, * as wildcard."))
        .build();
    let hosts = adw::EntryRow::builder()
        .title(gettext("For hosts (e.g. bender, *.tuxedo.de)"))
        .text(profile.hosts.join(", "))
        .build();
    let dirs = adw::EntryRow::builder()
        .title(gettext("For folders, including subfolders (e.g. ~/client)"))
        .text(profile.directories.join(", "))
        .build();
    let list = |text: &str| -> Vec<String> {
        text.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect()
    };
    for (row, is_hosts) in [(&hosts, true), (&dirs, false)] {
        let d = dialog.downgrade();
        let i = id.clone();
        row.connect_changed(move |e| {
            let Some(d) = d.upgrade() else { return };
            let items = list(&e.text());
            update(&d, &i, |p| if is_hosts { p.hosts = items } else { p.directories = items });
        });
    }
    auto.add(&hosts);
    auto.add(&dirs);
    page.add(&auto);

    // eigene Vorlage für den Reiter-Namen (leer = Einstellung unter Allgemein)
    let tab_label = adw::EntryRow::builder()
        .title(gettext("Tab name template (empty = preferences)"))
        .text(profile.tab_label.as_str())
        .tooltip_text(gettext("{n} number · {program} program · {dir} folder · {path} full path · {host} host over SSH · {title} shell title · {profile} profile"))
        .build();
    let d = dialog.downgrade();
    let i = id.clone();
    tab_label.connect_changed(move |e| {
        if let Some(d) = d.upgrade() {
            let text = e.text().trim().to_string();
            update(&d, &i, |p| p.tab_label = text);
        }
    });
    let labels = adw::PreferencesGroup::builder().title(gettext("Tabs")).build();
    labels.add(&tab_label);
    page.add(&labels);

    // Prozessende
    let end = adw::PreferencesGroup::new();
    let exit = adw::ComboRow::builder()
        .title(gettext("When the process exits"))
        .model(&gtk::StringList::new(&[
            gettext("Show bar").as_str(),
            gettext("Close pane (on success)").as_str(),
            gettext("Restart").as_str(),
        ]))
        .selected(match profile.exit_action {
            ExitAction::Hold => 0,
            ExitAction::Close => 1,
            ExitAction::Restart => 2,
        })
        .build();
    let d = dialog.downgrade();
    let i = id.clone();
    exit.connect_selected_notify(move |row| {
        let action = match row.selected() {
            1 => ExitAction::Close,
            2 => ExitAction::Restart,
            _ => ExitAction::Hold,
        };
        if let Some(d) = d.upgrade() {
            update(&d, &i, |p| p.exit_action = action);
        }
    });
    end.add(&exit);
    page.add(&end);

    // Löschen
    let danger = adw::PreferencesGroup::new();
    let delete = gtk::Button::builder()
        .label(gettext("Delete profile"))
        .css_classes(["destructive-action", "pill"])
        .halign(gtk::Align::Center)
        .build();
    delete.set_sensitive(manager.profiles().len() > 1);
    danger.add(&delete);
    page.add(&danger);

    let toolbar = adw::ToolbarView::new();
    toolbar.add_top_bar(&adw::HeaderBar::new());
    toolbar.set_content(Some(&page));
    let nav = adw::NavigationPage::builder().title(profile.name.as_str()).child(&toolbar).build();
    // Beim Öffnen landet der Fokus im Namensfeld, der Name ist markiert: ein Tastendruck
    // überschriebe ihn, und gespeichert wird sofort (QA-16). Bei einem vorhandenen Profil
    // die Markierung aufheben, Schreibmarke ans Ende.
    if !new {
        let name = name.clone();
        nav.connect_shown(move |_| {
            let name = name.clone();
            glib::idle_add_local_once(move || {
                let end = name.text().chars().count() as i32;
                name.select_region(end, end);
            });
        });
    }
    nav.connect_hidden(move |_| release());
    let d = dialog.downgrade();
    delete.connect_clicked(move |_| {
        let Some(d) = d.upgrade() else { return };
        match ProfileManager::get().delete(&id) {
            Ok(()) => {
                d.pop_subpage();
            }
            Err(e) => d.add_toast(crate::window::plain_toast(&e)),
        }
    });
    nav
}

/// Kachel: links helle, rechts dunkle Fassung, darunter der Name. Der Widget-Name trägt die
/// Paletten-Id, damit die Auswahl sie ohne Nachschlagen kennt.
fn scheme_tile(pal: &Palette) -> gtk::Box {
    let preview =
        gtk::DrawingArea::builder().content_width(132).content_height(64).css_classes(["parkett-tile-preview"]).build();
    preview.set_overflow(gtk::Overflow::Hidden);
    let (light, dark) = (pal.light.clone(), pal.dark.clone());
    preview.set_draw_func(move |_, cr, w, h| {
        let half = w as f64 / 2.0;
        draw_face(cr, &light, 0.0, half, h as f64);
        draw_face(cr, &dark, half, half, h as f64);
    });
    let label = gtk::Label::builder()
        .label(pal.name.as_str())
        .ellipsize(gtk::pango::EllipsizeMode::End)
        .css_classes(["caption"])
        .build();
    let tile =
        gtk::Box::builder().orientation(gtk::Orientation::Vertical).spacing(6).css_classes(["parkett-tile"]).build();
    tile.set_widget_name(&pal.id);
    tile.append(&preview);
    tile.append(&label);
    if !pal.has_variants() {
        tile.set_tooltip_text(Some(&gettext("Only one variant – applies to light and dark")));
    }
    tile.update_property(&[gtk::accessible::Property::Label(&trf("Color scheme {name}", &[("name", &pal.name)]))]);
    tile
}

/// Zeichnet eine Fassung: Hintergrund, eine „Textzeile“ in Vordergrundfarbe und Farbfelder.
fn draw_face(cr: &gtk::cairo::Context, face: &Face, x: f64, w: f64, h: f64) {
    let set = |c: &parkett_core::palette::Rgb| {
        let (r, g, b) = c.to_f64();
        cr.set_source_rgb(r, g, b);
    };
    set(&face.background);
    cr.rectangle(x, 0.0, w, h);
    let _ = cr.fill();
    set(&face.colors[2]);
    cr.rectangle(x + 8.0, 12.0, 10.0, 6.0);
    let _ = cr.fill();
    set(&face.foreground);
    cr.rectangle(x + 22.0, 12.0, w - 34.0, 6.0);
    let _ = cr.fill();
    let size = ((w - 16.0) / 6.0).min(10.0);
    for (n, idx) in [1usize, 2, 3, 4, 5, 6].iter().enumerate() {
        set(&face.colors[*idx]);
        cr.rectangle(x + 8.0 + n as f64 * size, 30.0, size - 2.0, size - 2.0);
        let _ = cr.fill();
    }
    set(&face.foreground);
    cr.rectangle(x + 8.0, 46.0, (w - 16.0) * 0.55, 5.0);
    let _ = cr.fill();
}

/// Zeile, Kürzel-Anzeige und Aktion einer Zeile auf der Kürzel-Seite.
type ShortcutRow = (adw::ActionRow, gtk::Label, &'static str);

fn shortcuts_page(dialog: &adw::PreferencesDialog) -> adw::PreferencesPage {
    use crate::shortcuts;
    let page = adw::PreferencesPage::builder().title(gettext("Shortcuts")).icon_name("input-keyboard-symbolic").build();
    let rows: Rc<RefCell<Vec<ShortcutRow>>> = Rc::default();
    for (n, section) in shortcuts::SECTIONS.iter().enumerate() {
        let group = adw::PreferencesGroup::builder().title(gettext(*section)).build();
        if n == 0 {
            group.set_description(Some(&gettext("Click a row and press a new combination.")));
            let reset = gtk::Button::builder()
                .label(gettext("Reset all"))
                .css_classes(["flat"])
                .valign(gtk::Align::Center)
                .build();
            reset.connect_clicked(|_| shortcuts::reset_all());
            group.set_header_suffix(Some(&reset));
        }
        for e in shortcuts::ENTRIES.iter().filter(|e| e.section == *section) {
            let label = gtk::Label::builder().css_classes(["dim-label"]).build();
            let row = adw::ActionRow::builder().title(gettext(e.label)).activatable(true).build();
            row.add_suffix(&label);
            let d = dialog.downgrade();
            let action = e.action;
            row.connect_activated(move |_| {
                if let Some(d) = d.upgrade() {
                    capture_shortcut(&d, action);
                }
            });
            group.add(&row);
            rows.borrow_mut().push((row, label, e.action));
        }
        page.add(&group);
    }
    let refresh = {
        let rows = rows.clone();
        move || {
            for (row, label, action) in rows.borrow().iter() {
                label.set_label(&shortcuts::label(&shortcuts::effective(action)));
                row.set_subtitle(&if shortcuts::is_custom(action) { gettext("Custom") } else { String::new() });
            }
        }
    };
    refresh();
    let settings = crate::settings::settings();
    let release =
        release_on_destroy(&page, &settings, settings.connect_changed(Some("custom-shortcuts"), move |_, _| refresh()));
    dialog.connect_closed(move |_| release());
    page
}

/// Nimmt eine neue Tastenkombination auf. Esc bricht ab, Rücktaste schaltet das Kürzel aus.
/// Während der Aufnahme sind Fenster- und App-Kürzel abgeschaltet, sonst lösten sie aus.
fn capture_shortcut(prefs: &adw::PreferencesDialog, action: &'static str) {
    use crate::shortcuts;
    let Some(entry) = shortcuts::entry(action) else { return };
    let current = shortcuts::label(&shortcuts::effective(action));
    let hint = trf(
        "Current: {current}\nPress a new combination. Esc cancels, Backspace disables the shortcut.",
        &[("current", &current)],
    );
    let dialog = adw::AlertDialog::new(Some(&gettext(entry.label)), Some(&hint));
    dialog.add_responses(&[("cancel", gettext("Cancel").as_str()), ("default", gettext("Restore default").as_str())]);
    dialog.set_close_response("cancel");

    let window = prefs.root().and_downcast::<crate::window::Window>();
    let app = window.as_ref().and_then(|w| w.application());
    if let Some(w) = &window {
        w.set_shortcuts_enabled(false);
    }
    if let Some(app) = &app {
        for e in shortcuts::ENTRIES.iter().filter(|e| e.action.starts_with("app.")) {
            app.set_accels_for_action(e.action, &[]);
        }
    }
    let restore = {
        let window = window.clone();
        move || {
            if let Some(w) = &window {
                w.set_shortcuts_enabled(true);
                if let Some(app) = w.application() {
                    shortcuts::apply_app_accels(&app);
                }
            }
        }
    };
    dialog.connect_closed(move |_| restore());
    dialog.connect_response(Some("default"), move |_, _| shortcuts::set(action, None));

    let keys = gtk::EventControllerKey::new();
    keys.set_propagation_phase(gtk::PropagationPhase::Capture);
    let d = dialog.downgrade();
    keys.connect_key_pressed(move |_, key, _, state| {
        let Some(d) = d.upgrade() else { return glib::Propagation::Proceed };
        let plain = (state & gtk::accelerator_get_default_mod_mask()).is_empty();
        if plain && key == gtk::gdk::Key::Escape {
            d.close();
            return glib::Propagation::Stop;
        }
        if plain && key == gtk::gdk::Key::BackSpace {
            shortcuts::set(action, Some(""));
            d.close();
            return glib::Propagation::Stop;
        }
        match shortcuts::from_key(key, state) {
            Ok(None) => glib::Propagation::Stop,
            Err(msg) => {
                d.set_body(&gettext(msg));
                glib::Propagation::Stop
            }
            Ok(Some(accel)) => {
                let taken =
                    shortcuts::conflict(action, &accel).map(|e| e.label).or_else(|| shortcuts::reserved(&accel));
                if let Some(other) = taken {
                    let label = shortcuts::label(std::slice::from_ref(&accel));
                    let other = gettext(other);
                    d.set_body(&trf(
                        "{label} is already assigned to “{other}”. Please press a different combination.",
                        &[("label", &label), ("other", &other)],
                    ));
                } else {
                    shortcuts::set(action, Some(&accel));
                    d.close();
                }
                glib::Propagation::Stop
            }
        }
    });
    dialog.add_controller(keys);
    dialog.present(Some(prefs));
}

/// Funktion, die eine Liste neu aufbaut; von ihren eigenen Knöpfen aus aufrufbar.
type Refill = Rc<RefCell<Option<Rc<dyn Fn()>>>>;

/// Lesezeichen umbenennen, sortieren, löschen und Ordner hinzufügen. Jede Änderung wird
/// sofort gespeichert; die Liste wird danach neu aufgebaut.
fn bookmarks_page(dialog: &adw::PreferencesDialog) -> adw::PreferencesPage {
    let page = adw::PreferencesPage::builder().title(gettext("Bookmarks")).icon_name("user-bookmarks-symbolic").build();
    let add =
        gtk::Button::builder().label(gettext("Add folder…")).css_classes(["flat"]).valign(gtk::Align::Center).build();
    let group = adw::PreferencesGroup::builder()
        .title(gettext("Bookmarks"))
        .description(gettext("Folders for the command palette (~) and the main menu: Enter opens a new tab, Ctrl+Enter a new pane, Alt+Enter switches to it in the active pane."))
        .header_suffix(&add)
        .build();
    page.add(&group);
    let rows: Rc<RefCell<Vec<gtk::Widget>>> = Rc::default();
    // Fehler beim Speichern als Einblendung im Dialog
    let store = {
        let dialog = dialog.downgrade();
        move |list: &[parkett_core::bookmark::Bookmark]| {
            if let Err(e) = crate::bookmarks::save(list) {
                if let Some(d) = dialog.upgrade() {
                    d.add_toast(crate::window::plain_toast(&trf(
                        "Bookmarks not saved: {error}",
                        &[("error", &e.to_string())],
                    )));
                }
            }
        }
    };
    let fill: Refill = Rc::default();
    let f = {
        let group = group.downgrade();
        let rows = rows.clone();
        let fill = fill.clone();
        let store = store.clone();
        move || {
            let Some(group) = group.upgrade() else { return };
            for r in rows.borrow_mut().drain(..) {
                group.remove(&r);
            }
            let list = crate::bookmarks::load();
            if list.is_empty() {
                let empty = adw::ActionRow::builder()
                    .title(gettext("No bookmarks yet"))
                    .subtitle(gettext("In the terminal: command palette or main menu › “Current folder as bookmark”"))
                    .sensitive(false)
                    .build();
                group.add(&empty);
                rows.borrow_mut().push(empty.upcast());
                return;
            }
            let n = list.len();
            for (i, b) in list.iter().enumerate() {
                // Name bearbeitbar, Pfad als Überschrift
                let row = adw::EntryRow::builder().title(b.path.as_str()).text(b.name.as_str()).build();
                let store_name = store.clone();
                row.connect_changed(move |e| {
                    let mut list = crate::bookmarks::load();
                    if let Some(entry) = list.get_mut(i) {
                        entry.name = e.text().trim().to_string();
                        store_name(&list);
                    }
                });
                for (icon, tip, delta) in [
                    ("go-up-symbolic", gettext("Move up"), -1i32),
                    ("go-down-symbolic", gettext("Move down"), 1),
                    ("user-trash-symbolic", gettext("Delete"), 0),
                ] {
                    let button = gtk::Button::builder()
                        .icon_name(icon)
                        .tooltip_text(tip)
                        .valign(gtk::Align::Center)
                        .css_classes(["flat"])
                        .build();
                    button.set_sensitive(match delta {
                        -1 => i > 0,
                        1 => i + 1 < n,
                        _ => true,
                    });
                    let store = store.clone();
                    let fill = fill.clone();
                    button.connect_clicked(move |_| {
                        let mut list = crate::bookmarks::load();
                        if i >= list.len() {
                            return;
                        }
                        match delta {
                            0 => {
                                list.remove(i);
                            }
                            d => list.swap(i, (i as i32 + d) as usize),
                        }
                        store(&list);
                        // nach dem Klick neu aufbauen, nicht mitten im Signal des Knopfs
                        let fill = fill.clone();
                        glib::idle_add_local_once(move || {
                            if let Some(f) = fill.borrow().clone() {
                                f();
                            }
                        });
                    });
                    row.add_suffix(&button);
                }
                group.add(&row);
                rows.borrow_mut().push(row.upcast());
            }
        }
    };
    let f: Rc<dyn Fn()> = Rc::new(f);
    *fill.borrow_mut() = Some(f.clone());
    f();
    // fill hält sich über die Knöpfe selbst fest; mit der Seite auflösen
    page.connect_destroy(move |_| {
        fill.borrow_mut().take();
    });
    let d = dialog.downgrade();
    add.connect_clicked(move |button| {
        let Some(d) = d.upgrade() else { return };
        let chooser = gtk::FileDialog::builder().title(gettext("Folder as bookmark")).modal(true).build();
        let window = button.root().and_downcast::<gtk::Window>();
        let f = f.clone();
        chooser.select_folder(window.as_ref(), gio::Cancellable::NONE, move |res| {
            let Ok(folder) = res else { return };
            let Some(path) = folder.path().and_then(|p| p.to_str().map(String::from)) else { return };
            match crate::bookmarks::add(&path) {
                Ok(true) => f(),
                Ok(false) => d.add_toast(crate::window::plain_toast(&gettext("This folder is already a bookmark"))),
                Err(e) => d.add_toast(crate::window::plain_toast(&trf(
                    "Bookmarks not saved: {error}",
                    &[("error", &e.to_string())],
                ))),
            }
        });
    });
    page
}

fn templates_page(dialog: &adw::PreferencesDialog) -> adw::PreferencesPage {
    let page = adw::PreferencesPage::builder().title(gettext("Templates")).icon_name("view-grid-symbolic").build();
    let group = adw::PreferencesGroup::builder()
        .title(gettext("Templates"))
        .description(gettext("A template opens a tab with a ready-made layout, profiles and start commands. New templates are made from a running tab: menu on the New button › “Save current tab as template…”."))
        .build();
    page.add(&group);
    let rows: Rc<RefCell<Vec<gtk::Widget>>> = Rc::default();
    let fill = {
        let group = group.downgrade();
        let rows = rows.clone();
        let dialog = dialog.downgrade();
        move || {
            let Some(group) = group.upgrade() else { return };
            for r in rows.borrow_mut().drain(..) {
                group.remove(&r);
            }
            let templates = ProfileManager::get().templates();
            if templates.is_empty() {
                let empty = adw::ActionRow::builder()
                    .title(gettext("No templates yet"))
                    .subtitle(gettext("Split a tab as you like and save it as a template"))
                    .sensitive(false)
                    .build();
                group.add(&empty);
                rows.borrow_mut().push(empty.upcast());
            }
            for t in templates {
                let n = t.layout.panes().len();
                let dir = if t.directory.trim().is_empty() {
                    gettext("Choose directory when opening")
                } else {
                    t.directory.clone()
                };
                let row = adw::ActionRow::builder()
                    .title(glib::markup_escape_text(&t.name).as_str())
                    .subtitle(
                        glib::markup_escape_text(&ntrf(
                            "{n} pane · {dir}",
                            "{n} panes · {dir}",
                            n as u32,
                            &[("dir", &dir)],
                        ))
                        .as_str(),
                    )
                    .activatable(true)
                    .build();
                row.add_suffix(&gtk::Image::from_icon_name("go-next-symbolic"));
                let id = t.id.clone();
                let dialog = dialog.clone();
                row.connect_activated(move |_| {
                    let Some(d) = dialog.upgrade() else { return };
                    // inzwischen gelöscht (z. B. aus einem zweiten Fenster)?
                    match template_editor(&d, &id) {
                        Some(page) => d.push_subpage(&page),
                        None => d.add_toast(crate::window::plain_toast(&gettext("The template no longer exists"))),
                    }
                });
                group.add(&row);
                rows.borrow_mut().push(row.upcast());
            }
        }
    };
    fill();
    let release =
        release_on_destroy(&group, &ProfileManager::get(), ProfileManager::get().connect_templates_changed(fill));
    dialog.connect_closed(move |_| release());
    page
}

fn update_template(dialog: &adw::PreferencesDialog, id: &str, f: impl FnOnce(&mut parkett_core::template::Template)) {
    let manager = ProfileManager::get();
    let Some(mut t) = manager.template(id) else { return };
    f(&mut t);
    if let Err(e) = manager.save_template(&t) {
        dialog.add_toast(crate::window::plain_toast(&trf("Not saved: {error}", &[("error", &e.to_string())])));
    }
}

fn template_editor(dialog: &adw::PreferencesDialog, id: &str) -> Option<adw::NavigationPage> {
    let manager = ProfileManager::get();
    let template = manager.template(id)?;
    let id = template.id.clone();
    let page = adw::PreferencesPage::new();

    let general = adw::PreferencesGroup::new();
    let name = adw::EntryRow::builder().title(gettext("Name")).text(&template.name).build();
    let d = dialog.downgrade();
    let i = id.clone();
    name.connect_changed(move |e| {
        let text = e.text().trim().to_string();
        if let (false, Some(d)) = (text.is_empty(), d.upgrade()) {
            update_template(&d, &i, |t| t.name = text);
        }
    });
    let dir = adw::EntryRow::builder()
        .title(gettext("Fixed directory (empty = choose when opening)"))
        .text(&template.directory)
        .build();
    let d = dialog.downgrade();
    let i = id.clone();
    dir.connect_changed(move |e| {
        let text = e.text().trim().to_string();
        if let Some(d) = d.upgrade() {
            update_template(&d, &i, |t| t.directory = text);
        }
    });
    general.add(&name);
    general.add(&dir);
    page.add(&general);

    // Vorschau der Aufteilung mit Nummern
    let preview = gtk::DrawingArea::builder().content_height(140).css_classes(["parkett-tile-preview"]).build();
    preview.set_overflow(gtk::Overflow::Hidden);
    let (layout, specs) = template.instantiate();
    let names: Vec<String> =
        specs.iter().map(|(_, s)| if s.name.is_empty() { gettext("Shell") } else { s.name.clone() }).collect();
    preview.set_draw_func(move |area, cr, w, h| {
        let fg = area.color();
        let rects = layout.geometry(w as f32, h as f32);
        for (n, (_, r)) in rects.iter().enumerate() {
            cr.set_source_rgba(fg.red() as f64, fg.green() as f64, fg.blue() as f64, 0.08 + 0.04 * (n % 2) as f64);
            cr.rectangle(r.x as f64 + 2.0, r.y as f64 + 2.0, r.w as f64 - 4.0, r.h as f64 - 4.0);
            let _ = cr.fill();
            cr.set_source_rgba(fg.red() as f64, fg.green() as f64, fg.blue() as f64, 0.9);
            cr.move_to(r.x as f64 + 10.0, r.y as f64 + 22.0);
            cr.set_font_size(13.0);
            let _ = cr.show_text(&format!("{}  {}", n + 1, names.get(n).map(String::as_str).unwrap_or("")));
        }
    });
    // Aufteilung ändern: je Bereich teilen/entfernen, hier angleichen; danach die Seite neu
    let equalize =
        gtk::Button::builder().label(gettext("Equalize")).css_classes(["flat"]).valign(gtk::Align::Center).build();
    let pg = adw::PreferencesGroup::builder()
        .title(gettext("Layout"))
        .description(gettext(
            "Comes from the tab the template was saved from; split or remove panes with the buttons on them.",
        ))
        .header_suffix(&equalize)
        .build();
    let edit_layout = {
        let d = dialog.downgrade();
        let i = id.clone();
        move |f: &dyn Fn(&mut parkett_core::template::TemplateNode)| {
            let Some(d) = d.upgrade() else { return };
            update_template(&d, &i, |t| f(&mut t.layout));
            let (d2, i2) = (d.clone(), i.clone());
            // neu aufbauen nach dem Klick, nicht mitten im Signal des Knopfs
            glib::idle_add_local_once(move || {
                d2.pop_subpage();
                if let Some(page) = template_editor(&d2, &i2) {
                    d2.push_subpage(&page);
                }
            });
        }
    };
    let edit_layout = Rc::new(edit_layout);
    let e = edit_layout.clone();
    equalize.connect_clicked(move |_| e(&|l| l.equalize()));
    pg.add(&preview);
    page.add(&pg);

    let profiles = manager.profiles();
    let mut profile_names = vec![gettext("Default profile")];
    profile_names.extend(profiles.iter().map(|p| p.name.clone()));
    let refs: Vec<&str> = profile_names.iter().map(String::as_str).collect();
    let count = template.layout.panes().len();
    for (n, spec) in template.layout.panes().iter().enumerate() {
        let buttons = gtk::Box::builder().spacing(2).valign(gtk::Align::Center).build();
        for (icon, tip, action) in [
            ("parkett-split-right-symbolic", gettext("Split right"), 0u8),
            ("parkett-split-down-symbolic", gettext("Split down"), 1),
            ("user-trash-symbolic", gettext("Remove pane"), 2),
        ] {
            let b = gtk::Button::builder().icon_name(icon).tooltip_text(tip).css_classes(["flat"]).build();
            b.set_sensitive(action != 2 || count > 1);
            let e = edit_layout.clone();
            b.connect_clicked(move |_| {
                e(&|l| {
                    match action {
                        0 => l.split_pane(n, parkett_core::template::SplitDirection::Horizontal),
                        1 => l.split_pane(n, parkett_core::template::SplitDirection::Vertical),
                        _ => l.remove_pane(n),
                    };
                })
            });
            buttons.append(&b);
        }
        let g = adw::PreferencesGroup::builder()
            .title(trf("Pane {n}", &[("n", &(n + 1).to_string())]))
            .header_suffix(&buttons)
            .build();
        let pname = adw::EntryRow::builder().title(gettext("Name")).text(&spec.name).build();
        let profile = adw::ComboRow::builder().title(gettext("Profile")).model(&gtk::StringList::new(&refs)).build();
        profile.set_selected(profiles.iter().position(|p| p.id == spec.profile).map(|i| i as u32 + 1).unwrap_or(0));
        let command =
            adw::EntryRow::builder().title(gettext("Start command (typed into the shell)")).text(&spec.command).build();
        let subdir = adw::EntryRow::builder().title(gettext("Subdirectory")).text(&spec.subdir).build();
        for (row, field) in [(&pname, 0u8), (&command, 1), (&subdir, 2)] {
            let d = dialog.downgrade();
            let i = id.clone();
            row.connect_changed(move |e| {
                let text = e.text().to_string();
                if let Some(d) = d.upgrade() {
                    update_template(&d, &i, |t| {
                        if let Some(p) = t.layout.panes_mut().into_iter().nth(n) {
                            match field {
                                0 => p.name = text,
                                1 => p.command = text,
                                _ => p.subdir = text,
                            }
                        }
                    });
                }
            });
        }
        let d = dialog.downgrade();
        let i = id.clone();
        let ids: Vec<String> = profiles.iter().map(|p| p.id.clone()).collect();
        profile.connect_selected_notify(move |row| {
            let value = match row.selected() {
                0 => String::new(),
                k => ids.get(k as usize - 1).cloned().unwrap_or_default(),
            };
            if let Some(d) = d.upgrade() {
                update_template(&d, &i, |t| {
                    if let Some(p) = t.layout.panes_mut().into_iter().nth(n) {
                        p.profile = value;
                    }
                });
            }
        });
        g.add(&pname);
        g.add(&profile);
        g.add(&command);
        g.add(&subdir);
        page.add(&g);
    }

    let danger = adw::PreferencesGroup::new();
    let delete = gtk::Button::builder()
        .label(gettext("Delete template"))
        .css_classes(["destructive-action", "pill"])
        .halign(gtk::Align::Center)
        .build();
    danger.add(&delete);
    page.add(&danger);
    let d = dialog.downgrade();
    delete.connect_clicked(move |_| {
        let Some(d) = d.upgrade() else { return };
        match ProfileManager::get().delete_template(&id) {
            Ok(()) => {
                d.pop_subpage();
            }
            Err(e) => d.add_toast(crate::window::plain_toast(&e)),
        }
    });

    let toolbar = adw::ToolbarView::new();
    toolbar.add_top_bar(&adw::HeaderBar::new());
    toolbar.set_content(Some(&page));
    let nav = adw::NavigationPage::builder().title(template.name.as_str()).child(&toolbar).build();
    // wie bei Profilen (QA-16): Name nicht markiert, sonst überschreibt ihn ein Tastendruck
    nav.connect_shown(move |_| {
        let name = name.clone();
        glib::idle_add_local_once(move || {
            let end = name.text().chars().count() as i32;
            name.select_region(end, end);
        });
    });
    Some(nav)
}

/// Auswahlzeile für ein Profilfeld; `set` bekommt den gewählten Index.
fn choice_row(
    dialog: &adw::PreferencesDialog,
    id: &str,
    title: &str,
    labels: &[&str],
    selected: usize,
    set: impl Fn(&mut Profile, usize) + 'static,
) -> adw::ComboRow {
    let row =
        adw::ComboRow::builder().title(title).model(&gtk::StringList::new(labels)).selected(selected as u32).build();
    let d = dialog.downgrade();
    let id = id.to_string();
    row.connect_selected_notify(move |row| {
        if let Some(d) = d.upgrade() {
            let n = row.selected() as usize;
            update(&d, &id, |p| set(p, n));
        }
    });
    row
}
