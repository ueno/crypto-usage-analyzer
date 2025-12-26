# Install desktop file
configure_file(
  input: 'org.gnome.CryptoUsageAnalyzer.desktop.in',
  output: 'org.gnome.CryptoUsageAnalyzer.desktop',
  copy: true,
  install: true,
  install_dir: datadir / 'applications'
)

desktop_file = join_paths(meson.current_build_dir(), 'org.gnome.CryptoUsageAnalyzer.desktop')

desktop_utils = find_program('desktop-file-validate', required: false)
if desktop_utils.found()
  test('Validate desktop file', desktop_utils,
    args: [desktop_file]
  )
endif

# Install metainfo file
configure_file(
  input: 'org.gnome.CryptoUsageAnalyzer.metainfo.xml.in',
  output: 'org.gnome.CryptoUsageAnalyzer.metainfo.xml',
  copy: true,
  install: true,
  install_dir: datadir / 'metainfo'
)

appstream_file = join_paths(meson.current_build_dir(), 'org.gnome.CryptoUsageAnalyzer.metainfo.xml')

appstreamcli = find_program('appstreamcli', required: false)
if appstreamcli.found()
  test('Validate appstream file', appstreamcli,
    args: ['validate', '--no-net', '--explain', appstream_file]
  )
endif
