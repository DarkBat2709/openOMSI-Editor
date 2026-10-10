# Editor 0.7.22-pre – zusätzliche KI-Verbindungen

Testkandidat auf Basis von openOMSI 0.2.27. Kein Update auf die neueste
Upstream-Version. Bestehende Straßen und Geradeauspfade bleiben beim Hinzufügen
unverändert. Der Installer erstellt einen separaten Editor-Ordner.

## Installation unter Linux x64

1. Paket in einen neuen Ordner entpacken; die alte Installation behalten.
2. Im neuen Quellordner `bash INSTALLIEREN.sh` ausführen.
3. Der Installer prüft die Quelldateien, kompiliert und führt die Regressionstests
   aus. Erst danach installiert er `openOMSI-Editor-0.7.22-pre` neben dem Quellordner.
4. Bei einem Fehler `update-0.7.22-pre.log` zur Prüfung schicken. Ein vorhandener
   Zielordner wird nicht überschrieben.
5. Den neuen Editor mit dem bisherigen Spielinhaltsordner starten.

## Neue Verbindung: von links nach oben

1. Karte sichern und zunächst an einer Kopie testen.
2. Im Spline-Editor einen Straßen-/Terrain-Spline in der Nähe auswählen.
3. `KI-Fahrwege entlang der Straße` öffnen.
4. Unten `Zusätzliche KI-Verbindung erstellen` auswählen.
5. Den **gelben Anschluss am Ende der ankommenden Fahrspur** anklicken.
6. Den **lila Anschluss am Anfang der abgehenden Fahrspur** anklicken. Bei einer
   Verbindung nach oben die Spur mit nach oben gerichteten Pfeilen verwenden.
7. Die zwei grünen Bogenpunkte mit der linken Maustaste ziehen. Sie verschieben
   sich entlang der jeweiligen Anschlussrichtung; dadurch bleiben die Ein- und
   Ausfahrt tangential angeschlossen. Die Vorschau zeigt den Verlauf und Pfeile.
8. Bei Bedarf `Alle Fahrzeuge` / `Nur Busse` und die Spurbreite einstellen.
9. `Verbindung hinzufügen (Enter)` klicken. Erst dieser Schritt fügt den neuen
   KI-Spline hinzu. `Schließen (Esc)` verwirft die Vorschau.
10. `Strg+S` speichern; die Karte neu laden, damit die KI das Netz neu aufbaut.
    `Strg+Z` nimmt das Hinzufügen im Editor wieder zurück.

Der Geradeausweg bleibt erhalten. Die neue Verbindung besitzt eigene,
richtungsspezifische Anschlüsse. Auch wenn die ursprüngliche Spur einen fest
zugewiesenen Nachfolger hat, kann sie zusätzlich den neuen Abbiegepfad erreichen.
Weitere Abbiegeverbindungen werden einzeln hinzugefügt.

## Grenzen dieser ersten Umsetzung

- Es werden **bestehende Fahrwegenden mit Fahrweganfängen** verbunden. Ein Klick
  mitten auf einen langen, durchgehenden Pfad erzeugt keinen neuen Verzweigungspunkt.
  Dafür wird eine zusätzliche Teilungsfunktion benötigt.
- Abstand zwischen Anschlüssen: 1–200 m; höchstens 20 m Höhenunterschied.
- Pro Verbindung eine Fahrtrichtung. Gegenrichtungen als eigene Verbindung anlegen.
- Die Bogenform lässt sich vor dem Hinzufügen einstellen. Nach dem Hinzufügen
  zum Korrigieren zunächst rückgängig machen und neu anlegen; ein spezieller
  Wiederöffnen-Dialog für gespeicherte Verbindungsbögen ist noch nicht enthalten.
- Keine automatische Ampelzuordnung oder Fahrplanerstellung. Bestehende
  Verkehrsregeln einer Kreuzung müssen bei der Kartenprüfung berücksichtigt werden.
- Mehrdeutige übereinanderliegende Anschlüsse werden abgewiesen, nicht geraten.
- Die deutsche und englische PDF-Anleitung wurden auf 0.7.22-pre aktualisiert.
  Beide enthalten die drei neuen Screenshots und eine Schrittfolge auf Seite 23.

## Prüfstand

Bei der ursprünglichen Quellpaketerstellung waren `cargo` und `rustc` nicht
verfügbar. Das spätere lokale Installationsprotokoll endet mit einer erfolgreichen
Release-Kompilierung und Installation von 0.7.22-pre. Der interaktive Kartentest
und die GitHub-Prüfungen sind gesondert zu bestätigen.
Die neuen Rust-Regressionen werden vom Installer vor der Installation ausgeführt:

- Kurvengeometrie und persistente Anschlussreferenzen;
- Geradeausweg plus zusätzlicher Abzweig bei vollständigem/inkrementellem Netzaufbau;
- Speichern, Neuladen und Rückgängig ohne Änderungen an den bestehenden Straßen;
- Busfreigabe und ungültige Anschlusswahl.

Zusätzlich bleiben die bisherigen Diagnose-, KI-Pfad-, Baukasten- und
Editor-UI-Tests aktiv. Die reparierten Umgebungsvariablen-Einträge aus dem aktuellen
GitHub-Stand sind enthalten und werden ebenfalls geprüft.

## English quick steps

Select a nearby road spline, open the AI paths panel, then choose
`Zusätzliche KI-Verbindung erstellen` (create an additional AI connection).
Click a yellow incoming lane end, then a magenta outgoing lane start. Drag the
two green curve handles, choose vehicle access and add the connection. Save and
reload the map for AI traffic. Existing straight-ahead paths remain unchanged.
This first version connects lane endpoints only, not arbitrary interior points.
