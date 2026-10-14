# openOMSI Editor 0.8.2-pre - Building and performance update

Based on openOMSI 0.2.38. Community preview for Windows x64 and Linux x64.

## English

- Select multiple splines with Ctrl+click and move them together by dragging or using I/J/K/L; U/O changes the group's height. Ctrl+Z undoes the whole movement. Members stay within their original tiles; invalid moves leave the entire group unchanged. Other geometry tools require a single selection.
- Filter spline selection by roads, sidewalks, markings or other assets. Choose All when an asset's metadata is incomplete. Tab cycles overlapping hits and returns to single selection.
- Faster repeated spline selection through cached tile data; object rows share planning data and prepare their initial data in the background.
- Correct Ctrl+Z / Ctrl+Y handling on German keyboard layouts in terrain mode and Ctrl+Z in spline mode. Terrain also supports Ctrl+Shift+Z for redo.
- Updated German and English operating guides with group selection, filters and undo instructions.
- Linux package layout remains compatible with the existing editor updater (LICENSE and README under docs/).

Practical Linux checks passed on the large Merge test map: groups up to 50 selected splines, undo, tile boundaries, save/reload, height and texture tools, terrain to road, passenger/service preservation in a separate driving test, and a 25-minute mixed building session. The tester confirmed all desired edits survived reloading and the session remained smooth. Full map reload still pauses the game. Fresh VM installation and gameplay tests remain planned for v0.9; Windows CI checks do not replace them.

## Deutsch

- Mehrere Splines mit Strg+Klick auswählen und gemeinsam per Maus oder I/J/K/L verschieben; U/O ändert die Höhe. Strg+Z nimmt die ganze Bewegung zurück. Mitglieder bleiben in ihren ursprünglichen Kacheln; ungültige Bewegungen verändern die gesamte Gruppe nicht. Andere Geometriewerkzeuge benötigen eine Einzelauswahl.
- Spline-Auswahl nach Straßen, Gehwegen, Markierungen oder sonstigen Assets filtern. Bei unvollständigen Metadaten Alle wählen. Tab wechselt überlagerte Treffer und kehrt zur Einzelauswahl zurück.
- Schnellere wiederholte Auswahl durch zwischengespeicherte Kacheldaten. Objektgruppen teilen ihre Planungsdaten; ihre erste Vorbereitung läuft im Hintergrund.
- Strg+Z / Strg+Y auf deutschen Tastaturen im Gelände-Modus und Strg+Z im Spline-Modus korrigiert. Gelände unterstützt auch Strg+Umschalt+Z zum Wiederholen.
- Deutsche und englische Bedienungsanleitung um Gruppenauswahl, Filter und Rückgängig ergänzt.
- Linux-Paket bleibt mit dem bestehenden Editor-Updater kompatibel (LICENSE und README unter docs/).

Praktische Linux-Tests auf der großen Merge-Testkarte bestanden: Auswahl bis 50 Splines, Rückgängig, Kachelgrenzen, Speichern/Neuladen, Höhen und Texturen, Gelände an Straße, Dienst-/Fahrgasterhalt im separaten Fahrtest sowie 25 Minuten gemischtes Bauen. Alle gewünschten Bauänderungen waren nach dem Neuladen vorhanden; die Bedienung blieb laut Tester flüssig. Vollständiges Kartenneuladen pausiert weiterhin das Spiel. Frische VM-Installations- und Spieltests bleiben für v0.9 vorgesehen; Windows-CI-Prüfungen ersetzen diese nicht.

## Installation / Update

Back up maps and close the editor. Extract the package into a new folder. Windows: start-editor.cmd. Linux: bash start-editor.sh "/path/to/content" --launcher. Transfer or reuse required content separately. Original OMSI 2 content is required and not included. Linux requires glibc 2.39 or newer.

Karten sichern und Editor schließen. Paket in einen neuen Ordner entpacken. Windows: start-editor.cmd. Linux: bash start-editor.sh "/Pfad/zum/Inhaltsordner" --launcher. Benötigte Inhalte separat übernehmen oder weiterverwenden. Originale OMSI-2-Inhalte werden benötigt und sind nicht enthalten. Linux benötigt glibc 2.39 oder neuer.

[Demonstration video / Vorstellungsvideo (German, MP4)](https://github.com/DarkBat2709/openOMSI-Editor/releases/download/v0.8.0-pre/openOMSI-Editor-Vorstellung-DE.mp4) - filmed with 0.8.0-pre; shows the basic in-game workflow.

## Maintainer validation notes

The fork now records its existing editor modules in the size/architecture budgets. The previous budgets described the upstream game and already rejected the 0.8.1 editor modules. This update also records the tested group-selection and undo additions; these larger modules remain refactoring debt. Future size growth is still constrained and crate dependency rules are unchanged. Only the two packaged PDF guides receive a bounded 8 MB file-size allowance. Game-platform release PR builds target main; editor branches use the dedicated Linux/Windows editor validation workflow.
