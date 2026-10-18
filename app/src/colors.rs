//! Farbschema des Prototyps (identisch mit den Mockups), hell und dunkel.

use gtk::gdk::RGBA;

pub struct Scheme {
    pub fg: &'static str,
    pub bg: &'static str,
    pub palette: [&'static str; 16],
}

pub const LIGHT: Scheme = Scheme {
    fg: "#1e1e22",
    bg: "#ffffff",
    palette: [
        "#241f31", "#c01c28", "#26a269", "#a2734c", "#1e78e4", "#9841bb", "#0e8a9c", "#77767b",
        "#5e5c64", "#e01b24", "#2ec27e", "#b88a00", "#3584e4", "#c061cb", "#2190a4", "#241f31",
    ],
};

pub const DARK: Scheme = Scheme {
    fg: "#e4e3e6",
    bg: "#1d1d20",
    palette: [
        "#3d3846", "#ff7b72", "#57e389", "#f8e45c", "#62a0ea", "#dc8add", "#5bc8e0", "#c0bfbc",
        "#77767b", "#ff938c", "#8ff0a4", "#f9f06b", "#99c1f1", "#f0a6f0", "#93ddf2", "#ffffff",
    ],
};

impl Scheme {
    pub fn rgba(hex: &str) -> RGBA {
        RGBA::parse(hex).expect("gültige Farbe im eingebauten Schema")
    }
}
