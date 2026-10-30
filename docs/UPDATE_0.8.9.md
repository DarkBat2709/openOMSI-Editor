# openOMSI Editor 0.8.9-pre

## Deutsch

Änderungen gegenüber 0.8.8.1:

- openOMSI-Basis von 0.3.33 auf 0.3.45 aktualisiert.
- Bushaltestellen direkt im Objektmodus platzieren, benennen und konfigurieren.
- Startpunkte platzieren, ausrichten und benennen; Übernahme in die Startpunktliste beim Speichern.
- Neuer Dialog Routen und Linien: Haltestellen in Fahrtrichtung wählen, Linie, eindeutigen Routennamen, Abfahrt und Fahrtdauer eingeben; optional eine vorhandene KI-Gruppe mit ihrem Namen zuordnen.
- Native Fahrplandaten speichern; vorhandene Umläufe bleiben erhalten. Routen folgen verbundenen, gerichteten KI-Fahrwegen. Fehlende Verbindungen werden abgewiesen.
- Deutsche und englische PDF-Anleitungen mit aktuellen Launcher-/Editorbildern und zwei neuen Kapiteln, jeweils 41 Seiten.

Nach dem Platzieren von Haltestellen oder Straßenänderungen speichern und neu laden; nach dem Speichern einer Route erneut laden. Die Gegenrichtung als eigene Route anlegen. HOF-/IBIS-Daten und Ansagen werden nicht automatisch erstellt. Bearbeitung bestehender Routen, alternative Zwischenpunkte und automatische Tagesfahrpläne sind noch nicht enthalten. Siehe [Bedienung und Grenzen](docs/PORTIERUNG_0.3.45.md).


## English

Changes since 0.8.8.1:

- Updated engine base from openOMSI 0.3.33 to 0.3.45.
- Place, name and configure bus stops directly in object mode.
- Place, orient and name entry points; save them in the map's entry-point list.
- New Routes and lines dialog: choose stops in travel order, enter a line, unique route name, departure and duration, and optionally select an existing AI group by name.
- Save native timetable data while preserving existing duties. Routes follow connected, directed AI paths; missing connections are rejected.
- Updated German and English PDF guides with current launcher/editor screenshots and two new chapters, 41 pages each.

Save and reload after placing stops or changing roads, and reload after saving a route. Return trips require their own route. HOF/IBIS data and announcements are not generated automatically. Editing existing routes, alternative waypoints and automatic full-day schedules are not included in this first route editor. See [route instructions](docs/PORTIERUNG_0.3.45.md).


## Installation

Windows x64: ZIP vollständig in einen neuen Ordner entpacken und start-editor.cmd starten. Linux x64: Archiv entpacken und bash start-editor.sh "/Pfad/zum/Inhaltsordner" starten. Originale OMSI-2-Inhalte erforderlich; Karten vorher sichern. Linux benötigt glibc 2.39 oder neuer.

Windows x64: extract the complete ZIP into a new folder and run start-editor.cmd. Linux x64: extract the archive and run bash start-editor.sh "/path/to/content". Original OMSI 2 content is required; back up maps first. Linux requires glibc 2.39 or newer.
