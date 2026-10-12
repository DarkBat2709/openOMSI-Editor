# Objektneigung / Object tilt — 0.7.9-pre

## Deutsch

Im Objektmodus lassen sich gesetzte Kreuzungen und Kreisverkehre aus dem
Baukasten neigen. Das funktioniert auch nach dem erneuten Laden einer Karte,
wenn die zum Bauteil gehörende `junction.junction.json` noch vorhanden ist.
Andere Objektarten werden von dieser Neigungsfunktion noch nicht unterstützt.

| Taste | Funktion | Schritt / mit Umschalt |
|---|---|---|
| N / M | Drehen | 5° / 0,5° |
| Pos1 / Ende | Längsneigung erhöhen / verringern | 0,5° / 0,05° |
| Bild↑ / Bild↓ | Querneigung erhöhen / verringern | 0,5° / 0,05° |
| U / O | Höhe verringern / erhöhen | 0,5 m / 0,05 m |
| Rücktaste | Neigung auf den Stand vor der ersten Neigungsänderung dieser Bearbeitung zurücksetzen | danach wieder normale Rücksetzfunktion |
| Entf | Löschen / Wiederherstellen | keine Neigungsfunktion |
| Strg+S | Karte speichern | anschließend Karte neu laden, um KI-Fahrwege zu aktualisieren |

1. Im Objektmodus ein Bauteil anklicken/auswählen.
2. Pos1/Ende und Bild↑/Bild↓ verwenden. Die Infozeile zeigt beide Winkel an.
   Die Achsen beziehen sich auf das Bauteil, nicht auf die Kamerarichtung.
3. Bei Bedarf mit Umschalt fein einstellen und mit N/M drehen oder verschieben.
4. Speichern und die Karte neu laden. Die Winkel bleiben gespeichert.

Beim Kopieren und erneuten Platzieren werden die Winkel übernommen; während
der Platzierung funktionieren die Neigungstasten ebenfalls. Die Neigung ist
auf −89° bis +89° begrenzt. Nach Löschen kann Entf das Bauteil wiederherstellen;
anschließend setzt Rücktaste die Neigung zurück. Rücktaste auf einem gelöschten
Bauteil verwendet zunächst die bisherige Wiederherstellungs-/Rücksetzfunktion.

**Ab 0.7.10-pre:** Explizit angeschlossene Straßen folgen dem Bauteil beim
Verschieben, Drehen und Neigen. Das entfernte Ende bleibt stehen. Rücktaste
setzt die zusammengehörige Lageänderung mit den Straßen zurück. Unmögliche
Anpassungen werden abgelehnt. Einzelheiten: [Straßenanschlüsse](JUNCTION_CONNECTIONS.md).
KI-Fahrwege werden nach Speichern und Neuladen aktualisiert. Keine Weltkoordinatenkarten.

Im Spline-Modus bleiben ,/. für die Krümmung und Pos1/Ende für die Steigung
zuständig. Diese Anleitung ergänzt die vorhandenen PDFs; diese sind in diesem
Update nicht neu erstellt worden.

## English

In object mode, placed builder junctions and roundabouts can be tilted, including
after reloading the map. Keep the asset's `junction.junction.json` file alongside
it. Other object types are not yet supported by the tilt controls.

| Key | Operation | Step / with Shift |
|---|---|---|
| N / M | Rotate | 5° / 0.5° |
| Home / End | Increase / decrease pitch | 0.5° / 0.05° |
| Page Up / Page Down | Increase / decrease bank | 0.5° / 0.05° |
| U / O | Lower / raise | 0.5 m / 0.05 m |
| Backspace | Restore tilt before the first tilt change of this edit session | subsequent presses use the original reset behaviour |
| Delete | Delete / restore | no tilt operation |
| Ctrl+S | Save map | reload the map to update traffic paths |

Select the object, adjust its pitch/bank and check the values in the information
bar. Axes follow the object, not the camera. Moving or turning preserves tilt.
Copying and placement preserve it too; the same keys also work during placement.
Both angles are limited to −89°…+89°. After deletion, use Delete to restore the
object before resetting tilt. Backspace on a deleted object first uses the
existing restore/reset behaviour.

**Since 0.7.10-pre:** Explicitly connected roads follow movement, rotation and tilt.
Their far endpoints stay fixed. Backspace restores the combined pose and road
changes. Impossible fits are rejected. See [road connections](JUNCTION_CONNECTIONS.md).
Traffic paths refresh on map reload. Only standard OMSI maps are supported.
Spline-mode curvature and gradient keys are unchanged.

This supplements the existing PDF guides; the PDFs have not been regenerated.
