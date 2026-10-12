# Lokaler Portierungskandidat auf openOMSI 0.2.38

Dieser Stand übernimmt den Editor 0.7.23-pre einschließlich der lokalen Erweiterungen auf die offizielle openOMSI-Basis 0.2.38 (Tag v0.2.38, Commit 3323229cbe592f5e6d9c271abd5863dc38ab80a1).

Er ist ein separater Testkandidat, keine veröffentlichte Version 0.8 oder 1.0. Die bisherige Installation und Spielkarten werden durch das Installationsskript nicht überschrieben. Das manuelle Bearbeiten und Speichern einer Karte im gestarteten Kandidaten verändert jedoch wie üblich deren Dateien: für die Erprobung eine Kartenkopie verwenden.

## Übernommene Funktionen

- Objekt-, Straßen-, Gelände-, Kreuzungs- und Kreisverkehrbearbeitung.
- KI-Fahrwege, Verbindungen, Fahrtrichtungen und Höhenkorrekturen.
- Brücke/Tunnel/Untergrund, Arbeitsebene und Geländeansicht.
- Verschiebbare Bedienfenster und deutsche/englische Bedienungsanleitungen.
- Speichern/Rückgängig und Neuladen mit Fahrgästen, Buszustand und aktivem Linienauftrag.
- Getrennte manuelle Updates für das separate Spiel und das Editorpaket.

## Anpassungen an die neue Basis

Gemeinsame Editorzustände liegen im neuen Szenenmodul; Bedienung und Bearbeitungsabläufe bleiben im Anwendungsmodul. Launcher und Updatekanäle verwenden die neuen Bibliotheksgrenzen. Die Zustandsübernahme des eigenen Busses bleibt von der neuen upstream-Funktion zum Neuladen von Fahrzeugdateien getrennt. Beim Kartenneuladen werden auch die neu ausgelagerten GPU-Zustände von Fahrgästen und KI-Verkehr zurückgesetzt.

Der neue Selbsttest vor einer Updateinstallation bleibt aktiv. Gleichzeitige Selbsttests erhalten unterschiedliche temporäre Antwortdateien. Die Editorübersetzungen überschreiben keine neu hinzugekommenen upstream-Schlüssel.

## Bauen und separat installieren (Linux x64)

`python3 scripts/editor/install_basis_0_2_38.py --install-dir /Pfad/zu/einem/neuen/Editorordner`

Das Quellpaket muss seine passende SOURCE-SHA256.json enthalten. Das Skript prüft diese, baut mit `standalone-editor` und setzt die Basisversion auf 0.2.38. Ein vorhandener Zielordner wird abgelehnt. Historische install_0_7_*.py-Skripte gehören zu früheren Paketen und sind für diesen Kandidaten nicht zu verwenden.

Start: `bash start-editor.sh "/Pfad/zum/Spielinhaltsordner"`

## Verbleibende Freigabeprüfung

Automatische Tests und Programm-Selbsttest ersetzen keine interaktive Fahrt. Vor einer Freigabe: Testkarte öffnen, Objekte/Straßen/KI-Wege bearbeiten, speichern und neu laden; während einer Fahrt Fahrgäste im Bus und an Haltestellen, Buszustand, Kamera und Linienauftrag kontrollieren; Brücke/Tunnel, Undo, Katalog und verschiebbare Fenster ausprobieren. Updates nur mit dafür vorgesehenen Testinstallationen erproben. Windows- und VM-Tests bleiben für die geplante 0.9-Prüfphase offen.

Die beigefügten PDFs beschreiben die Bedienung des Editors. Dieser technische Portierungsstand und der Prüfbericht gehören nicht als Ausblick in die Bedienungsanleitungen.
