# Editor-Start – 0.7.5-pre (Entwicklung)

Im Launcher steht **Editor** direkt unter **Sitzungen**, vor **Mods**.
Fahren und Mehrspieler bleiben zusammen.

## Neue Karte

1. Öffne **Editor**.
2. Gib einen **Kartennamen** ein. **Autor** und **Beschreibung** sind optional.
3. Prüfe den angezeigten **Speicherort**. Neue Karten werden im `maps`-Ordner
   des aktiven openOMSI-Spielinhaltsordners angelegt. Der Original-OMSI-2-Ordner
   wird dafür nicht verwendet. Der Kartenname bestimmt auch den Ordnernamen.
4. Klicke **Karte erstellen und bearbeiten**.

Die Karte startet mit einer ebenen 300 × 300 m großen Kachel, einer generierten
Grundtextur und freier Kamera direkt im Editor. Es werden keine Busse oder
Fahrpläne benötigt. Weitere Kacheln ergänzt du mit dem vorhandenen Tile-Werkzeug.
Autor und Beschreibung stehen in der Kartenbeschreibung in `global.cfg`.
Vorhandene Kartennamen (auch mit anderer Groß-/Kleinschreibung) werden abgelehnt.

## Vorhandene Karte

Wähle unter **Vorhandene Karte bearbeiten** eine Karte und klicke **Im Editor
öffnen**. Sie wird mit freier Kamera ohne ausgewählten Bus oder Fahrplan geöffnet.
Die bisherigen Speicherfunktionen und der Editor während einer Fahrt bleiben erhalten.
Wenn bereits eine Sitzung läuft, beende sie vor dem direkten Editorstart.

## Prüfen vor Veröffentlichung

- Neue Karte mit Umlaut und Leerzeichen anlegen, Terrain sichtbar, Kamera beweglich.
- Gelände ändern, speichern, schließen und die Karte erneut im Editor öffnen.
- Objekt/Spline platzieren und eine weitere Kachel anlegen; speichern und erneut laden.
- Gleichen Kartennamen erneut versuchen: vorhandene Karte bleibt unverändert.
- Eine bestehende Karte direkt öffnen, anschließend normalen Fahrmodus testen.
- Linux und Windows: CI-Prüfungen und Release-Build ausführen.

Die PDF-Anleitungen für 0.7.4-pre beschreiben die bisherigen Werkzeuge. Diese
Ergänzung beschreibt den neuen Einstieg; die PDFs wurden noch nicht überarbeitet.

# Editor start – English

Choose **Editor** below **Sessions** in the launcher. Enter a map name and,
optionally, an author and short description. The displayed destination is the
active openOMSI content folder's `maps` directory. Click **Karte erstellen und
bearbeiten** to create a flat, single-tile map and open it with a free camera.
Existing map folders are never overwritten. **Im Editor öffnen** opens the
selected existing map without a bus or driving duty. The in-game editor remains
available. This development change still requires interactive validation.

## Ergänzung 0.7.6-pre: Letzte Editorposition

Der Editor merkt sich pro Karte die Kameraposition, Blickrichtung und den Zoom beim
Speichern, Ausschalten des Editors und normalen Beenden der Sitzung. Beim direkten
Start über **Editor** wird diese Position wiederhergestellt. Die Einstellung liegt
lokal im openOMSI-Benutzerdatenordner (`editor-views`), nicht in der Karte.
Ein normaler Fahrstart und explizite Kameraangaben bleiben unverändert.
Ohne gültige gespeicherte Position startet die Karte wie bisher. Ein Absturz kann
den letzten noch nicht gemerkten Standort verlieren.

Zum Prüfen: zu einer markanten Stelle fliegen, speichern und die Sitzung beenden.
Die Karte direkt im Editor erneut öffnen: Standort und Blickrichtung sollten
übereinstimmen. Eine zweite Karte muss ihren eigenen Standort behalten.

Die Startkamera wird bei neuen und vorhandenen Karten mindestens 35 m über die
Geländehöhe an ihrem Standort gesetzt. Eine höhere gemerkte Position bleibt erhalten.
Ohne Lesezeichen blickt sie wie die neue Karte mit 30° Neigung schräg nach unten. Liegt der alte Standort außerhalb der
vorhandenen Kacheln, wird die nächste vorhandene Kachel gewählt.
