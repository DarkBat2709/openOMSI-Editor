# Editor 0.7.14-pre – KI-Pfade über die verbundene Straße

Im neuen game.log stoppt Spline 722334 die gesamte Kettenvorschau: 0.7.13
unterstützte zwar Profilkorrekturen, aber noch keine Überhöhung/Schrägstellung.
Dieser Abschnitt liegt außerhalb der bereitgestellten Testkachel.

## Änderungen

- Überhöhung, Schrägstellung und gültige Profilkorrekturen werden jetzt unterstützt.
  Vorschau und neue tatsächliche KI-Fahrspuren verwenden die Oberflächengeometrie
  der Straße einschließlich ihrer Überhöhungsbreite.
- „Bereich: verbundene Splinekette“ ist beim Öffnen voreingestellt. Der obere
  Knopf schaltet weiterhin auf einen einzelnen Abschnitt um.
- Abschnitte mit schon erzeugten KI-Pfaden oder eigenen Fahrzeugpfaden werden
  beim Ergänzen übersprungen. Die Anzeige zählt neue und bereits versorgte
  Abschnitte getrennt. Vorhandene Regeln und Pfade bleiben unverändert; die
  Vorschau und aktuellen Einstellungen betreffen nur neue Abschnitte.
- Neue Pfaddefinitionen erhalten eigene Dateinamen. Alte Definitionen werden
  nicht überschrieben. Speichern, Laden und Rückgängig bleiben verfügbar.

Eine Straße muss tatsächlich über die Spline-Verbindungen verknüpft sein;
optisches Aneinanderlegen reicht nicht. Verzweigungen, Objektkreuzungen und
fehlende Verbindungen werden nicht automatisch mit neuen Abbiegepfaden versehen.
Bis zu 500 zusammenhängende Abschnitte werden unterstützt. Offene Pfadenden
werden weiterhin gemeldet; ein unpassender seitlicher Versatz oder unterschiedliche
Straßenprofile können trotz verbundener Straßen einen KI-Anschluss verhindern.
Vorhandene KI-Pfade passen sich beim späteren Verschieben der Straße nicht
selbstständig an. Busfahrpläne entstehen nicht automatisch.

## Installation

ZIP in `/home/chris/Projekte/openOMSI-Editor` entpacken. Dann:

```bash
cd "/home/chris/Projekte/openOMSI-Editor/openomsi-source-0.2.27-editor-0.7.14-pre"
bash INSTALLIEREN.sh
```

Nach erfolgreicher Prüfung und Release-Build erzeugt der Installer automatisch
`openOMSI-Editor-0.7.14-pre` neben dem Quellordner. Den Zielordner nicht vorab
anlegen. Bestehende Installationen bleiben erhalten.

Erst nach der Meldung „FERTIG“ starten:

```bash
bash "/home/chris/Projekte/openOMSI-Editor/openOMSI-Editor-0.7.14-pre/start-editor.sh" "/home/chris/Games/openOMSI-0.1.1541-linux-x64"
```

Im Editor muss 0.7.14-pre stehen. Desktop-Verknüpfungen gegebenenfalls auf den
neuen Startpfad umstellen. Bei einem Baufehler `update-0.7.14-pre.log` aus dem
Quellordner schicken.

## Test auf der Karte

1. Einen sichtbaren Straßen-/Terrain-Spline auswählen, beispielsweise 9859278.
2. „KI-Fahrwege entlang der Straße“ öffnen. Oben muss „verbundene Splinekette“ stehen.
3. Vorschau der neuen Abschnitte, Anzahl bereits vorhandener Pfade, Fahrtrichtung,
   Spurbreite, Höhe und Anschlüsse prüfen. Die Fehlermeldung zu Spline 722334
   darf wegen Überhöhung oder Schrägstellung nicht mehr erscheinen.
4. „KI-Pfade anlegen“, Strg+S, anschließend Karte neu laden.

Bereits versorgte Abschnitte müssen nicht gelöscht werden. Falls alle ausgewählten
Abschnitte Pfade haben, meldet das Werkzeug dies und erzeugt keine Duplikate.
Ein interaktiver Test der vollständigen Benutzerkarte steht noch aus.
