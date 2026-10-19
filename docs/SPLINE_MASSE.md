# Spline-Maße und Live-Vorschau (Teststand 0.8.8-pre-spline4)

Im Spline-Modus öffnet **Spline-Maße …** ein verschiebbares Zusatzfenster.
Zahlenfeld anklicken und tippen; gültige Eingaben aktualisieren die blaue,
durchscheinende Vorschau sofort. Enter bestätigt, Esc verwirft die aktive Eingabe.
Dezimalpunkt und Dezimalkomma funktionieren.

- **Länge (m):** Länge entlang der Mittellinie, 0,5 bis 500 m.
- **Radius (m):** 0 ist gerade, das Vorzeichen bestimmt die Kurvenrichtung.
- **Kurvenwinkel (°):** Gesamtwinkel, beispielsweise +90° oder -90°.
  Nach einer Winkeleingabe bleibt dieser Winkel bei Längenänderungen erhalten;
  der Radius wird angepasst. Nach einer Radiuseingabe bleibt stattdessen der Radius erhalten.
- **Steigung (%):** gleichmäßige Steigung des Abschnitts.
- **Starthöhe (m):** absolute Höhe, kein Abstand über dem Gelände.
  0 m kann unter dem Gelände liegen. Eine vorhandene Straße auswählen und
  **Auswahl einlesen** übernimmt unter anderem deren Höhe.

## Neue Splines setzen

1. Spline-Maße öffnen und **Neue Splines mit Maßen: AN** einschalten.
2. Spline-Typ im Katalog wählen. Die blaue Vorschau folgt dem Mauszeiger über dem Gelände.
3. **N / M** drehen die Anfangsrichtung in Schritten von 5° (Zahleneingabe vorher mit Enter abschließen).
4. Mit **WASD / Q / E** die Kamera bewegen. Diese Tasten verlassen eine aktive
   Zahleneingabe; ungültiger Text wird dabei verworfen.
5. Maus zum Maßfenster bewegen: Die Vorschau bleibt am zuletzt angezeigten Ort.
   Maße ändern und die neue Form ansehen. Es wird noch nichts in die Karte geschrieben.
6. **Ein Linksklick auf das Gelände setzt den Spline** an der angezeigten Position.
   Weitere Klicks setzen weitere Splines. Esc beendet die Platzierung.
7. **Schließen** beendet die Vorschau und Platzierung, auch bei ungültigen Zahlen.
   Danach können vorhandene Splines wieder ausgewählt und mit Entf gelöscht werden.
8. Strg+S speichert, Strg+Z macht einen gesetzten Spline mitsamt seinen Kachelabschnitten rückgängig.

Neue Splines dürfen bestehende Kachelgrenzen überschreiten. Verbundene Abschnitte
bewahren Gesamtlänge, Kurve, Steigung und Texturverlauf. Fehlt eine benötigte Kachel,
wird der gesamte Setzvorgang abgewiesen. Die Vorschau selbst verändert keine Karte.

## Vorhandenen Spline ändern

Platzierung mit Esc beenden, einen Spline auswählen und Spline-Maße öffnen.
Gültige Eingaben ändern den ausgewählten Spline automatisch; die blaue Darstellung
zeigt die neue Form sofort, der sichtbare Straßenaufbau wird nachgeladen.
Enter bestätigt das Zahlenfeld. Strg+Z macht eine zusammenhängende Feldeingabe
rückgängig. Strg+S speichert. Schließen beendet die Bearbeitung, bereits automatisch
übernommene Änderungen bleiben bestehen.

Ein Wechsel der Auswahl lädt deren Maße automatisch. Verschieben, Höhenänderungen
und Rückgängig werden in den Feldern nachgeführt, solange kein Zahlenfeld aktiv ist.
Auch beim Verlängern über bestehende Kachelgrenzen bleibt der vorhandene Spline mit
seiner ID und seinen Objektzuordnungen erhalten. Alle betroffenen Kacheln werden
aktualisiert. Fehlende Kacheln oder ungültige Werte werden im Fenster gemeldet.
Angrenzende Splines werden nicht automatisch verschoben; Anschlüsse prüfen.
Das vorhandene Höhenprofil wird durch eine gleichmäßige Steigung ersetzt.
Die Fortsetzungsfunktion C ist unverändert.

## English

**Spline dimensions …** opens a draggable modeless panel. Valid numeric input updates
a translucent blue preview immediately; Enter confirms and Esc discards pending text.
Enable new spline dimensions and choose a catalogue spline. The preview follows the
mouse over terrain. N/M rotates its initial heading by 5 degrees. WASD/Q/E moves the
camera and releases numeric input focus. Hovering the panel keeps the preview at its
last position, so dimension edits can be inspected before placement.
One left click on terrain places the spline; further clicks place more. Esc ends
placement. Close always exits the preview and placement, including invalid input.
Existing splines update automatically on valid input. One field edit forms one undo
step. Closing keeps applied changes. Selection and external edits refresh the fields.
Existing splines can extend across existing tiles while retaining their ID and attachments;
linked neighbours do not move automatically. Ctrl+S saves; Ctrl+Z undoes changes.
Height is absolute; 0 m may be below terrain. After editing angle, length changes keep
that angle; after editing radius they keep that radius instead.
