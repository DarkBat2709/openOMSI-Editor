# openOMSI Editor 0.8.1-pre

Based on openOMSI 0.2.38. Small update to the 0.8 preview series.

## English

- Separate Objects, Splines and AI paths toolboxes, with workspace tabs at the top.
- Terrain to road (Shift+F6) adjusts only terrain samples inside the selected road footprint. Road to terrain (F6) remains available. The terrain grid can affect interpolation at the road edge; there is no wider shoulder smoothing in the new operation.
- Junction and roundabout builders keep their title-bar drag across rendered frames. Modal mouse routing now permits dragging.
- Direct access to additional AI connections from the AI path manager.
- Updated German and English operating guides, 34 pages each, with matching workspace screenshots and no roadmap.

Back up maps, close the editor, extract the new package into a new folder and use its starter. Windows: start-editor.cmd. Linux: bash start-editor.sh "/path/to/content" --launcher. Reuse or transfer the needed content separately. Original OMSI 2 content is required and is not included. Linux binaries require glibc 2.39 or newer. Fresh Linux and Windows VM tests remain outstanding.

## Deutsch

- Getrennte Werkzeugboxen für Objekte, Splines und KI-Fahrwege mit Umschaltern oben.
- Gelände an Straße (Umschalt+F6) passt nur Geländerasterpunkte innerhalb der gewählten Straßenfläche an. Straße an Gelände (F6) bleibt erhalten. Die Rasterdreiecke können die sichtbare Form an der Kante beeinflussen; die neue Funktion glättet keinen breiteren Rand.
- Kreuzungs- und Kreisverkehr-Baukästen behalten ihre Ziehbewegung über das Neuzeichnen hinweg. Mausbewegungen werden auch bei geöffneten Dialogen weitergereicht.
- Zusätzliche KI-Verbindungen direkt aus der KI-Pfadverwaltung öffnen.
- Deutsche und englische Bedienungsanleitung mit jeweils 34 Seiten, passenden Bildern und ohne Ausblick aktualisiert.

Karten sichern, Editor schließen, das neue Paket in einen neuen Ordner entpacken und dessen Startdatei benutzen. Windows: start-editor.cmd. Linux: bash start-editor.sh "/Pfad/zum/Inhaltsordner" --launcher. Benötigte Inhalte separat weiterverwenden oder übernehmen. Originale OMSI-2-Inhalte werden benötigt und sind nicht enthalten. Linux benötigt glibc 2.39 oder neuer. Frische Linux- und Windows-VM-Tests stehen weiterhin aus.
