# openOMSI Editor v0.8.3-pre — Engine update to 0.2.50

## English

- Updated the bundled engine from openOMSI 0.2.38 to 0.2.50 while preserving the editor's building tools.
- Restored spline selection handles, AI path overlays and editing handles in the new rendering stages, including editor-only mode.
- Fixed the driver's timetable paper: generated textures in the user cache are now resolved by their absolute file paths.
- Both packages include the standalone editor launcher. Linux starters explicitly select the bundled engine and reuse the content folder supplied by the user.

Practical Linux tests on the large Merge map passed: editor-only display, launcher start, editing during a passenger service, subsequent passenger exchange, and map reload retaining passengers and the selected duty. The timetable paper is visible after the fix. Automated regressions and package checks supplement these tests. Fresh Windows/Linux VM installation and gameplay tests remain planned for v0.9.

The editor bundles its own engine: updating the separate game does not update the editor. Sharing the 0.2.50 engine base does not make editor-specific map extensions compatible with the unmodified game. Keep map backups; custom AI paths, deletion markers and profile transitions still require this fork.

## Deutsch

- Mitgelieferte Engine von openOMSI 0.2.38 auf 0.2.50 aktualisiert; die Bauwerkzeuge des Editors bleiben enthalten.
- Spline-Auswahlpunkte, KI-Pfade und Bearbeitungsgriffe in die neuen Zeichenroutinen eingebunden, auch im reinen Editormodus.
- Fahrplanzettel im Bus korrigiert: erzeugte Texturen im Benutzer-Cache werden über ihren vollständigen Dateipfad gefunden.
- Beide Pakete enthalten den separaten Editor-Launcher. Der Linux-Starter wählt ausdrücklich die mitgelieferte Engine und verwendet den angegebenen Inhaltsordner.

Praktische Linux-Tests auf der großen Merge-Karte bestanden: Anzeige im Editormodus, Start über den Launcher, Bearbeitung während einer Fahrt, anschließender Fahrgastwechsel sowie Kartenneuladen mit Erhalt von Fahrgästen und Dienst. Der Fahrplanzettel ist nach der Korrektur sichtbar. Automatisierte Regressionen und Paketprüfungen ergänzen diese Tests. Frische Windows-/Linux-VM-Installations- und Spieltests bleiben für v0.9 vorgesehen.

Der Editor bringt seine eigene Engine mit. Ein Update des separaten Spiels aktualisiert den Editor nicht. Die gemeinsame Basis 0.2.50 macht editorspezifische Kartenerweiterungen nicht automatisch mit dem unveränderten Spiel kompatibel. Karten sichern: eigene KI-Pfade, Löschmarkierungen und Profilübergänge benötigen weiterhin diesen Fork.

## Installation / Update

Close the editor and extract into a new folder. Windows: start-editor.cmd; Microsoft Edge WebView2 Runtime is required. Linux: bash start-editor.sh "/path/to/content". Reuse or transfer your existing content separately; original OMSI 2 content is required and not included. Linux package requires glibc 2.39 or newer.

Editor schließen und in einen neuen Ordner entpacken. Windows: start-editor.cmd; Microsoft Edge WebView2 Runtime wird benötigt. Linux: bash start-editor.sh "/Pfad/zum/Inhaltsordner". Vorhandene Inhalte getrennt weiterverwenden oder übernehmen; originale OMSI-2-Inhalte werden benötigt und sind nicht enthalten. Das Linux-Paket benötigt glibc 2.39 oder neuer.

The included German and English operating guides describe the unchanged building controls (guide edition 0.8.2). / Die enthaltenen deutschen und englischen Bedienungsanleitungen beschreiben die unveränderten Bauwerkzeuge (Anleitungsstand 0.8.2).

[Demonstration video / Vorstellungsvideo (German, MP4)](https://github.com/DarkBat2709/openOMSI-Editor/releases/download/v0.8.0-pre/openOMSI-Editor-Vorstellung-DE.mp4) — filmed with 0.8.0-pre; shows the in-game building workflow.
