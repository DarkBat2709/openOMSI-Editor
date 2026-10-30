# Editor 0.8.9-pre – openOMSI 0.3.45

Lokale Vorabversion. Die bisherige Installation bleibt als Rückfallmöglichkeit erhalten.

## Neu: Haltestellen, Startpunkte, Routen und Linien

Im Objektmodus des Editors stehen „Bushaltestelle platzieren“, „Startpunkt platzieren“ und „Routen und Linien …“ bereit. Die originalen OMSI-Hilfsobjekte aus Sceneryobjects/Generic müssen installiert sein.

1. Straßen und durchgehend verbundene KI-Fahrwege anlegen.
2. Bushaltestellen platzieren. Über „Text bearbeiten“ Namen, Einsteigerzahlen, Aussteigergewicht, Haltestellenlänge und die Einstiegsseite einstellen. Haltestellen nahe an der richtigen Fahrspur platzieren, damit die Zuordnung eindeutig ist.
3. Einen Startpunkt platzieren, ausrichten und benennen. Er wird beim Speichern auch in die Startpunktliste der Karte übernommen.
4. Karte speichern und neu laden. Das Routenfenster liest den gespeicherten Fahrwegstand.
5. „Routen und Linien …“ öffnen. Linie, eindeutigen Routennamen, Abfahrt (HH:MM) und Fahrtdauer in Minuten eingeben. Eine KI-Gruppe ist optional; für planmäßige KI-Fahrzeuge muss deren Name zu einer vorhandenen Gruppe der Karte passen.
6. Die Haltestellen der Reihe nach anklicken. Bei fehlenden Fahrwegverbindungen wird der nächste Abschnitt abgewiesen. „Letzte Haltestelle entfernen“ korrigiert die Auswahl.
7. „Route und Linie speichern“ erzeugt native TTData-Dateien: Fahrt (.ttp), Linie mit einem neuen Umlauf (.ttl) und fehlende Haltestellenverbindungen (StnLinks.cfg). Bestehende Umläufe bleiben erhalten. Neue Routen benötigen einen eigenen Namen. Die Gegenrichtung wird als eigene Route angelegt.
8. Karte neu laden und den neuen Umlauf im Fahrplan auswählen. Den vollständigen Verlauf abfahren und Fahrtdauer/Anschlüsse kontrollieren.

## Umfang und Grenzen

Dies ist ein erster Routen- und Linieneditor. Der Fahrweg zwischen den Haltestellen wird automatisch aus dem gerichteten Straßennetz ermittelt. Eine Auswahl alternativer Wege, Zwischenpunkte, bestehender Umläufe zur Bearbeitung und ein grafischer Liniennetzplan sind noch nicht enthalten. Fahrtdauer und Abfahrt werden vom Benutzer festgelegt; es wird kein vollständiger Tagesfahrplan automatisch erzeugt.

Die HOF-Datei eines Busses wird nicht automatisch ergänzt. Für IBIS-Routencodes, Zielanzeigen und Ansagen sind weiterhin passende HOF-Daten und gegebenenfalls Ansagedateien erforderlich. Ein fahrbarer Umlauf ersetzt diese Fahrzeugdaten nicht.

Beim Speichern werden vorhandene bearbeitete Dateien mit .before-editor-Zeitstempel gesichert. Original-OMSI-Dateien dürfen nicht das Schreibziel sein. Vor umfangreichen Änderungen ist eine vollständige Kartensicherung sinnvoll.

## Technische Anpassungen

Die openOMSI-Änderungen bis v0.3.45 wurden integriert. Der eigene Editor bleibt in omsi-app; gemeinsame Szenen- und Diagnosefunktionen verwenden die neue Struktur. Angepasst wurden unter anderem Kartenladen, Terrainanpassung, Lichtzuordnung, Verkehrsstrukturen, Oberfläche, Übersetzungen und Launcher.

Die Linux-Version wird lokal geprüft. Ein erfolgreicher Linux-Build bestätigt keine Windows-Laufzeitprüfung. Diese Vorabversion ist noch keine veröffentlichte Version.
