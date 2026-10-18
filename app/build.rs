//! Kompiliert das GSettings-Schema nach $OUT_DIR/schemas, damit `cargo run` ohne
//! Installation funktioniert. Ein Meson-Build (PARKETT_INSTALLED gesetzt) nutzt das
//! installierte Schema.
use std::path::PathBuf;
use std::process::Command;

fn main() {
    let out = PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR")).join("schemas");
    std::fs::create_dir_all(&out).expect("Schema-Ordner anlegen");
    let schema = "../data/de.linuxundich.Parkett.gschema.xml";
    println!("cargo:rerun-if-changed={schema}");
    println!("cargo:rerun-if-env-changed=PARKETT_INSTALLED");
    std::fs::copy(schema, out.join("de.linuxundich.Parkett.gschema.xml")).expect("Schema kopieren");
    let status = Command::new("glib-compile-schemas")
        .arg("--strict")
        .arg(&out)
        .status()
        .expect("glib-compile-schemas fehlt");
    assert!(status.success(), "glib-compile-schemas ist fehlgeschlagen");
}
