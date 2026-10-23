// SPDX-License-Identifier: GPL-3.0-or-later
//! Befehlspalette (Konzept A+, aus C): eine Liste für Reiter, Bereiche, Aktionen, Vorlagen
//! und Profile. Präfixe: „>“ nur Aktionen, „@“ nur Reiter/Bereiche, „~“ nur Lesezeichen,
//! „#“ nur Vorlagen.
//! Jede Aktion zeigt ihr Kürzel – so lernt man es nebenbei.

use std::cell::RefCell;
use std::rc::Rc;

use adw::prelude::*;
use gtk::glib;
use parkett_core::fuzzy::{self, Scope};

use crate::profiles::ProfileManager;
use crate::shortcuts;
use crate::window::Window;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Group {
    Places,
    Bookmarks,
    Templates,
    Actions,
}

impl Group {
    fn title(self) -> &'static str {
        match self {
            Group::Places => "Reiter und Bereiche",
            Group::Bookmarks => "Lesezeichen",
            Group::Templates => "Vorlagen",
            Group::Actions => "Aktionen",
        }
    }
}

/// Was ein Eintrag ausführt
type Run = Rc<dyn Fn(&Window)>;

/// Eingabe, Strg+Eingabe (neuer Bereich) oder Alt+Eingabe (im aktiven Bereich)
#[derive(Clone, Copy)]
enum How {
    Run,
    Pane,
    Here,
}

struct Item {
    group: Group,
    title: String,
    subtitle: String,
    accel: String,
    icon: &'static str,
    run: Run,
    /// Strg+Eingabe: Ziel in einem neuen Bereich (Profile, Lesezeichen); sonst wie `run`
    alt: Option<Run>,
    /// Alt+Eingabe: im aktiven Bereich (Lesezeichen: „cd“); sonst wie `run`
    here: Option<Run>,
}

fn collect(win: &Window) -> Vec<Item> {
    let mut items = Vec::new();
    for (ti, tab) in win.tabs().iter().enumerate() {
        let tab_title = tab.title();
        let cwd = tab.cwd().unwrap_or_default();
        items.push(Item {
            alt: None,
            here: None,
            group: Group::Places,
            title: tab_title.clone(),
            subtitle: format!("Reiter {} · {cwd}", ti + 1),
            accel: if ti < 9 { shortcuts::label(&[format!("<Control><Alt>{}", ti + 1)]) } else { String::new() },
            icon: "tab-new-symbolic",
            run: Rc::new(move |w: &Window| w.select_tab(ti)),
        });
        let panes = tab.layout().panes();
        if panes.len() > 1 {
            for (pi, id) in panes.iter().enumerate() {
                let pane = tab.pane(*id);
                let Some(pane) = pane else { continue };
                let id = *id;
                items.push(Item {
                    alt: None,
                    here: None,
                    group: Group::Places,
                    title: format!("{tab_title} › {}", pane.terminal().title().unwrap_or_else(|| "Shell".into())),
                    subtitle: format!("Bereich {} · {}", pi + 1, pane.terminal().cwd().unwrap_or_default()),
                    accel: String::new(),
                    icon: "view-dual-symbolic",
                    run: Rc::new(move |w: &Window| {
                        w.select_tab(ti);
                        if let Some(t) = w.tabs().get(ti) {
                            t.focus_pane(id);
                        }
                    }),
                });
            }
        }
    }
    for b in crate::bookmarks::load() {
        let dir = crate::bookmarks::resolve(&b);
        let act = |name: &'static str, dir: String| -> Run {
            Rc::new(move |w: &Window| {
                let _ = WidgetExt::activate_action(w, name, Some(&dir.to_variant()));
            })
        };
        items.push(Item {
            group: Group::Bookmarks,
            title: b.name.clone(),
            subtitle: b.path.clone(),
            accel: String::new(),
            icon: "folder-symbolic",
            run: act("win.open-bookmark", dir.clone()),
            alt: Some(act("win.bookmark-pane", dir.clone())),
            here: Some(act("win.bookmark-cd", dir)),
        });
    }
    for t in ProfileManager::get().templates() {
        let n = t.layout.panes().len();
        let id = t.id.clone();
        items.push(Item {
            alt: None,
            here: None,
            group: Group::Templates,
            title: format!("{} öffnen", t.name),
            subtitle: format!("{n} {} · {}", if n == 1 { "Bereich" } else { "Bereiche" }, t.commands().join(", ")),
            accel: String::new(),
            icon: "view-grid-symbolic",
            run: Rc::new(move |w: &Window| w.open_template(&id)),
        });
    }
    for p in ProfileManager::get().profiles() {
        let id = p.id.clone();
        items.push(Item {
            group: Group::Actions,
            title: format!("Neuer Reiter mit Profil „{}“", p.name),
            subtitle: ProfileManager::get().palette(&p.palette).name,
            accel: String::new(),
            icon: "tab-new-symbolic",
            run: Rc::new({
                let id = id.clone();
                move |w: &Window| {
                    let _ = WidgetExt::activate_action(w, "win.new-tab-profile", Some(&id.to_variant()));
                }
            }),
            alt: Some(Rc::new(move |w: &Window| {
                let _ = WidgetExt::activate_action(w, "win.split-profile", Some(&id.to_variant()));
            })),
            here: None,
        });
    }
    for e in shortcuts::ENTRIES.iter().filter(|e| e.action != "win.palette") {
        let action = e.action;
        items.push(Item {
            alt: None,
            here: None,
            group: Group::Actions,
            title: e.label.to_string(),
            subtitle: e.section.to_string(),
            accel: {
                let a = shortcuts::effective(action);
                if a.is_empty() {
                    String::new()
                } else {
                    shortcuts::label(&a)
                }
            },
            icon: "system-run-symbolic",
            run: Rc::new(move |w: &Window| w.run_action(action)),
        });
    }
    items.push(Item {
        alt: None,
        here: None,
        group: Group::Actions,
        title: "Aktuellen Reiter als Vorlage speichern …".into(),
        subtitle: "Vorlagen".into(),
        accel: String::new(),
        icon: "document-save-symbolic",
        run: Rc::new(|w: &Window| {
            let _ = WidgetExt::activate_action(w, "win.save-template", None);
        }),
    });
    items
}

fn row(item: &Item) -> gtk::ListBoxRow {
    let b = gtk::Box::builder().spacing(12).margin_start(12).margin_end(12).margin_top(6).margin_bottom(6).build();
    b.append(&gtk::Image::from_icon_name(item.icon));
    let text = gtk::Box::builder().orientation(gtk::Orientation::Vertical).hexpand(true).build();
    text.append(
        &gtk::Label::builder().label(item.title.as_str()).xalign(0.0).ellipsize(gtk::pango::EllipsizeMode::End).build(),
    );
    if !item.subtitle.is_empty() {
        text.append(
            &gtk::Label::builder()
                .label(item.subtitle.as_str())
                .xalign(0.0)
                .ellipsize(gtk::pango::EllipsizeMode::End)
                .css_classes(["dim-label", "caption"])
                .build(),
        );
    }
    b.append(&text);
    if !item.accel.is_empty() {
        b.append(&gtk::Label::builder().label(item.accel.as_str()).css_classes(["dim-label", "caption"]).build());
    }
    let r = gtk::ListBoxRow::builder().child(&b).build();
    r.set_widget_name(item.group.title());
    r
}

pub fn show(win: &Window) {
    let items = Rc::new(collect(win));
    let entry = gtk::SearchEntry::builder()
        .placeholder_text(
            "Befehl, Reiter, Lesezeichen oder Vorlage … (> Aktionen, @ Reiter, ~ Lesezeichen, # Vorlagen)",
        )
        .hexpand(true)
        .build();
    let list =
        gtk::ListBox::builder().selection_mode(gtk::SelectionMode::Browse).css_classes(["navigation-sidebar"]).build();
    list.set_header_func(|row, before| {
        let group = row.widget_name();
        if before.is_none_or(|b| b.widget_name() != group) {
            let h = gtk::Label::builder()
                .label(group.as_str())
                .xalign(0.0)
                .css_classes(["heading", "dim-label"])
                .margin_start(12)
                .margin_top(10)
                .margin_bottom(4)
                .build();
            row.set_header(Some(&h));
        } else {
            row.set_header(gtk::Widget::NONE);
        }
    });
    let scroll =
        gtk::ScrolledWindow::builder().child(&list).vexpand(true).hscrollbar_policy(gtk::PolicyType::Never).build();
    let content = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(6)
        .margin_top(12)
        .margin_start(12)
        .margin_end(12)
        .margin_bottom(6)
        .build();
    content.append(&entry);
    content.append(&scroll);
    let footer = gtk::Label::builder()
        .label("↑↓ wählen · Eingabe ausführen · Strg+Eingabe in neuem Bereich · Alt+Eingabe Lesezeichen hier · Esc")
        .css_classes(["dim-label", "caption"])
        .margin_bottom(6)
        .build();
    content.append(&footer);
    let dialog = adw::Dialog::builder().title("Befehle").content_width(640).content_height(480).child(&content).build();

    // sichtbare Treffer: Index in `items`
    let shown: Rc<RefCell<Vec<usize>>> = Rc::default();
    let refill = {
        let items = items.clone();
        let list = list.clone();
        let shown = shown.clone();
        move |query: &str| {
            list.remove_all();
            let (scope, q) = fuzzy::split_scope(query);
            let mut hits: Vec<(i32, usize)> = items
                .iter()
                .enumerate()
                .filter(|(_, it)| match scope {
                    Scope::All => true,
                    Scope::Actions => it.group == Group::Actions,
                    Scope::Places => it.group == Group::Places,
                    Scope::Templates => it.group == Group::Templates,
                    Scope::Bookmarks => it.group == Group::Bookmarks,
                })
                .filter_map(|(i, it)| fuzzy::score(q, &format!("{} {}", it.title, it.subtitle)).map(|s| (s, i)))
                .collect();
            let order = |g: Group| match g {
                Group::Places => 0,
                Group::Bookmarks => 1,
                Group::Templates => 2,
                Group::Actions => 3,
            };
            if q.trim().is_empty() {
                hits.sort_by_key(|(_, i)| (order(items[*i].group), *i));
            } else {
                // beste Treffer zuerst, aber Gruppen zusammenhalten (Reihenfolge der besten Gruppe)
                let mut best = [i32::MIN; 4];
                for (s, i) in &hits {
                    let g = order(items[*i].group);
                    best[g] = best[g].max(*s);
                }
                hits.sort_by_key(|(s, i)| {
                    (std::cmp::Reverse(best[order(items[*i].group)]), order(items[*i].group), std::cmp::Reverse(*s), *i)
                });
            }
            hits.truncate(60);
            *shown.borrow_mut() = hits.iter().map(|(_, i)| *i).collect();
            for (_, i) in &hits {
                list.append(&row(&items[*i]));
            }
            if let Some(first) = list.row_at_index(0) {
                list.select_row(Some(&first));
            }
        }
    };
    refill("");

    let run = {
        let items = items.clone();
        let shown = shown.clone();
        let win = win.downgrade();
        let dialog = dialog.downgrade();
        move |index: i32, how: How| {
            let Some(i) = shown.borrow().get(index.max(0) as usize).copied() else { return };
            let it = &items[i];
            let f = match how {
                How::Run => None,
                How::Pane => it.alt.clone(),
                How::Here => it.here.clone(),
            }
            .unwrap_or_else(|| it.run.clone());
            if let Some(d) = dialog.upgrade() {
                d.close();
            }
            // nach dem Schließen ausführen, damit der Fokus beim Terminal landet
            let win = win.clone();
            glib::idle_add_local_once(move || {
                if let Some(w) = win.upgrade() {
                    f(&w);
                }
            });
        }
    };
    let r2 = refill.clone();
    entry.connect_search_changed(move |e| r2(&e.text()));
    let run2 = run.clone();
    let l2 = list.clone();
    entry.connect_activate(move |_| run2(l2.selected_row().map(|r| r.index()).unwrap_or(0), How::Run));
    let run3 = run.clone();
    list.connect_row_activated(move |_, r| run(r.index(), How::Run));
    let d = dialog.downgrade();
    entry.connect_stop_search(move |_| {
        if let Some(d) = d.upgrade() {
            d.close();
        }
    });
    // Pfeiltasten im Suchfeld bewegen die Auswahl; der Fokus bleibt im Suchfeld
    let keys = gtk::EventControllerKey::new();
    let l3 = list.clone();
    let sc = scroll.clone();
    let l4 = list.clone();
    keys.connect_key_pressed(move |_, key, _, state| {
        let enter = matches!(key, gtk::gdk::Key::Return | gtk::gdk::Key::KP_Enter);
        let how = if state.contains(gtk::gdk::ModifierType::CONTROL_MASK) {
            Some(How::Pane)
        } else if state.contains(gtk::gdk::ModifierType::ALT_MASK) {
            Some(How::Here)
        } else {
            None
        };
        if let (true, Some(how)) = (enter, how) {
            run3(l4.selected_row().map(|r| r.index()).unwrap_or(0), how);
            return glib::Propagation::Stop;
        }
        let delta = match key {
            gtk::gdk::Key::Down => 1,
            gtk::gdk::Key::Up => -1,
            gtk::gdk::Key::Page_Down => 8,
            gtk::gdk::Key::Page_Up => -8,
            _ => return glib::Propagation::Proceed,
        };
        let cur = l3.selected_row().map(|r| r.index()).unwrap_or(0);
        let mut n = 0;
        while l3.row_at_index(n).is_some() {
            n += 1;
        }
        if let Some(r) = (n > 0).then(|| l3.row_at_index((cur + delta).clamp(0, n - 1))).flatten() {
            l3.select_row(Some(&r));
            // ins Sichtfeld scrollen, ohne der Zeile den Fokus zu geben
            if let Some(p) = r.compute_point(&l3, &gtk::graphene::Point::new(0.0, 0.0)) {
                let adj = sc.vadjustment();
                let (y, h) = (p.y() as f64, r.height() as f64);
                if y < adj.value() {
                    adj.set_value(y);
                } else if y + h > adj.value() + adj.page_size() {
                    adj.set_value(y + h - adj.page_size());
                }
            }
        }
        glib::Propagation::Stop
    });
    entry.add_controller(keys);
    dialog.present(Some(win));
    entry.grab_focus();
}
