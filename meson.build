project(
  'parkett',
  'rust',
  version: '0.4.0',
  license: 'GPL-3.0-or-later',
  meson_version: '>= 1.0',
)

gnome = import('gnome')
i18n = import('i18n')

app_id = 'de.linuxundich.Parkett'
prefix = get_option('prefix')
bindir = prefix / get_option('bindir')
datadir = prefix / get_option('datadir')
localedir = prefix / get_option('localedir')

dependency('gtk4', version: '>= 4.18')
dependency('libadwaita-1', version: '>= 1.8')
dependency('vte-2.91-gtk4', version: '>= 0.80')

subdir('data')
subdir('po')

# cargo baut das Programm; Meson kopiert es an die erwartete Stelle und installiert es.
cargo_build = find_program('build-aux/cargo-build.sh')
custom_target(
  'cargo-build',
  build_by_default: true,
  build_always_stale: true,
  console: true,
  output: 'parkett',
  install: true,
  install_dir: bindir,
  command: [cargo_build, meson.project_source_root(), meson.project_build_root(), get_option('buildtype'), '@OUTPUT@', localedir],
)

cargo = find_program('cargo')
test('cargo-test', cargo,
  args: ['test', '--manifest-path', meson.project_source_root() / 'Cargo.toml', '--quiet'],
  env: ['CARGO_TARGET_DIR=' + meson.project_build_root() / 'target'],
  timeout: 600,
)

gnome.post_install(
  glib_compile_schemas: true,
  gtk_update_icon_cache: true,
  update_desktop_database: true,
)
