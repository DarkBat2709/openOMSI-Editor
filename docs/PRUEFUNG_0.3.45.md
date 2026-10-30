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
