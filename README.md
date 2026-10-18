# openOMSI Editor — In-game map editor by DarkBat2709

**[English](#english) · [Deutsch](#deutsch)**

Free, experimental community map editor for openOMSI: edit roads, splines, AI traffic paths, terrain and scenery directly in the game. Requires original OMSI 2 content.

Kostenloser, experimenteller Community-Karteneditor für openOMSI: Straßen, Splines, KI-Fahrwege, Gelände und Objekte direkt im Spiel bearbeiten. Originale OMSI-2-Inhalte werden benötigt.

**[Downloads](https://github.com/DarkBat2709/openOMSI-Editor/releases/tag/v0.8.5-pre)** · **[Video](https://github.com/DarkBat2709/openOMSI-Editor/releases/download/v0.8.0-pre/openOMSI-Editor-Vorstellung-DE.mp4)** · **[English guide](https://github.com/DarkBat2709/openOMSI-Editor/releases/download/v0.8.5-pre/openOMSI-Editor-Quick-Guide-EN.pdf)** · **[Deutsche Anleitung](https://github.com/DarkBat2709/openOMSI-Editor/releases/download/v0.8.5-pre/openOMSI-Editor-Kurzanleitung.pdf)**

## English

### What is openOMSI Editor?

**openOMSI Editor by DarkBat2709** is a free community fork of [openOMSI](https://github.com/openOMSI-org/openOMSI) with extended in-game map-building tools. It combines driving and map editing in the same application. It is an independent community project, not an official openOMSI release.

**New in 0.8.5-pre:** Windows startup fix: the portable package now includes the Microsoft DirectX shader compiler. [Update notes](docs/UPDATE_0.8.5.md).

**Current editor release: 0.8.5-pre**, based on **openOMSI 0.2.50**. Information updated: **18 October 2026**.

### What can I build and edit?

- Place, move, rotate, copy and delete scenery objects and road splines.
- Connect roads, create junctions and roundabouts.
- Create and edit AI traffic paths, shape curves and add turning connections.
- Shape and paint terrain; create maps and tiles.
- Adjust road heights and gradients for bridges and tunnels, use a work plane and hide terrain while editing. Decks, walls and portals use catalogue assets.
- Use movable editor windows and German or English controls.
- Edit during a driving session and reload the map with the bus and onboard passengers preserved within the limits described in the guides.

### Does official openOMSI already include an editor?

Yes. The official [object editor](https://github.com/openOMSI-org/openOMSI/blob/main/docs/USER_GUIDE.md#object-editor) supports changes to existing scenery objects. Its documentation excludes splines, terrain and adding new objects. DarkBat2709's openOMSI Editor extends map editing with the tools listed above. The projects share the openOMSI foundation; this fork builds on the work of its contributors.

### Download and requirements

- **[Windows x64 portable ZIP](https://github.com/DarkBat2709/openOMSI-Editor/releases/download/v0.8.5-pre/openOMSI-Editor-0.8.5-pre-windows-x64.zip)** · [SHA256 checksum](https://github.com/DarkBat2709/openOMSI-Editor/releases/download/v0.8.5-pre/openOMSI-Editor-0.8.5-pre-windows-x64.zip.sha256)

For Windows: extract all files to a new writable folder, double-click `start-editor.cmd`, and configure your original OMSI 2 installation in the launcher. No Python, Rust or administrator rights are required. See `START-HERE.txt` in the ZIP. The package is unsigned; settings and logs may be shared with an existing openOMSI installation.

Ready-to-run **0.8.5-pre packages are available for Linux x64 and Windows x64**. The Linux package requires **glibc 2.39 or newer**. It was built on Linux Mint 22.3. An original OMSI 2 installation with its content is required. OMSI 2, third-party maps and mods are not included.

- **[Linux x64 editor package](https://github.com/DarkBat2709/openOMSI-Editor/releases/download/v0.8.5-pre/openOMSI-Editor-0.8.5-pre-linux-x64.tar.gz)**
- **[SHA256 checksum](https://github.com/DarkBat2709/openOMSI-Editor/releases/download/v0.8.5-pre/openOMSI-Editor-0.8.5-pre-linux-x64.tar.gz.sha256)**
- **[English operating guide (PDF)](https://github.com/DarkBat2709/openOMSI-Editor/releases/download/v0.8.5-pre/openOMSI-Editor-Quick-Guide-EN.pdf)**
- **[German operating guide (PDF)](https://github.com/DarkBat2709/openOMSI-Editor/releases/download/v0.8.5-pre/openOMSI-Editor-Kurzanleitung.pdf)**

On Linux, extract the archive to a separate folder and run:

```bash
bash start-editor.sh "/path/to/openOMSI-content"
```

Set up the original OMSI 2 installation as described in the guide. Back up your maps before editing. The editor bundles its own engine; updating a separate openOMSI game installation does not update this editor package. GitHub's automatic “Source code” downloads contain source files, not the ready-to-run editor.

Linux and Windows CI builds and automated tests passed for the released source. The Windows ZIP and starter passed clean-extraction self-tests. Fresh Windows VM installation and gameplay testing are still pending.

### Demonstration

[Watch or download the demonstration video (MP4)](https://github.com/DarkBat2709/openOMSI-Editor/releases/download/v0.8.0-pre/openOMSI-Editor-Vorstellung-DE.mp4): **3:25 minutes, Full HD, German captions, no audio**. It shows passenger boarding, connecting a road gap, editing AI paths, shaping the landscape and continuing the journey.

## Deutsch

### Was ist der openOMSI Editor?

Der **openOMSI Editor von DarkBat2709** ist ein kostenloser Community-Fork von [openOMSI](https://github.com/openOMSI-org/openOMSI) mit erweiterten Werkzeugen für den Kartenbau direkt im Spiel. Busfahren und Kartenbearbeitung finden im selben Programm statt. Es handelt sich um ein eigenständiges Community-Projekt, keine offizielle openOMSI-Veröffentlichung.

**Neu in 0.8.5-pre:** Windows-Startkorrektur: Das portable Paket enthält jetzt Microsofts DirectX-Shader-Compiler. [Änderungen](docs/UPDATE_0.8.5.md).

**Aktuelle Editor-Version: 0.8.5-pre**, auf Basis von **openOMSI 0.2.50**. Stand dieser Informationen: **18. Oktober 2026**.

### Was kann ich bauen und bearbeiten?

- Landschaftsobjekte und Straßensplines platzieren, verschieben, drehen, kopieren und löschen.
- Straßen verbinden sowie Kreuzungen und Kreisverkehre erstellen.
- KI-Fahrwege erstellen und bearbeiten, Kurven formen und Abbiegeverbindungen ergänzen.
- Gelände formen und texturieren sowie Karten und Tiles anlegen.
- Straßenhöhen und Steigungen für Brücken und Tunnel einstellen, eine Arbeitsebene verwenden und das Gelände beim Bearbeiten ausblenden. Bauteile, Wände und Portale stammen aus dem Katalog.
- Verschiebbare Editorfenster und deutsche oder englische Bedienung nutzen.
- Während einer Fahrt bauen und die Karte mit Übernahme des Busses und der Fahrgäste im Bus neu laden; Umfang und Grenzen stehen in den Anleitungen.

### Hat das offizielle openOMSI bereits einen Editor?

Ja. Der offizielle [Objekteditor](https://github.com/openOMSI-org/openOMSI/blob/main/docs/USER_GUIDE.md#object-editor) ermöglicht Änderungen an vorhandenen Landschaftsobjekten. Seine Dokumentation schließt Splines, Gelände und das Hinzufügen neuer Objekte aus. Der openOMSI Editor von DarkBat2709 ergänzt die oben beschriebenen Kartenbauwerkzeuge. Dieser Fork baut auf der Arbeit der openOMSI-Mitwirkenden auf.

### Download und Voraussetzungen

- **[Windows x64: portables ZIP](https://github.com/DarkBat2709/openOMSI-Editor/releases/download/v0.8.5-pre/openOMSI-Editor-0.8.5-pre-windows-x64.zip)** · [SHA256-Prüfsumme](https://github.com/DarkBat2709/openOMSI-Editor/releases/download/v0.8.5-pre/openOMSI-Editor-0.8.5-pre-windows-x64.zip.sha256)

Unter Windows: Alle Dateien in einen neuen beschreibbaren Ordner entpacken, `start-editor.cmd` doppelt anklicken und im Launcher die originale OMSI-2-Installation einrichten. Python, Rust und Administratorrechte sind nicht erforderlich. Siehe `START-HERE.txt` im ZIP. Das Paket ist nicht signiert; Einstellungen und Logs können mit einer vorhandenen openOMSI-Installation geteilt werden.

Fertige Pakete von **0.8.5-pre stehen für Linux x64 und Windows x64 bereit**. Das Linux-Paket benötigt **glibc 2.39 oder neuer**. Es wurde auf Linux Mint 22.3 gebaut. Eine originale OMSI-2-Installation mit ihren Inhalten wird benötigt. OMSI 2, fremde Karten und Mods sind nicht im Paket enthalten.

- **[Linux-x64-Editorpaket](https://github.com/DarkBat2709/openOMSI-Editor/releases/download/v0.8.5-pre/openOMSI-Editor-0.8.5-pre-linux-x64.tar.gz)**
- **[SHA256-Prüfsumme](https://github.com/DarkBat2709/openOMSI-Editor/releases/download/v0.8.5-pre/openOMSI-Editor-0.8.5-pre-linux-x64.tar.gz.sha256)**
- **[Englische Bedienungsanleitung (PDF)](https://github.com/DarkBat2709/openOMSI-Editor/releases/download/v0.8.5-pre/openOMSI-Editor-Quick-Guide-EN.pdf)**
- **[Deutsche Bedienungsanleitung (PDF)](https://github.com/DarkBat2709/openOMSI-Editor/releases/download/v0.8.5-pre/openOMSI-Editor-Kurzanleitung.pdf)**

Unter Linux das Archiv in einen eigenen Ordner entpacken und starten:

```bash
bash start-editor.sh "/Pfad/zum/openOMSI-Inhaltsordner"
```

Die originale OMSI-2-Installation gemäß Anleitung einrichten. Karten vor dem Bearbeiten sichern. Der Editor enthält seine eigene Engine; ein Update einer separaten openOMSI-Spielinstallation aktualisiert das Editorpaket nicht. Die automatisch von GitHub angebotenen „Source code“-Archive enthalten Quellcode und sind nicht das startfertige Editorpaket.

Die automatischen Builds und Tests für Linux und Windows sind für den veröffentlichten Quellstand bestanden. Das Windows-ZIP und die Startdatei haben Selbsttests nach frischem Entpacken bestanden. Tests einer frischen Windows-VM-Installation und des Spielbetriebs stehen noch aus.

### Vorstellungsvideo

[Vorstellung ansehen oder herunterladen (MP4)](https://github.com/DarkBat2709/openOMSI-Editor/releases/download/v0.8.0-pre/openOMSI-Editor-Vorstellung-DE.mp4): **3:25 Minuten, Full HD, deutsche Einblendungen, ohne Ton**. Zu sehen sind das Aufnehmen von Fahrgästen, das Schließen einer Straßenlücke, das Bearbeiten von KI-Fahrwegen und Landschaft sowie die anschließende Weiterfahrt.

## Source code and feedback / Quellcode und Rückmeldungen

- [Windows package source / Quellstand Windows-Paket: 2d684846](https://github.com/DarkBat2709/openOMSI-Editor/tree/2d684846402c10a40e92c727be045c5b63e49545)
- [Original release source / Ursprünglicher Release-Quellstand: v0.8.5-pre](https://github.com/DarkBat2709/openOMSI-Editor/tree/v0.8.5-pre)
- [All releases / Alle Veröffentlichungen](https://github.com/DarkBat2709/openOMSI-Editor/releases)
- [Report a bug / Fehler melden](https://github.com/DarkBat2709/openOMSI-Editor/issues)

For bug reports, include the editor version, steps to reproduce the problem and a relevant log excerpt.
Bitte bei Fehlerberichten die Editor-Version, Schritte zum Nachstellen und einen passenden Logauszug angeben.

[Download and information page / Download- und Informationsseite](https://github.com/DarkBat2709/openOMSI-Editor-Download)

<details>
<summary>Original openOMSI documentation (upstream project, separate downloads)</summary>


<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="assets/logos/openomsi-wordmark-light.svg">
    <img alt="openOMSI" src="assets/logos/openomsi-wordmark-dark.svg" width="420">
  </picture>
</p>

<p align="center">
  <a href="https://github.com/openOMSI-org/openOMSI/releases/latest"><img alt="Version" src="https://img.shields.io/github/v/release/openOMSI-org/openOMSI?label=version&color=f47f30&style=for-the-badge"></a>
  <a href="https://openomsi.org/"><img alt="Playing now" src="https://img.shields.io/endpoint?url=https%3A%2F%2Fopenomsi.savvabestbrother.workers.dev%2Fbadge&label=playing%20now&style=for-the-badge&logo=steam&logoColor=white"></a>
  <a href="https://github.com/openOMSI-org/openOMSI/actions/workflows/release.yml"><img alt="Build" src="https://img.shields.io/github/actions/workflow/status/openOMSI-org/openOMSI/release.yml?branch=main&style=for-the-badge&label=build"></a>
  <a href="https://openomsi.org/"><img alt="Docs" src="https://img.shields.io/badge/docs-website-2d3138?style=for-the-badge"></a>
  <a href="https://discord.gg/VG2EKVafYG"><img alt="Discord" src="https://img.shields.io/badge/discord-join%20us-5865F2?style=for-the-badge&logo=discord&logoColor=white"></a>
  <a href="https://buymeacoffee.com/usonskyyy"><img alt="Buy me a coffee" src="https://img.shields.io/badge/buy%20me%20a%20coffee-support-ffdd00?style=for-the-badge&logo=buymeacoffee&logoColor=black"></a>
  <a href="https://ko-fi.com/usonance"><img alt="Ko-fi" src="https://img.shields.io/badge/ko--fi-support-29abe0?style=for-the-badge&logo=kofi&logoColor=white"></a>
  <a href="LICENSE"><img alt="License" src="https://img.shields.io/github/license/openOMSI-org/openOMSI?style=for-the-badge"></a>
</p>

> [!WARNING]
> **Early release. Expect bugs.** openOMSI is in an early stage of development: things may be
> missing, broken or change between versions. Please report problems in
> [Issues](https://github.com/openOMSI-org/openOMSI/issues) or on our
> [Discord server](https://discord.gg/VG2EKVafYG).

**openOMSI** is a from-scratch recreation of the bus simulator **OMSI 2**, written in Rust:
64-bit, multithreaded, with a modern renderer (Metal / Vulkan / DirectX 12 through wgpu),
and fully compatible with the existing maps, buses, scenery and mods.

> [!IMPORTANT]
> **openOMSI needs an original copy of OMSI 2.** It contains no game content of its own: it
> plays on the maps, vehicles and other files of an installed OMSI 2 and **will not start without one**.


## Download

Every commit to `main` is built by GitHub Actions and published on the
[**Releases**](https://github.com/openOMSI-org/openOMSI/releases) page:

| Platform | File |
| --- | --- |
| Windows x64 / ARM64 | `openOMSI-<version>-windows-x64.zip` / `-windows-arm64.zip` - run `openomsi.exe` |
| macOS (Apple silicon / Intel) | `openOMSI-<version>-macos-arm64.zip` / `-macos-x64.zip` - open `openOMSI.app` |
| Linux x64 / ARM64 | `openOMSI-<version>-linux-x64.zip` / `-linux-arm64.zip` - run `openomsi` |
| Android (arm64, 8.0+) | `openOMSI-<version>-android-arm64.apk` - see [docs/ANDROID.md](docs/ANDROID.md) |
| Dedicated server | `openOMSI-<version>-server-linux-x64.zip` (also `-linux-arm64`, `-windows-x64`, `-windows-arm64`) - see [docs/SERVER.md](docs/SERVER.md) |

Start the game, point the launcher to your OMSI 2 folder once, pick a map, a bus and a duty,
and drive. Mods go into the folder next to the game (or through the launcher's **Mods**
page); the original installation is never written to.

From 0.1.7 on the launcher updates itself: when a newer release is out it asks at the start
and, with your yes, downloads it, replaces the program and starts again (on Android through
the system's installer). Settings → General → Updates switches the check off or installs without
asking.

## Installation

**You need an installed OMSI 2** (Steam or retail, any version) with its stock content -
the maps Grundorf and Berlin-Spandau and the stock buses (MAN SD200/SD202, NL). openOMSI
brings no game content of its own; it plays the original's maps, buses and mods.

1. **Download** the file for your system from
   [Releases](https://github.com/openOMSI-org/openOMSI/releases) (table above) and unpack it
   into a folder of its own that you can write to - your Documents, a games folder, or the
   OMSI 2 folder itself. Not `Program Files`: the launcher could not update itself there.
2. **Start it.**
   * **Windows:** `openomsi.exe`. Windows SmartScreen may warn about an unknown app: *More
     info* → *Run anyway*.
   * **macOS:** open `openOMSI.app`. The first time, macOS may refuse an app from the
     internet: right-click → *Open* → *Open*, or run
     `xattr -dr com.apple.quarantine /path/to/openOMSI.app` once.
   * **Linux:** `./openomsi` (run `chmod +x openomsi` if it does not start). It needs a
     Vulkan or OpenGL driver (Mesa: `mesa-vulkan-drivers`, or your GPU vendor's driver).
   * **Android:** see [docs/ANDROID.md](docs/ANDROID.md) - the OMSI 2 folder is copied onto the
     phone first.
3. **Point it at OMSI 2.** The launcher usually finds the installation by itself (Steam
   libraries, the usual folders). If not, open **Setup** and choose the OMSI 2 folder - the
   one with `Omsi.exe`, `maps` and `Vehicles` in it (the folder, or `Omsi.exe` itself) - and
   press **Save**. The Steam version is under
   `…\Steam\steamapps\common\OMSI 2`.
4. **Drive:** pick a bus, a map and a duty on the **Drive** page and press **Start the duty**.

**Mods** are installed on the **Mods** page (a folder or a `.zip`, or dropped on the window) or
by putting them into the `Mods` folder next to the game; the OMSI 2 folder is never written
to.

### When something goes wrong

* **"The original OMSI 2 was not found"** - choose the folder under Setup (step 3); the
  message says what the chosen folder lacks.
* **The game closes after a few seconds, or "the graphics device was lost"** - update the
  graphics driver (NVIDIA, AMD or Intel's own, not the one Windows installs). On Windows you
  can also switch to DirectX 12: Settings → Graphics → Graphics API (the launcher offers it after such a
  crash).
* **An older graphics card** (no Vulkan): openOMSI falls back to DirectX 12 and then OpenGL by
  itself; Settings → Graphics → Graphics API chooses one.
* **Stuck at a bridge or an invisible wall** on a mod map: Esc → Options → *Collisions with
  objects* switches collisions with the map's objects off (Settings has it too).
* **Multiplayer: you do not meet the others** - both players need the host's map (a map in
  the OMSI 2 folder is not passed on; one from the Mods page is). The joining game switches
  to the host's map by itself and says in the HUD when it is not installed.
* **Keys do not do what you set:** Controls - the page shows which driving keys are in use;
  a key you change there takes effect at once.
* **Anything else:** when the game ends on an error, the launcher shows it with *Copy report*
  and *Report on GitHub*. The logs are in `~/.openomsi` (Windows: `C:\Users\<you>\.openomsi`),
  `game.log` for the last game. Setup → *Export diagnostics* (or `openomsi --export-diagnostics
  <file.zip>`) saves a support package for an issue on this computer: the program, the system,
  the graphics card, driver and settings, the controllers and the last game's map and bus, but
  no folder paths, names, chat, LAN codes or addresses, and of the logs only which events
  happened. Nothing is sent; look inside before you attach it.

## Goals

1. **1:1 behaviour.** Every content format of the original - maps, splines, scenery objects,
   vehicles, scripts, timetables, HOF files, fonts, weather, tickets, situations, plugins -
   loads and behaves exactly as in OMSI 2.2.032. Existing maps and mods work unchanged.
2. **No original code or assets.** Nothing from the original is copied; the formats are
   described in [docs/FORMATS.md](docs/FORMATS.md).
3. **A better engine.** 64-bit address space, streaming and texture loading on worker threads,
   no 2 GB limit, no single-thread stalls, LAN multiplayer and a dedicated server.

## Documentation

The full documentation is on the website: **https://openomsi.org/**. The same
pages live in [`docs/`](docs):

| Document | What is in it |
| --- | --- |
| [User guide](docs/USER_GUIDE.md) | running, controls, launcher, settings, mods, LAN play, debug switches |
| [Virtual reality](docs/VR.md) | OpenXR setup, VR settings and controls on Windows |
| [Android](docs/ANDROID.md) | the mobile version: install, touch controls, building the APK |
| [Modding](docs/MODDING.md) | limits lifted for modders: more interior lights, larger textures, additions OMSI 2 ignores |
| [PBR materials](docs/PBR.md) | normal, roughness, metalness and occlusion maps for mods |
| [Building](docs/BUILDING.md) | building from source on macOS, Windows, Linux and Android |
| [Content formats](docs/FORMATS.md) | every OMSI 2 file format |
| [Architecture](docs/ARCHITECTURE.md) | crates, threading, renderer, roadmap |
| [Routes](docs/ROUTES.md) | how the original runs timetables, chrono, HOF, IBIS |
| [Plugins](docs/PLUGINS.md) | Lua and WebAssembly plugins (the API of every group, examples), compiled `.oop` plugins, OMSI plugin DLLs and the 32-bit plugin host |
| [Development Tools](https://github.com/openOMSI-org/openOMSI-Development-Tools) | the plugin workbench: projects, the API documentation, the `.oop` compiler and signing keys (Windows, Linux, macOS) |
| [Dedicated server](docs/SERVER.md) | hosting a session without a window |
| [Versioning & releases](docs/VERSIONING.md) | the `MAJOR.MINOR.COMMIT` scheme and the CI |
| [Changelog](CHANGELOG.md) | what changed in each version |

## Building from source

```sh
git clone https://github.com/openOMSI-org/openOMSI.git && cd openOMSI
scripts/build-macos.sh        # macOS   → dist/macos/openOMSI.app
scripts\build-windows.cmd     # Windows → dist\windows\openomsi.exe
scripts/build-linux.sh        # Linux   → dist/linux/openomsi
scripts/build-android.sh      # Android → dist/android/openOMSI-<version>.apk
scripts/build-server.sh       # server  → dist/server
```

Needs the latest [Rust stable](https://rustup.rs) and the platform's C toolchain; details in
[docs/BUILDING.md](docs/BUILDING.md).

## Repository layout

```
openOMSI/
├── VERSION            MAJOR.MINOR of the next release (edited by hand)
├── crates/            the engine, one crate per subsystem of the original
│   ├── omsi-app/        the game binary `openomsi` (window, launcher, HUD, server mode)
│   ├── omsi-launcher-core/  launcher data side + `openomsi-launcher` terminal tool
│   ├── omsi-cfg/        text files, code pages, virtual file system, content roots
│   ├── omsi-script/     the OMSI script language (compiler + VM)
│   ├── omsi-o3d/ omsi-model/ omsi-texture/ omsi-geometry/   meshes, models, textures, splines
│   ├── omsi-map/ omsi-scenery/ omsi-timetable/ omsi-vehicle/ omsi-content/   content formats
│   ├── omsi-sim/        vehicles, AI traffic, people, physics
│   ├── omsi-render/     the wgpu renderer
│   ├── omsi-audio/ omsi-net/ omsi-plugin/ omsi-ui/   sound, multiplayer, plugins, UI toolkit
├── tools/             developer tools: omsi-check (format coverage)
├── scripts/           build scripts for every platform, version.sh, packaging files
├── assets/            fonts, Material icons, app icons (assets/icons/app), logos (assets/logos)
├── docs/              documentation (also published as the website)
├── site/              the GitHub Pages website
└── .github/workflows/ CI: release builds for every commit, the website
```

## Contributing

Issues and pull requests are welcome - see [CONTRIBUTING.md](CONTRIBUTING.md).

## Support

openOMSI is made in free time. If you enjoy it and want to help it along, you can buy me a
coffee or support it on Ko-fi - thank you!

<p>
  <a href="https://buymeacoffee.com/usonskyyy"><img alt="Buy me a coffee" src="https://cdn.buymeacoffee.com/buttons/v2/default-yellow.png" height="48"></a>
  <a href="https://ko-fi.com/usonance"><img alt="Support me on Ko-fi" src="https://storage.ko-fi.com/cdn/kofi2.png?v=3" height="48"></a>
</p>

## License

openOMSI is released under the [MIT License](LICENSE). OMSI and OMSI 2 are trademarks of their
respective owners. openOMSI is an independent project and is not affiliated with them.

</details>
