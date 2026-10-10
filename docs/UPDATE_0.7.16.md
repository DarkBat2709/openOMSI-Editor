# Editor 0.7.16-pre – vorhandene KI-Pfade bearbeiten/ersetzen

## Bedienung

Eine Straße oder einen zugehörigen erzeugten KI-Spline auswählen und
„KI-Fahrwege entlang der Straße“ öffnen. Bei einem eindeutig zugeordneten,
bereits vorhandenen Editor-KI-Spline werden dessen Einstellungen geladen und
„Modus: vorhandene KI-Pfade ersetzen“ sowie „ausgewählter Spline“ eingestellt.

Spuranzahl, Richtung, Spurbreite, seitlicher Versatz, Höhe und Fahrzeugfreigaben
ändern. Beim Umschalten von zwei auf eine Spur bleibt Spur 1 an ihrer bisherigen
seitlichen Position (innerhalb der Versatzgrenzen). Die Gegenspur entfällt.
Die Vorschau prüfen: Liegt die verbleibende Spur auf der falschen Fahrbahn,
Versatz und Richtung korrigieren. „KI-Pfade ersetzen (Enter)“ übernimmt die
Änderung; Strg+Z stellt den vorherigen Zustand wieder her.

Für den Grünstreifen zunächst nur den ausgewählten Abschnitt bearbeiten.
Zum bewussten Anwenden derselben Einstellungen auf alle erzeugten KI-Splines
einer verbundenen Straße kann oben auf „verbundene Splinekette“ umgeschaltet
werden. Unterschiedliche alte Einstellungen der Kette werden dann vereinheitlicht.
Abschnitte ohne erzeugte KI-Pfade werden im Ersetzen-Modus nicht neu angelegt.
Dafür auf „Modus: fehlende KI-Pfade ergänzen“ umschalten.

## Verhalten

- Referenzstraße und sichtbare Geometrie bleiben unverändert.
- Bestehende KI-Spline-IDs und Verbindungen bleiben erhalten; keine Duplikate.
- Alte Markierungen der ersetzten KI-Splines werden während einer gültigen
  Vorschau ausgeblendet, sodass die neuen Spuren erkennbar sind.
- Abbrechen schließt nur die Vorschau; die alten Pfade bleiben bestehen.
- Eine Ersetzung der gesamten Auswahl ist ein gemeinsamer Rückgängig-Schritt.
- Speichern und erneutes Laden erhalten die neue Pfaddefinition und Fahrzeugregeln.
- Unabhängige Regeln wie Geschwindigkeitslimits bleiben für erhaltene Pfadindizes
  erhalten; Regeln entfernter Spuren entfallen.
- Eigene Pfade normaler Straßenprofile und Objektkreuzungen werden nicht ersetzt.
  Mehrdeutige Zuordnungen zu mehreren erzeugten KI-Splines werden abgewiesen.

Nach Änderungen Strg+S und für die tatsächliche Verkehrssimulation Karte neu laden.
Der Editor verbindet offene Pfadenden nicht automatisch und erzeugt keine Fahrpläne.

## Installation

ZIP unter `/home/chris/Projekte/openOMSI-Editor` entpacken:

```bash
cd "/home/chris/Projekte/openOMSI-Editor/openomsi-source-0.2.27-editor-0.7.16-pre"
bash INSTALLIEREN.sh
```

Nach erfolgreicher Prüfung und Release-Build wird automatisch der neue
Installationsordner angelegt. Nach „FERTIG“ starten:

```bash
bash "/home/chris/Projekte/openOMSI-Editor/openOMSI-Editor-0.7.16-pre/start-editor.sh" "/home/chris/Games/openOMSI-0.1.1541-linux-x64"
```

Die bisherige Installation bleibt erhalten. Bei Bauproblemen das Protokoll
`update-0.7.16-pre.log` aus dem neuen Quellordner schicken.
Ein interaktiver Test mit der vollständigen Benutzerkarte steht noch aus.
