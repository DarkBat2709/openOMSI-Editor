#!/usr/bin/env python3
"""Install the reviewed Git bundle in a separate worktree; build before installing.

Run the copy shipped beside candidate.json and editor-update.bundle.
No push, reset, checkout of an existing branch, or game-content writes.
"""
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile

BASE = '04497112ab4343960c8348813112e5c452f4704e'
BRANCH = 'update/openomsi-0.2.27'
VERSION = '0.7.11-pre'


def run(*args, cwd=None, env=None):
    print('+', ' '.join(map(str, args)), flush=True)
    subprocess.run(list(map(str, args)), cwd=cwd, env=env, check=True)


def git(source, *args):
    return subprocess.check_output(['git', '-C', str(source), *args], text=True).strip()


def install(source, destination):
    """Only publish a complete new folder after all build gates passed."""
    destination = Path(destination)
    if destination.exists():
        raise ValueError(f'Ziel existiert bereits und bleibt unverändert: {destination}')
    binary = source / 'target/release/openomsi'
    steam = source / 'target/release/libsteam_api.so'
    if not binary.is_file() or not steam.is_file():
        raise ValueError('Build-Ausgabe oder Steam-Laufzeit fehlt; keine Installation.')
    with tempfile.TemporaryDirectory(prefix='.editor-install-', dir=destination.parent) as tmp:
        staging = Path(tmp) / 'editor'
        staging.mkdir()
        shutil.copy2(binary, staging / 'openomsi')
        shutil.copy2(steam, staging / 'libsteam_api.so')
        shutil.copytree(source / 'assets', staging / 'assets')
        shutil.copytree(source / 'docs', staging / 'docs')
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
            f'Editor {VERSION}\nopenOMSI base 0.2.27\nCommit {git(source, "rev-parse", "HEAD")}\n'
            'Linux build/test gates passed. Interactive map test still required.\n', encoding='utf-8')
        # rename is on the same filesystem; never overwrite an old editor installation.
        if destination.exists():
            raise ValueError(f'Ziel wurde inzwischen angelegt: {destination}')
        staging.rename(destination)


def main():
    if not sys.platform.startswith('linux') or os.uname().machine != 'x86_64':
        raise ValueError('Dieses Vorbereitungspaket ist für Linux x64.')
    for name in ('git', 'cargo', 'rustc', 'python3'):
        if shutil.which(name) is None:
            raise ValueError(f'{name} fehlt. Bitte die bisherige Build-Umgebung verwenden.')
    package = Path(__file__).resolve().parent
    metadata = json.loads((package / 'candidate.json').read_text(encoding='utf-8'))
    sha = metadata['commit']
    if len(sha) != 40 or any(c not in '0123456789abcdef' for c in sha):
        raise ValueError('Ungültiger Commit im Paket.')
    default = Path.home() / 'Projekte/openOMSI-Editor/openomsi-source-0.2.20-editor-0.7.2-pre'
    if len(sys.argv) > 1:
        source = Path(sys.argv[1]).expanduser().resolve()
    elif default.is_dir():
        source = default.resolve()
    else:
        source = Path(input('Pfad zum bisherigen Quellprojekt: ').strip()).expanduser().resolve()
    if git(source, 'rev-parse', 'HEAD') != BASE:
        raise ValueError('Quellprojekt ist nicht auf 04497112. Nichts zurücksetzen; bitte update-0.7.11-pre.log schicken.')
    if git(source, 'status', '--porcelain'):
        raise ValueError('Im Quellprojekt liegen noch Änderungen. Bitte zuerst sichern/committen; nichts wird überschrieben.')
    work = source.parent / 'openomsi-source-0.2.27-editor-0.7.11-pre'
    destination = source.parent / 'openOMSI-Editor-0.7.11-pre'
    if destination.exists():
        raise ValueError(f'Der neue Editor-Ordner existiert bereits: {destination}')
    bundle = package / 'editor-update.bundle'
    run('git', '-C', source, 'bundle', 'verify', bundle)
    run('git', '-C', source, 'fetch', '--no-tags', bundle, 'refs/heads/' + BRANCH)
    if git(source, 'rev-parse', 'FETCH_HEAD') != sha:
        raise ValueError('Bundle und Versionsangabe passen nicht zusammen.')
    if work.exists():
        if git(work, 'rev-parse', '--show-toplevel') != str(work.resolve()):
            raise ValueError('Der neue Quellordner ist kein eigenes Git-Arbeitsverzeichnis.')
        if git(work, 'rev-parse', 'HEAD') != sha or git(work, 'status', '--porcelain'):
            raise ValueError('Vorhandener neuer Quellordner wurde verändert; er bleibt erhalten.')
    else:
        run('git', '-C', source, 'worktree', 'add', '-b', BRANCH, work, sha)
    env = dict(os.environ, OPENOMSI_VERSION='0.2.27-editor-' + VERSION)
    # Keep output and installation tied to this worktree, even if a shared target was set.
    env['CARGO_TARGET_DIR'] = str(work / 'target')
    run('python3', 'scripts/editor/test_windows_install.py', cwd=work, env=env)
    run('python3', 'scripts/editor/test_candidate_install.py', cwd=work, env=env)
    run('python3', 'scripts/editor/test_editor_i18n.py', cwd=work, env=env)
    run('cargo', 'check', '--workspace', '--locked', cwd=work, env=env)
    run('cargo', 'test', '--workspace', '--locked', '--no-fail-fast',
        '--features', 'omsi-app/standalone-editor', cwd=work, env=env)
    run('cargo', 'build', '--release', '--locked', '-p', 'omsi-app',
        '--features', 'standalone-editor', cwd=work, env=env)
    install(work, destination)
    print('\nFERTIG: Tests, Build und separate Installation erfolgreich.')
    print('Neuer Editor:', destination)
    print('Neuer Quellordner:', work)
    print('Der bisherige Editor, das ursprüngliche Quellprojekt und GitHub bleiben unverändert.')
    print('Bitte erst auf einer Kartenkopie testen. Noch kein Release veröffentlichen.')


if __name__ == '__main__':
    try:
        main()
    except (ValueError, OSError, subprocess.CalledProcessError) as exc:
        print(f'\nAbgebrochen: {exc}\nBitte update-0.7.11-pre.log schicken.', file=sys.stderr)
        sys.exit(1)
