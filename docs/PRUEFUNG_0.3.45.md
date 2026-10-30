# Lokale Prüfung am 30.10.2026

Quellzweig: `codex/editor-0.3.45-lines`, Integration von `upstream-v0.3.45`.
Version: `0.3.45-editor-0.8.9-pre`.

- Anwendung: 640 Tests erfolgreich, 18 regulär ignoriert.
- Simulation: 853 Tests erfolgreich, 31 ignoriert.
- Szene: 230 Tests erfolgreich, 7 ignoriert.
- Launcher-Kern: 116 Tests erfolgreich, 2 ignoriert.
- Zusätzlicher GPU-Test der neuen Routen-/Objektfenster erfolgreich, Bilder in 1280×900 und 800×600 visuell geprüft.
- Speichertests für Haltestellen/Startpunkte: Anlegen, Benennen, Entfernen, Erhalt der Originaldatei und erneutes Einlesen erfolgreich.
- Routentests: gerichteter Weg, Ablehnung fehlender Wege, native Fahrt-/Linien-Dateien, Erhalt vorhandener Umläufe, mehrfaches Ergänzen und Überschreibschutz erfolgreich.
- 1171 Editor-Übersetzungseinträge geprüft, DE/EN-Wechsel und verschachtelte Meldungen erfolgreich.
- Linux-Editor und Launcher im Release-Profil gebaut; Selbsttest und Versionsausgabe erfolgreich.
- Grundorf mit Original-OMSI-Inhalten ohne Fenster geladen und nach PNG gerendert; Bild geprüft. Keine Kartenänderung gespeichert.
- Python-Paket-/Installationstests und Syntax-/Importprüfung der betroffenen Launcher-JavaScript-Dateien erfolgreich.

Die ignorierten Tests wurden nicht pauschal ausgeführt; der neue GPU-Fenstertest wurde separat gestartet. Keine Windows-Laufzeitprüfung und noch keine manuelle komplette Linienfahrt durch den Benutzer. HOF-/IBIS-Erstellung ist nicht Teil dieser Fassung. Bedienung und Grenzen stehen in `PORTIERUNG_0.3.45.md`.

Der lokale Launcher-Build verwendet separat entpackte Ubuntu-Entwicklungspakete unter `/tmp/editor-webkit-dev`; die ausführbare Datei verwendet die bereits installierten Systembibliotheken. Es wurde kein Systempaket installiert oder ersetzt.

## Release-Prüfungen und Integration

Die Größenbudgets wurden auf den zusammengeführten Stand 0.3.45 plus Editor angepasst, ohne die Prüfung oder Abhängigkeitsgrenzen abzuschalten: omsi-app 96606 Zeilen; model/lib.rs 1639 (unverändert aus upstream-v0.3.45), net/ws.rs 1508, scene/props.rs 1808, scene/staging.rs 1501 und scene/tests.rs 1636. Die Szenendateien enthalten den übernommenen Editor-Speicher-/Ladecode und Regressionstests. window_event (344 Zeilen) und lights (303) enthalten zusätzlich Editor-Eingabe bzw. Editor-Anzeigen. Dies dokumentiert die Integrationsbasis; künftiges Wachstum unterliegt weiterhin den bestehenden Grenzen. Eine größere Aufteilung dieser bestehenden Module gehört nicht in die Veröffentlichungsvorbereitung.

Zehn Secret-Scan-Treffer waren ausschließlich Dateiprüfsummen in SOURCE-SHA256.json. Jeder Wert wurde gegen die jeweilige Datei im gemeldeten Commit verifiziert. Nur die konkreten historischen Fund-Fingerprints sind ausgenommen; kein allgemeiner Pfad- oder Regel-Ausschluss.

Der Security-Audit erforderte zstd 0.14.3, zstd-safe 8.2.0 und zstd-sys 2.1.2. Die Abhängigkeiten wurden aktualisiert; Kompressions-/Netzwerktests und Release-Build werden erneut ausgeführt.
