project(
  'crypto-usage-analyzer',
  'rust',
  version: '0.1.0',
  meson_version: '>= 0.59',
  license: 'GPL-3.0-or-later',
)

dependency('gtk4', version: '>= 4.0')
dependency('libadwaita-1', version: '>= 1.0')
dependency('cairo', version: '>= 1.16')

cargo = find_program('cargo')
cargo_script = find_program('build-aux/cargo.sh')

prefix = get_option('prefix')
bindir = prefix / get_option('bindir')
datadir = prefix / get_option('datadir')

if get_option('profile') == 'development'
  profile = 'dev'
  rust_target = 'debug'
  message('Building in development mode')
else
  profile = 'release'
  rust_target = 'release'
  message('Building in release mode')
endif

app_id = 'org.gnome.CryptoUsageAnalyzer'

cargo_options = [ '--manifest-path', meson.project_source_root() / 'Cargo.toml' ]
cargo_options += [ '--target-dir', meson.project_build_root() / 'target' ]

if profile == 'release'
  cargo_options += [ '--release' ]
endif

cargo_env = [ 'CARGO_HOME=' + meson.project_build_root() / 'cargo-home' ]

cargo_build = custom_target(
  'cargo-build',
  build_by_default: true,
  build_always_stale: true,
  output: 'crypto-usage-analyzer',
  console: true,
  install: true,
  install_dir: bindir,
  command: [
    cargo_script,
    meson.project_build_root(),
    meson.project_source_root(),
    '@OUTPUT@',
    rust_target,
    app_id,
  ]
)

subdir('data')

meson.add_install_script('build-aux/meson-post-install.sh')
