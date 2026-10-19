// SPDX-License-Identifier: GPL-3.0-or-later
//! Verwaltung von Profilen und Farbschemata für die ganze Anwendung.
//!
//! Ein GObject mit dem Signal „changed“, damit Terminals und Dialoge sich anmelden und
//! Änderungen sofort übernehmen (Live-Vorschau beim Wählen eines Farbschemas).

use std::path::{Path, PathBuf};

use gtk::glib;
use gtk::glib::subclass::prelude::*;
use gtk::glib::subclass::Signal;
use gtk::prelude::*;
use parkett_core::palette::{self, Palette, DEFAULT_PALETTE};
use parkett_core::profile::{Profile, ProfileStore};
use parkett_core::template::{Template, TemplateStore};

mod imp {
    use std::cell::{OnceCell, RefCell};
    use std::sync::OnceLock;

    use super::*;

    #[derive(Default)]
    pub struct ProfileManager {
        pub store: OnceCell<ProfileStore>,
        pub profiles: RefCell<Vec<Profile>>,
        pub palettes: RefCell<Vec<Palette>>,
        pub template_store: OnceCell<TemplateStore>,
        pub templates: RefCell<Vec<Template>>,
        pub errors: RefCell<Vec<String>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ProfileManager {
        const NAME: &'static str = "ParkettProfileManager";
        type Type = super::ProfileManager;
    }

    impl ObjectImpl for ProfileManager {
        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();
            SIGNALS
                .get_or_init(|| vec![Signal::builder("changed").build(), Signal::builder("templates-changed").build()])
        }
    }
}

glib::wrapper! {
    pub struct ProfileManager(ObjectSubclass<imp::ProfileManager>);
}

thread_local! {
    static MANAGER: ProfileManager = ProfileManager::load();
}

fn user_palette_dir() -> PathBuf {
    glib::user_data_dir().join("parkett").join("palettes")
}

impl ProfileManager {
    pub fn get() -> ProfileManager {
        MANAGER.with(|m| m.clone())
    }

    fn load() -> ProfileManager {
        let m: ProfileManager = glib::Object::new();
        let store = ProfileStore::new(glib::user_config_dir().join("parkett").join("profiles"));
        m.imp().store.set(store).ok();
        m.imp().template_store.set(TemplateStore::new(glib::user_config_dir().join("parkett").join("templates"))).ok();
        m.reload();
        m
    }

    /// Liest Profile und Paletten neu von der Platte.
    pub fn reload(&self) {
        let imp = self.imp();
        let mut errors = Vec::new();
        match imp.store.get().unwrap().load_all() {
            Ok((profiles, errs)) => {
                errors.extend(errs);
                *imp.profiles.borrow_mut() = profiles;
            }
            Err(e) => {
                errors.push(format!("Profile: {e}"));
                *imp.profiles.borrow_mut() = vec![Profile { id: "standard".into(), ..Profile::default() }];
            }
        }
        let mut palettes = palette::builtin();
        if let Ok(entries) = std::fs::read_dir(user_palette_dir()) {
            let mut user: Vec<Palette> = entries
                .flatten()
                .map(|e| e.path())
                .filter(|p| p.extension().is_some_and(|e| e == "palette"))
                .filter_map(|p| {
                    let id = p.file_stem()?.to_str()?.to_string();
                    match std::fs::read_to_string(&p)
                        .map_err(|e| e.to_string())
                        .and_then(|t| Palette::parse_ptyxis(&id, &t).map_err(|e| e.to_string()))
                    {
                        Ok(pal) => Some(pal),
                        Err(e) => {
                            errors.push(format!("{}: {e}", p.display()));
                            None
                        }
                    }
                })
                .filter(|p| !palettes.iter().any(|b| b.id == p.id))
                .collect();
            user.sort_by_key(|p| p.name.to_lowercase());
            palettes.extend(user);
        }
        *imp.palettes.borrow_mut() = palettes;
        match imp.template_store.get().unwrap().load_all() {
            Ok((t, errs)) => {
                errors.extend(errs);
                *imp.templates.borrow_mut() = t;
            }
            Err(e) => errors.push(format!("Vorlagen: {e}")),
        }
        for e in &errors {
            glib::g_warning!("parkett", "{e}");
        }
        *imp.errors.borrow_mut() = errors;
    }

    /// Dateien, die beim letzten Laden nicht lesbar waren (Pfad: Fehler).
    pub fn errors(&self) -> Vec<String> {
        self.imp().errors.borrow().clone()
    }

    /// Nur Vorlagen geändert – Terminals müssen dafür nicht neu einfärben.
    pub fn connect_templates_changed(&self, f: impl Fn() + 'static) -> glib::SignalHandlerId {
        self.connect_local("templates-changed", false, move |_| {
            f();
            None
        })
    }

    pub fn connect_changed(&self, f: impl Fn() + 'static) -> glib::SignalHandlerId {
        self.connect_local("changed", false, move |_| {
            f();
            None
        })
    }

    /// Neu einlesen, falls es `id` nicht gibt (Profil von Hand angelegt, während Parkett lief).
    /// `true`, wenn es danach existiert.
    pub fn ensure_profile(&self, id: &str) -> bool {
        if self.imp().profiles.borrow().iter().any(|p| p.id == id) {
            return true;
        }
        self.reload();
        self.emit_changed();
        self.imp().profiles.borrow().iter().any(|p| p.id == id)
    }

    fn emit_changed(&self) {
        self.emit_by_name::<()>("changed", &[]);
    }

    pub fn profiles(&self) -> Vec<Profile> {
        self.imp().profiles.borrow().clone()
    }

    pub fn default_id(&self) -> String {
        let id = crate::settings::settings().string("default-profile").to_string();
        let profiles = self.imp().profiles.borrow();
        if profiles.iter().any(|p| p.id == id) {
            id
        } else {
            profiles[0].id.clone()
        }
    }

    pub fn set_default(&self, id: &str) {
        let _ = crate::settings::settings().set_string("default-profile", id);
        self.emit_changed();
    }

    /// Profil `id`, sonst das Standardprofil (z. B. nach Löschen).
    pub fn profile(&self, id: &str) -> Profile {
        let profiles = self.imp().profiles.borrow();
        profiles.iter().find(|p| p.id == id).cloned().unwrap_or_else(|| {
            let def = crate::settings::settings().string("default-profile").to_string();
            profiles.iter().find(|p| p.id == def).unwrap_or(&profiles[0]).clone()
        })
    }

    /// Speichert und benachrichtigt alle Terminals (Live-Vorschau).
    pub fn save(&self, profile: &Profile) -> Result<(), String> {
        self.imp().store.get().unwrap().save(profile).map_err(|e| e.to_string())?;
        {
            let mut list = self.imp().profiles.borrow_mut();
            match list.iter_mut().find(|p| p.id == profile.id) {
                Some(p) => *p = profile.clone(),
                None => list.push(profile.clone()),
            }
            list.sort_by_key(|p| p.name.to_lowercase());
        }
        self.emit_changed();
        Ok(())
    }

    /// Neues Profil mit den Einstellungen des Standardprofils.
    pub fn create(&self, name: &str) -> Result<Profile, String> {
        let base = self.profile(&self.default_id());
        let id = self.imp().store.get().unwrap().new_id(name);
        let p = Profile { id, name: name.to_string(), ..base };
        self.save(&p)?;
        Ok(p)
    }

    pub fn delete(&self, id: &str) -> Result<(), String> {
        if self.imp().profiles.borrow().len() <= 1 {
            return Err("Das letzte Profil kann nicht gelöscht werden".into());
        }
        self.imp().store.get().unwrap().delete(id).map_err(|e| e.to_string())?;
        self.imp().profiles.borrow_mut().retain(|p| p.id != id);
        if crate::settings::settings().string("default-profile") == id {
            let first = self.imp().profiles.borrow()[0].id.clone();
            let _ = crate::settings::settings().set_string("default-profile", &first);
        }
        self.emit_changed();
        Ok(())
    }

    pub fn templates(&self) -> Vec<Template> {
        self.imp().templates.borrow().clone()
    }

    pub fn template(&self, id: &str) -> Option<Template> {
        self.imp().templates.borrow().iter().find(|t| t.id == id).cloned()
    }

    /// Speichert eine Vorlage; ohne Id bekommt sie eine aus dem Namen.
    pub fn save_template(&self, template: &Template) -> Result<Template, String> {
        let mut t = template.clone();
        if t.id.is_empty() {
            t.id = self.imp().template_store.get().unwrap().new_id(&t.name);
        }
        self.imp().template_store.get().unwrap().save(&t).map_err(|e| e.to_string())?;
        {
            let mut list = self.imp().templates.borrow_mut();
            match list.iter_mut().find(|x| x.id == t.id) {
                Some(x) => *x = t.clone(),
                None => list.push(t.clone()),
            }
            list.sort_by_key(|x| x.name.to_lowercase());
        }
        self.emit_by_name::<()>("templates-changed", &[]);
        Ok(t)
    }

    pub fn delete_template(&self, id: &str) -> Result<(), String> {
        self.imp().template_store.get().unwrap().delete(id).map_err(|e| e.to_string())?;
        self.imp().templates.borrow_mut().retain(|t| t.id != id);
        self.emit_by_name::<()>("templates-changed", &[]);
        Ok(())
    }

    pub fn palettes(&self) -> Vec<Palette> {
        self.imp().palettes.borrow().clone()
    }

    pub fn palette(&self, id: &str) -> Palette {
        let list = self.imp().palettes.borrow();
        list.iter()
            .find(|p| p.id == id)
            .or_else(|| list.iter().find(|p| p.id == DEFAULT_PALETTE))
            .unwrap_or(&list[0])
            .clone()
    }

    /// Importiert eine Ptyxis-Palette oder ein Tilix-Schema und legt es im Ptyxis-Format
    /// unter `$XDG_DATA_HOME/parkett/palettes/` ab.
    pub fn import(&self, path: &Path) -> Result<Palette, String> {
        let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("schema");
        let mut pal = Palette::parse_any(stem, &text).map_err(|e| e.to_string())?;
        let dir = user_palette_dir();
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let base = palette::slug(&pal.name);
        let mut id = base.clone();
        let mut n = 2;
        while self.imp().palettes.borrow().iter().any(|p| p.id == id) || dir.join(format!("{id}.palette")).exists() {
            id = format!("{base}-{n}");
            n += 1;
        }
        pal.id = id.clone();
        std::fs::write(dir.join(format!("{id}.palette")), pal.to_ptyxis()).map_err(|e| e.to_string())?;
        self.imp().palettes.borrow_mut().push(pal.clone());
        self.emit_changed();
        Ok(pal)
    }
}
