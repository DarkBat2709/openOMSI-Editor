# Editor 0.7.17-pre – KI-Pfade mit der Maus formen und verbinden

## Freie Bearbeitung

Straße oder erzeugten KI-Spline auswählen, „KI-Fahrwege entlang der Straße“
öffnen und „Freiformpunkte bearbeiten“ anklicken. Das Werkzeug wechselt auf
den ausgewählten Abschnitt. Pro Fahrspur werden zunächst neun grüne Punkte
angezeigt, einschließlich Anfang und Ende. Einen Punkt mit gedrückter linker
Maustaste ziehen und loslassen. Der ausgewählte Punkt ist gelb.

„Fahrspur 1/2 wählen“ wechselt die bearbeitete Richtung. Die andere Fahrspur
bleibt geometrisch unverändert. Die Punkte formen eine geglättete Kurve; jede
Verschiebung ist als Vorschau sichtbar. Es handelt sich nicht um eine Änderung
der sichtbaren Straße. „Punkt zurücksetzen“ setzt die Verschiebung dieses
Punktes zurück. Die Maus verschiebt horizontal; „Punkt höher/tiefer“ verändert
die Höhe um 0,1 m. Maximal 50 m Verschiebung pro lokaler Richtung.

„KI-Pfade ersetzen“ übernimmt bestehende Pfade mit gleichen IDs, „anlegen“
erzeugt neue. Schließen verwirft die Vorschau. Nach Übernehmen stellt Strg+Z
den gesamten vorherigen Zustand wieder her. Strg+S speichert, anschließend
Karte neu laden, damit der laufende Verkehr das neue Netz verwendet.

Für zwei benachbarte Fahrstreifen derselben Fahrtrichtung den Spurknopf
bis „Zwei Spuren in gleicher Richtung“ umschalten. Der Knopf wechselt zwischen
Gegenverkehr, einer Spur und zwei gleichgerichteten Spuren. Danach beide
Fahrspuren über die Spurwahl getrennt bearbeiten.

## Anschlusspunkte an Kreuzungen

Einen grünen Anfangs-/Endpunkt auswählen. Passende Gegenanschlüsse im Umkreis
von 50 m erscheinen lila. Ein Fahrwegende kann an den Anfang eines anderen
Fahrwegs anschließen; ein Anfang an ein anderes Ende. Den Punkt auf das
gewünschte lila Ziel ziehen und loslassen. Innerhalb von 1,5 m rastet er ein.
Die Endtangente wird mit einem zusätzlichen nahen Stützpunkt ausgerichtet.

Die gewählte Verbindung wird mit Kachel, Objekt-/Spline-ID, Pfadindex und
Richtung gespeichert. Das Verkehrsnetz berücksichtigt diese Zuordnung beim
Aufbau und Nachladen: An diesem Ende wird nicht beliebig auf einen anderen
nahen Fahrweg verzweigt. Das Ziel muss weiterhin geometrisch erreichbar sein.
Die Anzeige zählt feste Anschlüsse. Ein Endpunkt, der erneut frei verschoben
wird, löst seine feste Zuordnung. Richtungsumkehr löst bestehende Zuordnungen.
Sind mehrere verschiedene Ziele praktisch gleich nah, wird keine eindeutige
Verbindung behauptet; die Meldung weist auf die Mehrdeutigkeit hin.

Für die beschriebene Kreuzung nacheinander den Fahrweg der Geradeausspur und
den der Rechtsabbiegerspur auswählen, ihren Verlauf anpassen und jeweils auf
den passenden Anschluss ziehen. Keine automatische Prüfung von Vorfahrt,
Ampeln, Kollisionen oder ausreichend großen Kurvenradien. Beim Anschluss an
einen vorhandenen Kreuzungspfad bleiben dessen eigene Verkehrsregeln bestehen;
beim freien Überbrücken einer Kreuzung entsteht keine neue Ampelsteuerung.

## Grenzen und Kompatibilität

Freiform ist abschnittsweise. Die Kettenfunktion wird für bereits frei geformte
Pfade gesperrt, damit individuelle Formen nicht versehentlich vereinheitlicht
werden. Zusammenhängende Abschnitte lassen sich nacheinander bearbeiten und
über ihre Enden anschließen. Originale Fahrzeugpfade in normalen Splineprofilen
oder Kreuzungsobjekten werden nicht überschrieben.

Freiformpunkte und feste Verbindungen sind eine Erweiterung dieser angepassten
openOMSI-Version. Eine unveränderte Hauptversion oder ältere Editorversion wertet
sie nicht aus und würde die normalen Basis-Spuren verwenden. Für den KI-Verkehr
mit diesen Freiformpfaden deshalb den hier enthaltenen angepassten Runtime nutzen.
Ältere Editoren nicht zum Überschreiben solcher Pfade verwenden.

## Installation

ZIP unter `/home/chris/Projekte/openOMSI-Editor` entpacken:

```bash
cd "/home/chris/Projekte/openOMSI-Editor/openomsi-source-0.2.27-editor-0.7.17-pre"
bash INSTALLIEREN.sh
```

Nach „FERTIG“ starten:

```bash
bash "/home/chris/Projekte/openOMSI-Editor/openOMSI-Editor-0.7.17-pre/start-editor.sh" "/home/chris/Games/openOMSI-0.1.1541-linux-x64"
```

Die Installation wird neu angelegt, vorhandene Installationen bleiben erhalten.
Interaktiver Test auf der vollständigen Benutzerkarte steht noch aus.
