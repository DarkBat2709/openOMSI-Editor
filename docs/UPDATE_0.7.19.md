# Editor 0.7.19-pre – Pfadanzeige im Hintergrund

Testversion auf Basis von 0.7.18 und openOMSI 0.2.27.

## Änderungen

- Der erste Aufbau der KI-Pfadanzeige läuft auf einem Hintergrundthread. Die
  Oberfläche wartet nicht auf dessen Ergebnis. Die Pfade erscheinen nach Abschluss.
- Pro Anzeige läuft höchstens ein Auftrag gleichzeitig. Ergebnisse vor einer
  Bearbeitung oder einem Kartenwechsel werden verworfen. Bei Kamerabewegungen
  wird anschließend der aktuelle Bereich berechnet.
- Bereits eingelesene Kacheldaten werden bei der Initialisierung der Objektreihen
  wiederverwendet. Ihre vollständige Spline-Auswertung wird nicht wiederholt.
- Der Vergleich von Basis- und Chrono-Splines sucht über die ID, prüft weiterhin
  die vollständige Gleichheit und berücksichtigt auch doppelte IDs.
- Zusätzliche Diagnosebereiche: object_rows_initialize, object_rows_refresh,
  editor_reload_prepare und editor_reload_tile (mit Kachel).

Die 19 Kacheln aus der letzten Log werden nicht pauschal reduziert: benachbarte
Kacheln können wegen Geländeanpassungen und Objektverbindungen erforderlich sein.
Die neuen Messpunkte unterscheiden Vorbereitung, Objektreihen und einzelne
Kachelaktualisierungen. Noch keine Aussage, dass alle Hänger behoben sind.
Die Diagnose bleibt nur beim Diagnose-Start aktiv.

## Installation

ZIP unter /home/chris/Projekte/openOMSI-Editor entpacken, dann:

```bash
cd "/home/chris/Projekte/openOMSI-Editor/openomsi-source-0.2.27-editor-0.7.19-pre"
bash INSTALLIEREN.sh
```

Der Installer kompiliert, führt Tests aus und erstellt erst nach erfolgreichem
Release-Build eine neue Installation. Bestehende Versionen werden nicht überschrieben.
Das Paket enthält Quellcode, keine fertige Binary.

Erst nach FERTIG:

```bash
bash "/home/chris/Projekte/openOMSI-Editor/openOMSI-Editor-0.7.19-pre/start-editor-diagnose.sh" "/home/chris/Games/openOMSI-0.1.1541-linux-x64"
```

KI-Pfade einblenden und dieselben Bearbeitungsschritte wiederholen. Uhrzeit eines
verbleibenden Hängers merken; game.log vor dem nächsten Start sichern und schicken.
Nach der Untersuchung normal mit start-editor.sh starten, dann ist die Diagnose aus.

## Prüfstatus

Installer-Python und Shell-Syntax, Versionsreferenzen, Paketmanifest und ZIP geprüft.
Regressionsprüfungen für asynchrone Pfadanzeige, Verwerfen veralteter Ergebnisse,
Cache-Wiederverwendung, Änderungen und Rückgängig sind im Quellcode enthalten.
Diese Rust-Tests und der Build konnten hier nicht ausgeführt werden: Die vorherige
Rust-Bauumgebung wurde durch automatische Arbeitsbereichsbereinigung entfernt.
Der Installer führt sie auf dem Zielrechner aus. Kein interaktiver Kartentest.
Bei einem Buildfehler update-0.7.19-pre.log schicken; bisherige Installation behalten.
