//! Einstellungsdialog (AdwPreferencesDialog): Seiten „Allgemein“ und „Profile“.
//! Änderungen gelten sofort (GNOME-Stil); ein gewähltes Farbschema färbt laufende
//! Terminals des Profils gleich mit um (Live-Vorschau).

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use adw::prelude::*;
use gtk::{gio, glib};
use parkett_core::palette::{Face, Palette};
use parkett_core::profile::{ExitAction, Profile};

use crate::profiles::ProfileManager;

pub fn show(parent: Option<&gtk::Window>) {
    let dialog = adw::PreferencesDialog::builder().title("Einstellungen").search_enabled(true).build();
    dialog.add(&general_page());
    dialog.add(&profiles_page(&dialog));
    dialog.present(parent);
}

fn general_page() -> adw::PreferencesPage {
    let settings = crate::settings::settings();
    let page = adw::PreferencesPage::builder().title("Allgemein").icon_name("preferences-system-symbolic").build();

    let look = adw::PreferencesGroup::builder().title("Darstellung").build();
    let dim = adw::SwitchRow::builder().title("Inaktive Bereiche abblenden").subtitle("Zeigt in geteilten Reitern, welcher Bereich den Fokus hat").build();
    settings.bind("dim-unfocused", &dim, "active").build();
    let tabbar = adw::SwitchRow::builder().title("Reiterleiste bei einem Reiter ausblenden").build();
    settings.bind("autohide-tab-bar", &tabbar, "active").build();
    look.add(&dim);
    look.add(&tabbar);
    page.add(&look);

    let behaviour = adw::PreferencesGroup::builder().title("Verhalten").build();
    let dir = adw::ComboRow::builder()
        .title("Neue Terminals öffnen in")
        .model(&gtk::StringList::new(&["Verzeichnis des aktiven Bereichs", "Persönlichem Ordner"]))
        .build();
    dir.set_selected(if settings.string("new-terminal-directory") == "home" { 1 } else { 0 });
    dir.connect_selected_notify(|row| {
        let _ = crate::settings::settings().set_string("new-terminal-directory", if row.selected() == 1 { "home" } else { "current" });
    });
    let confirm = adw::SwitchRow::builder().title("Vor dem Schließen nachfragen").subtitle("Wenn in einem Bereich noch ein Programm läuft").build();
    settings.bind("confirm-close", &confirm, "active").build();
    let scrollback = adw::SpinRow::builder()
        .title("Zeilen im Verlauf")
        .subtitle("−1 = unbegrenzt")
        .adjustment(&gtk::Adjustment::new(10000.0, -1.0, 1_000_000.0, 1000.0, 10000.0, 0.0))
        .build();
    settings.bind("scrollback-lines", &scrollback, "value").build();
    behaviour.add(&dir);
    behaviour.add(&confirm);
    behaviour.add(&scrollback);
    page.add(&behaviour);
    page
}

fn profiles_page(dialog: &adw::PreferencesDialog) -> adw::PreferencesPage {
    let page = adw::PreferencesPage::builder().title("Profile").icon_name("applications-utilities-symbolic").build();
    let add = gtk::Button::builder().icon_name("list-add-symbolic").tooltip_text("Neues Profil").css_classes(["flat"]).valign(gtk::Align::Center).build();
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
                let subtitle = if p.id == default { format!("{} · Standardprofil", pal.name) } else { pal.name.clone() };
                let row = adw::ActionRow::builder().title(glib::markup_escape_text(&p.name).as_str()).subtitle(subtitle.as_str()).activatable(true).build();
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
    let default = adw::SwitchRow::builder().title("Standardprofil").subtitle("Für neue Reiter").active(is_default).sensitive(!is_default).build();
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
    let import = gtk::Button::builder().label("Importieren …").css_classes(["flat"]).valign(gtk::Align::Center).tooltip_text("Ptyxis-Palette (.palette) oder Tilix-Schema (.json)").build();
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
                    let note = if pal.has_variants() { String::new() } else { " (nur eine Fassung, gilt für hell und dunkel)".into() };
                    d.add_toast(adw::Toast::new(&format!("„{}“ importiert{note}", pal.name)));
                }
                Err(e) => d.add_toast(adw::Toast::new(&format!("Import fehlgeschlagen: {e}"))),
            }
        });
    });

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
    let command = adw::EntryRow::builder().title("Befehl statt Shell (z. B. ssh bender)").text(&profile.command).build();
    let directory = adw::EntryRow::builder().title("Startverzeichnis (leer = Einstellung)").text(&profile.directory).build();
    let env = adw::EntryRow::builder().title("Umgebungsvariablen (NAME=wert, mit Leerzeichen getrennt)").text(profile.environment.join(" ")).build();
    for (row, which) in [(&command, 0u8), (&directory, 1), (&env, 2)] {
        let d = dialog.downgrade();
        let i = id.clone();
        row.connect_changed(move |e| {
            let text = e.text().to_string();
            let Some(d) = d.upgrade() else { return };
            update(&d, &i, |p| match which {
                0 => p.command = text,
                1 => p.directory = text,
                _ => {
                    p.environment = glib::shell_parse_argv(text.trim())
                        .map(|v| v.into_iter().map(|s| s.to_string_lossy().into_owned()).collect())
                        .unwrap_or_default()
                }
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
    let delete = gtk::Button::builder().label("Profil löschen").css_classes(["destructive-action", "pill"]).halign(gtk::Align::Center).build();
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
    let preview = gtk::DrawingArea::builder().content_width(132).content_height(64).css_classes(["parkett-tile-preview"]).build();
    preview.set_overflow(gtk::Overflow::Hidden);
    let (light, dark) = (pal.light.clone(), pal.dark.clone());
    preview.set_draw_func(move |_, cr, w, h| {
        let half = w as f64 / 2.0;
        draw_face(cr, &light, 0.0, half, h as f64);
        draw_face(cr, &dark, half, half, h as f64);
    });
    let label = gtk::Label::builder().label(pal.name.as_str()).ellipsize(gtk::pango::EllipsizeMode::End).css_classes(["caption"]).build();
    let tile = gtk::Box::builder().orientation(gtk::Orientation::Vertical).spacing(6).css_classes(["parkett-tile"]).build();
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
    let set = |c: &parkett_core::palette::Rgb| cr.set_source_rgb(c.r as f64 / 255.0, c.g as f64 / 255.0, c.b as f64 / 255.0);
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
