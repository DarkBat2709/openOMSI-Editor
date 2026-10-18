"""Create a self-contained Windows x64 editor ZIP from a validated MSVC build."""
import argparse
import hashlib
from pathlib import Path
import shutil
import tempfile
import zipfile

VERSION = '0.8.7-pre'
NAME = f'openOMSI-Editor-{VERSION}-windows-x64'
RUNTIME = ('vcruntime140.dll', 'vcruntime140_1.dll', 'msvcp140.dll')
DXC_FILES = ('dxcompiler.dll', 'dxil.dll', 'LICENCE-MIT.txt', 'LICENSE-LLVM.txt', 'LICENSE-MS.txt')
GUIDES = ('openOMSI-Editor-Kurzanleitung.pdf', 'openOMSI-Editor-Quick-Guide-EN.pdf')
START = r'''@echo off
setlocal
cd /d "%~dp0"
set "OMSI_NO_UPDATE=1"
set "OMSI_CONTENT=%~dp0Content"
if not exist "%OMSI_CONTENT%" mkdir "%OMSI_CONTENT%"
if not exist "%OMSI_CONTENT%" (
  echo Cannot create Content folder. Extract the ZIP into a writable folder first.
  pause
  exit /b 1
)
"%~dp0openomsi.exe" %*
if errorlevel 1 (
  echo Editor could not start. See START-HERE.txt and the log in %%USERPROFILE%%\.openomsi\game.log
  pause
  exit /b 1
)
'''
HELP = '''openOMSI Editor - 0.8.7-pre - Windows x64

ENGLISH
1. Right-click the downloaded ZIP and choose Extract All. Do not run inside the ZIP.
2. Choose a new writable folder, for example Documents\\openOMSI-Editor.
   Do not extract into OMSI 2, another openOMSI installation or Program Files.
3. Double-click start-editor.cmd. Microsoft Edge WebView2 Runtime is required for the launcher.
   Windows 10/11 usually include it. Python and Rust are not required.
4. In the launcher setup, select your original OMSI 2 installation.
   Original OMSI 2 content is required and is not included in this download.
5. Read docs\\openOMSI-Editor-Quick-Guide-EN.pdf before editing; back up maps.

The supplied start file uses this package's Content folder for add-ons and map changes.
Launcher settings and logs use the existing user profile and may be shared with openOMSI.
The editor has its own bundled engine. Keep separate game and editor installations.
To update this preview, extract a new version into a new folder. Back up and transfer
needed Content files separately. Do not overwrite a running installation.
This portable package creates no installer registration or desktop shortcut.

Troubleshooting: keep all DLLs beside openomsi.exe; extract the whole ZIP again if one
is missing. Do not download individual DLLs from third-party DLL websites.
A warning about an unknown publisher is possible; this preview is not code-signed.
Download only from the project release and compare the supplied SHA256 checksum.
Windows 11 VM: DirectX 12 software-renderer startup, Grundorf display, camera use and normal exit tested. Full gameplay and affected physical GPUs still need confirmation.

DEUTSCH
1. Rechtsklick auf die ZIP-Datei, dann Alle extrahieren. Nicht direkt im ZIP starten.
2. Einen neuen beschreibbaren Ordner verwenden, z.B. Dokumente\\openOMSI-Editor.
   Nicht in OMSI 2, eine andere openOMSI-Installation oder Programme entpacken.
3. start-editor.cmd doppelt anklicken. Der Launcher benoetigt Microsoft Edge WebView2
   Runtime (meist in Windows 10/11 enthalten). Python und Rust sind nicht erforderlich.
4. Im Launcher unter Einrichtung die originale OMSI-2-Installation auswaehlen.
   Deren Spielinhalte werden benoetigt und sind nicht im Download enthalten.
5. docs\\openOMSI-Editor-Kurzanleitung.pdf lesen und Karten vor Aenderungen sichern.

Die Startdatei nutzt den Content-Ordner dieses Pakets fuer Zusatzinhalte und Karten.
Launcher-Einstellungen und Logs liegen im vorhandenen Benutzerprofil und koennen
mit openOMSI geteilt werden. Spiel und Editor in getrennten Ordnern aufbewahren.
Updates dieser Vorabversion in einen neuen Ordner entpacken. Benoetigte Content-Dateien
vorher sichern und getrennt uebernehmen. Keine laufende Installation ueberschreiben.
Das portable Paket legt keinen Deinstallationseintrag und keine Desktopverknuepfung an.

Bei fehlenden DLLs das komplette ZIP erneut entpacken. Alle DLLs gehoeren neben die EXE.
Keine einzelnen DLLs von fremden Downloadseiten beziehen. Diese Vorabversion ist nicht
signiert; eine Meldung ueber einen unbekannten Herausgeber ist deshalb moeglich.
Nur vom Projekt-Release herunterladen und die SHA256-Pruefsumme vergleichen.
Windows-11-VM: DirectX-12-Softwarestart, Grundorf-Anzeige, Kamera und normales Beenden getestet. Vollstaendiger Spieltest und Bestaetigung auf betroffenen Grafikkarten stehen aus.
'''


def package(source, binary, runtime, output, revision, dxc):
    source, binary, runtime, output, dxc = map(Path, (source, binary, runtime, output, dxc))
    launcher = binary.with_name('openomsi-launcher-app.exe')
    required = [binary, launcher, source/'LICENSE', source/'assets/steam_redist/steam_api64.dll']
    required += [runtime/name for name in RUNTIME]
    required += [dxc/name for name in DXC_FILES]
    required += [source/'docs'/name for name in GUIDES]
    for path in required:
        if not path.is_file():
            raise ValueError(f'Missing package dependency: {path}')
    output.mkdir(parents=True, exist_ok=True)
    archive = output/(NAME+'.zip')
    checksum = output/(NAME+'.zip.sha256')
    if archive.exists() or checksum.exists():
        raise ValueError('Package already exists; use a new output directory.')
    with tempfile.TemporaryDirectory(prefix='editor-windows-package-') as temp:
        root = Path(temp)/NAME
        root.mkdir()
        shutil.copy2(binary, root/'openomsi.exe')
        shutil.copy2(launcher, root/launcher.name)
        for name in DXC_FILES:
            shutil.copy2(dxc/name, root/name)
        for name in RUNTIME:
            shutil.copy2(runtime/name, root/name)
        shutil.copy2(source/'assets/steam_redist/steam_api64.dll', root/'steam_api64.dll')
        shutil.copytree(source/'assets', root/'assets')
        (root/'docs').mkdir()
        for name in GUIDES:
            shutil.copy2(source/'docs'/name, root/'docs'/name)
        shutil.copy2(source/'LICENSE', root/'LICENSE')
        (root/'start-editor.cmd').write_bytes(START.replace('\n', '\r\n').encode('ascii'))
        (root/'START-HERE.txt').write_bytes(HELP.replace('\n', '\r\n').encode('ascii'))
        (root/'VERSION-EDITOR.txt').write_text(
            f'Editor {VERSION}\nopenOMSI-Basis 0.3.33\nQuellstand: {revision}\nWindows x64 portable preview\n', encoding='utf-8')
        with zipfile.ZipFile(archive, 'x', zipfile.ZIP_DEFLATED) as zipout:
            for p in sorted(root.rglob('*')):
                if p.is_file():
                    zipout.write(p, p.relative_to(root.parent).as_posix())
    checksum.write_text(hashlib.sha256(archive.read_bytes()).hexdigest()+'  '+archive.name+'\n', encoding='ascii')
    print(archive)
    return archive


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    for arg in ('source', 'binary', 'runtime', 'output', 'revision', 'dxc'):
        parser.add_argument('--'+arg, required=True)
    package(**vars(parser.parse_args()))
