# Editor auf openOMSI 0.3.33: separater Entwicklungskandidat

Der Entwicklungszweig `codex/editor-basis-0.3.33` führt den Editor 0.8.6-pre
mit dem offiziellen Tag `v0.3.33` (Commit `741e1bc9`) zusammen. Die neue
Editor-Kennung lautet `0.8.7-pre`; dies ist noch keine Veröffentlichung.
Die weiterhin verfügbare Version 0.8.6-pre bleibt auf Basis 0.2.50.

## Übernahme

- Vollständige Engine-Basis 0.3.33 einschließlich ihrer Renderer-, Speicher-,
  Kartenlade- und Prozessor-Kompatibilitätsänderungen.
- Die Editor-Erweiterungen für Objekte, Splines, KI-Wege, Gelände, Kreuzungen und
  Kreisverkehre bleiben enthalten. Dazu gehören gekoppelte Markierungen,
  Straßenprofile, Terrain-Anpassung und die Korrektur kachelübergreifender Saves.
- Das Neuladen verwendet die neue Texturbudget-Schnittstelle.
- Legacy-Spline-Ketten werden durch die entsprechende Implementierung der neuen
  Basis aufgelöst. Die bisherigen Regressionstests werden weiter ausgeführt.
- Die geprüfte Erkennung objektbasierter Einstiegspunkte wird an den Aufrufstellen
  der neuen Basis verwendet. Deaktivierte Definitionsblöcke, doppelte Kacheln und
  vorhandene Einstiegspunktlisten werden weiterhin berücksichtigt.
- Gelände-Anweisungen folgen ihrem Datensatz; die neue Basis unterstützt zusätzlich
  die Zuordnung zu Spline-Anhängen. Editor-Löschmarkierungen bleiben erhalten.
- Der offizielle Stable-/Beta-Updater bleibt im eigenständigen Editor deaktiviert.
  Die getrennten Editor- und Spiel-Updatekanäle bleiben bestehen.
- Die Übersetzungen beider Stände sind zusammengeführt; doppelte Schlüssel wurden
  aufgelöst. Zusätzliche Tests für Skriptansichten liegen in einer eigenen Datei.

## Lizenz und Veröffentlichung

Die neue Basis verwendet `AGPL-3.0-only`. Der vollständige Lizenztext samt den
historischen MIT-Hinweisen bleibt in `LICENSE`, und die Cargo-Metadaten folgen
der übernommenen Basis. Alte veröffentlichte Versionen werden nicht umetikettiert.
Vor Veröffentlichung gehören der passende Quellstand und die Bauanweisungen zum
Release. Fremde Inhalte, Karten, Diagnoseabbilder und private Daten gehören nicht
in das Paket.

## Prüfung und Installation

Die Prüfung umfasst die Workspace-Tests mit `omsi-app/standalone-editor`, die
Standardkonfiguration, Paket-/Installationsverträge, DE/EN-Texte sowie Architektur
und Dateigrößen. Windows und Linux werden zusätzlich im Editor-CI geprüft.
Ein Build allein bestätigt noch keinen erfolgreichen Kartenbau-Praxistest.

Für Linux baut `scripts/editor/install_basis_0_3_33.py` eine getrennte Installation.
Der vorhandene Inhaltsordner wird dem Startskript übergeben. Ein bereits laufender
Editor wird dadurch nicht verändert. Die bisherige Installation bleibt erhalten.

## Nächste Erweiterung nach dem Basiswechsel

Die gewünschte Karten-/Luftbildvorlage ist vorgemerkt: lokale PNG/JPG-Dateien
maßstäblich über mehrere Kacheln auf dem Gelände anzeigen, verschieben, drehen,
Transparenz/Sichtbarkeit einstellen, Ausrichtung sperren und mit der Karte speichern.
Die Auswahl von Kartenanbietern oder Online-Diensten folgt später. Diese Funktion
ist noch nicht Teil dieses Basiswechsels.
