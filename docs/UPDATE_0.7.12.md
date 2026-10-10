# Editor 0.7.12-pre – KI-Fahrwege und Armkennzeichnung

Basis: das bereitgestellte Quellpaket 0.7.11-pre / openOMSI 0.2.27.
Dieses Paket enthält Quellcode und einen Linux-x64-Installer, keine fertig gebaute Programmdatei.

## Installation unter Linux

1. ZIP nach `/home/chris/Projekte/openOMSI-Editor` entpacken. Es entsteht der neue Quellordner `openomsi-source-0.2.27-editor-0.7.12-pre`.
2. In diesem neuen Ordner ein Terminal öffnen und `bash INSTALLIEREN.sh` ausführen. Internet und die bisherige Rust-Bauumgebung einschließlich ALSA-/udev-Entwicklungspaketen werden benötigt. Der erste Bau kann dauern.
3. Erst nach erfolgreichen Prüfungen und Release-Build wird daneben `openOMSI-Editor-0.7.12-pre` angelegt. Existierende Installationsordner werden nicht überschrieben. Bei einem Baufehler kann derselbe Befehl erneut ausgeführt werden; Cargo nutzt die bisherigen Ergebnisse.
4. Den neuen Editor starten:

```bash
bash "/home/chris/Projekte/openOMSI-Editor/openOMSI-Editor-0.7.12-pre/start-editor.sh" "/home/chris/Games/openOMSI-0.1.1541-linux-x64"
```

Der erste Parameter ist dein bisheriger **openOMSI-Inhaltsordner**, nicht die ursprüngliche OMSI-2-Installation und nicht der Editorordner. Bei einem anderen Inhaltsordner den Pfad entsprechend ändern. Zusätzliche bisher verwendete Startparameter können angehängt werden.
Die Anzeige im Editor muss **0.7.12-pre** lauten. Eine alte Verknüpfung startet weiterhin den alten Editor; ihr Ziel muss auf das neue Startskript zeigen.

Das Skript schreibt weder nach GitHub noch in die bestehende Installation oder in Karten. Ein Baufehler steht in `update-0.7.12-pre.log` im neuen Quellordner.

## Kreuzungen und Kreisverkehre

- A, B, C und bei vier Armen D stehen direkt an den Armenden der Vorschau.
- Der ausgewählte Arm hat eine gelbe Umrandung und eine aktive Buchstabenschaltfläche.
- Auswahl über die bisherigen Arm-Schaltflächen oder direkt über den Buchstaben in der Vorschau.
- Drehen, andere Winkel und gebogene Kreuzungsarme behalten dieselbe Zuordnung.
- Die Kennzeichnungen werden nur in der Vorschau gezeichnet; exportierte Fahrbahnen enthalten keine Buchstaben oder gelben Linien.

## KI-Fahrwege entlang einer vorhandenen Straße

1. Im Spline-Modus den Straßen-/Terrain-Spline auswählen, etwa `8mTerrain_Spline.sli`.
2. Im Werkzeugbereich **KI-Fahrwege entlang der Straße** öffnen.
3. Einzelnen Spline oder verbundene Splinekette wählen. Die Kette benötigt eindeutige, gegenseitige Splineverbindungen.
4. Eine Spur/Einbahnstraße oder zwei Spuren mit Gegenverkehr einstellen. Pfeile zeigen die Fahrtrichtung. **Richtung umgekehrt** dreht die Richtung beider Spuren um; bei zwei Spuren wechselt dadurch die Verkehrsseite.
5. Spurbreite, seitlichen Versatz und Höhe anpassen. Die Höhe bezieht sich auf die Spline-Grundkurve, nicht auf eine automatisch erkannte Asphaltoberfläche. Bei erhöhten Profilen die Vorschau entsprechend anheben.
6. Pro Richtung **Alle Fahrzeuge** oder **Nur Busse** wählen: Blau = alle, Orange = Busse. Nur Busse schließt auch Taxis aus.
7. Offene Pfadenden und Anschlüsse in der Vorschau prüfen, dann **KI-Pfade anlegen**.
8. `Strg+Z` nimmt die gesamte Einfügung zurück. `Strg+S` speichert die Karte. Danach die Karte neu laden, damit KI und Navigation das neue Netz verwenden.

Die ursprüngliche Straßen-/Terrain-Geometrie bleibt erhalten. Zusätzliche unsichtbare Splines tragen die Fahrzeugpfade. Ihre SLI-Dateien stehen im Inhaltsordner unter `Splines/openOMSI_Editor/AI`; die Karte speichert die individuellen Fahrzeugregeln. Bei Weitergabe einer Karte diese SLI-Dateien mitliefern.
**KI-Pfade anzeigen / ausblenden** zeigt vorhandene Fahrzeugpfade in der geladenen Umgebung mit Richtungspfeilen. Im Katalog unter Straßen gibt es außerdem die Kategorie **KI-Pfade** für unsichtbare Pfad-Splines. Freies Platzieren aus dem Katalog verwendet die Richtungen des Profils und erlaubt alle Fahrzeugtypen; die obige Zugangsauswahl gilt beim Anlegen entlang einer Referenzstraße.

## Grenzen dieser Version

- Die strikte Busbeschränkung `editor_bus_only` benötigt diese angepasste openOMSI-Ausführung. Unverändertes openOMSI 0.2.27 und OMSI 2 unterstützen diese Zusatzregel nicht; dort können die normalen Bus-/no_cars-Regeln auch Taxis zulassen. Keine entsprechende Garantie für andere Programme.
- Nur Busse erlaubt den Verkehr; es erzeugt keine Buslinien, Fahrpläne, Haltestellen oder Fahrzeuggruppen. Diese müssen in der Karte vorhanden sein.
- Bestehende Fahrzeugpfade werden nicht automatisch ersetzt. Das Werkzeug blockiert Referenzstraßen mit eigenen Fahrzeugpfaden und eine doppelte Generierung auf derselben Referenz. Zum Ändern rückgängig machen oder die erzeugten KI-Splines löschen und neu anlegen.
- Die zusätzlichen Pfade bleiben nachträglich eigenständige Splines. Änderungen an der Referenzstraße verschieben bereits erzeugte Pfade nicht automatisch mit.
- Anschlussprüfung: geladene Nachbarpfade und generierte Abschnitte, höchstens 0,5 m Abstand und passende Fahrtrichtung. Sie meldet offene Enden; keine automatische Erzeugung von Abbiegepfaden, Vorfahrt oder Ampelschaltungen. Kreuzungs-/Kreisverkehrbaukästen liefern ihre eigenen inneren Pfade.
- Überhöhte, seitlich verzogene und mit Profilübergängen versehene Referenzsplines werden vorerst abgelehnt. Maximal 500 Abschnitte, zusätzlich begrenzte Vorschaugröße.
- Rückgängig nimmt die Karteneinfügung zurück; unbenutzte erzeugte SLI-Dateien können im Inhaltsordner verbleiben.
- Ein interaktiver Test mit deiner Karte und deinen Fahrzeugen steht noch aus. Die mitgelieferte Prüfnotiz dokumentiert die hier tatsächlich ausgeführten Checks.
