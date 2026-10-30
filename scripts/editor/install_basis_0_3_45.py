#!/usr/bin/env python3
"""Build this source package and atomically install a separate Linux editor."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile

VERSION = '0.8.9-pre'
ROOT = Path(__file__).resolve().parents[2]


def run(*args, cwd=ROOT, env=None):
    print('+', ' '.join(map(str, args)), flush=True)
    subprocess.run(list(map(str, args)), cwd=cwd, env=env, check=True)


def verify_package(source):
    manifest = json.loads((source / 'SOURCE-SHA256.json').read_text(encoding='utf-8'))
    if manifest['version'] != VERSION:
        raise ValueError('Versionsangabe des Pakets stimmt nicht.')
    for name, expected in manifest['files'].items():
        relative = Path(name)
        if relative.is_absolute() or '..' in relative.parts:
            raise ValueError('Ungültiger Paketpfad.')
        path = source / relative
        if path.is_symlink() or not path.is_file() or hashlib.sha256(path.read_bytes()).hexdigest() != expected:
            raise ValueError(f'Quelldatei fehlt oder wurde verändert: {name}. Bitte das Paket erneut in einen neuen Ordner entpacken.')


def install(source, destination, profile="release"):
    if destination.exists():
        raise ValueError(f'Ziel existiert bereits: {destination}. Es wird nicht überschrieben.')
    binary = Path(os.environ.get('CARGO_TARGET_DIR', source / 'target')) / profile / 'openomsi'
    steam = Path(os.environ.get('CARGO_TARGET_DIR', source / 'target')) / profile / 'libsteam_api.so'
    launcher = Path(os.environ.get('CARGO_LAUNCHER_TARGET_DIR', os.environ.get('CARGO_TARGET_DIR', source / 'target'))) / profile / 'openomsi-launcher-app'
    if not binary.is_file() or not steam.is_file() or not launcher.is_file():
        raise ValueError('Build-Ausgabe oder Steam-Laufzeit fehlt.')
    destination.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix='.editor-0.8.9-', dir=destination.parent) as folder:
        staging = Path(folder) / 'editor'
        staging.mkdir()
        for item in [binary, steam, launcher]:
            shutil.copy2(item, staging / item.name)
        (staging / 'openomsi').chmod(0o755)
        for name in ['assets', 'docs']:
            shutil.copytree(source / name, staging / name)
        # Existing editor updaters accept documents only below docs/.
        # Keep the archive root compatible with already installed releases.
        for name in ['LICENSE', 'README.md']:
            shutil.copy2(source / name, staging / 'docs' / name)
        (staging / 'start-editor.sh').write_text('''#!/usr/bin/env bash
set -euo pipefail
editor_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
if [[ $# -lt 1 ]]; then
  echo 'Start: bash start-editor.sh "/Pfad/zum/openOMSI-Spielinhaltsordner"'
  exit 2
fi
content_dir="$(cd -- "$1" && pwd)"
shift
# Older desktop shortcuts explicitly requested the launcher. A bare start now opens it.
if [[ "${1:-}" == "--launcher" ]]; then shift; fi
export OMSI_CONTENT="$content_dir"
export OMSI_NO_UPDATE=1
export OPENOMSI_BIN="$editor_dir/openomsi"
cd -- "$content_dir"
exec "$editor_dir/openomsi" "$@"
''', encoding='utf-8')
        (staging / 'start-editor.sh').chmod(0o755)
        (staging / 'start-editor-diagnose.sh').write_text('''#!/usr/bin/env bash
set -euo pipefail
editor_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
export OPENOMSI_EDITOR_DIAGNOSTICS=1
exec bash "$editor_dir/start-editor.sh" "$@"
''', encoding='utf-8')
        (staging / 'start-editor-diagnose.sh').chmod(0o755)
        (staging / 'VERSION-EDITOR.txt').write_text(
            f'Editor {VERSION}\nopenOMSI-Basis 0.3.45\nQuellpaket SHA256: '
            + hashlib.sha256((source / 'SOURCE-SHA256.json').read_bytes()).hexdigest()
            + '\nLokale Linux-Vorabversion; automatisierte Prüfungen, praktische Kartenprüfung separat erforderlich.\n', encoding='utf-8')
        if destination.exists():
            raise ValueError('Zielordner wurde inzwischen angelegt.')
        staging.rename(destination)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--install-dir', type=Path, default=ROOT.parent / ('openOMSI-Editor-' + VERSION + '-Basis-0.3.45'))
    args = parser.parse_args()
    if not sys.platform.startswith('linux') or os.uname().machine != 'x86_64':
        raise ValueError('Dieser Installer ist für Linux x64.')
    destination = args.install_dir.expanduser().absolute()
    if destination.exists():
        raise ValueError(f'Ziel existiert bereits und bleibt erhalten: {destination}')
    for name in ['cargo', 'rustc', 'cc', 'pkg-config']:
        if shutil.which(name) is None:
            raise ValueError(f'{name} fehlt. Bitte die bisherige Rust-Bauumgebung verwenden.')
    verify_package(ROOT)
    env = dict(os.environ, OPENOMSI_VERSION='0.3.45-editor-' + VERSION,
               CARGO_TARGET_DIR=os.environ.get('CARGO_TARGET_DIR', str(ROOT / 'target')))
    run('cargo', 'check', '--locked', '-p', 'omsi-app', '--features', 'standalone-editor', env=env)
    for group in ['editor_path_edit', 'editor_reload::tests', 'editor_diagnostics::tests', 'traffic_editor::tests', 'traffic_connector::tests', 'generated_ai', 'arm_labels_keep_identity', 'editor_dock_keeps_catalogue']:
        run('cargo', 'test', '--locked', '-p', 'omsi-app', '--features', 'standalone-editor', '--lib', group, env=env)
    run('cargo', 'test', '--locked', '-p', 'omsi-sim', '--lib', 'additional_connector', env=env)
    run('cargo', 'test', '--locked', '-p', 'omsi-cfg', '--lib', 'flags', env=env)
    run('cargo', 'build', '--release', '--locked', '-p', 'omsi-app', '--features', 'standalone-editor', env=env)
    run('cargo', 'build', '--release', '--locked', '--manifest-path', ROOT / 'launcher/src-tauri/Cargo.toml', '--features', 'standalone-editor', env=env)
    install(ROOT, destination)
    print(f'\nFERTIG. Neuer Editor: {destination}\nStartskript: {destination / "start-editor.sh"}')
    print('Bisherigen Inhaltsordner an das Startskript übergeben. Anleitung: docs/PORTIERUNG_0.3.45.md')


if __name__ == '__main__':
    try:
        main()
    except (ValueError, OSError, subprocess.CalledProcessError) as exc:
        print(f'\nAbgebrochen: {exc}\nBitte update-0.8.9-pre.log schicken.', file=sys.stderr)
        sys.exit(1)
