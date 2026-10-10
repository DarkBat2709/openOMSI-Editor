"""Windows x64 build/install helper. No game content is bundled or copied."""
import argparse
import datetime
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys

BASE = '0506141a494ee94a965a1e00843db80a6e265eae'
VERSION = '0.7.21-pre'
BUNDLE = Path(__file__).resolve().parent


def run(*args, cwd=None, env=None):
    subprocess.run([str(a) for a in args], cwd=cwd, env=env, check=True)


def git_ok(source, *args):
    return subprocess.run(['git', '-C', str(source), *map(str, args)],
                          stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL).returncode == 0


def install(binary, game, editor, assets):
    binary, game, editor, assets = [Path(p).resolve() for p in (binary, game, editor, assets)]
    if not binary.is_file() or not (game / 'openomsi.exe').is_file():
        raise ValueError('Built openomsi.exe or official Windows game installation missing.')
    if not assets.is_dir():
        raise ValueError('Matching source assets directory missing.')
    if editor == game or game in editor.parents or editor in game.parents:
        raise ValueError('Choose a separate editor folder beside the official game.')
    if editor == binary.parent or editor in binary.parents or binary.parent in editor.parents:
        raise ValueError('Choose an editor folder separate from the build output.')
    marker = editor / 'EDITOR-INSTALLATION.txt'
    if editor.exists() and any(editor.iterdir()) and not marker.is_file():
        raise ValueError('Destination is not an editor installation; choose an empty folder.')
    editor.mkdir(parents=True, exist_ok=True)
    if (editor / 'assets').is_symlink():
        raise ValueError('Editor assets must not be a link.')
    stamp = datetime.datetime.now().strftime('%Y%m%d-%H%M%S-%f')
    target = editor / 'openomsi.exe'
    if target.exists():
        shutil.copy2(target, editor / ('openomsi.exe.before-editor-' + stamp))
    # Preserve runtime libraries from the matching official Windows x64 install;
    # then prefer libraries supplied by this build.
    for folder in (game, binary.parent):
        for dll in folder.glob('*.dll'):
            shutil.copy2(dll, editor / dll.name)
    steam = assets / 'steam_redist' / 'steam_api64.dll'
    if not steam.is_file():
        raise ValueError('Matching x64 Steam runtime is missing.')
    shutil.copy2(steam, editor / steam.name)
    shutil.copytree(assets, editor / 'assets', dirs_exist_ok=True)
    temporary = editor / ('openomsi-' + stamp + '.tmp')
    try:
        shutil.copy2(binary, temporary)
        os.replace(temporary, target)  # Fails safely if the running program locks it.
    finally:
        temporary.unlink(missing_ok=True)
    (editor / 'editor-config.json').write_text(json.dumps({'content': str(game)}, ensure_ascii=False), encoding='utf-8')
    (editor / 'start-editor.ps1').write_text('''$ErrorActionPreference = 'Stop'
$config = Get-Content -LiteralPath (Join-Path $PSScriptRoot 'editor-config.json') -Raw -Encoding UTF8 | ConvertFrom-Json
$env:OMSI_CONTENT = $config.content
$env:OMSI_NO_UPDATE = '1'
Push-Location -LiteralPath $config.content
try {
    & (Join-Path $PSScriptRoot 'openomsi.exe') @args
    exit $LASTEXITCODE
} finally { Pop-Location }
''', encoding='utf-8')
    (editor / 'start-editor.cmd').write_text('@echo off\r\npowershell.exe -NoProfile -ExecutionPolicy Bypass -File "%~dp0start-editor.ps1" %*\r\n', encoding='ascii')
    marker.write_text(f'openOMSI Spline-Editor {VERSION} / base 0.2.27\nContent: {game}\nStart: start-editor.cmd\n', encoding='utf-8')
    print('Installed:', editor, '\nStart:', editor / 'start-editor.cmd')


def build(source, game=None, editor=None):
    if sys.platform != 'win32':
        raise ValueError('Run this build on Windows x64 with the MSVC Rust toolchain.')
    source = Path(source).resolve()
    for program in ('git', 'cargo', 'rustc'):
        if shutil.which(program) is None:
            raise ValueError(f'{program} is missing from PATH.')
    if not (source / 'Cargo.toml').is_file():
        raise ValueError('Pass --source pointing to the complete 0.7.12 editor checkout.')
    if not git_ok(source, 'merge-base', '--is-ancestor', BASE, 'HEAD'):
        raise ValueError('Source does not contain the required openOMSI 0.2.27 base.')
    ui = source / 'crates/omsi-app/src/ui.rs'
    if not ui.is_file() or f'Object editor {VERSION}' not in ui.read_text(encoding='utf-8'):
        raise ValueError('Source does not contain the expected editor version.')
    env = dict(os.environ, OPENOMSI_VERSION='0.2.27-editor-' + VERSION)
    run('cargo', 'test', '--workspace', '--locked', '--features', 'omsi-app/standalone-editor', cwd=source, env=env)
    run('cargo', 'build', '--release', '--locked', '--target', 'x86_64-pc-windows-msvc',
        '-p', 'omsi-app', '--features', 'standalone-editor', cwd=source, env=env)
    binary = source / 'target/x86_64-pc-windows-msvc/release/openomsi.exe'
    if game:
        game = Path(game).resolve()
        install(binary, game, editor or game.parent / 'openOMSI-Spline-Editor', source / 'assets')
    else:
        print('Built:', binary, '\nInstall using: python windows_editor.py install BINARY GAME EDITOR ASSETS')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest='command', required=True)
    b = sub.add_parser('build')
    b.add_argument('--source', type=Path, default=BUNDLE.parents[1])
    b.add_argument('--game', type=Path)
    b.add_argument('--editor', type=Path)
    i = sub.add_parser('install')
    for name in ('binary', 'game', 'editor', 'assets'):
        i.add_argument(name, type=Path)
    args = vars(parser.parse_args())
    command = args.pop('command')
    if command == 'install' and sys.platform != 'win32':
        parser.error('Installation is for Windows only.')
    try:
        (build if command == 'build' else install)(**args)
    except (ValueError, OSError, subprocess.CalledProcessError) as error:
        parser.exit(1, f'Editor: {error}\n')


if __name__ == '__main__':
    main()
