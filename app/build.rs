// SPDX-License-Identifier: GPL-3.0-or-later
//! Compiles the GSettings schema to $OUT_DIR/schemas and the translations to
//! $OUT_DIR/locale, so that `cargo run` works without installing. A Meson build
//! (PARKETT_INSTALLED, PARKETT_LOCALEDIR set) uses the installed files.
use std::path::PathBuf;
use std::process::Command;

fn main() {
    let out = PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR")).join("schemas");
    std::fs::create_dir_all(&out).expect("Schema-Ordner anlegen");
    let schema = "../data/de.linuxundich.Parkett.gschema.xml";
    println!("cargo:rerun-if-changed={schema}");
    println!("cargo:rerun-if-env-changed=PARKETT_INSTALLED");
    std::fs::copy(schema, out.join("de.linuxundich.Parkett.gschema.xml")).expect("Schema kopieren");
    let status =
        Command::new("glib-compile-schemas").arg("--strict").arg(&out).status().expect("glib-compile-schemas fehlt");
    assert!(status.success(), "glib-compile-schemas ist fehlgeschlagen");
    compile_translations(&PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR")));
}

/// po/*.po → $OUT_DIR/locale/<lang>/LC_MESSAGES/parkett.mo (without msgfmt: no translations)
fn compile_translations(out_dir: &std::path::Path) {
    println!("cargo:rerun-if-changed=../po");
    println!("cargo:rerun-if-env-changed=PARKETT_LOCALEDIR");
    let Ok(entries) = std::fs::read_dir("../po") else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().is_none_or(|e| e != "po") {
            continue;
        }
        let Some(lang) = path.file_stem().and_then(|s| s.to_str()) else { continue };
        let target = out_dir.join("locale").join(lang).join("LC_MESSAGES");
        std::fs::create_dir_all(&target).expect("Ordner für Übersetzungen anlegen");
        match Command::new("msgfmt").arg("-o").arg(target.join("parkett.mo")).arg(&path).status() {
            Ok(s) if s.success() => {}
            Ok(_) => panic!("msgfmt: {} ist fehlerhaft", path.display()),
            Err(_) => println!("cargo:warning=msgfmt fehlt, Parkett läuft ohne Übersetzungen"),
        }
    }
}
