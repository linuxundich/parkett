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

use crate::profiles::ProfileManager;

pub fn show(parent: Option<&gtk::Window>) {
    let dialog = adw::PreferencesDialog::builder().title("Einstellungen").search_enabled(true).build();
    dialog.add(&general_page());
    dialog.add(&profiles_page(&dialog));
    dialog.add(&templates_page(&dialog));
    dialog.add(&shortcuts_page(&dialog));
    dialog.present(parent);
}

fn general_page() -> adw::PreferencesPage {
    let settings = crate::settings::settings();
    let page = adw::PreferencesPage::builder().title("Allgemein").icon_name("preferences-system-symbolic").build();

    let look = adw::PreferencesGroup::builder().title("Darstellung").build();
    let dim = adw::SwitchRow::builder()
        .title("Inaktive Bereiche abblenden")
        .subtitle("Zeigt in geteilten Reitern, welcher Bereich den Fokus hat")
        .build();
    settings.bind("dim-unfocused", &dim, "active").build();
    let tabbar = adw::SwitchRow::builder().title("Reiterleiste bei einem Reiter ausblenden").build();
    settings.bind("autohide-tab-bar", &tabbar, "active").build();
    let context = adw::SwitchRow::builder()
        .title("Kontextfarbe für root und SSH")
        .subtitle("Linie, Chip und getönter Hintergrund, Farben aus dem Farbschema")
        .build();
    settings.bind("context-colors", &context, "active").build();
    let style = adw::ComboRow::builder()
        .title("Stil")
        .model(&gtk::StringList::new(&["Systemstil folgen", "Hell", "Dunkel"]))
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
    let title = adw::ComboRow::builder()
        .title("Bereichstitel bei geteilten Reitern")
        .model(&gtk::StringList::new(&["Schwebend", "Als Zeile"]))
        .build();
    title.set_selected(if settings.string("pane-title") == "row" { 1 } else { 0 });
    title.connect_selected_notify(|row| {
        let _ =
            crate::settings::settings().set_string("pane-title", if row.selected() == 1 { "row" } else { "floating" });
    });
    look.add(&title);
    look.add(&dim);
    look.add(&context);
    look.add(&tabbar);
    page.add(&look);

    let behaviour = adw::PreferencesGroup::builder().title("Verhalten").build();
    let dir = adw::ComboRow::builder()
        .title("Neue Terminals öffnen in")
        .model(&gtk::StringList::new(&["Verzeichnis des aktiven Bereichs", "Persönlichem Ordner"]))
        .build();
    dir.set_selected(if settings.string("new-terminal-directory") == "home" { 1 } else { 0 });
    dir.connect_selected_notify(|row| {
        let _ = crate::settings::settings()
            .set_string("new-terminal-directory", if row.selected() == 1 { "home" } else { "current" });
    });
    let confirm = adw::SwitchRow::builder()
        .title("Vor dem Schließen nachfragen")
        .subtitle("Wenn in einem Bereich noch ein Programm läuft")
        .build();
    settings.bind("confirm-close", &confirm, "active").build();
    let scrollback = adw::SpinRow::builder()
        .title("Zeilen im Verlauf")
        .subtitle("−1 = unbegrenzt")
        .adjustment(&gtk::Adjustment::new(10000.0, -1.0, 1_000_000.0, 1000.0, 10000.0, 0.0))
        .build();
    settings.bind("scrollback-lines", &scrollback, "value").build();
    behaviour.add(&dir);
    behaviour.add(&confirm);
    let paste = adw::SwitchRow::builder()
        .title("Vor mehrzeiligem Einfügen warnen")
        .subtitle("Zeigt den Text vorher; gilt für Strg+Umschalt+V und das Kontextmenü")
        .build();
    settings.bind("warn-multiline-paste", &paste, "active").build();
    behaviour.add(&paste);
    let restore = adw::SwitchRow::builder()
        .title("Letzte Sitzung wiederherstellen")
        .subtitle("Fenster, Reiter, Aufteilung und Verzeichnisse – ohne die Programme darin")
        .build();
    settings.bind("restore-session", &restore, "active").build();
    behaviour.add(&restore);
    let notify = adw::SwitchRow::builder()
        .title("Benachrichtigen, wenn ein Befehl endet")
        .subtitle("Nach mindestens 10 Sekunden Laufzeit, wenn der Bereich nicht zu sehen ist")
        .build();
    settings.bind("notify-command-finished", &notify, "active").build();
    behaviour.add(&notify);
    let sixel = adw::SwitchRow::builder().title("Sixel-Grafiken anzeigen").build();
    if crate::terminal::sixel_supported() {
        sixel.set_subtitle("Bilder im Terminal, z. B. von img2sixel oder chafa");
        settings.bind("enable-sixel", &sixel, "active").build();
    } else {
        sixel.set_subtitle("Die installierte VTE-Bibliothek ist ohne Sixel gebaut");
        sixel.set_sensitive(false);
    }
    behaviour.add(&sixel);
    behaviour.add(&scrollback);
    page.add(&behaviour);
    page
}

fn profiles_page(dialog: &adw::PreferencesDialog) -> adw::PreferencesPage {
    let page = adw::PreferencesPage::builder().title("Profile").icon_name("applications-utilities-symbolic").build();
    let add = gtk::Button::builder()
        .icon_name("list-add-symbolic")
        .tooltip_text("Neues Profil")
        .css_classes(["flat"])
        .valign(gtk::Align::Center)
        .build();
    let group = adw::PreferencesGroup::builder()
        .title("Profile")
        .description("Ein Profil legt fest, wie ein Terminal startet und welches Farbschema es nutzt.")
        .header_suffix(&add)
        .build();
    page.add(&group);

    let rows: Rc<RefCell<Vec<adw::ActionRow>>> = Rc::default();
    let fill = {
        let group = group.clone();
        let rows = rows.clone();
        let dialog = dialog.downgrade();
        move || {
            for r in rows.borrow_mut().drain(..) {
                group.remove(&r);
            }
            let manager = ProfileManager::get();
            let default = manager.default_id();
            for p in manager.profiles() {
                let pal = manager.palette(&p.palette);
                let subtitle =
                    if p.id == default { format!("{} · Standardprofil", pal.name) } else { pal.name.clone() };
                let row = adw::ActionRow::builder()
                    .title(glib::markup_escape_text(&p.name).as_str())
                    .subtitle(subtitle.as_str())
                    .activatable(true)
                    .build();
                row.add_suffix(&gtk::Image::from_icon_name("go-next-symbolic"));
                let id = p.id.clone();
                let dialog = dialog.clone();
                row.connect_activated(move |_| {
                    if let Some(d) = dialog.upgrade() {
                        d.push_subpage(&profile_editor(&d, &id));
                    }
                });
                group.add(&row);
                rows.borrow_mut().push(row);
            }
        }
    };
    fill();
    let handler = ProfileManager::get().connect_changed(fill);
    let handler = RefCell::new(Some(handler));
    dialog.connect_closed(move |_| {
        if let Some(h) = handler.borrow_mut().take() {
            ProfileManager::get().disconnect(h);
        }
    });

    let weak = dialog.downgrade();
    add.connect_clicked(move |_| {
        let Some(d) = weak.upgrade() else { return };
        match ProfileManager::get().create("Neues Profil") {
            Ok(p) => d.push_subpage(&profile_editor(&d, &p.id)),
            Err(e) => d.add_toast(adw::Toast::new(&format!("Profil nicht angelegt: {e}"))),
        }
    });
    page
}

/// Ändert das gespeicherte Profil `id` und speichert sofort.
fn update(dialog: &adw::PreferencesDialog, id: &str, f: impl FnOnce(&mut Profile)) {
    let manager = ProfileManager::get();
    let mut p = manager.profile(id);
    if p.id != id {
        return; // inzwischen gelöscht
    }
    f(&mut p);
    if let Err(e) = manager.save(&p) {
        dialog.add_toast(adw::Toast::new(&format!("Nicht gespeichert: {e}")));
    }
}

fn profile_editor(dialog: &adw::PreferencesDialog, id: &str) -> adw::NavigationPage {
    let manager = ProfileManager::get();
    let profile = manager.profile(id);
    let id = profile.id.clone();
    let page = adw::PreferencesPage::new();

    // Allgemein
    let general = adw::PreferencesGroup::new();
    let name = adw::EntryRow::builder().title("Name").text(&profile.name).build();
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
        .title("Standardprofil")
        .subtitle("Für neue Reiter")
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
        .label("Importieren …")
        .css_classes(["flat"])
        .valign(gtk::Align::Center)
        .tooltip_text("Ptyxis-Palette (.palette) oder Tilix-Schema (.json)")
        .build();
    let schemes = adw::PreferencesGroup::builder()
        .title("Farbschema")
        .description("Jedes Schema hat eine helle und eine dunkle Fassung (links/rechts); Parkett folgt dem Systemstil. Die Auswahl gilt sofort.")
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
    let handler = RefCell::new(Some(ProfileManager::get().connect_changed({
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
    })));
    let d = dialog.downgrade();
    let i = id.clone();
    import.connect_clicked(move |button| {
        let Some(d) = d.upgrade() else { return };
        let filters = gio::ListStore::new::<gtk::FileFilter>();
        let filter = gtk::FileFilter::new();
        filter.set_name(Some("Farbschemata (.palette, .json)"));
        filter.add_pattern("*.palette");
        filter.add_pattern("*.json");
        filters.append(&filter);
        let chooser = gtk::FileDialog::builder().title("Farbschema importieren").filters(&filters).modal(true).build();
        let window = button.root().and_downcast::<gtk::Window>();
        let i = i.clone();
        chooser.open(window.as_ref(), gio::Cancellable::NONE, move |res| {
            let Ok(file) = res else { return };
            let Some(path) = file.path() else { return };
            match ProfileManager::get().import(&path) {
                Ok(pal) => {
                    update(&d, &i, |p| p.palette = pal.id.clone());
                    let note = if pal.has_variants() {
                        String::new()
                    } else {
                        " (nur eine Fassung, gilt für hell und dunkel)".into()
                    };
                    d.add_toast(adw::Toast::new(&format!("„{}“ importiert{note}", pal.name)));
                }
                Err(e) => d.add_toast(adw::Toast::new(&format!("Import fehlgeschlagen: {e}"))),
            }
        });
    });

    // Schrift, Cursor, Glocke
    let look = adw::PreferencesGroup::builder().title("Schrift und Cursor").build();
    let system_font = adw::SwitchRow::builder()
        .title("Systemschrift verwenden")
        .subtitle("Festbreitenschrift aus den GNOME-Einstellungen")
        .active(profile.font.trim().is_empty())
        .build();
    let font_dialog = gtk::FontDialog::builder().title("Schrift für das Terminal").modal(true).build();
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
    let font_row = adw::ActionRow::builder().title("Schrift").sensitive(!system_font.is_active()).build();
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
        .title("Deckkraft des Hintergrunds")
        .subtitle("In Prozent; unter 100 scheint der Desktop durch")
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
        "Cursorform",
        &["Block", "Strich", "Unterstrich"],
        shapes.iter().position(|c| *c == profile.cursor_shape).unwrap_or(0),
        move |p, n| p.cursor_shape = shapes[n],
    ));
    let blinks = [CursorBlink::System, CursorBlink::On, CursorBlink::Off];
    look.add(&choice_row(
        dialog,
        &id,
        "Cursor blinkt",
        &["Wie im System", "Immer", "Nie"],
        blinks.iter().position(|c| *c == profile.cursor_blink).unwrap_or(0),
        move |p, n| p.cursor_blink = blinks[n],
    ));
    let bells = [Bell::Sound, Bell::Mark, Bell::None];
    look.add(&choice_row(
        dialog,
        &id,
        "Glocke",
        &["Ton und Reiter markieren", "Nur Reiter markieren", "Aus"],
        bells.iter().position(|c| *c == profile.bell).unwrap_or(0),
        move |p, n| p.bell = bells[n],
    ));
    page.add(&look);

    // Start
    let start = adw::PreferencesGroup::builder().title("Start").build();
    let login = adw::SwitchRow::builder().title("Als Login-Shell starten").active(profile.login_shell).build();
    let d = dialog.downgrade();
    let i = id.clone();
    login.connect_active_notify(move |row| {
        if let Some(d) = d.upgrade() {
            let v = row.is_active();
            update(&d, &i, |p| p.login_shell = v);
        }
    });
    let command =
        adw::EntryRow::builder().title("Befehl statt Shell (z. B. ssh bender)").text(&profile.command).build();
    let directory =
        adw::EntryRow::builder().title("Startverzeichnis (leer = Einstellung)").text(&profile.directory).build();
    let env = adw::EntryRow::builder()
        .title("Umgebungsvariablen (NAME=wert, mit Leerzeichen getrennt)")
        .text(profile.environment.join(" "))
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

    // Prozessende
    let end = adw::PreferencesGroup::new();
    let exit = adw::ComboRow::builder()
        .title("Wenn der Prozess endet")
        .model(&gtk::StringList::new(&["Leiste anzeigen", "Bereich schließen (bei Erfolg)", "Neu starten"]))
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
        .label("Profil löschen")
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
    nav.connect_hidden(move |_| {
        if let Some(h) = handler.borrow_mut().take() {
            ProfileManager::get().disconnect(h);
        }
    });
    let d = dialog.downgrade();
    delete.connect_clicked(move |_| {
        let Some(d) = d.upgrade() else { return };
        match ProfileManager::get().delete(&id) {
            Ok(()) => {
                d.pop_subpage();
            }
            Err(e) => d.add_toast(adw::Toast::new(&e)),
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
        tile.set_tooltip_text(Some("Nur eine Fassung – gilt für hell und dunkel"));
    }
    tile.update_property(&[gtk::accessible::Property::Label(&format!("Farbschema {}", pal.name))]);
    tile
}

/// Zeichnet eine Fassung: Hintergrund, eine „Textzeile“ in Vordergrundfarbe und Farbfelder.
fn draw_face(cr: &gtk::cairo::Context, face: &Face, x: f64, w: f64, h: f64) {
    let set =
        |c: &parkett_core::palette::Rgb| cr.set_source_rgb(c.r as f64 / 255.0, c.g as f64 / 255.0, c.b as f64 / 255.0);
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
    let page = adw::PreferencesPage::builder().title("Kürzel").icon_name("input-keyboard-symbolic").build();
    let rows: Rc<RefCell<Vec<ShortcutRow>>> = Rc::default();
    for (n, section) in shortcuts::SECTIONS.iter().enumerate() {
        let group = adw::PreferencesGroup::builder().title(*section).build();
        if n == 0 {
            group.set_description(Some("Zeile anklicken und neue Kombination drücken."));
            let reset = gtk::Button::builder()
                .label("Alle zurücksetzen")
                .css_classes(["flat"])
                .valign(gtk::Align::Center)
                .build();
            reset.connect_clicked(|_| shortcuts::reset_all());
            group.set_header_suffix(Some(&reset));
        }
        for e in shortcuts::ENTRIES.iter().filter(|e| e.section == *section) {
            let label = gtk::Label::builder().css_classes(["dim-label"]).build();
            let row = adw::ActionRow::builder().title(e.label).activatable(true).build();
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
                row.set_subtitle(if shortcuts::is_custom(action) { "Eigene Belegung" } else { "" });
            }
        }
    };
    refresh();
    let id = crate::settings::settings().connect_changed(Some("custom-shortcuts"), move |_, _| refresh());
    let id = RefCell::new(Some(id));
    dialog.connect_closed(move |_| {
        if let Some(id) = id.borrow_mut().take() {
            crate::settings::settings().disconnect(id);
        }
    });
    page
}

/// Nimmt eine neue Tastenkombination auf. Esc bricht ab, Rücktaste schaltet das Kürzel aus.
/// Während der Aufnahme sind Fenster- und App-Kürzel abgeschaltet, sonst lösten sie aus.
fn capture_shortcut(prefs: &adw::PreferencesDialog, action: &'static str) {
    use crate::shortcuts;
    let Some(entry) = shortcuts::entry(action) else { return };
    let current = shortcuts::label(&shortcuts::effective(action));
    let hint =
        format!("Aktuell: {current}\nNeue Kombination drücken. Esc bricht ab, Rücktaste schaltet das Kürzel aus.");
    let dialog = adw::AlertDialog::new(Some(entry.label), Some(&hint));
    dialog.add_responses(&[("cancel", "Abbrechen"), ("default", "Standard wiederherstellen")]);
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
                d.set_body(msg);
                glib::Propagation::Stop
            }
            Ok(Some(accel)) => {
                let taken =
                    shortcuts::conflict(action, &accel).map(|e| e.label).or_else(|| shortcuts::reserved(&accel));
                if let Some(other) = taken {
                    let label = shortcuts::label(std::slice::from_ref(&accel));
                    d.set_body(&format!(
                        "{label} ist schon „{other}“ zugeordnet. Bitte eine andere Kombination drücken."
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

fn templates_page(dialog: &adw::PreferencesDialog) -> adw::PreferencesPage {
    let page = adw::PreferencesPage::builder().title("Vorlagen").icon_name("view-grid-symbolic").build();
    let group = adw::PreferencesGroup::builder()
        .title("Vorlagen")
        .description("Eine Vorlage öffnet einen Reiter mit fertiger Aufteilung, Profilen und Startbefehlen. Neue Vorlagen entstehen aus einem laufenden Reiter: Menü am Neu-Knopf › „Aktuellen Reiter als Vorlage speichern …“.")
        .build();
    page.add(&group);
    let rows: Rc<RefCell<Vec<gtk::Widget>>> = Rc::default();
    let fill = {
        let group = group.clone();
        let rows = rows.clone();
        let dialog = dialog.downgrade();
        move || {
            for r in rows.borrow_mut().drain(..) {
                group.remove(&r);
            }
            let templates = ProfileManager::get().templates();
            if templates.is_empty() {
                let empty = adw::ActionRow::builder()
                    .title("Noch keine Vorlagen")
                    .subtitle("Teile einen Reiter wie gewünscht auf und speichere ihn als Vorlage")
                    .sensitive(false)
                    .build();
                group.add(&empty);
                rows.borrow_mut().push(empty.upcast());
            }
            for t in templates {
                let n = t.layout.panes().len();
                let dir = if t.directory.trim().is_empty() {
                    "Verzeichnis beim Öffnen wählen".to_string()
                } else {
                    t.directory.clone()
                };
                let row = adw::ActionRow::builder()
                    .title(glib::markup_escape_text(&t.name).as_str())
                    .subtitle(format!("{n} {} · {dir}", if n == 1 { "Bereich" } else { "Bereiche" }).as_str())
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
                        None => d.add_toast(adw::Toast::new("Die Vorlage gibt es nicht mehr")),
                    }
                });
                group.add(&row);
                rows.borrow_mut().push(row.upcast());
            }
        }
    };
    fill();
    let handler = RefCell::new(Some(ProfileManager::get().connect_templates_changed(fill)));
    dialog.connect_closed(move |_| {
        if let Some(h) = handler.borrow_mut().take() {
            ProfileManager::get().disconnect(h);
        }
    });
    page
}

fn update_template(dialog: &adw::PreferencesDialog, id: &str, f: impl FnOnce(&mut parkett_core::template::Template)) {
    let manager = ProfileManager::get();
    let Some(mut t) = manager.template(id) else { return };
    f(&mut t);
    if let Err(e) = manager.save_template(&t) {
        dialog.add_toast(adw::Toast::new(&format!("Nicht gespeichert: {e}")));
    }
}

fn template_editor(dialog: &adw::PreferencesDialog, id: &str) -> Option<adw::NavigationPage> {
    let manager = ProfileManager::get();
    let template = manager.template(id)?;
    let id = template.id.clone();
    let page = adw::PreferencesPage::new();

    let general = adw::PreferencesGroup::new();
    let name = adw::EntryRow::builder().title("Name").text(&template.name).build();
    let d = dialog.downgrade();
    let i = id.clone();
    name.connect_changed(move |e| {
        let text = e.text().trim().to_string();
        if let (false, Some(d)) = (text.is_empty(), d.upgrade()) {
            update_template(&d, &i, |t| t.name = text);
        }
    });
    let dir = adw::EntryRow::builder()
        .title("Festes Verzeichnis (leer = beim Öffnen wählen)")
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
        specs.iter().map(|(_, s)| if s.name.is_empty() { "Shell".into() } else { s.name.clone() }).collect();
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
    let pg = adw::PreferencesGroup::builder()
        .title("Aufteilung")
        .description("Die Aufteilung kommt aus dem Reiter, aus dem die Vorlage gespeichert wurde.")
        .build();
    pg.add(&preview);
    page.add(&pg);

    let profiles = manager.profiles();
    let mut profile_names = vec!["Standardprofil".to_string()];
    profile_names.extend(profiles.iter().map(|p| p.name.clone()));
    let refs: Vec<&str> = profile_names.iter().map(String::as_str).collect();
    for (n, spec) in template.layout.panes().iter().enumerate() {
        let g = adw::PreferencesGroup::builder().title(format!("Bereich {}", n + 1).as_str()).build();
        let pname = adw::EntryRow::builder().title("Name").text(&spec.name).build();
        let profile = adw::ComboRow::builder().title("Profil").model(&gtk::StringList::new(&refs)).build();
        profile.set_selected(profiles.iter().position(|p| p.id == spec.profile).map(|i| i as u32 + 1).unwrap_or(0));
        let command =
            adw::EntryRow::builder().title("Startbefehl (wird in die Shell getippt)").text(&spec.command).build();
        let subdir = adw::EntryRow::builder().title("Unterverzeichnis").text(&spec.subdir).build();
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
        .label("Vorlage löschen")
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
            Err(e) => d.add_toast(adw::Toast::new(&e)),
        }
    });

    let toolbar = adw::ToolbarView::new();
    toolbar.add_top_bar(&adw::HeaderBar::new());
    toolbar.set_content(Some(&page));
    Some(adw::NavigationPage::builder().title(template.name.as_str()).child(&toolbar).build())
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
