# Spline-Maße (Teststand 0.8.8-pre)

Im Spline-Modus öffnet **Spline-Maße …** ein kleines verschiebbares Zusatzfenster.
An der Titelzeile ziehen. Zahlenfeld anklicken, Zahl eingeben und mit Enter bestätigen;
Dezimalpunkt und Dezimalkomma werden akzeptiert. Esc verwirft die aktive Zahleneingabe.

- **Länge (m):** Länge entlang der Spline-Mittellinie, auch in Kurven; 0,5 bis 500 m.
- **Radius (m):** 0 ist gerade; positive/negative Radien erzeugen die beiden Kurvenrichtungen.
- **Steigung (%):** gleichmäßige Steigung des gesamten Abschnitts. 4 % bedeuten 4 cm Höhengewinn pro Meter; 5 m ergeben 0,20 m Höhenunterschied.
- **Starthöhe (m):** absolute Höhe des Spline-Anfangs, kein Abstand über dem Gelände.

## Bestehenden Spline ändern

Einen einzelnen Spline auswählen, **Auswahl einlesen** drücken, die Werte ändern
und **Auf Auswahl anwenden** drücken. Das vorhandene Höhenprofil wird durch die
gewählte gleichmäßige Steigung ersetzt. Angrenzende Straßen oder separate Markierungen
werden dabei nicht automatisch umgeformt: Anschlüsse anschließend prüfen.
Strg+Z macht die Änderung rückgängig, Strg+S speichert sie.

## Einen neuen Spline mit genau 5 m legen

1. Den Spline-Typ im Katalog wählen.
2. Spline-Maße öffnen, Länge 5, Radius 0, gewünschte Steigung und Starthöhe eingeben.
3. **Neue Splines mit Maßen: AN** einschalten.
4. Wie gewohnt den Startpunkt und danach die Richtung festlegen. Der Abstand des
   zweiten Klicks bestimmt nicht die Länge; der neue Spline erhält exakt 5 m.
5. Strg+S speichern. Für freies Platzieren die Maßvorgabe wieder ausschalten.

Das Schließen des Fensters lässt eine aktivierte Maßvorgabe bestehen. Sie kann durch
erneutes Öffnen abgeschaltet werden. Die Vorgabe gilt für die Zwei-Punkt-Neuanlage,
nicht für die bestehende Fortsetzungsfunktion C. Sie wird nicht als globale Einstellung gespeichert.
Kachelübergreifende numerische Änderungen werden abgewiesen; kürzer anlegen oder den
Startpunkt verschieben. Die bestehende freie Platzierung bleibt unverändert.

## English

**Spline dimensions …** opens a draggable panel in spline mode. Enter length and
signed radius in metres (radius 0 means straight), constant gradient in percent,
and absolute start height in metres. Read the selection, edit values and apply,
or enable **New spline dimensions** before two-point placement. The second point
sets the initial direction; the entered length is measured along the curve.
Ctrl+Z undoes the change, Ctrl+S saves. Linked ends and separate markings need
checking. Closing the panel keeps a placement preset enabled until switched off.
Numeric placement currently stays within one tile and does not affect continuation C.
