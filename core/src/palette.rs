// SPDX-License-Identifier: GPL-3.0-or-later
//! Farbschemata im Paletten-Format von Ptyxis, dazu Import von Tilix-Schemata (JSON).
//!
//! Eine Palette hat eine helle und eine dunkle Fassung (`Face`). Fehlende Farben für
//! root- und SSH-Bereiche werden wie bei Ptyxis aus der Palette abgeleitet
//! (`ptyxis-palette.c: ptyxis_palette_load_face`).

use std::collections::HashMap;
use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Rgb {
    /// `#rrggbb` oder `#rgb`, Groß-/Kleinschreibung egal.
    pub fn parse(s: &str) -> Option<Rgb> {
        let h = s.trim().strip_prefix('#')?;
        // Nach Bytes geschnitten: Nicht-ASCII würde mitten in einem Zeichen schneiden
        if !h.is_ascii() {
            return None;
        }
        let v = |i: usize, n: usize| u8::from_str_radix(&h[i..i + n], 16).ok();
        match h.len() {
            6 => Some(Rgb { r: v(0, 2)?, g: v(2, 2)?, b: v(4, 2)? }),
            3 => Some(Rgb { r: v(0, 1)? * 17, g: v(1, 1)? * 17, b: v(2, 1)? * 17 }),
            // GdkRGBA kann #rrrrggggbbbb (Tilix-Dateien aus gnome-terminal)
            12 => Some(Rgb { r: v(0, 2)?, g: v(4, 2)?, b: v(8, 2)? }),
            _ => None,
        }
    }

    pub fn hex(&self) -> String {
        format!("#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
    }

    /// Relative Leuchtdichte nach WCAG.
    pub fn luminance(&self) -> f64 {
        let lin = |c: u8| {
            let c = c as f64 / 255.0;
            if c <= 0.03928 {
                c / 12.92
            } else {
                ((c + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * lin(self.r) + 0.7152 * lin(self.g) + 0.0722 * lin(self.b)
    }

    pub fn is_dark(&self) -> bool {
        self.luminance() < 0.18
    }

    /// Kontrastverhältnis nach WCAG (1 bis 21).
    pub fn contrast(&self, other: &Rgb) -> f64 {
        let (a, b) = (self.luminance(), other.luminance());
        (a.max(b) + 0.05) / (a.min(b) + 0.05)
    }

    /// `alpha` Anteil von `fg` über `self`.
    pub fn mix(&self, fg: &Rgb, alpha: f64) -> Rgb {
        let m = |b: u8, f: u8| ((1.0 - alpha) * b as f64 + alpha * f as f64).round().clamp(0.0, 255.0) as u8;
        Rgb { r: m(self.r, fg.r), g: m(self.g, fg.g), b: m(self.b, fg.b) }
    }
}

/// Eine Fassung (hell oder dunkel) einer Palette.
#[derive(Clone, Debug, PartialEq)]
pub struct Face {
    pub foreground: Rgb,
    pub background: Rgb,
    pub cursor: Option<Rgb>,
    pub cursor_foreground: Option<Rgb>,
    pub colors: [Rgb; 16],
    /// Hintergrund für Bereiche mit root-Rechten (Kontextfarbe)
    pub superuser_background: Rgb,
    /// Hintergrund für Bereiche mit SSH-Verbindung (Kontextfarbe)
    pub remote_background: Rgb,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Palette {
    /// Dateiname ohne Endung, z. B. „gnome“
    pub id: String,
    pub name: String,
    pub light: Face,
    pub dark: Face,
}

#[derive(Debug, PartialEq)]
pub enum PaletteError {
    Missing(String),
    BadColor(String, String),
    Json(String),
}

impl fmt::Display for PaletteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PaletteError::Missing(what) => write!(f, "{what} fehlt"),
            PaletteError::BadColor(key, v) => write!(f, "{key}: „{v}“ ist keine Farbe"),
            PaletteError::Json(e) => write!(f, "kein gültiges Tilix-Schema: {e}"),
        }
    }
}

impl std::error::Error for PaletteError {}

type Section = HashMap<String, String>;

/// Minimaler Parser für das GKeyFile-Format der Paletten (Abschnitte, Schlüssel=Wert, #-Kommentare).
fn parse_keyfile(text: &str) -> HashMap<String, Section> {
    let mut out: HashMap<String, Section> = HashMap::new();
    let mut current = String::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
            continue;
        }
        if let Some(name) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
            current = name.to_string();
            out.entry(current.clone()).or_default();
        } else if let Some((k, v)) = line.split_once('=') {
            out.entry(current.clone()).or_default().insert(k.trim().to_string(), v.trim().to_string());
        }
    }
    out
}

fn color(sec: &Section, key: &str) -> Result<Option<Rgb>, PaletteError> {
    match sec.get(key) {
        None => Ok(None),
        Some(v) => Rgb::parse(v).map(Some).ok_or_else(|| PaletteError::BadColor(key.into(), v.clone())),
    }
}

fn required(sec: &Section, key: &str, scope: &str) -> Result<Rgb, PaletteError> {
    color(sec, key)?.ok_or_else(|| PaletteError::Missing(format!("{key} in [{scope}]")))
}

impl Face {
    /// Hintergrund markierter Zeichen: Akzentfarbe des Systems über dem Hintergrund, wie
    /// die Textauswahl in GTK. Ohne eigene Farbe zeichnet VTE die Auswahl invertiert – im
    /// dunklen Stil ein greller heller Block. Die Schrift behält ihre Farbe.
    pub fn selection(&self, accent: &Rgb) -> Rgb {
        self.background.mix(accent, if self.background.is_dark() { 0.3 } else { 0.25 })
    }

    /// Baut eine Fassung und leitet fehlende Kontextfarben ab (Ptyxis-Regeln).
    pub fn new(foreground: Rgb, background: Rgb, colors: [Rgb; 16], cursor: Option<Rgb>) -> Face {
        let dark = background.is_dark();
        Face {
            foreground,
            background,
            cursor,
            cursor_foreground: None,
            colors,
            superuser_background: background.mix(&colors[1], if dark { 0.33 } else { 0.5 }),
            remote_background: background.mix(&colors[12], if dark { 0.33 } else { 0.5 }),
        }
    }

    fn from_section(sec: &Section, scope: &str) -> Result<Face, PaletteError> {
        let mut colors = [Rgb { r: 0, g: 0, b: 0 }; 16];
        for (i, c) in colors.iter_mut().enumerate() {
            *c = required(sec, &format!("Color{i}"), scope)?;
        }
        let cursor = match color(sec, "CursorBackground")? {
            Some(c) => Some(c),
            None => color(sec, "Cursor")?,
        };
        let mut face =
            Face::new(required(sec, "Foreground", scope)?, required(sec, "Background", scope)?, colors, cursor);
        face.cursor_foreground = color(sec, "CursorForeground")?;
        if let Some(c) = color(sec, "SuperuserBackground")? {
            face.superuser_background = c;
        }
        if let Some(c) = color(sec, "RemoteBackground")? {
            face.remote_background = c;
        }
        Ok(face)
    }

    fn write(&self, out: &mut String) {
        out.push_str(&format!("Foreground={}\nBackground={}\n", self.foreground.hex(), self.background.hex()));
        if let Some(c) = self.cursor {
            out.push_str(&format!("Cursor={}\n", c.hex()));
        }
        if let Some(c) = self.cursor_foreground {
            out.push_str(&format!("CursorForeground={}\n", c.hex()));
        }
        for (i, c) in self.colors.iter().enumerate() {
            out.push_str(&format!("Color{i}={}\n", c.hex()));
        }
        out.push_str(&format!(
            "SuperuserBackground={}\nRemoteBackground={}\n",
            self.superuser_background.hex(),
            self.remote_background.hex()
        ));
    }
}

impl Palette {
    /// Liest eine Ptyxis-Palette. Ohne `[Light]`/`[Dark]` (ältere Dateien aus Gogh) stehen
    /// die Farben im Abschnitt `[Palette]` und gelten für beide Fassungen.
    pub fn parse_ptyxis(id: &str, text: &str) -> Result<Palette, PaletteError> {
        let kf = parse_keyfile(text);
        let info = kf.get("Palette").ok_or_else(|| PaletteError::Missing("[Palette]".into()))?;
        let name = info.get("Name").cloned().unwrap_or_else(|| id.to_string());
        let (light, dark) = match (kf.get("Light"), kf.get("Dark")) {
            (Some(l), Some(d)) => (Face::from_section(l, "Light")?, Face::from_section(d, "Dark")?),
            (Some(l), None) => {
                let f = Face::from_section(l, "Light")?;
                (f.clone(), f)
            }
            (None, Some(d)) => {
                let f = Face::from_section(d, "Dark")?;
                (f.clone(), f)
            }
            (None, None) => {
                let f = Face::from_section(info, "Palette")?;
                (f.clone(), f)
            }
        };
        Ok(Palette { id: id.to_string(), name, light, dark })
    }

    /// Liest ein Tilix-Farbschema (`data/schemes/*.json` bei Tilix). Tilix kennt nur eine
    /// Fassung; sie gilt für hell und dunkel.
    pub fn parse_tilix(id: &str, text: &str) -> Result<Palette, PaletteError> {
        let v: serde_json::Value = serde_json::from_str(text).map_err(|e| PaletteError::Json(e.to_string()))?;
        let s = |k: &str| v.get(k).and_then(|x| x.as_str());
        let c = |k: &str| -> Result<Option<Rgb>, PaletteError> {
            match s(k) {
                None => Ok(None),
                Some(x) => Rgb::parse(x).map(Some).ok_or_else(|| PaletteError::BadColor(k.into(), x.into())),
            }
        };
        let name = s("name").unwrap_or(id).to_string();
        let fg = c("foreground-color")?.ok_or_else(|| PaletteError::Missing("foreground-color".into()))?;
        let bg = c("background-color")?.ok_or_else(|| PaletteError::Missing("background-color".into()))?;
        let list =
            v.get("palette").and_then(|p| p.as_array()).ok_or_else(|| PaletteError::Missing("palette".into()))?;
        if list.len() < 16 {
            return Err(PaletteError::Missing(format!("16 Farben in palette (nur {})", list.len())));
        }
        let mut colors = [Rgb { r: 0, g: 0, b: 0 }; 16];
        for (i, slot) in colors.iter_mut().enumerate() {
            let x = list[i].as_str().unwrap_or("");
            *slot = Rgb::parse(x).ok_or_else(|| PaletteError::BadColor(format!("palette[{i}]"), x.into()))?;
        }
        let use_cursor = v.get("use-cursor-color").and_then(|b| b.as_bool()).unwrap_or(false);
        let cursor = if use_cursor { c("cursor-background-color")? } else { None };
        let mut face = Face::new(fg, bg, colors, cursor);
        if use_cursor {
            face.cursor_foreground = c("cursor-foreground-color")?;
        }
        Ok(Palette { id: id.to_string(), name, light: face.clone(), dark: face })
    }

    /// Erkennt das Format am Inhalt: JSON-Objekt = Tilix, sonst Ptyxis.
    pub fn parse_any(id: &str, text: &str) -> Result<Palette, PaletteError> {
        if text.trim_start().starts_with('{') {
            Palette::parse_tilix(id, text)
        } else {
            Palette::parse_ptyxis(id, text)
        }
    }

    pub fn face(&self, dark: bool) -> &Face {
        if dark {
            &self.dark
        } else {
            &self.light
        }
    }

    /// Hat die Palette wirklich zwei verschiedene Fassungen?
    pub fn has_variants(&self) -> bool {
        self.light != self.dark
    }

    /// Schreibt die Palette im Ptyxis-Format (für importierte Schemata).
    pub fn to_ptyxis(&self) -> String {
        let mut out = format!("[Palette]\nName={}\n\n[Light]\n", self.name);
        self.light.write(&mut out);
        out.push_str("\n[Dark]\n");
        self.dark.write(&mut out);
        out
    }
}

/// Dateiname-taugliche Id aus einem Anzeigenamen („Solarized Dark“ → „solarized-dark“).
pub fn slug(name: &str) -> String {
    let mut s = String::new();
    for ch in name.chars().flat_map(|c| c.to_lowercase()) {
        match ch {
            'a'..='z' | '0'..='9' => s.push(ch),
            'ä' => s.push_str("ae"),
            'ö' => s.push_str("oe"),
            'ü' => s.push_str("ue"),
            'ß' => s.push_str("ss"),
            _ if !s.ends_with('-') && !s.is_empty() => s.push('-'),
            _ => {}
        }
    }
    let s = s.trim_end_matches('-').to_string();
    if s.is_empty() {
        "schema".into()
    } else {
        s
    }
}

macro_rules! p {
    ($id:literal) => {
        ($id, include_str!(concat!("../../data/palettes/", $id, ".palette")))
    };
}

/// Die mitgelieferten Schemata (Dateien in data/palettes, ins Programm eingebettet), in der
/// Reihenfolge der Auswahl.
const BUILTIN: &[(&str, &str)] = &[
    p!("gnome"),
    p!("gnome-high-contrast"),
    p!("tango"),
    p!("solarized"),
    p!("selenized"),
    p!("nord"),
    p!("dracula"),
    p!("catppuccin"),
    p!("gruvbox"),
    p!("everforest"),
    p!("one"),
    p!("github"),
    p!("papercolor"),
    p!("vs-code"),
];

/// Die mitgelieferten Schemata, geparst.
pub fn builtin() -> Vec<Palette> {
    BUILTIN
        .iter()
        .map(|(id, text)| Palette::parse_ptyxis(id, text).unwrap_or_else(|e| panic!("eingebaute Palette {id}: {e}")))
        .collect()
}

pub const DEFAULT_PALETTE: &str = "gnome";

/// Schema für GNOMEs hohen Kontrast (*Barrierefreiheit › Hoher Kontrast*).
pub const HIGH_CONTRAST_PALETTE: &str = "gnome-high-contrast";

/// Palette, die ein Bereich tatsächlich zeigt: bei hohem Kontrast des Systems das
/// Hochkontrast-Schema statt des Schemas aus dem Profil.
pub fn effective_id(chosen: &str, high_contrast: bool) -> &str {
    if high_contrast {
        HIGH_CONTRAST_PALETTE
    } else {
        chosen
    }
}

/// Mitgelieferte Schemata vorn in fester Reihenfolge, danach die eigenen alphabetisch –
/// auch gleich nach einem Import, nicht erst beim nächsten Start.
pub fn sort(list: &mut [Palette]) {
    let rank = |p: &Palette| match BUILTIN.iter().position(|(id, _)| *id == p.id) {
        Some(i) => (0, i, String::new()),
        None => (1, 0, p.name.to_lowercase()),
    };
    list.sort_by_cached_key(rank);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colors_parse_and_print() {
        assert_eq!(Rgb::parse("#1d1d20"), Some(Rgb { r: 0x1d, g: 0x1d, b: 0x20 }));
        assert_eq!(Rgb::parse("#FFF"), Some(Rgb { r: 255, g: 255, b: 255 }));
        assert_eq!(Rgb::parse("#ffff00000000"), Some(Rgb { r: 255, g: 0, b: 0 }));
        assert_eq!(Rgb::parse("rot"), None);
        assert_eq!(Rgb::parse("#12345"), None);
        assert_eq!(Rgb { r: 1, g: 2, b: 255 }.hex(), "#0102ff");
    }

    #[test]
    fn dark_detection_and_mix() {
        assert!(Rgb::parse("#1c1c1f").unwrap().is_dark());
        assert!(!Rgb::parse("#ffffff").unwrap().is_dark());
        let black = Rgb { r: 0, g: 0, b: 0 };
        let white = Rgb { r: 255, g: 255, b: 255 };
        assert_eq!(black.mix(&white, 0.5), Rgb { r: 128, g: 128, b: 128 });
    }

    /// Text muss in jeder eingebauten Fassung lesbar sein (QA-10: „One“ dunkel hatte 2,7:1).
    /// Grenze 4,0 statt WCAG 4,5, weil Solarized hell im Original bei 4,1:1 liegt.
    #[test]
    fn builtin_foreground_is_readable() {
        let ratio = |a: &Rgb, b: &Rgb| {
            let (x, y) = (a.luminance(), b.luminance());
            (x.max(y) + 0.05) / (x.min(y) + 0.05)
        };
        for p in builtin() {
            for (name, face) in [("hell", &p.light), ("dunkel", &p.dark)] {
                let r = ratio(&face.foreground, &face.background);
                assert!(r >= 4.0, "{} {name}: Vordergrund nur {r:.1}:1", p.name);
            }
        }
    }

    #[test]
    fn all_builtin_palettes_load_with_two_variants() {
        let all = builtin();
        assert_eq!(all.len(), 14);
        for p in &all {
            assert!(p.has_variants(), "{} hat keine getrennten Fassungen", p.id);
            assert!(!p.light.background.is_dark(), "{}: helle Fassung hat dunklen Grund", p.id);
            assert!(p.dark.background.is_dark(), "{}: dunkle Fassung hat hellen Grund", p.id);
        }
        assert!(all.iter().any(|p| p.id == DEFAULT_PALETTE));
    }

    #[test]
    fn ptyxis_gnome_values_and_derived_context_colors() {
        let p = builtin().into_iter().find(|p| p.id == "gnome").unwrap();
        assert_eq!(p.name, "GNOME");
        assert_eq!(p.dark.background.hex(), "#1c1c1f");
        assert_eq!(p.light.colors[1].hex(), "#c01c28");
        // abgeleitet: 33 % Color1 über dunklem Grund
        assert_eq!(p.dark.superuser_background, p.dark.background.mix(&p.dark.colors[1], 0.33));
        assert_eq!(p.light.remote_background, p.light.background.mix(&p.light.colors[12], 0.5));
    }

    #[test]
    fn ptyxis_single_section_and_errors() {
        let mut text = String::from("[Palette]\nName=Alt\nForeground=#ffffff\nBackground=#000000\n");
        for i in 0..16 {
            text.push_str(&format!("Color{i}=#{:02x}{:02x}{:02x}\n", i * 10, i * 5, i));
        }
        let p = Palette::parse_ptyxis("alt", &text).unwrap();
        assert_eq!(p.light, p.dark);
        assert!(!p.has_variants());
        assert_eq!(Palette::parse_ptyxis("x", "[Light]\n"), Err(PaletteError::Missing("[Palette]".into())));
        let broken = text.replace("Color3=#1e0f03", "Color3=blau");
        assert!(matches!(Palette::parse_ptyxis("x", &broken), Err(PaletteError::BadColor(k, _)) if k == "Color3"));
    }

    #[test]
    fn tilix_scheme_import() {
        let json = r##"{"name": "Solarized Dark", "use-theme-colors": false,
            "foreground-color": "#839496", "background-color": "#002B36",
            "palette": ["#073642","#DC322F","#859900","#B58900","#268BD2","#D33682","#2AA198","#EEE8D5",
                        "#002B36","#CB4B16","#586E75","#657B83","#839496","#6C71C4","#93A1A1","#FDF6E3"]}"##;
        let p = Palette::parse_any("solarized-dark", json).unwrap();
        assert_eq!(p.name, "Solarized Dark");
        assert_eq!(p.dark.background.hex(), "#002b36");
        assert_eq!(p.light, p.dark);
        assert!(Palette::parse_tilix("x", r#"{"name":"kaputt"}"#).is_err());
        assert!(Palette::parse_tilix("x", "kein json").is_err());
    }

    #[test]
    fn roundtrip_through_ptyxis_format() {
        for p in builtin() {
            let again = Palette::parse_ptyxis(&p.id, &p.to_ptyxis()).unwrap();
            assert_eq!(again, p, "{}", p.id);
        }
    }

    #[test]
    fn slugs() {
        assert_eq!(slug("Solarized Dark"), "solarized-dark");
        assert_eq!(slug("Grün & Blau!"), "gruen-blau");
        assert_eq!(slug("***"), "schema");
    }

    #[test]
    fn non_ascii_colour_is_rejected() {
        assert_eq!(Rgb::parse("#aäbcd"), None);
        assert_eq!(Rgb::parse("#äää"), None);
    }

    /// GNOME-Akzentfarben (libadwaita 1.6+): Blau, Türkis, Grün, Gelb, Orange, Rot, Pink,
    /// Lila, Schiefer.
    const ACCENTS: [&str; 9] =
        ["#3584e4", "#2190a4", "#3a944a", "#c88800", "#ed5b00", "#e62d42", "#d56199", "#9141ac", "#6f8396"];

    #[test]
    fn selection_stays_readable() {
        for p in builtin() {
            for (name, face) in [("hell", &p.light), ("dunkel", &p.dark)] {
                for accent in ACCENTS {
                    let sel = face.selection(&Rgb::parse(accent).unwrap());
                    // Schrift auf der Auswahl: 4,5:1, bei Schemata mit wenig Kontrast (Solarized
                    // hat selbst nur 4,1:1) höchstens 40 % weniger als ohne Auswahl
                    let text = face.foreground.contrast(&sel);
                    let need = 4.5f64.min(0.6 * face.foreground.contrast(&face.background));
                    assert!(text >= need, "{} {name}, Akzent {accent}: Schrift auf Auswahl {text:.2}:1", p.id);
                    // Auswahl hebt sich vom Grund ab (nur Helligkeit; der Farbton kommt dazu)
                    let edge = face.background.contrast(&sel);
                    assert!(edge >= 1.1, "{} {name}, Akzent {accent}: Auswahl kaum sichtbar ({edge:.2}:1)", p.id);
                }
            }
        }
    }

    #[test]
    fn high_contrast_overrides_profile() {
        assert_eq!(effective_id("nord", false), "nord");
        assert_eq!(effective_id("nord", true), HIGH_CONTRAST_PALETTE);
        assert!(builtin().iter().any(|p| p.id == HIGH_CONTRAST_PALETTE));
    }

    #[test]
    fn imported_palettes_sorted_after_builtin() {
        let user = |id: &str, name: &str| Palette { id: id.into(), name: name.into(), ..builtin()[0].clone() };
        let mut list = builtin();
        list.push(user("zenburn", "Zenburn"));
        list.push(user("ayu", "ayu"));
        list.push(user("monokai", "Monokai"));
        sort(&mut list);
        let ids: Vec<&str> = list.iter().map(|p| p.id.as_str()).collect();
        let n = BUILTIN.len();
        assert_eq!(ids[..n], BUILTIN.iter().map(|(id, _)| *id).collect::<Vec<_>>()[..]);
        assert_eq!(ids[n..], ["ayu", "monokai", "zenburn"]);
    }
}
