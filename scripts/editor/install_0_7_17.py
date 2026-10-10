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

VERSION = '0.7.17-pre'
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


def install(source, destination):
    if destination.exists():
        raise ValueError(f'Ziel existiert bereits: {destination}. Es wird nicht überschrieben.')
    binary = source / 'target/release/openomsi'
    steam = source / 'target/release/libsteam_api.so'
    if not binary.is_file() or not steam.is_file():
        raise ValueError('Build-Ausgabe oder Steam-Laufzeit fehlt.')
    destination.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix='.editor-0.7.17-', dir=destination.parent) as folder:
        staging = Path(folder) / 'editor'
        staging.mkdir()
        for item in [binary, steam]:
            shutil.copy2(item, staging / item.name)
        (staging / 'openomsi').chmod(0o755)
        for name in ['assets', 'docs']:
            shutil.copytree(source / name, staging / name)
        (staging / 'start-editor.sh').write_text('''#!/usr/bin/env bash
set -euo pipefail
editor_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
if [[ $# -lt 1 ]]; then
  echo 'Start: bash start-editor.sh "/Pfad/zum/openOMSI-Spielinhaltsordner"'
  exit 2
fi
content_dir="$(cd -- "$1" && pwd)"
shift
export OMSI_CONTENT="$content_dir"
export OMSI_NO_UPDATE=1
cd -- "$content_dir"
exec "$editor_dir/openomsi" "$@"
''', encoding='utf-8')
        (staging / 'start-editor.sh').chmod(0o755)
        (staging / 'VERSION-EDITOR.txt').write_text(
            f'Editor {VERSION}\nopenOMSI-Basis 0.2.27\nQuellpaket SHA256: '
            + hashlib.sha256((source / 'SOURCE-SHA256.json').read_bytes()).hexdigest()
            + '\nInteraktiver Kartentest noch erforderlich.\n', encoding='utf-8')
        if destination.exists():
            raise ValueError('Zielordner wurde inzwischen angelegt.')
        staging.rename(destination)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--install-dir', type=Path, default=ROOT.parent / ('openOMSI-Editor-' + VERSION))
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
    env = dict(os.environ, OPENOMSI_VERSION='0.2.27-editor-' + VERSION,
               CARGO_TARGET_DIR=str(ROOT / 'target'))
    run('cargo', 'check', '--locked', '-p', 'omsi-app', '--features', 'standalone-editor', env=env)
    for group in ['traffic_editor::tests', 'generated_ai', 'arm_labels_keep_identity', 'editor_dock_keeps_catalogue']:
        run('cargo', 'test', '--locked', '-p', 'omsi-app', '--features', 'standalone-editor', '--lib', group, env=env)
    run('cargo', 'build', '--release', '--locked', '-p', 'omsi-app', '--features', 'standalone-editor', env=env)
    install(ROOT, destination)
    print(f'\nFERTIG. Neuer Editor: {destination}\nStartskript: {destination / "start-editor.sh"}')
    print('Bisherigen Inhaltsordner an das Startskript übergeben. Anleitung: docs/UPDATE_0.7.17.md')


if __name__ == '__main__':
    try:
        main()
    except (ValueError, OSError, subprocess.CalledProcessError) as exc:
        print(f'\nAbgebrochen: {exc}\nBitte update-0.7.17-pre.log schicken.', file=sys.stderr)
        sys.exit(1)
