desktop_conf = configuration_data()
desktop_conf.set('BINDIR', bindir)
desktop_file = configure_file(
  input: app_id + '.desktop.in',
  output: app_id + '.desktop',
  configuration: desktop_conf,
  install: true,
  install_dir: datadir / 'applications',
)

metainfo_file = configure_file(
  input: app_id + '.metainfo.xml.in',
  output: app_id + '.metainfo.xml',
  copy: true,
  install: true,
  install_dir: datadir / 'metainfo',
)

install_data(app_id + '.gschema.xml', install_dir: datadir / 'glib-2.0' / 'schemas')
install_data('icons/hicolor/scalable/apps/' + app_id + '.svg', install_dir: datadir / 'icons/hicolor/scalable/apps')
install_data('icons/hicolor/symbolic/apps/' + app_id + '-symbolic.svg', install_dir: datadir / 'icons/hicolor/symbolic/apps')
install_data('icons/hicolor/scalable/actions/parkett-split-right-symbolic.svg', 'icons/hicolor/scalable/actions/parkett-split-down-symbolic.svg', install_dir: datadir / 'icons/hicolor/scalable/actions')

# Kontextmenü in Nautilus; wirkt nur mit nautilus-python, schadet ohne nicht
nautilus_ext = configure_file(
  input: 'nautilus/parkett-nautilus.py.in',
  output: 'parkett-nautilus.py',
  configuration: desktop_conf,
  install: true,
  install_dir: datadir / 'nautilus-python' / 'extensions',
)

desktop_file_validate = find_program('desktop-file-validate', required: false)
if desktop_file_validate.found()
  test('validate-desktop', desktop_file_validate, args: [desktop_file])
endif
appstreamcli = find_program('appstreamcli', required: false)
if appstreamcli.found()
  test('validate-metainfo', appstreamcli, args: ['validate', '--no-net', '--explain', metainfo_file])
endif
compile_schemas = find_program('glib-compile-schemas', required: false)
if compile_schemas.found()
  test('validate-schema', compile_schemas, args: ['--strict', '--dry-run', meson.current_source_dir()])
endif
python3 = find_program('python3', required: false)
if python3.found()
  test('compile-nautilus-extension', python3, args: ['-c', 'import ast, sys; ast.parse(open(sys.argv[1]).read())', nautilus_ext])
endif
