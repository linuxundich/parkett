// SPDX-License-Identifier: GPL-3.0-or-later
//! Color schemes in Ptyxis' palette format, plus import of Tilix schemes (JSON).
//!
//! A palette has a light and a dark variant (`Face`). The colors of the context marking
//! (root, SSH, container, other user) come from the palette by hue, with the GNOME colors as
//! fallback; tint and dimming keep text readable.

use std::collections::HashMap;
use std::fmt;

use crate::context::Kind;
use crate::i18n::{gettext, trf};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Rgb {
    /// `#rrggbb` or `#rgb`, case-insensitive.
    pub fn parse(s: &str) -> Option<Rgb> {
        let h = s.trim().strip_prefix('#')?;
        // sliced by bytes: non-ASCII would cut in the middle of a character
        if !h.is_ascii() {
            return None;
        }
        let v = |i: usize, n: usize| u8::from_str_radix(&h[i..i + n], 16).ok();
        match h.len() {
            6 => Some(Rgb { r: v(0, 2)?, g: v(2, 2)?, b: v(4, 2)? }),
            3 => Some(Rgb { r: v(0, 1)? * 17, g: v(1, 1)? * 17, b: v(2, 1)? * 17 }),
            // GdkRGBA accepts #rrrrggggbbbb (Tilix files from gnome-terminal)
            12 => Some(Rgb { r: v(0, 2)?, g: v(4, 2)?, b: v(8, 2)? }),
            _ => None,
        }
    }

    /// Components 0.0–1.0 for Cairo and GdkRGBA – the one conversion for the whole app (D3).
    pub fn to_f64(&self) -> (f64, f64, f64) {
        (self.r as f64 / 255.0, self.g as f64 / 255.0, self.b as f64 / 255.0)
    }

    /// Counterpart of `to_f64`; values outside 0.0–1.0 are clamped.
    pub fn from_f64(r: f64, g: f64, b: f64) -> Rgb {
        let v = |x: f64| (x * 255.0).round().clamp(0.0, 255.0) as u8;
        Rgb { r: v(r), g: v(g), b: v(b) }
    }

    pub fn hex(&self) -> String {
        format!("#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
    }

    /// Relative luminance per WCAG.
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

    /// Contrast ratio per WCAG (1 to 21).
    pub fn contrast(&self, other: &Rgb) -> f64 {
        let (a, b) = (self.luminance(), other.luminance());
        (a.max(b) + 0.05) / (a.min(b) + 0.05)
    }

    /// Hue (0–360°), saturation and lightness (0–1), as in CSS `hsl()`.
    pub fn hsl(&self) -> (f64, f64, f64) {
        let (r, g, b) = self.to_f64();
        let (max, min) = (r.max(g).max(b), r.min(g).min(b));
        let l = (max + min) / 2.0;
        if max == min {
            return (0.0, 0.0, l);
        }
        let d = max - min;
        let s = if l > 0.5 { d / (2.0 - max - min) } else { d / (max + min) };
        let h = if max == r {
            (g - b) / d + if g < b { 6.0 } else { 0.0 }
        } else if max == g {
            (b - r) / d + 2.0
        } else {
            (r - g) / d + 4.0
        };
        (h * 60.0, s, l)
    }

    /// `alpha` share of `fg` over `self`.
    pub fn mix(&self, fg: &Rgb, alpha: f64) -> Rgb {
        let m = |b: u8, f: u8| ((1.0 - alpha) * b as f64 + alpha * f as f64).round().clamp(0.0, 255.0) as u8;
        Rgb { r: m(self.r, fg.r), g: m(self.g, fg.g), b: m(self.b, fg.b) }
    }
}

/// The colorful palette colors (normal and bright, without black, white and grays).
const ANSI_COLORS: [usize; 12] = [1, 2, 3, 4, 5, 6, 9, 10, 11, 12, 13, 14];

/// One variant (light or dark) of a palette.
#[derive(Clone, Debug, PartialEq)]
pub struct Face {
    pub foreground: Rgb,
    pub background: Rgb,
    pub cursor: Option<Rgb>,
    pub cursor_foreground: Option<Rgb>,
    pub colors: [Rgb; 16],
    /// background for panes with root privileges, if the palette sets one (Ptyxis format)
    pub superuser_background: Option<Rgb>,
    /// background for panes with an SSH connection, if the palette sets one
    pub remote_background: Option<Rgb>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Palette {
    /// file name without extension, e.g. "gnome"
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
            PaletteError::Missing(what) => f.write_str(&trf("{what} is missing", &[("what", what)])),
            PaletteError::BadColor(key, v) => {
                f.write_str(&trf("{key}: “{value}” is not a color", &[("key", key), ("value", v)]))
            }
            PaletteError::Json(e) => f.write_str(&trf("not a valid Tilix scheme: {error}", &[("error", e)])),
        }
    }
}

impl std::error::Error for PaletteError {}

type Section = HashMap<String, String>;

/// Minimal parser for the GKeyFile format of the palettes (sections, key=value, # comments).
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
    /// Background of selected characters: the system accent color over the background, like
    /// text selection in GTK. Without its own color VTE draws the selection inverted – in the
    /// dark style a glaring bright block. The text keeps its color.
    pub fn selection(&self, accent: &Rgb) -> Rgb {
        self.background.mix(accent, if self.background.is_dark() { 0.3 } else { 0.25 })
    }

    pub fn new(foreground: Rgb, background: Rgb, colors: [Rgb; 16], cursor: Option<Rgb>) -> Face {
        Face {
            foreground,
            background,
            cursor,
            cursor_foreground: None,
            colors,
            superuser_background: None,
            remote_background: None,
        }
    }

    /// Strong color of a context (line, frame, badge). Fixed meaning – root red, SSH blue,
    /// container purple, other user amber – the shade from the palette: the color with the
    /// nearest hue that is saturated enough and stands out from the background by 3:1. The slots
    /// `Color1`/`Color12` were wrong in 9 of 28 variants (Papercolor: SSH orange, root line green).
    /// Without a match the GNOME color (Adwaita palette) of the matching lightness.
    pub fn context_color(&self, kind: Kind) -> Rgb {
        let (hue, tolerance, light, dark) = match kind {
            Kind::Superuser => (0.0, 12.0, "#c01c28", "#f66151"),
            Kind::Remote => (215.0, 25.0, "#1c71d8", "#62a0ea"),
            Kind::Container => (285.0, 25.0, "#813d9c", "#c061cb"),
            Kind::OtherUser => (40.0, 12.0, "#c64600", "#ffa348"),
        };
        let mut best: Option<(f64, Rgb)> = None;
        for c in ANSI_COLORS.iter().map(|&i| self.colors[i]) {
            let (h, s, l) = c.hsl();
            let d = (h - hue).abs().min(360.0 - (h - hue).abs());
            if d <= tolerance
                && s >= 0.3
                && (0.2..0.85).contains(&l)
                && c.contrast(&self.background) >= 3.0
                && best.is_none_or(|(bd, _)| d < bd)
            {
                best = Some((d, c));
            }
        }
        best.map(|(_, c)| c)
            .unwrap_or_else(|| Rgb::parse(if self.background.is_dark() { dark } else { light }).unwrap())
    }

    /// Background of a pane in this context. Only root and SSH are tinted (SSH half as
    /// strongly), the other contexts keep the background. The tint may weaken the contrast of the
    /// text by at most 12 % and that of the palette colors by at most 15 %, otherwise it gets
    /// weaker, if need be down to nothing. A background set in the palette counts as the strongest tint.
    pub fn context_background(&self, kind: Kind) -> Rgb {
        let dark = self.background.is_dark();
        let (explicit, share) = match kind {
            Kind::Superuser => (self.superuser_background, 1.0),
            Kind::Remote => (self.remote_background, 0.5),
            _ => return self.background,
        };
        let (target, max) = match explicit {
            Some(c) => (c, share),
            None => (self.context_color(kind), share * if dark { 0.16 } else { 0.12 }),
        };
        let text = self.foreground.contrast(&self.background);
        let colors = self.weakest_color(&self.background);
        let mut alpha = max;
        while alpha > 0.005 {
            let tinted = self.background.mix(&target, alpha);
            if self.foreground.contrast(&tinted) >= 0.88 * text && self.weakest_color(&tinted) >= 0.85 * colors {
                return tinted;
            }
            alpha -= 0.01;
        }
        self.background
    }

    /// Lowest contrast of the colorful palette colors on `background`.
    fn weakest_color(&self, background: &Rgb) -> f64 {
        ANSI_COLORS.iter().map(|&i| self.colors[i].contrast(background)).fold(f64::INFINITY, f64::min)
    }

    /// Opacity of the dimming of inactive panes over `background`: at most 25 %, and the dimmed
    /// text keeps 4.5:1 (it was 38 % – with tint, text fell to 2:1 in light schemes).
    pub fn dim_alpha(&self, background: &Rgb) -> f64 {
        let mut alpha: f64 = 0.25;
        while alpha > 0.005 {
            if self.foreground.mix(background, alpha).contrast(background) >= 4.5 {
                return alpha;
            }
            alpha -= 0.01;
        }
        0.0
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
        face.superuser_background = color(sec, "SuperuserBackground")?;
        face.remote_background = color(sec, "RemoteBackground")?;
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
        if let Some(c) = self.superuser_background {
            out.push_str(&format!("SuperuserBackground={}\n", c.hex()));
        }
        if let Some(c) = self.remote_background {
            out.push_str(&format!("RemoteBackground={}\n", c.hex()));
        }
    }
}

impl Palette {
    /// Reads a Ptyxis palette. Without `[Light]`/`[Dark]` (older files from Gogh) the
    /// colors are in the `[Palette]` section and apply to both variants.
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
            // Without colors in the [Palette] section, [Light]/[Dark] is what is really missing; otherwise
            // the message would say "Color0 missing in [Palette]" and point at the wrong section (QA-15)
            (None, None) if !info.contains_key("Foreground") && !info.contains_key("Color0") => {
                return Err(PaletteError::Missing(gettext("Section [Light] or [Dark]")));
            }
            (None, None) => {
                let f = Face::from_section(info, "Palette")?;
                (f.clone(), f)
            }
        };
        Ok(Palette { id: id.to_string(), name, light, dark })
    }

    /// Reads a Tilix color scheme (`data/schemes/*.json` in Tilix). Tilix has only one
    /// variant; it applies to light and dark.
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
            return Err(PaletteError::Missing(trf(
                "16 colors in palette (only {count})",
                &[("count", &list.len().to_string())],
            )));
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

    /// Detects the format from the content: JSON object = Tilix, otherwise Ptyxis.
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

    /// Does the palette really have two different variants?
    pub fn has_variants(&self) -> bool {
        self.light != self.dark
    }

    /// Writes the palette in Ptyxis format (for imported schemes).
    pub fn to_ptyxis(&self) -> String {
        let mut out = format!("[Palette]\nName={}\n\n[Light]\n", self.name);
        self.light.write(&mut out);
        out.push_str("\n[Dark]\n");
        self.dark.write(&mut out);
        out
    }
}

/// File-name-safe id from a display name ("Solarized Dark" → "solarized-dark").
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

/// The bundled schemes (files in data/palettes, embedded in the program), in the
/// order of the selection.
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

/// The bundled schemes, parsed.
pub fn builtin() -> Vec<Palette> {
    BUILTIN
        .iter()
        .map(|(id, text)| Palette::parse_ptyxis(id, text).unwrap_or_else(|e| panic!("eingebaute Palette {id}: {e}")))
        .collect()
}

pub const DEFAULT_PALETTE: &str = "gnome";

/// Scheme for GNOME's high contrast (*Accessibility › High Contrast*).
pub const HIGH_CONTRAST_PALETTE: &str = "gnome-high-contrast";

/// Palette a pane actually shows: with the system's high contrast, the
/// high-contrast scheme instead of the scheme from the profile.
pub fn effective_id(chosen: &str, high_contrast: bool) -> &str {
    if high_contrast {
        HIGH_CONTRAST_PALETTE
    } else {
        chosen
    }
}

/// Bundled schemes first in a fixed order, then the custom ones alphabetically –
/// also right after an import, not only on the next start.
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
    fn float_roundtrip() {
        let c = Rgb::parse("#3584e4").unwrap();
        let (r, g, b) = c.to_f64();
        assert_eq!(Rgb::from_f64(r, g, b), c);
        assert_eq!(Rgb::from_f64(-0.5, 1.5, 0.0), Rgb { r: 0, g: 255, b: 0 });
    }

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

    /// Text must be readable in every built-in variant (QA-10: "One" dark had 2.7:1).
    /// Threshold 4.0 instead of WCAG 4.5, because Solarized light is at 4.1:1 in the original.
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
    fn ptyxis_gnome_values() {
        let p = builtin().into_iter().find(|p| p.id == "gnome").unwrap();
        assert_eq!(p.name, "GNOME");
        assert_eq!(p.dark.background.hex(), "#1c1c1f");
        assert_eq!(p.light.colors[1].hex(), "#c01c28");
        assert_eq!(p.dark.superuser_background, None);
    }

    /// Approach C over all built-in schemes, light and dark: hue matches the meaning, the line
    /// stands out, tint and dimming keep text readable.
    #[test]
    fn context_colors_in_all_palettes() {
        for p in builtin() {
            for (name, f) in [("light", &p.light), ("dark", &p.dark)] {
                let ctx = format!("{} {name}", p.id);
                for (kind, hue) in
                    [(Kind::Superuser, 0.0), (Kind::Remote, 215.0), (Kind::Container, 285.0), (Kind::OtherUser, 40.0)]
                {
                    let c = f.context_color(kind);
                    let (h, _, _) = c.hsl();
                    let d = (h - hue).abs().min(360.0 - (h - hue).abs());
                    assert!(d <= 30.0, "{ctx} {kind:?}: hue {h:.0} for {}", c.hex());
                    assert!(
                        c.contrast(&f.background) >= 3.0,
                        "{ctx} {kind:?}: line only {:.1}:1",
                        c.contrast(&f.background)
                    );
                    let bg = f.context_background(kind);
                    let text = f.foreground.contrast(&f.background);
                    assert!(f.foreground.contrast(&bg) >= 0.88 * text - 1e-9, "{ctx} {kind:?}: tint too strong");
                    let dim = f.dim_alpha(&bg);
                    assert!(dim <= 0.25);
                    let dimmed = f.foreground.mix(&bg, dim).contrast(&bg);
                    assert!(dim == 0.0 || dimmed >= 4.5, "{ctx} {kind:?}: dimmed text {dimmed:.1}:1");
                }
                assert_eq!(f.context_background(Kind::Container), f.background);
                assert_ne!(f.context_background(Kind::Superuser), f.background, "{ctx}: root without tint");
            }
        }
        // Papercolor dark puts orange in Color12 and green in Color9 (screenshot 2026-10-25)
        let paper = builtin().into_iter().find(|p| p.id == "papercolor").unwrap();
        assert_eq!(paper.dark.context_color(Kind::Superuser).hex(), "#f66151");
        assert_eq!(paper.dark.context_color(Kind::Remote).hex(), "#5fafd7");
    }

    #[test]
    fn explicit_context_background_is_kept_if_readable() {
        let mut f = builtin().into_iter().find(|p| p.id == "gnome").unwrap().dark;
        let soft = f.background.mix(&Rgb::parse("#ff0000").unwrap(), 0.1);
        f.superuser_background = Some(soft);
        assert_eq!(f.context_background(Kind::Superuser), soft);
        let mut text = String::new();
        f.write(&mut text);
        assert!(text.contains("SuperuserBackground=") && !text.contains("RemoteBackground="));
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
        assert_eq!(
            Palette::parse_ptyxis("x", "[Palette]\nName=Leer\n"),
            Err(PaletteError::Missing("Section [Light] or [Dark]".into()))
        );
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

    /// GNOME accent colors (libadwaita 1.6+): blue, teal, green, yellow, orange, red, pink,
    /// purple, slate.
    const ACCENTS: [&str; 9] =
        ["#3584e4", "#2190a4", "#3a944a", "#c88800", "#ed5b00", "#e62d42", "#d56199", "#9141ac", "#6f8396"];

    #[test]
    fn selection_stays_readable() {
        for p in builtin() {
            for (name, face) in [("hell", &p.light), ("dunkel", &p.dark)] {
                for accent in ACCENTS {
                    let sel = face.selection(&Rgb::parse(accent).unwrap());
                    // text on the selection: 4.5:1, for schemes with little contrast (Solarized
                    // itself only has 4.1:1) at most 40 % less than without selection
                    let text = face.foreground.contrast(&sel);
                    let need = 4.5f64.min(0.6 * face.foreground.contrast(&face.background));
                    assert!(text >= need, "{} {name}, Akzent {accent}: Schrift auf Auswahl {text:.2}:1", p.id);
                    // selection stands out from the background (lightness only; the hue adds to it)
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

    /// High contrast: colored ANSI tones easy to read, normal and bright distinguishable (QA-10).
    #[test]
    fn high_contrast_colors_reach_targets() {
        let p = builtin().into_iter().find(|p| p.id == HIGH_CONTRAST_PALETTE).unwrap();
        for (name, face, normal, bright) in [("hell", &p.light, 7.0, 4.5), ("dunkel", &p.dark, 4.5, 7.0)] {
            for i in (1..=6).chain(9..=14) {
                let need = if i < 8 { normal } else { bright };
                let r = face.colors[i].contrast(&face.background);
                assert!(r >= need, "{name}: Color{i} nur {r:.2}:1, nötig {need}:1");
            }
            for i in 1..=6 {
                assert_ne!(face.colors[i], face.colors[i + 8], "{name}: Color{i} und Color{} gleich", i + 8);
            }
        }
    }
}
