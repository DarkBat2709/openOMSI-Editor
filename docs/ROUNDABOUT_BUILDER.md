# Kreisverkehrbaukasten / Roundabout builder — 0.7.8-pre

## Deutsch

Im Werkzeugfenster liegt **Kreisverkehr** direkt neben **Kreuzung bauen**.
Der Baukasten erstellt einen einspurigen Kreisverkehr mit drei oder vier
zweispurigen, geraden Zufahrten (je eine Fahrtrichtung).

1. **Kreisverkehr** öffnen und drei oder vier Zufahrten wählen.
2. Mit **Arm A–D** die Zufahrt wählen. Breite, Länge ab Mittelpunkt und Winkel
   gelten für diese Zufahrt. Die Mündungen sind leicht aufgeweitet; Ein-/Ausfahrkurven schließen tangential an.
   Inselradius, Ringfahrbahnbreite und Inselhöhe gelten
   für den gesamten Kreisverkehr. Die Länge muss mindestens Außenradius + 8 m
   betragen. Bei einer größeren Insel gegebenenfalls alle Zufahrten verlängern.
3. Asphalt und Inseltextur wählen. Ohne Texturen zeigt die Vorschau einfarbige
   Flächen. **Breite / Textur vom gewählten Spline** übernimmt die Straßenbreite
   für die gewählte Zufahrt und die Asphalttextur für den gesamten Kreisverkehr.
   Texturausschnitt U und Texturbreite haben die gewohnten Ziffern-/Plus-/Minusfelder.
4. Die Vorschau und die blauen Fahrwege prüfen. Die Fahrtrichtung wird aus
   Rechts-/Linksverkehr der Karte übernommen. **Rückgängig/Wiederholen** gilt
   für die Maße und Texturwahl. Ungültige Geometrie wird mit einer Meldung gesperrt.
5. **Speichern & platzieren**, Ziel anklicken, bei Bedarf N/M zum Drehen und U/O
   für die Höhe verwenden. **Strg+S** speichert die Platzierung in der Karte.
6. Zum Anschließen erst den Straßenspline im Splinemodus wählen, dann den
   platzierten Kreisverkehr im Objektmodus wählen und **Kreisverkehr** öffnen.
   Zufahrt wählen, **Straße mit gewähltem Arm verbinden**, Vorschau prüfen und
   **Platzierten Kreisverkehr aktualisieren**. **Strg+S** speichert die Änderungen.
7. Karte neu laden, damit KI und Verkehrsregeln neu aufgebaut werden.

Beim Speichern erhält jeder erzeugte Fahrweg native Kartenregeln: Vorfahrt 192
auf dem Ring und den Ausfahrten, 64 auf Einfahrten sowie 20 km/h. Erneutes
Speichern ersetzt nur den vom Baukasten markierten Regelblock. Andere Regeln
bleiben erhalten. Verkehrsschilder werden noch nicht automatisch gesetzt.

Eigene Baukastenobjekte können später ausgewählt und aktualisiert werden;
Position und ID bleiben erhalten, bestehende Straßenanschlüsse werden geprüft.
**Projekt laden** lädt dagegen eine Vorlage zum Platzieren eines neuen Objekts.
Die exportierten Dateien liegen wie bei Kreuzungen unter
`Sceneryobjects/openOMSI_Editor/Junctions/`. Format 2 kennzeichnet Kreisverkehre;
alte T-/X-Projekte (Format 1) bleiben lesbar.

Diese erste Version hat eine Ringfahrspur, drei/vier Zufahrten, keine
Fußgängerwege, keinen überfahrbaren Innenring und keine automatischen Schilder.
Mehrspurige und Turbo-Kreisverkehre sind nicht enthalten. Die KI-Fahrbarkeit
(insbesondere lange Busse, Einfädeln und Vorfahrt) muss in der Karte getestet
werden; geprüfte Fahrweggeometrie allein ersetzt keinen Fahrtest.

Die vorhandenen PDF-Anleitungen bleiben auf ihrem bisherigen Stand; dieses
Dokument ergänzt den neuen Baukasten. Weitere Sprachen sind zurückgestellt.

## English

Choose **Build roundabout** next to **Build junction** in the tool panel.
This first version creates a single circulating lane with three or four straight,
two-way approaches. Select Arm A–D to change that entrance's width, direction
and length from the centre. Island radius, ring width and island height are
shared. Every entrance must extend at least 8 m beyond the outer ring radius.

Choose asphalt/island textures or import the selected road's width and asphalt.
UV crop and texture width retain digit-sensitive +/- controls. Inspect the 3D
preview and AI paths, then **Save & place**, click the map and save with Ctrl+S.
Traffic direction follows the map's left-/right-hand traffic setting.

To attach a road, select its spline, switch to object mode, select the placed
roundabout and reopen the builder. Select an arm, prepare the road connection,
review it and update the placed roundabout. Save and reload the map for AI.
Updates keep the object's placement/ID and validate linked roads. Loading a
project file instead creates a template for a new placement.

Saving writes native per-path priority (192 circulating/exiting, 64 entering)
and 20 km/h speed rules. Only the builder-owned rule block is refreshed;
unrelated rules are retained. Signs, pedestrian paths, truck aprons, multiple
circulating lanes and turbo roundabouts are not generated in this version.
Drive-test long buses, merging and yielding in-game before publication.
Old junction projects remain supported. The existing PDFs are unchanged; this
file is the addendum for the new feature. Further translations are deferred.

## Verification

- Actual geometry/export code exercised with a focused local Rust harness:
  both traffic directions, 3/4 approaches, closed ring with shared entry/exit
  nodes, island clearance, native O3D/SCO roundtrip, textured export, project
  reload, undo/redo and invalid geometry rejection.
- Generated native map rules parsed with the real OMSI map parser; path counts,
  indices, priorities, repeated saves, removal and preservation of manual
  rules/sign text checked.
- Existing viewport layout regression exercised using the production layout
  functions; DE/EN translator checks cover the additional strings.
- Full application check is blocked in this environment before app compilation
  by existing ashpd/zvariant and system-library setup problems. The installer
  runs application unit tests and the release build on the user's system before
  replacing the executable. In-game mouse interaction, connected-road updates
  and real AI traffic still require manual testing.


## Neigung / Tilt (0.7.9-pre)

Pos1/Ende und Bild↑/Bild↓ neigen gesetzte Bauteile.
Home/End and Page Up/Page Down tilt placed objects.
See [controls and limitations / Bedienung und Grenzen](OBJECT_ANGLES.md).

## Verkehrsinseln je Zufahrt / Splitter islands (0.8.8-pre)

Im Kreisverkehr-Baukasten oben **Verkehrsinseln …** wählen. **Arm A–D** bestimmt,
welche Zufahrt bearbeitet wird. **Verkehrsinsel: AN/AUS** schaltet nur diese Insel;
die Formtaste wechselt zwischen **Abgerundet** und **Tropfenform**. Länge, Breite,
Abstand vom äußeren Kreisfahrbahnrand und Höhe werden in Metern eingestellt.
Die Vorschau wird nach Bestätigung einer Zahl oder nach +/− neu berechnet.
**Kreisverkehr-Maße** führt zurück zu den bisherigen Einstellungen.

Die Fahrwege werden um die Insel herumgeführt. Die Anschlussbreite und die
Fahrweg-Endpunkte am äußeren Ende bleiben erhalten. Die nutzbare Fahrbahnbreite
neben der Insel wird entsprechend reduziert; mindestens 2,75 m je Richtung sind
nötig. Zu breite, zu lange oder zu nah am Ring liegende Inseln sperren den Export
mit einer Meldung. Die Zufahrt kann auf derselben Einstellungsseite verbreitert
oder verlängert werden. Verbreiterte Zufahrten müssen zur angeschlossenen Straße
passen; bestehende Anschlüsse werden beim Aktualisieren geprüft.

Die Verkehrsinsel verwendet die gewählte Inseltextur und erhält bei aktivierter
Inselrandlinie eine eigene umlaufende Markierung. Verwende Asphalt ohne bereits
aufgemalte Mittellinie: Eine Linie in der Asphalttextur lässt sich nicht entfernen.
Änderungen sind rückgängig/wiederholbar und werden im Baukastenprojekt gespeichert.
Alte Projekte starten ohne Verkehrsinseln. Speichern/Platzieren bzw. Aktualisieren,
Strg+S und erneutes Laden der Karte übernehmen die geänderten KI-Wege und Regeln.
Fußgängerquerungen, Zebrastreifen und Ampeln sind noch nicht Bestandteil dieser
Ausbaustufe. Lange Busse müssen zusätzlich im Spiel getestet werden.

Choose **Traffic islands …**, select Arm A–D, then enable its splitter island.
Choose **Rounded** or **Teardrop** and set length, width, distance from the outer
ring edge, and height in metres. Confirm numeric input to rebuild the preview.
**Roundabout dimensions** returns to the original controls. Existing projects
load with all splitter islands disabled.

Lane centres deflect around the kerbs while the external port width and endpoints
remain fixed. Each direction needs at least 2.75 m of usable width. Invalid island
sizes or placement block export. Island texture and island edge marking settings
also apply to splitters. Use unmarked asphalt. Undo, redo, project export/reload and
native priority rules include these changes. Save the map and reload for AI.
Pedestrian crossings and traffic lights remain subsequent development steps;
vehicle clearance for long buses still requires an in-game driving test.
