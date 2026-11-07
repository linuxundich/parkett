// Links libghostty-vt >= 1.0 (stable C ABI). Arch's `ghostty` package still ships the pre-1.0
// 0.1.0-dev library, which the bindings do not match, so require version 1.
fn main() {
    println!("cargo:rerun-if-env-changed=PKG_CONFIG_PATH");
    pkg_config::Config::new().atleast_version("1.0.0").probe("libghostty-vt").expect(
        "libghostty-vt >= 1.0.0 not found (build it from https://release.files.ghostty.org/libghostty-vt/ \
         and set PKG_CONFIG_PATH, see docs/LIBGHOSTTY.md)",
    );
}
