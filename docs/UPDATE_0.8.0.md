# openOMSI Editor 0.8.0-pre

Basis: openOMSI 0.2.38 (3323229cbe592f5e6d9c271abd5863dc38ab80a1).

## Änderungen

- Editorwerkzeuge auf die neue Engine-Basis portiert.
- Brücke/Tunnel/Untergrund: Straßenhöhen und Steigungen, Arbeitsebene und Geländeansicht.
- Verschiebbare Fenster und getrennte Updateprüfungen für Spiel und Editorpaket.
- Kartenneuladen übernimmt eigenen Bus, technischen Zustand, Linienauftrag und Fahrgäste im dokumentierten Umfang.
- Gelände-Texturierung lädt Texturen nach, ohne Simulationstiles neu aufzubauen.
- Spline-Neuaufbau wird gebündelt; Simulation wartet auf vollständige Weltlisten.
- Werbezustände eindeutig identifizierter Spline-Anbauten bleiben bei Neuaufbau erhalten.
- Aufwendiger Geometrie-Neuaufbau erfolgt nach Ende einer Verschiebegeste.
- Katalogkorrektur: Original-Splines, Objekte und Texturen bleiben neben Zusatzinhalten auffindbar; gleichnamige Zusatzinhalte behalten Vorrang.
- Deutsche und englische Bedienungsanleitung, jeweils 34 Seiten, ohne Ausblick.

## Bedienung

In einen separaten Ordner entpacken. `bash start-editor.sh "/Pfad/zum/Inhaltsordner"` starten.
Die Originalinstallation von OMSI 2 muss eingerichtet sein. Spielinhalte sind nicht enthalten.
Editor und separates openOMSI haben getrennte Updatekanäle.

## Tests und Grenzen

Die bisherigen praktischen Tests auf Linux umfassten Kartenbearbeitung, Speichern,
Neuladen, Fahrgäste, Straßen, KI-Pfade und Landschaftsgestaltung. Automatisierte
Ergebnisse für den endgültigen Stand stehen in PRUEFUNG_0.8.0.txt.
Eine vollständige Neuinstallation in Linux- und Windows-VMs ist für v0.9 geplant.
Kein Windows-Binärpaket wird ohne gesonderte Prüfung als getestet angeboten.

## English

Version 0.8.0-pre ports the editor to openOMSI 0.2.38, adds road-height/work-plane tools,
movable windows and separate update checks, and improves map reload, terrain painting,
advertisement state retention and editing responsiveness. The catalogue now enumerates
both add-on and original asset directories while retaining override priority.
Both operating guides are updated. Fresh Linux/Windows VM tests remain planned for v0.9.
Use a separate installation and back up maps before editing.
