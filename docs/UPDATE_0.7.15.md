# Editor 0.7.15-pre – vorhandene KI-Pfade zuverlässig anzeigen

Das Log vom 10.10.2026 um 12:34:14 UTC bestätigt 40 neu angelegte Abschnitte.
Die spätere Meldung „Alle ausgewählten Abschnitte haben bereits KI-Pfade“
bezeichnet vorhandene Pfade, nicht einen fehlgeschlagenen Anlegevorgang.

## Fehlerursache und Korrektur

Die bisherige Anzeige las world.lanes. Diese Liste ist eine Übergabewarteschlange,
die Verkehrssimulation oder Navigator beim Übernehmen leeren. Deshalb blieb die
Anzeige trotz vorhandener Pfade leer. Ein Kartenneustart löste das nicht zuverlässig.

Die Anzeige liest jetzt gespeicherte Spline-Pfade und aktuelle Editoränderungen
direkt aus der Karte. Neue Pfade erscheinen ohne Speichern oder Kartenneustart.
Nach bestätigten Änderungen einschließlich Rückgängig wird die Anzeige erneuert.
Gelöschte Pfade verschwinden. Beim Bewegen der Kamera wird der Anzeigebereich
nachgeladen. Objektpfade werden aus dem verfügbaren Verkehrs-/Navigatornetz ergänzt.

Im KI-Fenster gibt es einen eigenen Schalter mit erkennbarem Zustand:
„KI-Pfade: sichtbar – ausblenden“ bzw. „KI-Pfade: ausgeblendet – anzeigen“.
Er schaltet sowohl vorhandene Pfade als auch die Anlegevorschau. Der Schalter im
normalen Splinewerkzeug bleibt verfügbar und bestätigt seinen Zustand als Meldung.
Blaue Punkte/Pfeile kennzeichnen allgemeine Fahrzeugpfade, orange reine Buspfade.
Magenta Markierungen gehören weiterhin zur Splineauswahl, nicht zur Pfadanzeige.

Wird ein vom Werkzeug erzeugter unsichtbarer KI-Spline angeklickt, erkennt das
KI-Werkzeug dessen Referenzstraße anhand des gespeicherten Dateinamens. Eine
bereits vollständig versorgte Straße wird als „0 neue / … bereits mit Pfaden“
angezeigt; „Anlegen“ bleibt deaktiviert. Es werden keine Duplikate erzeugt.

## Installation

ZIP unter `/home/chris/Projekte/openOMSI-Editor` entpacken, anschließend:

```bash
cd "/home/chris/Projekte/openOMSI-Editor/openomsi-source-0.2.27-editor-0.7.15-pre"
bash INSTALLIEREN.sh
```

Nach erfolgreicher Prüfung, Tests und Release-Build wird automatisch der neue
Installationsordner `openOMSI-Editor-0.7.15-pre` angelegt. Nach „FERTIG“:

```bash
bash "/home/chris/Projekte/openOMSI-Editor/openOMSI-Editor-0.7.15-pre/start-editor.sh" "/home/chris/Games/openOMSI-0.1.1541-linux-x64"
```

## Prüfung auf der Karte

Vorhandene Pfade nicht erneut anlegen oder löschen. Eine Straße auswählen,
KI-Werkzeug öffnen, Anzeige auf „sichtbar“ stellen. Es sollen blaue/orange
Pfadpunkte und Richtungspfeile erscheinen. Ein-/Ausblenden muss diese Markierungen
umschalten; die magenta Splineauswahl bleibt davon unabhängig.

Neue Pfade sind direkt sichtbar. Für die tatsächliche Verkehrssimulation nach
Kartenänderungen weiterhin Strg+S und Karte neu laden. Offene Pfadenden und
Busfahrpläne werden durch diese Anzeigekorrektur nicht automatisch verbunden
oder erstellt. Ein interaktiver Test der vollständigen Benutzerkarte steht aus.
