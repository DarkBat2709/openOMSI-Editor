# Privater Portierungskandidat: Editor auf openOMSI 0.2.50

Stand: 14. Oktober 2026. Kein öffentliches Release und noch keine praktische Freigabe.

Ausgangspunkt ist Editor 0.8.2-pre (f0297253411c6bdc6fe11f755c67405613bbb169).
Die übernommene offizielle Basis ist v0.2.50
(8e9a6139ac7ba776325bd212864c3f37824b9cd3).
Der separate Arbeitszweig heißt `codex/editor-basis-0.2.50`.

## Was angepasst wurde

- Editor-Hooks an die neuen Frame-Kontexte und den festen Simulationstakt angepasst.
  Während eines Editor-Kachelumbaus bleiben Busphysik, Verkehr und Fahrgäste vor
  vorübergehend fehlenden Kartenteilen geschützt.
- Gelände- und Szenenzugriffe auf die neuen `WorldData`-Snapshots umgestellt.
- Fahrgastübernahme beim Neuladen auf die getrennte Simulation und Darstellung
  angepasst. Die Tests prüfen weiter wartende Fahrgäste, verschobene oder entfernte
  Haltestellen sowie den Zustand des eigenen Busses.
- Skriptzustände vor einem Kachelumbau aus der Simulation übernommen. Zugehörige
  Objektquelle und Beschriftung werden gegen die neue Platzierung geprüft, bevor
  beispielsweise die bisherige Werbeauswahl wiederhergestellt wird.
- Beschriftungsänderungen über die neue Simulations-Inbox übergeben.
- Editor-Seite mit neuer Karte und vorhandener Karte in den Tauri-Launcher übertragen.
  Deutsche Editorübersetzungen stehen auch diesem Launcher zur Verfügung.
- Offizieller Selbst-Updater und offizieller Versionswechsel bleiben in einem
  Editor-Build gesperrt. Separate Spiel-/Editor-Prüfungen sind vorhanden;
  im neuen Launcher führen verfügbare Updates zum manuellen Download.
- Linux- und Windows-Paketierung verlangen jetzt zusätzlich den Launcher.
  Linux benötigt WebKitGTK 4.1, Windows WebView2. Die bisherige Anleitung für
  die 0.2.38-Installation ist für diesen Kandidaten nicht anzuwenden.
- Die Architekturbaseline wurde auf die zusammengeführte 0.2.50-Struktur gesetzt;
  Abhängigkeitsregeln bleiben aktiv. Der Größencheck besteht ohne weitere Erhöhung.

## Automatische Prüfung

- Anwendung: 524 Tests bestanden, 5 ausdrücklich ignoriert.
- Komponenten: Launcher-API 20, Launcher-Core 70, Kartenformat 25,
  Plattform/Updater 38, Szene 126, Simulation 634 Tests bestanden.
- Zusätzlich bestanden: neuer API-Test für deutsche Editortexte und neuer
  Simulationstest für gezielte Beschriftungsübergabe.
- Nach letzten Anpassungen: 36 editorbezogene Anwendungstests und 8 Updater-Tests
  erneut erfolgreich. Ignorierte GPU-/Installations-/Langzeittests sind dadurch
  nicht als bestanden zu verstehen.
- Deutsch/Englisch: 1022 Editor-Schlüssel und echter Sprachwechsel geprüft.
- Linux-Installationsvertrag, Windows-Paketvertrag einschließlich fehlendem
  Launcher und Prüfsummen, sowie Kartenprüfer separat geprüft.
- Engine gebaut, Versionsausgabe und eingebauter Selbsttest erfolgreich.
- Launcher gebaut und in einem unsichtbaren Fenster geprüft: keine fehlgeschlagenen
  Prüfungen, keine JavaScript-Fehler; auch die neue Editor-Seite wurde angezeigt.
- Architekturschranken und Dateigrößenprüfung bestanden.

Die Plattformtests brauchen lokale Sockets. Ihre zunächst unter der Sandbox
fehlgeschlagenen Socketprüfungen bestanden beim erlaubten erneuten Lauf.
Unit-Tests initialisieren Steam nicht: parallele App-Testinstanzen dürfen die
prozessweite Steam-Laufzeit nicht gleichzeitig initialisieren. Das Verhalten
im gestarteten Spiel bleibt unverändert.

## Kompatibilität mit dem unveränderten offiziellen Spiel

Die Engine-Portierung und die Lesbarkeit eigener Karten-Erweiterungen sind zwei
getrennte Aufgaben. Dieser Kandidat behält die Editor-Leser und -Schreiber bei.
Ein separat installiertes offizielles openOMSI 0.2.50 erhält sie dadurch nicht.

Der Quellvergleich bestätigt fehlende Unterstützung im offiziellen Stand für:

| Editor-Daten | Bedeutung im offiziellen Spiel |
| --- | --- |
| `editor_lane_*`-Regeln in Karten | Eigene Kurven-, Richtungs-, Höhen- oder Verbindungsänderungen werden nicht gleich ausgewertet. |
| `editor_surface_path`, `editor_bus_only` | Zusatzregeln generierter KI-Wege werden nicht gleich ausgewertet. |
| `[editor_path_node]`, `[editor_path_connection]` in SLI-Dateien | Freiformpunkte und explizite KI-Verbindungen fehlen in der offiziellen Auswertung. |
| `[openomsi_profile_transition]` | Die eigene Profilanpassung wird nicht ausgewertet. |
| `[openomsi_editor_deleted]` | Ein als gelöscht markierter Spline kann im offiziellen Spiel wieder erscheinen. |

Der lesende Prüfhelfer `scripts/editor/check_map_compatibility.py` untersucht
MAP-Dateien im Kartenordner und in Chrono-Unterordnern, auch UTF-16. Sicherungsordner
werden nicht rekursiv als aktive Karte mitgezählt. Er erfasst die genannten
MAP-Erweiterungen; referenzierte SLI/SCO-Dateien, fehlende Assets und die tatsächliche
Befahrbarkeit sind ausdrücklich nicht vollständig geprüft.

Bei der Merge-Karte wurden solche Einträge in 21 MAP-Dateien gefunden. Die frühere
Annahme, dort seien keine vorhanden, ist korrigiert. Die Karte wurde für die Prüfung
nur gelesen, nicht verändert.

Nächster Kompatibilitätsschritt: anhand kleiner Kartenkopien eine Standardausgabe
prüfen. Bei gelöschten Splines müssen insbesondere indexbasierte Anbauten korrekt
neu zugeordnet werden. Freiform-KI benötigt eine passende Standardgeometrie; das
bloße Entfernen unbekannter Tags wäre keine verlustfreie Konvertierung.
Bis eine solche Ausgabe getestet ist, keine pauschale Zusage zur identischen
Wirkung aller Editoränderungen im offiziellen Spiel machen.

## Lokal testen und später freigeben

Der bereitgestellte lokale Kandidat verwendet das optimierte Entwicklungsprofil,
keinen Release-Build. Er dient zunächst dem Funktionsvergleich; seine Bildrate ist
kein abschließender Performance-Nachweis.

Start: `bash start-editor.sh "/Pfad/zu/einer/separaten/Inhaltskopie"`.
Bei der ersten praktischen Prüfung ausschließlich eine Kartenkopie verwenden.
Die Spielkarte nicht gleichzeitig in der alten und der neuen Engine bearbeiten.
Das Startskript schreibt wie bisher Einstellungen/Logs ins Benutzerprofil.
Die vorhandenen PDF-Handbücher sind die 0.8.2-Bedienungsanleitungen; die neue
Launcheransicht ist darin noch nicht neu bebildert.

Praktische Freigabe noch offen: Auswahl, Gruppenverschiebung, Undo/Redo,
Gelände/Textur, KI-Kurven, Speichern/Neuladen sowie Linienfahrt mit Fahrgastwechsel
und stabilen Werbetafeln. Danach Release-Build mit Performance-Vergleich und
Windows-Build prüfen. Die vollständigen frischen VM-Tests bleiben für 0.9 geplant.
Erst nach diesen Prüfungen einen öffentlichen Versionswechsel vornehmen.


## Nachtest: Editoranzeigen und Inhaltsordner (14.10.2026)

Der praktische Test fand fehlende Spline-Auswahlpunkte und KI-Pfade. Beim Port auf die neue LightsCx-Stufe fehlte der Editor-Overlay-Block. Wiederhergestellt: Spline-/Verbindungsmarker, KI-Pfade samt Auswahlzielen und Kurvengriffen, Audit-, Gehweg-, Straßenrand-, Gelände- und Objektmarker. Menus wird in dieser Stufe als schreibbar deklariert, da die Pfadansicht Auswahlziele aktualisiert. Die Darstellung benötigt keinen Spielerbus und keine aktive Verkehrssimulation.

Das Testlog verwendete den leeren Testpaketordner als OMSI_CONTENT. Die gemeldeten fehlenden Geländetexturen (u.a. Texture/LemmentalV2/Gras1_1.jpg und Texture/Rat_Feld1.bmp) liegen im bisherigen Spielinhaltsordner. Der lokale start-merge-test.sh setzt diesen Ordner explizit und öffnet die Merge-Karte im Editormodus. Keine Kartendateien verändert.

Validierung: cargo check und 524 App-Tests erfolgreich, 5 ignoriert. Sichtprüfung im laufenden Editormodus nach Neubau noch erforderlich.


## Nachtest: Fahrplanzettel (14.10.2026)

Nutzer bestätigt Bearbeitung während der Fahrt mit normalem Fahrgastwechsel sowie Erhalt des Dienstes nach Neuladen. Der leere Fahrplanzettel ist separat: Log meldet aktive BGN/300001-Dienstzuordnung und eine nicht gefundene file_schedule-Textur, obwohl die erzeugte PNG im Cache vorhanden ist. Die Fahrzeug-Freitextursuche erhält jetzt absolute vorhandene Pfade vor der OMSI-relativen Suche. Regressionstest deckt eine generierte Cache-Datei außerhalb der Fahrzeugordner einschließlich vorherigem Suchfehlschlag ab. Praktische Sichtprüfung des Zettels steht noch aus.
