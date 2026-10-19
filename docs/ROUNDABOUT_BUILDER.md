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
die Formtaste wechselt zwischen **Abgerundet** und **Tropfenform**. Bei der
Tropfenform zeigt die schmale Spitze zur anschließenden Straße, die breitere
Seite zum Kreisverkehr. Länge, Breite,
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
The teardrop tapers towards the external road, with its wider shoulder near the ring.
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

## Abgerundeter Keil nach Bildvorlage (0.8.8-pre-islands4)

Die zusätzliche Form **Abgerundeter Keil** hat einen breiten, quer zur Zufahrt
liegenden Kopf am Kreisverkehr und ein schmaleres, abgerundetes Straßenende.
Unter **Verkehrsinseln … → Insel nach Vorlage** wird sie für den gewählten Arm
aktiviert: 9 m Länge, 1,50 m Breite und 1 m Abstand zum äußeren Ringrand. Die
Inselhöhe bleibt erhalten. Anschließend lassen sich alle Maße weiter ändern.
Die Vorlage verändert nur den gewählten Arm und lässt sich rückgängig machen.

Bei dieser Form ist ein Abstand ab 0,75 m möglich. Die Fahrwege werden auch im
Übergang zur Kreisfahrbahn um die Insel geführt; äußere Straßenanschlüsse bleiben
an ihrer bisherigen Position. Ungültige Kombinationen von Maßen werden weiterhin
abgelehnt. Die Randlinie folgt auch dem breiten Inselkopf. Fußgängerquerungen,
Zebrastreifen, Sperrflächen und Ampeln aus den Referenzbildern werden durch diese
Formauswahl noch nicht erzeugt.

The **Rounded wedge** has a broad head facing the roundabout and a narrower,
rounded road end. **Traffic islands … → Reference splitter island** enables a
9 m × 1.5 m island with 1 m ring clearance on the selected arm, keeping its height.
All dimensions remain editable. This shape allows at least 0.75 m ring clearance;
lane deflection also follows the curved approach, keeping external ports fixed.
The preset is undoable. Crossings, hatching and traffic signals are separate features.

## Bordsteine / Kerbs (0.8.8-pre-islands5)

Die Kreisverkehr-Mittelinsel und alle drei Verkehrsinsel-Formen erhalten einen
umlaufenden betongrauen Bordstein mit Seitenwand und leicht abgeschrägter Oberkante.
Die Gras-/Pflastertextur bleibt auf der inneren Inselfläche. Der Bordstein wird auch
bei ausgeschalteter weißer Inselrandlinie erzeugt und übernimmt die jeweilige
Inselhöhe. Er liegt innerhalb der bisherigen Kontur; die Fahrwege bleiben erhalten.
Bereits platzierte Baukastenobjekte im Baukasten aktualisieren und die Karte speichern.

The central island and all splitter shapes have concrete-grey kerbs with vertical
faces and bevelled crowns, independent of painted edge lines. Island surface textures
stay on the inset top. Kerbs follow island height and remain inside the existing
footprint. Update previously placed builder objects and save the map to apply them.

### Bauablauf im Teststand 0.8.8-pre-islands6

Der Baukasten ist jetzt in vier direkt anwählbare Schritte gegliedert:

1. **Kreisform:** Anzahl der Zufahrten, Radius und Höhe der Kreisinsel sowie Breite der Ringfahrbahn einstellen. Der Außendurchmesser wird aus Radius und Ringbreite berechnet angezeigt.
2. **Zufahrten:** Arm A–D wählen und dessen gesamte Fahrbahnbreite, Länge ab Mittelpunkt und Richtung einstellen. Die Breite umfasst beide Fahrtrichtungen einschließlich einer gegebenenfalls vorhandenen Verkehrsinsel. Hier kann auch das Profil des gewählten Splines übernommen werden.
3. **Inseln:** Für den gewählten Arm die Verkehrsinsel einschalten, Form und Maße einstellen oder „Insel nach Vorlage“ verwenden. Die Bordsteine werden mit der Inselgeometrie erzeugt.
4. **Ausstattung:** Asphalt und Inselbelag wählen, Randlinien und gestrichelte Einfahrten einstellen. Zwischen Linienabständen und Texturausschnitt kann umgeschaltet werden. Die blauen KI-Wege in der Vorschau prüfen und anschließend „Speichern & einsetzen“ wählen.

Die Schritte können jederzeit gewechselt werden. Gültige laufende Eingaben werden beim Wechsel übernommen; ungültige Eingaben bleiben zum Korrigieren offen. Ein Seitenwechsel ändert keine Geometrie. Vorschau, Armwahl und Rückgängig bleiben verfügbar.

Die Gliederung orientiert sich an [stvo2Go: Kreisverkehre sicher bauen, beschildern und markieren](https://www.stvo2go.de/kreisverkehr-bauen-beschildern-markieren/). Es handelt sich um eine Bedienungsverbesserung des vorhandenen einstreifigen Baukastens, nicht um eine vollständige Umsetzung aller dort beschriebenen Bauformen und Entwurfsregeln. Fußgängerüberwege, Radfurten, Schilder, Ampeln, separate Ein-/Ausfahrbreiten und ein überfahrbarer Innenring werden in diesem Teststand nicht neu erzeugt.

### Getrennte Spuranzahlen (Teststand 0.8.8-pre-lanes1)

Unter **1 Kreisform** stellt „Spuren im Kreis“ 1 bis 4 umlaufende Spuren ein. Unter **2 Zufahrten** gilt „Spuren je Richtung“ separat für den gewählten Arm A-D: beispielsweise 2 bedeutet zwei Einfahrspuren und zwei Ausfahrspuren. Kreisfahrbahn und Arme dürfen unterschiedliche Spuranzahlen haben.

Beim Ändern der Spuranzahl wird die jeweilige bisherige Breite pro Spur beibehalten, soweit die Breitenbegrenzung dies zulässt. Die Ringbreite und die gesamte Armbreite bleiben anschließend manuell einstellbar. Ein größerer Ring verlängert die Arme bei Bedarf. Pro Spur werden mindestens 2,75 m Platz geprüft; Inseln beanspruchen zusätzlich Platz. Sehr breite Zufahrten brauchen ausreichend Ringbreite und Abstand zu benachbarten Armen. Unpassende Geometrie wird mit einer Meldung zurückgewiesen.

Für jede Spur werden eigene KI-Wege erzeugt. Unterschiedliche Spuranzahlen erhalten Ein-/Ausfahrverbindungen; zusätzliche Übergänge zwischen benachbarten Ringspuren ermöglichen das Erreichen aller Ausfahrten. Die Prioritätsregeln unterscheiden Einfahrt und Kreisverkehr auch bei unterschiedlichen Spuranzahlen. Dies ist ein geometrisch geprüfter Teststand; das Verhalten im tatsächlichen KI-Verkehr ist noch im Spiel zu prüfen.

Unter **4 Ausstattung** schaltet „Spurtrennlinien“ die automatisch erzeugten gestrichelten Spurtrenner im Ring und auf mehrspurigen Armen ein oder aus. Bereits aufgemalte Linien in einer Asphalttextur bleiben unverändert; für diese Funktion unmarkierten Asphalt verwenden. Vorhandene Straßensplines werden durch das Verändern der Spuranzahl nicht automatisch durch ein anderes Straßenprofil ersetzt: Anschlussbreite und Profil der bestehenden Straße müssen weiterhin passen.

Die Spuranzahlen werden im Projekt gespeichert, sind rückgängig machbar und erscheinen nach erneutem Öffnen wieder. Alte Projekte ohne diese Angaben laden mit jeweils einer Spur. Die Abbildungen in den PDF-Handbüchern zeigen noch den vorherigen Teststand islands6.

### KI-Pfadverknüpfung im Teststand lanes2

Erzeugte Kreuzungen tragen jetzt `[editor_exact_path_links]` in ihrer SCO-Datei.
Innerhalb desselben platzierten Objekts verbindet openOMSI nur gemeinsame
Pfadendpunkte (2 mm Rundungstoleranz). Die bisherige allgemeine Toleranz von
1,5 m konnte die 0,25–0,5 m langen Exportsegmente mitten in anderen Kurven
verknüpfen. Anschlüsse zu Straßen und anderen Objekten behalten ihre Toleranz.
Dies gilt sowohl beim vollständigen Laden als auch beim Nachladen von Kacheln.

Bereits platzierte Kreisverkehre müssen im Baukasten einmal aktualisiert,
gespeichert und die Karte neu geladen werden, damit ihre SCO-Datei die neue
Kennzeichnung enthält. Bestehende OMSI-Objekte ohne Kennzeichnung behalten das
bisherige Verhalten. Die Korrektur verhindert unbeabsichtigte Verbindungen;
sie garantiert keine bestimmte Ausfahrt nach einer festen Zahl von Runden.

### KI-Durchfahrt im Teststand lanes3

Der SCO-Export ergänzt `[editor_path_group]` (erster nativer Pfadindex, Anzahl).
openOMSI fasst zusammenhängende, gleich geregelte Segmente einer erzeugten
Fahrkurve beim Laden zusammen. Die nativen `[path]`-Einträge und ihre Regelindizes
bleiben im Export bestehen. Ungültige Gruppen, Lücken und unterschiedlich
geänderte Regeln werden nicht zusammengefasst. Dadurch sieht die begrenzte
KI-Vorausplanung vollständige Kurven statt nur weniger Meter Miniabschnitte;
Konflikt- und Vorfahrtsprüfungen erhalten längere Fahrwege.

Zufallsverkehr legt sich innerhalb einer gruppierten erzeugten Kreuzung auf
eine erreichbare Ausfahrt fest und folgt einem kürzesten Weg dorthin, statt an
jedem Kreisknoten erneut abzubiegen. Gesperrte Fahrwege bleiben ausgeschlossen;
Fahrplanrouten behalten ihren eigenen Verlauf. Weiterführende Ausfahrten werden
gegenüber kurzen Sackgassen bevorzugt. Bei isolierten Testobjekten bleiben
Endausfahrten nutzbar. Bestehende Kreisverkehre einmal im Baukasten aktualisieren,
speichern und die Karte neu laden, damit die Gruppierung wirksam wird.
