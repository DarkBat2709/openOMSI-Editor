# Editor 0.8.8-pre — Splines, Kreisverkehre und freie KI-Fahrwege

## Neu und verbessert gegenüber 0.8.7-pre

- **Spline-Maße:** Neues verschiebbares Fenster für Länge, Radius, Kurvenwinkel, Steigung und absolute Starthöhe.
- **Spline-Vorschau:** Neue Splines vor dem Platzieren an der Maus ansehen. Geänderte Maße aktualisieren die Vorschau; ein Klick setzt den Spline. Kamerabewegung bleibt außerhalb von Texteingaben verfügbar.
- **Vorhandene Splines bearbeiten:** Maße direkt ändern und danach mit den normalen Werkzeugen weiterarbeiten. Platzieren und Bearbeiten funktionieren auch über vorhandene Kachelgrenzen hinweg.
- **Kreisverkehr-Baukasten:** Bedienung in vier zusammenhängenden Schritten: Kreisform, Zufahrten, Inseln und Ausstattung.
- **Verkehrsinseln je Zufahrt:** Einzeln ein-/ausschalten; Form, Länge, Breite, Abstand zum Ring und Höhe einstellen. Abgerundete Inseln, Tropfen und abgerundete Keile mit Vorlageneinstellung.
- **Bordsteine und Markierungen:** Betonbordsteine an Kreis- und Verkehrsinseln; Fahrwege und Markierungen berücksichtigen die Inselgeometrie.
- **Mehrspurige Kreisverkehre:** 1 bis 4 Spuren auf der Kreisfahrbahn und separat einstellbare Spuranzahl je Zufahrt. KI-Wege passen sich an.
- **KI-Verkehr im Kreisverkehr:** Genauer verbundene Fahrwegenden, zusammenhängende Fahrkurven und geplante Ausfahrten verbessern Fahrverhalten, Vorausschau und Konflikterkennung. Unnötiges mehrfaches Kreisen wurde reduziert.
- **Freie KI-Verbindungen:** Neue Wege abschnittsweise auf der geladenen Fläche zeichnen, ohne zuerst zwei bestehende Anschlüsse wählen zu müssen. Jeder Abschnitt hat drei Formpunkte zusätzlich zu Start und Ende.
- **Verbindungsvorschau:** Formpunkte und Endpunkte ziehen, letzten Abschnitt entfernen oder Vorschau leeren. Erst Hinzufügen übernimmt die Verbindung; anschließend speichern und neu laden.
- **Speichern und Neuladen korrigiert:** Gelöschte Kreisverkehr-Fahrkurven erscheinen nicht mehr als kurze Reststücke. Gespeicherte Formänderungen werden auf die vollständige Fahrkurve angewendet.
- **Anleitungen:** Deutsche und englische PDF mit aktuellen Bildern, neuen KI-Anleitungen, geordneten Kapiteln und wiederhergestellter ursprünglicher Einleitung; jeweils 39 Seiten.

Die openOMSI-Basis bleibt **0.3.33**. Landkarten-/Luftbild-Unterlagen sowie automatisch erzeugte Fußgängerüberwege, Schilder und Ampeln sind noch nicht enthalten. KI-Verbindungen erzeugen keine automatische Ampelsteuerung. Karten vor dem Update sichern und nach Änderungen speichern, neu laden und den Verkehr prüfen.

## English — changes since 0.8.7-pre

- Movable spline dimensions window for length, radius, curve angle, gradient and absolute start height.
- Live mouse preview before placement, updated as dimensions change; camera movement remains available outside text inputs.
- Edit existing splines numerically and with ordinary tools across existing tile boundaries.
- Four-step roundabout builder: Circle, Approaches, Islands and Finish.
- Per-approach splitter islands with configurable shape, length, width, ring distance and height, including rounded, teardrop and rounded-wedge shapes.
- Concrete kerbs for central and splitter islands; island-aware paths and markings.
- Independently configurable 1–4 ring lanes and approach lanes, with matching AI paths.
- Improved roundabout endpoint connectivity, complete driving curves, exit planning, lookahead and conflict detection to reduce unnecessary repeated laps.
- Free AI path drawing as a chain of sections, with three form handles plus start/end per section; existing endpoint connection mode remains available.
- Editable connection preview, remove-last-point and clear-preview controls. Add before saving, then reload to update traffic.
- Fixed reloading of deleted and reshaped grouped roundabout paths: deleted curves no longer return as short fragments.
- Updated German and English PDF guides, 39 pages each, including new screenshots and the original introduction.

Engine base remains **openOMSI 0.3.33**. Map/aerial-image overlays and automatically generated crossings, signs and traffic lights are not included. AI connections do not generate traffic-light control. Back up maps before updating; save, reload and check traffic after editing.

## Download / Installation

Windows x64: ZIP vollständig in einen neuen beschreibbaren Ordner entpacken. Linux x64: TAR.GZ separat entpacken; glibc 2.39 oder neuer erforderlich. Originale OMSI-2-Inhalte werden benötigt und sind nicht enthalten. Spiel und Editor verwenden getrennte Updatekanäle.

Windows x64: extract the entire ZIP to a new writable folder. Linux x64: extract the TAR.GZ separately; glibc 2.39 or newer required. Original OMSI 2 content is required and not included. Game and editor update channels remain separate.
