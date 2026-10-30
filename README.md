# openOMSI Editor — In-game map editor by DarkBat2709

**[English](#english) · [Deutsch](#deutsch)**

Free, experimental community map editor for openOMSI: edit roads, splines, AI traffic paths, terrain and scenery directly in the game. Requires original OMSI 2 content.

Kostenloser, experimenteller Community-Karteneditor für openOMSI: Straßen, Splines, KI-Fahrwege, Gelände und Objekte direkt im Spiel bearbeiten. Originale OMSI-2-Inhalte werden benötigt.

**[Downloads](https://github.com/DarkBat2709/openOMSI-Editor/releases/tag/v0.8.9-pre)** · **[Video](https://github.com/DarkBat2709/openOMSI-Editor/releases/download/v0.8.0-pre/openOMSI-Editor-Vorstellung-DE.mp4)** · **[English guide](https://github.com/DarkBat2709/openOMSI-Editor/releases/download/v0.8.9-pre/openOMSI-Editor-Quick-Guide-EN.pdf)** · **[Deutsche Anleitung](https://github.com/DarkBat2709/openOMSI-Editor/releases/download/v0.8.9-pre/openOMSI-Editor-Kurzanleitung.pdf)**

## English

### What is openOMSI Editor?

**openOMSI Editor by DarkBat2709** is a free community fork of [openOMSI](https://github.com/openOMSI-org/openOMSI) with extended in-game map-building tools. It combines driving and map editing in the same application. It is an independent community project, not an official openOMSI release.

**Current editor release: 0.8.9-pre**, based on **openOMSI 0.3.45**. Information updated: **30 October 2026**.

### New in 0.8.9-pre

- Updated engine base from openOMSI 0.3.33 to 0.3.45.
- Place, name and configure bus stops directly in object mode.
- Place, orient and name entry points; save them in the map's entry-point list.
- New Routes and lines dialog: choose stops in travel order, enter a line, unique route name, departure and duration, and optionally select an existing AI group by name.
- Save native timetable data while preserving existing duties. Routes follow connected, directed AI paths; missing connections are rejected.
- Updated German and English PDF guides with current launcher/editor screenshots and two new chapters, 41 pages each.

Save and reload after placing stops or changing roads, and reload after saving a route. Return trips require their own route. HOF/IBIS data and announcements are not generated automatically. Editing existing routes, alternative waypoints and automatic full-day schedules are not included in this first route editor. See [route instructions](docs/PORTIERUNG_0.3.45.md).

### Hotfix 0.8.8.1

- Improved exit planning for detected hand-drawn roundabouts, reducing repeated laps.
- Random traffic prefers other reachable exits over returning to its incoming approach; occasional returns remain possible.
- Deleting a roundabout immediately removes its owned AI paths from editor display and selection. Save and reload to update traffic simulation.
- Updater support for four-part hotfix versions; automatic path generation is unchanged.

**Updating from 0.8.8-pre:** Download this hotfix manually and extract it into a new folder; the older updater does not recognize four-part versions. The included 0.8.8-pre PDF guides remain applicable.

### Features introduced in 0.8.8-pre

- Movable spline dimensions window for length, radius, curve angle, gradient and absolute start height.
- Live mouse preview before placement, updated as dimensions change; camera movement remains available outside text inputs.
- Edit existing splines numerically and with ordinary tools across existing tile boundaries.
- Four-step roundabout builder: Circle, Approaches, Islands and Finish.
- Per-approach splitter islands with configurable shape, length, width, ring distance and height, including rounded, teardrop and rounded-wedge shapes.
- Concrete kerbs for central and splitter islands; island-aware paths and markings.
- Independently configurable 1–4 ring lanes and approach lanes, with matching AI paths.
- Improved roundabout endpoint connectivity, complete driving curves, exit planning, lookahead and conflict detection to reduce unnecessary repeated laps.
- Free AI path drawing as a chain of sections, with three form handles plus start/end per section; existing endpoint connection mode remains available.
- Editable connection preview, remove-last-point and clear-preview controls. Add before saving, then reload to update traffic.
- Fixed reloading of deleted and reshaped grouped roundabout paths: deleted curves no longer return as short fragments.
- Updated German and English PDF guides, 39 pages each, including new screenshots and the original introduction.

Engine base is **openOMSI 0.3.45**. Map/aerial-image overlays and automatically generated crossings, signs and traffic lights are not included. AI connections do not generate traffic-light control. Back up maps before updating; save, reload and check traffic after editing.

This project uses the AGPL-3.0-only licence. Exact source and build instructions are linked below.

### Building tools introduced in 0.8.4-pre

Build asphalt and separate markings together, fit a marking to a complete selected road chain, and support terrain across the full road width. Roundabouts now offer outer edge lines, dashed entrance openings and adjustable line insets. Both German and English PDF guides include updated screenshots and instructions. These tools remain included in 0.8.8.1.

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

- **[Windows x64 portable ZIP](https://github.com/DarkBat2709/openOMSI-Editor/releases/download/v0.8.9-pre/openOMSI-Editor-0.8.9-pre-windows-x64.zip)** · [SHA256 checksum](https://github.com/DarkBat2709/openOMSI-Editor/releases/download/v0.8.9-pre/openOMSI-Editor-0.8.9-pre-windows-x64.zip.sha256)

For Windows: extract all files to a new writable folder, double-click `start-editor.cmd`, and configure your original OMSI 2 installation in the launcher. No Python, Rust or administrator rights are required. See `START-HERE.txt` in the ZIP. The package is unsigned; settings and logs may be shared with an existing openOMSI installation.

Ready-to-run **0.8.9-pre packages are available for Linux x64 and Windows x64**. The Linux package requires **glibc 2.39 or newer**. It was built on Linux Mint 22.3. An original OMSI 2 installation with its content is required. OMSI 2, third-party maps and mods are not included.

- **[Linux x64 editor package](https://github.com/DarkBat2709/openOMSI-Editor/releases/download/v0.8.9-pre/openOMSI-Editor-0.8.9-pre-linux-x64.tar.gz)**
- **[SHA256 checksum](https://github.com/DarkBat2709/openOMSI-Editor/releases/download/v0.8.9-pre/openOMSI-Editor-0.8.9-pre-linux-x64.tar.gz.sha256)**
- **[English operating guide (PDF)](https://github.com/DarkBat2709/openOMSI-Editor/releases/download/v0.8.9-pre/openOMSI-Editor-Quick-Guide-EN.pdf)**
- **[German operating guide (PDF)](https://github.com/DarkBat2709/openOMSI-Editor/releases/download/v0.8.9-pre/openOMSI-Editor-Kurzanleitung.pdf)**

On Linux, extract the archive to a separate folder and run:

```bash
bash start-editor.sh "/path/to/openOMSI-content"
```

Set up the original OMSI 2 installation as described in the guide. Back up your maps before editing. The editor bundles its own engine; updating a separate openOMSI game installation does not update this editor package. GitHub's automatic “Source code” downloads contain source files, not the ready-to-run editor.

Linux and Windows CI builds and automated tests passed for the released source. The Windows ZIP and starter passed clean-extraction self-tests. Fresh Windows VM installation and gameplay testing are still pending.

### Demonstration

The video shows version 0.8.0-pre. Version 0.8.8.1 groups the controls into separate toolboxes; the updated PDF guides show the current layout.

[Watch or download the demonstration video (MP4)](https://github.com/DarkBat2709/openOMSI-Editor/releases/download/v0.8.0-pre/openOMSI-Editor-Vorstellung-DE.mp4): **3:25 minutes, Full HD, German captions, no audio**. It shows passenger boarding, connecting a road gap, editing AI paths, shaping the landscape and continuing the journey.

## Deutsch

### Was ist der openOMSI Editor?

Der **openOMSI Editor von DarkBat2709** ist ein kostenloser Community-Fork von [openOMSI](https://github.com/openOMSI-org/openOMSI) mit erweiterten Werkzeugen für den Kartenbau direkt im Spiel. Busfahren und Kartenbearbeitung finden im selben Programm statt. Es handelt sich um ein eigenständiges Community-Projekt, keine offizielle openOMSI-Veröffentlichung.

**Aktuelle Editor-Version: 0.8.9-pre**, auf Basis von **openOMSI 0.3.45**. Stand dieser Informationen: **30. Oktober 2026**.

### Neu in 0.8.9-pre

- openOMSI-Basis von 0.3.33 auf 0.3.45 aktualisiert.
- Bushaltestellen direkt im Objektmodus platzieren, benennen und konfigurieren.
- Startpunkte platzieren, ausrichten und benennen; Übernahme in die Startpunktliste beim Speichern.
- Neuer Dialog Routen und Linien: Haltestellen in Fahrtrichtung wählen, Linie, eindeutigen Routennamen, Abfahrt und Fahrtdauer eingeben; optional eine vorhandene KI-Gruppe mit ihrem Namen zuordnen.
- Native Fahrplandaten speichern; vorhandene Umläufe bleiben erhalten. Routen folgen verbundenen, gerichteten KI-Fahrwegen. Fehlende Verbindungen werden abgewiesen.
- Deutsche und englische PDF-Anleitungen mit aktuellen Launcher-/Editorbildern und zwei neuen Kapiteln, jeweils 41 Seiten.

Nach dem Platzieren von Haltestellen oder Straßenänderungen speichern und neu laden; nach dem Speichern einer Route erneut laden. Die Gegenrichtung als eigene Route anlegen. HOF-/IBIS-Daten und Ansagen werden nicht automatisch erstellt. Bearbeitung bestehender Routen, alternative Zwischenpunkte und automatische Tagesfahrpläne sind noch nicht enthalten. Siehe [Bedienung und Grenzen](docs/PORTIERUNG_0.3.45.md).

### Hotfix 0.8.8.1

- Verbesserte Ausfahrtsplanung für erkannte, frei gezeichnete Kreisverkehr-Fahrwege; weniger zusätzliche Runden.
- Zufälliger KI-Verkehr bevorzugt andere erreichbare Ausfahrten gegenüber der Rückfahrt zur eigenen Zufahrt. Einzelne Rückfahrten bleiben möglich.
- Beim Löschen eines Kreisverkehrs verschwinden seine zugehörigen KI-Fahrwege sofort aus Editoranzeige und Auswahl. Speichern und Neuladen aktualisiert die Verkehrssimulation.
- Updater unterstützt vierteilige Hotfix-Versionen. Die automatische Fahrwegerzeugung bleibt unverändert.

**Update von 0.8.8-pre:** Diesen Hotfix manuell herunterladen und in einen neuen Ordner entpacken; der bisherige Updater erkennt vierteilige Versionen noch nicht. Die enthaltenen PDF-Anleitungen für 0.8.8-pre gelten weiterhin.

### Neuerungen aus 0.8.8-pre

- **Spline-Maße:** Neues verschiebbares Fenster für Länge, Radius, Kurvenwinkel, Steigung und absolute Starthöhe.
- **Spline-Vorschau:** Neue Splines vor dem Platzieren an der Maus ansehen. Geänderte Maße aktualisieren die Vorschau; ein Klick setzt den Spline. Kamerabewegung bleibt außerhalb von Texteingaben verfügbar.
- **Vorhandene Splines bearbeiten:** Maße direkt ändern und danach mit den normalen Werkzeugen weiterarbeiten. Platzieren und Bearbeiten funktionieren auch über vorhandene Kachelgrenzen hinweg.
- **Kreisverkehr-Baukasten:** Bedienung in vier zusammenhängenden Schritten: Kreisform, Zufahrten, Inseln und Ausstattung.
- **Verkehrsinseln je Zufahrt:** Einzeln ein-/ausschalten; Form, Länge, Breite, Abstand zum Ring und Höhe einstellen. Abgerundete Inseln, Tropfen und abgerundete Keile mit Vorlageneinstellung.
- **Bordsteine und Markierungen:** Betonbordsteine an Kreis- und Verkehrsinseln; Fahrwege und Markierungen berücksichtigen die Inselgeometrie.
- **Mehrspurige Kreisverkehre:** 1 bis 4 Spuren auf der Kreisfahrbahn und separat einstellbare Spuranzahl je Zufahrt. KI-Wege passen sich an.
- **KI-Verkehr im Kreisverkehr:** Genauer verbundene Fahrwegenden, zusammenhängende Fahrkurven und geplante Ausfahrten verbessern Fahrverhalten, Vorausschau und Konflikterkennung. Unnötiges mehrfaches Kreisen wurde reduziert.
- **Freie KI-Verbindungen:** Neue Wege abschnittsweise auf der geladenen Fläche zeichnen, ohne zuerst zwei bestehende Anschlüsse wählen zu müssen. Jeder Abschnitt hat drei Formpunkte zusätzlich zu Start und Ende.
- **Verbindungsvorschau:** Formpunkte und Endpunkte ziehen, letzten Abschnitt entfernen oder Vorschau leeren. Erst Hinzufügen übernimmt die Verbindung; anschließend speichern und neu laden.
- **Speichern und Neuladen korrigiert:** Gelöschte Kreisverkehr-Fahrkurven erscheinen nicht mehr als kurze Reststücke. Gespeicherte Formänderungen werden auf die vollständige Fahrkurve angewendet.
- **Anleitungen:** Deutsche und englische PDF mit aktuellen Bildern, neuen KI-Anleitungen, geordneten Kapiteln und wiederhergestellter ursprünglicher Einleitung; jeweils 39 Seiten.

Die openOMSI-Basis ist **0.3.45**. Landkarten-/Luftbild-Unterlagen sowie automatisch erzeugte Fußgängerüberwege, Schilder und Ampeln sind noch nicht enthalten. KI-Verbindungen erzeugen keine automatische Ampelsteuerung. Karten vor dem Update sichern und nach Änderungen speichern, neu laden und den Verkehr prüfen.

Das Projekt verwendet die AGPL-3.0-only-Lizenz. Quellstand und Bauanleitung sind unten verlinkt.

### Bauwerkzeuge aus 0.8.4-pre

Asphalt und separate Markierungen gemeinsam formen und weiterbauen; Markierungen an ganze ausgewählte Straßenketten anpassen; Gelände unter der vollständigen Straßenbreite stützen. Kreisverkehre bieten Außenrandlinien, gestrichelte Zufahrten und einstellbare Linienabstände. Beide PDF-Anleitungen enthalten neue Bilder und aktuelle Bedienhinweise. Diese Werkzeuge bleiben in 0.8.8.1 enthalten.

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

- **[Windows x64: portables ZIP](https://github.com/DarkBat2709/openOMSI-Editor/releases/download/v0.8.9-pre/openOMSI-Editor-0.8.9-pre-windows-x64.zip)** · [SHA256-Prüfsumme](https://github.com/DarkBat2709/openOMSI-Editor/releases/download/v0.8.9-pre/openOMSI-Editor-0.8.9-pre-windows-x64.zip.sha256)

Unter Windows: Alle Dateien in einen neuen beschreibbaren Ordner entpacken, `start-editor.cmd` doppelt anklicken und im Launcher die originale OMSI-2-Installation einrichten. Python, Rust und Administratorrechte sind nicht erforderlich. Siehe `START-HERE.txt` im ZIP. Das Paket ist nicht signiert; Einstellungen und Logs können mit einer vorhandenen openOMSI-Installation geteilt werden.

Fertige Pakete von **0.8.9-pre stehen für Linux x64 und Windows x64 bereit**. Das Linux-Paket benötigt **glibc 2.39 oder neuer**. Es wurde auf Linux Mint 22.3 gebaut. Eine originale OMSI-2-Installation mit ihren Inhalten wird benötigt. OMSI 2, fremde Karten und Mods sind nicht im Paket enthalten.

- **[Linux-x64-Editorpaket](https://github.com/DarkBat2709/openOMSI-Editor/releases/download/v0.8.9-pre/openOMSI-Editor-0.8.9-pre-linux-x64.tar.gz)**
- **[SHA256-Prüfsumme](https://github.com/DarkBat2709/openOMSI-Editor/releases/download/v0.8.9-pre/openOMSI-Editor-0.8.9-pre-linux-x64.tar.gz.sha256)**
- **[Englische Bedienungsanleitung (PDF)](https://github.com/DarkBat2709/openOMSI-Editor/releases/download/v0.8.9-pre/openOMSI-Editor-Quick-Guide-EN.pdf)**
- **[Deutsche Bedienungsanleitung (PDF)](https://github.com/DarkBat2709/openOMSI-Editor/releases/download/v0.8.9-pre/openOMSI-Editor-Kurzanleitung.pdf)**

Unter Linux das Archiv in einen eigenen Ordner entpacken und starten:

```bash
bash start-editor.sh "/Pfad/zum/openOMSI-Inhaltsordner"
```

Die originale OMSI-2-Installation gemäß Anleitung einrichten. Karten vor dem Bearbeiten sichern. Der Editor enthält seine eigene Engine; ein Update einer separaten openOMSI-Spielinstallation aktualisiert das Editorpaket nicht. Die automatisch von GitHub angebotenen „Source code“-Archive enthalten Quellcode und sind nicht das startfertige Editorpaket.

Die automatischen Builds und Tests für Linux und Windows sind für den veröffentlichten Quellstand bestanden. Das Windows-ZIP und die Startdatei haben Selbsttests nach frischem Entpacken bestanden. Tests einer frischen Windows-VM-Installation und des Spielbetriebs stehen noch aus.

### Vorstellungsvideo

Das Video zeigt Version 0.8.0-pre. In 0.8.8.1 sind die Bedienelemente in getrennten Werkzeugboxen angeordnet; die aktualisierten PDF-Anleitungen zeigen die aktuelle Oberfläche.

[Vorstellung ansehen oder herunterladen (MP4)](https://github.com/DarkBat2709/openOMSI-Editor/releases/download/v0.8.0-pre/openOMSI-Editor-Vorstellung-DE.mp4): **3:25 Minuten, Full HD, deutsche Einblendungen, ohne Ton**. Zu sehen sind das Aufnehmen von Fahrgästen, das Schließen einer Straßenlücke, das Bearbeiten von KI-Fahrwegen und Landschaft sowie die anschließende Weiterfahrt.

## Source code and feedback / Quellcode und Rückmeldungen

- [Release source / Quellstand beider Pakete: v0.8.9-pre](https://github.com/DarkBat2709/openOMSI-Editor/tree/v0.8.9-pre)
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
  <a href="https://discord.gg/openomsi"><img alt="Discord" src="https://img.shields.io/badge/discord-join%20us-5865F2?style=for-the-badge&logo=discord&logoColor=white"></a>
  <a href="https://buymeacoffee.com/usonskyyy"><img alt="Buy me a coffee" src="https://img.shields.io/badge/buy%20me%20a%20coffee-support-ffdd00?style=for-the-badge&logo=buymeacoffee&logoColor=black"></a>
  <a href="https://ko-fi.com/usonance"><img alt="Ko-fi" src="https://img.shields.io/badge/ko--fi-support-29abe0?style=for-the-badge&logo=kofi&logoColor=white"></a>
  <a href="LICENSE"><img alt="License" src="https://img.shields.io/github/license/openOMSI-org/openOMSI?style=for-the-badge"></a>
</p>

> [!WARNING]
> **Early development. Expect bugs.** Things may be missing, broken or change between
> versions. Please report problems in [Issues](https://github.com/openOMSI-org/openOMSI/issues)
> or on our [Discord server](https://discord.gg/openomsi).

**openOMSI** is a from-scratch recreation of the bus simulator **OMSI 2**, written in Rust:
64-bit, multithreaded, with a modern renderer (Metal / Vulkan / DirectX 12 through wgpu).
It loads the existing OMSI 2 maps, buses, scenery and mods as they are.

> [!IMPORTANT]
> **openOMSI needs an installed copy of OMSI 2.** It contains no game content of its own: it
> plays on the maps, vehicles and other files of your OMSI 2 and **will not start without one**.

## What you get

* **Your OMSI 2 content, unchanged** - maps, buses, AI traffic, scenery, scripts, timetables,
  HOF files, weather and plugins load the way OMSI 2.2.032 loads them. The OMSI 2 folder is
  never written to.
* **A new engine** - 64-bit (no 2 GB limit), tiles and textures loaded on worker threads,
  four graphics modes from *Vanilla* (looks like OMSI 2) to *Enhanced+* (physically based
  lighting with ray-traced shadows and reflections).
* **A launcher** - pick a map, line, tour, bus and livery on one screen with a route map and a
  live bus preview; mods from a folder or a `.zip`/`.7z`/`.rar`; keys and game controllers
  (wheels, pedals, force feedback); updates on a Stable or Beta channel.
* **Multiplayer** - LAN sessions and a dedicated server ([docs/SERVER.md](docs/SERVER.md)).
* **Every platform** - Windows, macOS, Linux and Android; VR through OpenXR on Windows
  ([docs/VR.md](docs/VR.md)).
* **More for modders** - PBR materials, larger textures, more lights, and plugins in Lua or
  compiled `.oop` (Lua or WebAssembly), plus OMSI's own plugin DLLs on Windows.

## Download

The newest stable version is always at
[**Releases → Latest**](https://github.com/openOMSI-org/openOMSI/releases/latest) and on the
[website](https://openomsi.org/#/download). Newer, untested builds are published as
*Pre-release* (beta) on the [Releases](https://github.com/openOMSI-org/openOMSI/releases) page.

| Platform | File |
| --- | --- |
| Windows x64 / ARM64 | `openOMSI-<version>-windows-x64-setup.exe` / `-windows-arm64-setup.exe` - installs openOMSI for you, no administrator rights (or the `.zip`: run `openomsi.exe`) |
| macOS (Apple silicon / Intel), macOS 11+ | `openOMSI-<version>-macos-arm64.dmg` / `-macos-x64.dmg` - drag `openOMSI.app` onto Applications (or the `.zip`) |
| Linux x64 / ARM64 | `openOMSI-<version>-linux-x64.zip` / `-linux-arm64.zip` - run `openomsi` |
| Android 8.0+ (arm64) | `openOMSI-<version>-android-arm64.apk` - see [docs/ANDROID.md](docs/ANDROID.md) |
| Dedicated server | `openOMSI-<version>-server-<linux\|windows>-<x64\|arm64>.zip` - see [docs/SERVER.md](docs/SERVER.md) |

`SHA256SUMS` in each release lists the checksums of all files.

The launcher keeps itself up to date: when a newer version is out it offers it at the start
and installs it with your yes. **Settings → General → Updates** chooses the channel (Stable or
Beta), turns the check off or installs without asking.

## Installation

You need **OMSI 2** installed (Steam or retail) with its stock content.

1. **Install** it. On Windows the `-setup.exe` installs openOMSI into your user folder
   (`%LOCALAPPDATA%\Programs\openOMSI`) with a Start menu entry; on macOS open the `.dmg` and
   drag `openOMSI.app` onto Applications. The `.zip` (Linux, or the portable Windows and macOS
   version) goes into a folder of its own that you can write to (your Documents, a games
   folder, or the OMSI 2 folder itself) - not `Program Files`, where the launcher cannot
   update itself.
2. **Start it.**
   * **Windows:** openOMSI in the Start menu (or `openomsi.exe`). SmartScreen may warn about
     an unknown app: *More info* → *Run anyway*.
   * **macOS:** `openOMSI.app`. If macOS refuses an app from the internet: right-click →
     *Open* → *Open*, or run `xattr -dr com.apple.quarantine /path/to/openOMSI.app` once.
   * **Linux:** `./openomsi` (`chmod +x openomsi` first if needed). It needs a Vulkan or
     OpenGL driver (Mesa: `mesa-vulkan-drivers`, or your GPU vendor's driver), and
     **WebKitGTK 4.1** for the launcher window (`libwebkit2gtk-4.1-0` on Debian/Ubuntu,
     `webkit2gtk4.1` on Fedora, `webkit2gtk-4.1` on Arch; for the Steam Deck see the [user
     guide](docs/USER_GUIDE.md#the-launcher)).
   * **Android:** see [docs/ANDROID.md](docs/ANDROID.md) - the OMSI 2 folder is copied onto
     the device first.
3. **Find OMSI 2.** The launcher usually finds the installation by itself (Steam libraries and
   the usual folders). If not, open **Setup**, press **Browse…** beside *OMSI 2 folder*, pick
   the folder with `Omsi.exe`, `maps` and `Vehicles` in it (Steam:
   `…\Steam\steamapps\common\OMSI 2`) and press **Save**.
4. **Drive:** choose a map, line, tour and bus on the **Drive** page and press **Start the duty**.

**Mods** are installed on the **Mods** page (a folder or an archive, or dropped onto the
window) or put into the `Mods` folder next to the game.

### When something goes wrong

* **"OMSI 2 was not found"** - choose the folder under **Setup** (step 3); when it is not
  saved, the message says what the chosen folder lacks.
* **The game closes after a few seconds, or "the graphics device was lost"** - update the
  graphics driver (from NVIDIA, AMD or Intel, not the one Windows installs). On Windows,
  **Settings → Graphics → Graphics API** can switch to DirectX 12, OpenGL or ANGLE (DirectX 11).
* **An older graphics card** - openOMSI falls back to another graphics API by itself; the same
  setting chooses one by hand.
* **Stuck at a bridge or an invisible wall** on a mod map - Esc → Options → *Collisions with
  objects* off (also under **Settings → Gameplay**).
* **Multiplayer: you do not meet the others** - every player needs the host's map. A map
  installed through the Mods page is passed on; one in the OMSI 2 folder is not.
* **Keys do not do what you set** - the **Controls** page shows which driving keys are in
  use (Settings → Driving → *Driving keys*); a key changed there takes effect at once.
* **Anything else** - when the game ends on an error, the launcher shows it with *Copy report*
  and *Report on GitHub*. Logs are in `~/.openomsi` (Windows: `C:\Users\<you>\.openomsi`),
  `game.log` for the last game. **Settings → General → Support → Support package** saves a
  support package into `~/.openomsi` (`openomsi --export-diagnostics <file.zip>` does the same
  from the command line): system, graphics card, driver, settings, controllers, last map and
  bus - no paths, names, chat or addresses. Nothing is sent; look inside before you attach it
  to an issue.

## Goals

1. **1:1 behaviour.** Every content format of the original loads and behaves as in OMSI
   2.2.032. Existing maps and mods work unchanged.
2. **No original code or assets.** Nothing from the original is copied; the formats are
   described in [docs/FORMATS.md](docs/FORMATS.md).
3. **A better engine.** 64-bit, multithreaded, multiplayer, and every platform.

## Documentation

The documentation is on the website, **https://openomsi.org/**, and in [`docs/`](docs):

| Document | What is in it |
| --- | --- |
| [User guide](docs/USER_GUIDE.md) | driving, the launcher, settings, mods, weather, LAN play |
| [Android](docs/ANDROID.md) | the mobile version: install, touch controls, building the APK |
| [Virtual reality](docs/VR.md) | OpenXR headsets on Windows |
| [Dedicated server](docs/SERVER.md) | hosting a multiplayer session without a window |
| [Modding](docs/MODDING.md) | what openOMSI allows beyond OMSI 2: more lights, larger textures, additions OMSI 2 ignores |
| [PBR materials](docs/PBR.md) | normal, roughness, metalness and occlusion maps |
| [Map packages](docs/MAP_PACKAGE.md) | optional surroundings a map can ship for the navigator and city map |
| [Plugins](docs/PLUGINS.md) | Lua and WebAssembly plugins, `.oop` files, OMSI plugin DLLs |
| [Development Tools](https://github.com/openOMSI-org/openOMSI-Development-Tools) | the plugin workbench: projects, API docs, the `.oop` compiler |
| [Add-ons](docs/ADDONS.md) | the launcher's directory of add-ons, and how to list yours |
| [Content formats](docs/FORMATS.md) | every OMSI 2 file format |
| [Routes](docs/ROUTES.md) | timetables, chrono, HOF files, IBIS |
| [Building](docs/BUILDING.md) | building from source on every platform |
| [Architecture](docs/ARCHITECTURE.md) | crates, threading, renderer |
| [Debug switches](docs/DEBUG_FLAGS.md) | environment variables for testing and bug reports |
| [Versioning & releases](docs/VERSIONING.md) | version numbers, beta and stable releases |
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

Needs the latest [Rust stable](https://rustup.rs) and the platform's C toolchain (on Linux
also the WebKitGTK 4.1 headers for the launcher); details in [docs/BUILDING.md](docs/BUILDING.md).

## Repository layout

```
openOMSI/
├── crates/        the game, one crate per subsystem (omsi-app is the program `openomsi`;
│                  formats, scripts, simulation, renderer, sound, network, plugins, launcher data)
├── launcher/      the launcher window (Tauri; web UI in launcher/ui)
├── android/       the Android app wrapper
├── apple/         the iPhone, iPad and Mac app wrapper (SwiftUI, not released yet)
├── services/      the "playing now" counter
├── tools/         omsi-check (format coverage), avatar and ambience pack builders, voice chat plugin
├── scripts/       build scripts for every platform, version.sh, release notes
├── assets/        fonts, icons, logos
├── docs/          documentation (also published on the website)
├── site/          the website
├── release/       health.json: released versions known to be broken
└── VERSION        MAJOR.MINOR of the next release
```

## Contributing

Issues and pull requests are welcome - see [CONTRIBUTING.md](CONTRIBUTING.md); how the project is
run, reviewed and released is in [GOVERNANCE.md](GOVERNANCE.md).

## Support

openOMSI is made in free time. If you enjoy it and want to help it along, you can buy me a
coffee or support it on Ko-fi - thank you!

<p>
  <a href="https://buymeacoffee.com/usonskyyy"><img alt="Buy me a coffee" src="https://cdn.buymeacoffee.com/buttons/v2/default-yellow.png" height="48"></a>
  <a href="https://ko-fi.com/usonance"><img alt="Support me on Ko-fi" src="https://storage.ko-fi.com/cdn/kofi2.png?v=3" height="48"></a>
</p>

## Acknowledgements

Some ideas and fixes in openOMSI date back to the public work of
[neoOMSI](https://github.com/neoOMSI/neoOMSI): their reports and pull requests pointed at
bugs and features that we then implemented in our own code. Thank you.

## License

openOMSI is released under the [GNU Affero General Public License, version 3 only](LICENSE)
(`AGPL-3.0-only`). The historical MIT copyright and permission notice is retained in
`LICENSE`; earlier MIT releases keep their terms. Third-party files keep their own licenses.
The launcher's bus preview encodes its pictures with
[jpeg-encoder](https://crates.io/crates/jpeg-encoder): this software is based in part on the
work of the Independent JPEG Group.
The openOMSI name and original project logos are marks of `usonskyyyy`, who created
the original codebase; see [TRADEMARKS.md](TRADEMARKS.md). The official organization is
[openOMSI.org (`openOMSI-org`)](https://github.com/openOMSI-org).
OMSI and OMSI 2 are trademarks of their
respective owners. openOMSI is an independent project and is not affiliated with them.

</details>
