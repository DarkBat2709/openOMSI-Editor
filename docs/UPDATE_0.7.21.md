# Editor 0.7.21-pre – Baukasten-Titel

Die beiden Fenstertitel im Spiel enthalten keine Versionsnummer mehr:
- Deutsch: Kreuzungs-Baukasten und Kreisverkehr-Baukasten.
- Englisch: Junction builder und Roundabout builder.

Die Versionsnummer des Editors bleibt 0.7.21-pre und ist weiterhin an anderen
Stellen sichtbar. Die Funktionen einschließlich der Mauskorrektur aus 0.7.20
bleiben unverändert. Die versionlosen Übersetzungsschlüssel müssen bei künftigen
Versionswechseln nicht mehr angepasst werden.

## Installation

Das Quellpaket in einen neuen Ordner entpacken und dort ausführen:

```bash
bash INSTALLIEREN.sh
```

Der Installer kompiliert und prüft den Editor, bevor er eine neue Installation
anlegt. Bestehende Installationen werden nicht überschrieben. Erst nach FERTIG
start-editor.sh aus der neuen Installation verwenden und den bisherigen
Spielinhaltsordner übergeben. Diagnose ist für diese Textänderung nicht nötig.

Titel und Übersetzungsschlüssel sowie Paketprüfsummen hier statisch geprüft.
Der dynamische Sprachtest konnte ohne rustc nicht ausgeführt werden. Kein lokaler Rust-Build,
weil die Bauumgebung in diesem Arbeitsbereich fehlt; Build und Regressionstests
führt der Installer auf dem Zielrechner aus.
