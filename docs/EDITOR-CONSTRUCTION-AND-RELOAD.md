# Lokale Erweiterung: Fahrgäste beim Neuladen und Höhenwerkzeuge

Diese Änderungen erweitern die lokale Entwicklungsfassung nach 0.7.23-pre.

## Neuladen während einer Fahrt

Beim Neuladen im Einzelspieler-Editor bleiben die Fahrgäste im gefahrenen Bus erhalten: Identität, Kleidung, Sitz-/Stehplatz, Bewegung in der Kabine, Ziel, Aussteigewunsch, Ticketzustand und Kilometerstand. Ein laufender Fahrscheinverkauf einschließlich Geld auf dem Kassentisch wird übernommen. Wartende Fahrgäste und Personen auf dem Weg zu ihrem Warteplatz werden an weiterhin geladenen Haltestellen übernommen, auch bei Verschiebungen bis 30 Meter. Ihre Warteplätze werden gegen die neue Karte abgeglichen. Bei kleinen Verschiebungen werden passende Warteplätze an der neuen Haltestelle verwendet; falls die alten Wartepunktobjekte außerhalb des neuen Haltestellenbereichs liegen, werden für die übernommenen Personen vorübergehende Plätze relativ zur neuen Position angelegt. Personen an gelöschten oder um mehr als 30 Meter versetzten Haltestellen werden nicht an ihren alten Stellen wiederhergestellt. Andere Fußgänger und KI-Busfahrgäste werden für die neu geladene Karte neu erzeugt. Grafikinstanzen des alten Kartenzustands werden nicht wiederverwendet.

Die Fortsetzung bleibt im Arbeitsspeicher. Sie erweitert nicht das normale Speichern/Laden einer Spielsituation und nicht LAN-Sitzungen. Der identische Bus muss beim Neuladen wiederhergestellt werden können. Gelöschte Haltestellen oder veränderte Linienführungen können weiter eine Anpassung des aktiven Linienauftrags erfordern.

Test im Spiel: Tour mit mehreren Fahrgästen starten, Bus an sicherer Stelle anhalten, Editor öffnen, kleine Änderung speichern und Karte neu laden. Fahrgäste und Fahrscheinverkauf prüfen; danach weiterfahren und den regulären Ausstieg beobachten. Eine Prüfung im echten Fahrbetrieb ist zusätzlich zu den automatischen Tests erforderlich.

## Brücke / Tunnel / Untergrund

Im Werkzeugpanel den neuen Eintrag öffnen. Im Spline-Modus zuerst die Straße auswählen.

- Start- und Endhöhe sind absolute Kartenhöhen in Metern. Die Steigungen an beiden Enden werden getrennt eingestellt. Zahlenfelder lassen sich anklicken und mit Enter bestätigen; Dezimalkomma ist zulässig.
- Brücke +5 m / Tunnel -5 m schlagen Höhen relativ zum Gelände an beiden Endpunkten vor. Die Vorgabe wird erst mit „Straßenhöhen übernehmen“ angewendet. Das Gelände selbst wird dabei nicht verändert. Steigungen über 20 Prozent werden auch innerhalb einer Rampe abgelehnt.
- Die bestehende Straße behält Profil, Texturen und KI-Pfade. Höhenanpassung an das Gelände wird für sie abgeschaltet. Verknüpfte Nachbarstraßen bleiben bestehen; ihre Enden anschließend mit den Verbindungsmöglichkeiten des Editors passend anschließen.
- „Arbeitsebene“ legt die Höhe für das Platzieren neuer Objekte und Splines sowie das Ziehen fest. Das funktioniert auch unter dem Gelände. Die Ebene bleibt nach dem Schließen des Fensters aktiv und wird im Editor-Hinweis angezeigt. Im selben Fenster wieder ausschalten, um normal auf dem Gelände zu platzieren.
- „Gelände ausgeblendet“ betrifft nur die Darstellung im geöffneten Editor. Gelände, Kollisionen und gespeicherte Karte bleiben unverändert. Nach Verlassen des Editors oder Öffnen des Spielmenüs ist das Gelände wieder sichtbar.
- Brückendeck, Geländer, Stützen, Tunnelwände und Portale aus dem installierten Katalog auswählen. Geländeöffnungen erfordern geeignete Assets mit terrainhole-Unterstützung oder eine modellierte offene Zufahrt. Die Höhenwerkzeuge erzeugen diese Bauteile nicht automatisch.

Straßenhöhen können mit Strg+Z rückgängig gemacht und mit Strg+S gespeichert werden. Die Arbeitsebene und das Ausblenden sind Ansichts-/Bedienoptionen, keine Änderungen der gespeicherten Karte.

## Verschiebbare Editorfenster

Fenster oben an ihrer Titelleiste mit gedrückter linker Maustaste ziehen. Das gilt auch für Katalog und Texturdetails, Kartenkacheln, Objekttexte, Kreuzungs- und Kreisverkehrbau, KI-Pfade, zusätzliche KI-Verbindungen, Verbindungsprüfung und das neue Höhenfenster. Schaltflächen und Vorschauen wandern mit; Schließen-Schaltflächen bleiben bedienbar. Die Fenster bleiben innerhalb des sichtbaren Bereichs und passen ihre Position beim Ändern der Fenstergröße an. Die größeren Dialoge behalten jeweils ihre eigene Position für die laufende Sitzung; die bestehende Werkzeugleiste und Straßen-/Geländepanels nutzen weiterhin ihre gemeinsame Dockposition.

## Nachbesserung nach Fahrversuch am 12.10.2026

Die Wiederherstellung übernimmt zusätzlich den laufenden Motor-/Getriebezustand einschließlich dynamischer Engine-Variablen, Skript-VM, Verschleißfaktor, Kilometerstand, Fahrphysik und Federungszustand. Kartenabhängige Abfragen und Grafikressourcen kommen aus dem neu aufgebauten Zustand. Beim Neuaufbau entstandene, noch ausstehende Kollisions-/Soundereignisse werden verworfen. Vorhandene technische Defekte werden übernommen, nicht durch eine automatische Reparatur beseitigt. Das Log schreibt die wichtigsten Motor-/Getriebe- und Verschleißwerte vor und nach dem Neuladen sowie die Zahl übernommener Wartender.

Erneuter Fahrversuch: einen betriebsbereiten Bus vor einer besetzten Haltestelle anhalten, Karte im Editor speichern/neuladen, Warnanzeigen und Wartebereich prüfen, dann Türen öffnen und Einsteigen prüfen.

Beim Neuladen bleiben die aktive Ansicht, die gewählte Innenkamera und die Sitzposition erhalten. Wer aus der Fahreransicht neu lädt, kehrt unmittelbar in die Fahreransicht zurück. Wer in der freien Editoransicht arbeitet, bleibt dort; F1 führt wie gewohnt zurück zum Fahrerplatz. Wartepunktobjekte und Unterstände sind weiterhin eigenständige Kartenobjekte. Für eine dauerhafte Verlegung einer Haltestelle auch diese Objekte passend platzieren; die Neuladeübernahme verschiebt vorhandene Wartende für die laufende Sitzung.

Das Brücken-/Tunnelfenster erlaubt Kamerabewegungen mit W/A/S/D und Q/E auch bei markierter Straße. Während der Eingabe eines Zahlenwerts sind die Bewegungstasten gesperrt; Enter oder Esc beendet die Eingabe. Umschauen erfolgt mit gedrückter rechter oder mittlerer Maustaste.
