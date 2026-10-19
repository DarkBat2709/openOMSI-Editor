# Freie KI-Fahrwege / Free AI paths

Im Fenster **Zusätzliche KI-Verbindung** auf **Modus: vorhandene Endpunkte
verbinden** klicken. Der Schalter wechselt zu **Modus: frei zeichnen**.

1. Auf die geladene Fläche klicken: Der erste Klick setzt den Start, der zweite
   das Ende des ersten Abschnitts. Weitere Klicks fügen Abschnitte an.
2. Jeder Abschnitt hat drei verschiebbare Formpunkte. Die gemeinsam genutzten
   Abschnittsenden lassen sich ebenfalls ziehen. Die Vorschau aktualisiert sich.
3. **Letzten Punkt entfernen** nimmt den letzten Abschnitt zurück;
   **Vorschau leeren** beginnt im freien Modus neu.
4. **Verbindung hinzufügen (Enter)** übernimmt die Vorschau. **Strg+S** übernimmt
   die vollständige Vorschau und speichert; anschließend Karte neu laden.

Punkte müssen mindestens 1 m auseinanderliegen. Jeder Abschnitt darf höchstens
200 m Luftlinie und 20 m Höhenunterschied überbrücken. Weitere Abschnitte dienen
für längere Fahrwege. Die Vorschau folgt der geladenen Straße beziehungsweise
im freien Modus ersatzweise dem Gelände. Vor dem Übernehmen wird geprüft, dass
sie auf vorhandenen Kacheln liegt. Straßen werden nicht verändert.

Die freien Fahrwege sind unsichtbare Splines mit gespeicherten Kurvenknoten.
Benachbarte Abschnitte werden über ihre Enden automatisch verbunden. Für den
Anschluss an andere KI-Wege müssen Position, Höhe und Fahrtrichtung passen.
Der Modus zum gezielten Verbinden vorhandener Endpunkte bleibt verfügbar.
Der Moduswechsel verwirft die noch nicht übernommene Vorschau.

The additional AI connection window now has a **Mode: free drawing** switch.
Click the loaded surface to add a chain of sections, then drag the shared
endpoints and the three form handles of each section. **Remove last point**
removes the last section; **Clear preview** starts again. **Add connection**
commits the preview, Ctrl+S saves, and reloading activates the traffic network.
Free paths retain their curve nodes in invisible spline definitions. Existing
endpoint connections remain available in the other mode.
