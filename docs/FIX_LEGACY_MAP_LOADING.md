# Kompatibilität beim Laden älterer OMSI-Karten

Diese Ergänzung bleibt auf der Editor-Basis 0.2.50 und enthält die vorherigen lokalen
Korrekturen für kachelübergreifendes Speichern neuer Objekte und die Zuordnung von
Gelände-Anweisungen zum richtigen Datensatz.

## Straßenketten

Kacheln vor Version 11 speichern eine Fortsetzung als Verknüpfung zum vorherigen
Spline. Der Parser erkennt diese bereits; der Weltlader setzt jetzt Position,
Höhe und Richtung der Fortsetzung anhand des tatsächlichen Endes der vorherigen
Kurve. Das funktioniert auch über mehrere Abschnitte, bei Steigungen und bei
`spline_h`. Nicht verknüpfte Abschnitte behalten ihre eigenen Positionen.

Basiskachel und Chrono-Ergänzungen werden jeweils anhand ihrer eigenen Version
aufgelöst, bevor sie zusammengeführt und auf das Weltkoordinatengitter skaliert
werden. Version 11 und neuer sowie Datensätze ohne Versionsangabe behalten ihre
expliziten Positionen. Der bestehende Schreibschutz des Spline-Editors für alte
Kachelformate bleibt bestehen.

## Einstiegspunkte

Nur wenn `global.cfg` keine Einstiegspunktliste enthält, werden gewöhnliche
Objektplatzierungen der gelisteten Basiskacheln untersucht. Eine SCO-Definition
mit aktivem `[entrypoint]` kennzeichnet einen Einstiegspunkt. Objektbeschriftung,
ID und ursprünglicher Kachelindex werden übernommen. Doppelte Kachellistungen
verschieben die Zuordnung nicht. Vorhandene explizite Einstiegspunkte haben Vorrang.

Die Definition eines wiederholt verwendeten Objekttyps wird innerhalb eines
Ladevorgangs nur einmal geprüft. Content-Suchpfade, Windows-Pfadtrenner und die
Groß-/Kleinschreibung vorhandener Dateien werden berücksichtigt. Fehlende Dateien
verhindern das Laden der Karte nicht. Die tatsächliche Höhe wird aus der
Objektplatzierung im Weltindex bezogen, da Objekthöhen geländerelativ sind.

Die Erkennung schreibt weder Kacheln noch `global.cfg` um. Chrono-exklusive
Einstiegsobjekte, Anhänge und Spline-Anhänge werden durch diesen Fallback nicht
zusätzlich als Einstiegspunkte erfasst.

## Prüfung

- Synthetische Straßenkette mit Viertelkreis, Höhenänderung, gerader Fortsetzung
  und unabhängigem Abschnitt; wiederholte Auflösung bleibt stabil.
- Moderne explizite Positionen bleiben erhalten.
- Alter Basisabschnitt mit moderner Chrono-Ergänzung und umgekehrte Kombination;
  gespeicherte Quelldateien bleiben unverändert.
- Einstiegspunkte mit Beschriftung, doppelten Objekt-IDs auf verschiedenen
  Kacheln, doppelter Kachellistung, gemischter Pfadschreibweise und deaktiviertem
  Definitionsblock; explizite Liste und fehlende Dateien.
- Tests von Karten-, Welt-, Launcher- und Editor-Bibliotheken sowie Architektur-
  und Größenprüfungen.

Am 18. Oktober 2026 hat Chris die vorgeschlagenen praktischen Tests als erfolgreich
bestätigt. Welche einzelne ältere Karte dabei verwendet wurde, wurde nicht dokumentiert.
