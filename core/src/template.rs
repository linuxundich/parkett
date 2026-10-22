// SPDX-License-Identifier: GPL-3.0-or-later
//! Vorlagen: ein ganzes Layout mit Profil, Startbefehl und Unterverzeichnis je Bereich.
//! Gespeichert als TOML (`$XDG_CONFIG_HOME/parkett/templates/<id>.toml`).
//!
//! Startbefehle werden in die Shell getippt (Vorschlag 5, CONCEPTS.md), nicht statt der
//! Shell gestartet: Nach Strg+C bleibt der Bereich als Shell nutzbar.

use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::layout::{Layout, Node, Orientation, PaneId};

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PaneSpec {
    /// Anzeigename (z. B. „Dev-Server“), nur für Vorlage und Editor
    pub name: String,
    /// Profil-Id; leer = Standardprofil
    pub profile: String,
    /// Wird nach dem Start in die Shell getippt; leer = nur Shell
    pub command: String,
    /// Relativ zum Verzeichnis der Vorlage; leer = dieses Verzeichnis
    pub subdir: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SplitDirection {
    /// nebeneinander
    Horizontal,
    /// übereinander
    Vertical,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum TemplateNode {
    Pane(PaneSpec),
    Split { direction: SplitDirection, ratio: f32, first: Box<TemplateNode>, second: Box<TemplateNode> },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Template {
    #[serde(skip)]
    pub id: String,
    pub name: String,
    /// Festes Verzeichnis; leer = beim Öffnen fragen
    #[serde(default)]
    pub directory: String,
    pub layout: TemplateNode,
}

impl TemplateNode {
    pub fn panes(&self) -> Vec<&PaneSpec> {
        match self {
            TemplateNode::Pane(p) => vec![p],
            TemplateNode::Split { first, second, .. } => {
                let mut v = first.panes();
                v.extend(second.panes());
                v
            }
        }
    }

    pub fn panes_mut(&mut self) -> Vec<&mut PaneSpec> {
        match self {
            TemplateNode::Pane(p) => vec![p],
            TemplateNode::Split { first, second, .. } => {
                let mut v = first.panes_mut();
                v.extend(second.panes_mut());
                v
            }
        }
    }
}

impl Template {
    pub fn to_toml(&self) -> String {
        toml::to_string_pretty(self).expect("Vorlage lässt sich immer serialisieren")
    }

    pub fn from_toml(id: &str, text: &str) -> Result<Template, toml::de::Error> {
        let mut t: Template = toml::from_str(text)?;
        t.id = id.to_string();
        Ok(t)
    }

    /// Layout für einen neuen Reiter: Bereiche bekommen Ids 1, 2, … in Lesereihenfolge.
    /// Rückgabe: Layout und je Id die Angaben zum Bereich.
    pub fn instantiate(&self) -> (Layout, Vec<(PaneId, PaneSpec)>) {
        let mut specs = Vec::new();
        fn build(n: &TemplateNode, specs: &mut Vec<(PaneId, PaneSpec)>) -> Node {
            match n {
                TemplateNode::Pane(p) => {
                    let id = specs.len() as PaneId + 1;
                    specs.push((id, p.clone()));
                    Node::Pane(id)
                }
                TemplateNode::Split { direction, ratio, first, second } => Node::Split {
                    orientation: match direction {
                        SplitDirection::Horizontal => Orientation::Horizontal,
                        SplitDirection::Vertical => Orientation::Vertical,
                    },
                    // „nan“/„inf“ aus einer Datei von Hand: clamp lässt NaN durch, und die Sitzung
                    // schriebe es als `null`, das sich nicht mehr laden lässt (R-08)
                    ratio: if ratio.is_finite() {
                        ratio.clamp(crate::layout::MIN_RATIO, 1.0 - crate::layout::MIN_RATIO)
                    } else {
                        0.5
                    },
                    first: Box::new(build(first, specs)),
                    second: Box::new(build(second, specs)),
                },
            }
        }
        let root = build(&self.layout, &mut specs);
        (Layout::from_node(root, 1).expect("fortlaufende Ids sind eindeutig"), specs)
    }

    /// Vorlage aus einem laufenden Reiter; `spec(id)` liefert die Angaben je Bereich.
    pub fn from_layout(name: &str, layout: &Layout, spec: &dyn Fn(PaneId) -> PaneSpec) -> Template {
        fn walk(n: &Node, spec: &dyn Fn(PaneId) -> PaneSpec) -> TemplateNode {
            match n {
                Node::Pane(id) => TemplateNode::Pane(spec(*id)),
                Node::Split { orientation, ratio, first, second } => TemplateNode::Split {
                    direction: match orientation {
                        Orientation::Horizontal => SplitDirection::Horizontal,
                        Orientation::Vertical => SplitDirection::Vertical,
                    },
                    ratio: (*ratio * 100.0).round() / 100.0,
                    first: Box::new(walk(first, spec)),
                    second: Box::new(walk(second, spec)),
                },
            }
        }
        Template {
            id: String::new(),
            name: name.to_string(),
            directory: String::new(),
            layout: walk(layout.root(), spec),
        }
    }

    /// Alle nicht leeren Startbefehle (für die Anzeige vor dem Ausführen).
    pub fn commands(&self) -> Vec<String> {
        self.layout.panes().iter().map(|p| p.command.trim().to_string()).filter(|c| !c.is_empty()).collect()
    }
}

/// Unterverzeichnis relativ zu `base`, wenn `path` darin liegt (sonst leer).
pub fn relative_subdir(base: &Path, path: &Path) -> String {
    path.strip_prefix(base).ok().map(|p| p.to_string_lossy().into_owned()).unwrap_or_default()
}

/// Vorlagen in einem Verzeichnis (eine Datei je Vorlage).
pub struct TemplateStore {
    dir: PathBuf,
}

impl TemplateStore {
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        TemplateStore { dir: dir.into() }
    }

    /// Alle Vorlagen nach Name; unlesbare Dateien werden gemeldet, nicht gelöscht.
    pub fn load_all(&self) -> io::Result<(Vec<Template>, Vec<String>)> {
        std::fs::create_dir_all(&self.dir)?;
        let mut out = Vec::new();
        let mut errors = Vec::new();
        for entry in std::fs::read_dir(&self.dir)? {
            let path = entry?.path();
            if path.extension().is_none_or(|e| e != "toml") {
                continue;
            }
            let Some(id) = path.file_stem().and_then(|s| s.to_str()).map(String::from) else { continue };
            match std::fs::read_to_string(&path)
                .map_err(|e| e.to_string())
                .and_then(|t| Template::from_toml(&id, &t).map_err(|e| e.to_string()))
            {
                Ok(t) => out.push(t),
                Err(e) => errors.push(format!("{}:\n{e}", path.display())),
            }
        }
        out.sort_by_key(|t| t.name.to_lowercase());
        Ok((out, errors))
    }

    pub fn save(&self, t: &Template) -> io::Result<()> {
        std::fs::create_dir_all(&self.dir)?;
        let tmp = self.dir.join(format!(".{}.toml.tmp", t.id));
        crate::paths::write_private(&tmp, &self.dir.join(format!("{}.toml", t.id)), &t.to_toml())
    }

    pub fn delete(&self, id: &str) -> io::Result<()> {
        std::fs::remove_file(self.dir.join(format!("{id}.toml")))
    }

    pub fn new_id(&self, name: &str) -> String {
        let base = crate::palette::slug(name);
        let mut id = base.clone();
        let mut n = 2;
        while self.dir.join(format!("{id}.toml")).exists() {
            id = format!("{base}-{n}");
            n += 1;
        }
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::Orientation::*;

    fn web() -> Template {
        let (l, two) = Layout::new().split(1, Horizontal).unwrap();
        let (l, _) = l.split(two, Vertical).unwrap();
        let l = l.set_ratio(&[], 0.56).unwrap();
        Template::from_layout("Web-Projekt", &l, &|id| PaneSpec {
            name: ["Shell", "Dev-Server", "Server"][id as usize - 1].into(),
            command: ["", "npm run dev", "ssh bender"][id as usize - 1].into(),
            profile: if id == 3 { "server".into() } else { String::new() },
            subdir: String::new(),
        })
    }

    #[test]
    fn from_layout_and_instantiate_roundtrip() {
        let t = web();
        let (layout, specs) = t.instantiate();
        assert_eq!(layout.panes(), vec![1, 2, 3]);
        assert_eq!(layout.ratio_at(&[]), Some(0.56));
        assert_eq!(specs[1].1.command, "npm run dev");
        assert_eq!(specs[2].1.profile, "server");
        assert_eq!(t.commands(), vec!["npm run dev".to_string(), "ssh bender".to_string()]);
    }

    #[test]
    fn toml_roundtrip_is_readable() {
        let t = Template { id: "web".into(), ..web() };
        let text = t.to_toml();
        assert!(text.contains("type = \"split\""), "{text}");
        assert!(text.contains("command = \"npm run dev\""), "{text}");
        assert_eq!(Template::from_toml("web", &text).unwrap(), t);
    }

    #[test]
    fn hand_written_minimal_template() {
        let text = "name = \"Zwei\"\n[layout]\ntype = \"split\"\ndirection = \"vertical\"\nratio = 2.0\n[layout.first]\ntype = \"pane\"\n[layout.second]\ntype = \"pane\"\ncommand = \"htop\"\n";
        let t = Template::from_toml("zwei", text).unwrap();
        assert_eq!(t.directory, "");
        let (l, specs) = t.instantiate();
        assert_eq!(l.ratio_at(&[]), Some(1.0 - crate::layout::MIN_RATIO), "Anteil wird begrenzt");
        assert_eq!(specs[1].1.command, "htop");
    }

    #[test]
    fn non_finite_ratio_becomes_half() {
        for r in ["nan", "inf", "-inf"] {
            let text = format!("name = \"x\"\n[layout]\ntype = \"split\"\ndirection = \"vertical\"\nratio = {r}\n[layout.first]\ntype = \"pane\"\n[layout.second]\ntype = \"pane\"\n");
            let (l, _) = Template::from_toml("x", &text).unwrap().instantiate();
            assert_eq!(l.ratio_at(&[]), Some(0.5), "{r}");
        }
    }

    #[test]
    fn subdir() {
        assert_eq!(relative_subdir(Path::new("/p/web"), Path::new("/p/web/theme")), "theme");
        assert_eq!(relative_subdir(Path::new("/p/web"), Path::new("/p/web")), "");
        assert_eq!(relative_subdir(Path::new("/p/web"), Path::new("/etc")), "");
    }

    #[test]
    fn store() {
        let dir = std::env::temp_dir().join(format!("parkett-test-templates-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let store = TemplateStore::new(&dir);
        assert!(store.load_all().unwrap().0.is_empty());
        let id = store.new_id("Web-Projekt");
        store.save(&Template { id: id.clone(), ..web() }).unwrap();
        assert_eq!(store.new_id("Web-Projekt"), "web-projekt-2");
        let (all, errs) = store.load_all().unwrap();
        assert!(errs.is_empty());
        assert_eq!(all[0].name, "Web-Projekt");
        store.delete(&id).unwrap();
        let _ = std::fs::remove_dir_all(&dir);
    }
}
