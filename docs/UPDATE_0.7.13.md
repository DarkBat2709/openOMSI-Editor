# Editor 0.7.13-pre – KI-Pfade nach dem Verbinden

## Korrektur

0.7.12 lehnte jede Profilkorrektur ab und brach damit die gesamte KI-Vorschau ab.
Der Verbindungseditor erzeugt solche Korrekturen auch bei normalen Terrain-Splines,
um die Straßenränder bündig anzuschließen. Der gemeldete Abschnitt 9859278 besitzt
eine solche Korrektur am Ende. Es handelt sich nicht um einen Bedienfehler.

0.7.13 übernimmt gültige Profilkorrekturen in die zusätzlichen KI-Splines.
Vorschau und tatsächliche KI-Spuren folgen damit denselben seitlichen, längsseitigen
und vertikalen Korrekturen wie die Referenz. Richtungspfeile berücksichtigen die
korrigierte Kurve. Fahrzeugregeln und Profilkorrekturen bleiben beim Speichern und
Laden erhalten. Vorschaufehler stehen jetzt mit Spline-ID in game.log.

Die ursprüngliche Straße und ihre Verbindungen werden dabei nicht geändert.
Überhöhungen und Schrägstellungen bleiben vorerst ausgeschlossen; die Meldung
benennt jetzt den betreffenden Spline. Ungültige Profilkorrekturen werden ebenfalls
abgewiesen. Die gewählte Spurbreite wird nicht automatisch an eine Verjüngung
angepasst: Vorschau prüfen und bei Bedarf Spurbreite/Versatz einstellen.

## Installation

ZIP unter `/home/chris/Projekte/openOMSI-Editor` entpacken. Dann:

```bash
cd "/home/chris/Projekte/openOMSI-Editor/openomsi-source-0.2.27-editor-0.7.13-pre"
bash INSTALLIEREN.sh
```

Das Quellpaket wird geprüft, getestet und als Release gebaut. Erst nach Erfolg
entsteht daneben der neue Ordner `openOMSI-Editor-0.7.13-pre`.
Den Zielordner nicht vorab anlegen oder aus einer alten Installation kopieren.
0.7.12 bleibt erhalten. Der neue Ordner entsteht automatisch durch den Installer.

Start nach der Meldung „FERTIG“:

```bash
bash "/home/chris/Projekte/openOMSI-Editor/openOMSI-Editor-0.7.13-pre/start-editor.sh" "/home/chris/Games/openOMSI-0.1.1541-linux-x64"
```

Eine vorhandene Desktop-Verknüpfung anschließend auf denselben neuen Startpfad
ändern. Im Editor muss 0.7.13-pre stehen. Das Installationsprotokoll heißt
`update-0.7.13-pre.log` im neuen Quellordner. Keine GitHub-Veröffentlichung erfolgt.

## Test auf der Karte

Den gespeicherten Abschnitt 9859278 auswählen, „KI-Fahrwege entlang der Straße“
öffnen und „Bereich: verbundene Splinekette“ wählen. Die frühere pauschale Meldung
über verjüngte Splines darf wegen der gültigen Profilkorrektur nicht mehr erscheinen.
Pfeile, Spurbreite, Höhe und offene Anschlüsse prüfen; dann „KI-Pfade anlegen“,
Strg+S und Karte neu laden. Straßenverbindungen allein erzeugen keine KI-Pfade.

Die sonstigen Funktionen und Grenzen aus UPDATE_0.7.12.md gelten weiter, mit der
oben beschriebenen Ausnahme für gültige Profilkorrekturen. Strikte Busbeschränkung
benötigt weiterhin diese angepasste openOMSI-Version; Busfahrpläne entstehen nicht
automatisch. Ein interaktiver Test mit der vollständigen Karte steht noch aus.
