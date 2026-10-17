# openOMSI Editor 0.8.4-pre

Engine base: openOMSI 0.2.50. Windows x64 and Linux x64 prerelease.

## Deutsch

- Asphalt und separate Markierungen gemeinsam auswählen, formen und mit C weiterbauen; Gruppen-Auswahl und Rückgängig verbessert.
- Länge, Radius, Steigung und Höhe des aktiven Splines auch bei Mehrfachauswahl anzeigen.
- Straße an Gelände für gemeinsam ausgewählte Ebenen; gelöschte Nachbarn an Kachelgrenzen richtig erkennen.
- **Markierung an Straße**: einen oder mehrere durchgehend verbundene Asphaltabschnitte und genau eine Markierung auswählen. Die Markierung übernimmt Geometrie und Höhenverlauf und wird bei Bedarf unterteilt. Ein Strg+Z nimmt den gesamten Schritt zurück.
- Die Markierung wird weiterverwendet, nicht am bisherigen Platz kopiert. Den Asphalt unter ihrem bisherigen Platz mit auswählen, wenn dort eine Markierung bleiben soll. Keine dauerhafte automatische Kopplung bei späterer Einzelbearbeitung.
- **Gelände an Straße** berücksichtigt die tatsächliche Straßenfläche und stützt angrenzende Rasterpunkte, damit keine rasterbedingten Lücken unter den Straßenrändern bleiben.
- Kreisverkehr: Außenrandlinien, gestrichelte Zufahrten, einstellbare Linienabstände je Arm und gemeinsame Strichbreite. Bereits in Asphalttexturen enthaltene Linien bleiben erhalten.
- Passende Asphalt- und Markierungsebenen gemeinsam an Baukasten-Arme anbinden; Markierungen nicht unabhängig auf volle Armbreite strecken.
- Deutsche und englische PDF-Anleitung mit neuen Bildern, Auswahlbeispielen und korrigierten Start-/Versionsangaben.

## English

- Select, shape and extend matching asphalt and marking layers together with C; improved group selection and undo.
- Show active spline length, radius, gradient and height with multiple splines selected.
- Fit selected road layers to terrain and recognize deleted neighbours across tile borders.
- **Marking to road** fits exactly one marking to one or more connected asphalt sections, subdividing it as needed. Undo restores the entire operation.
- The selected marking is reused, not copied at its previous location. Include the asphalt beneath that location when it must stay covered. This does not create permanent coupling for later single-layer edits.
- **Terrain to road** supports the actual road footprint and neighbouring grid vertices to close raster-related gaps beneath the road edges.
- Roundabout outer edge lines, dashed entrance openings, per-arm insets and shared stripe thickness. Lines already painted into asphalt textures remain visible.
- Connect matching asphalt and marking layers to builder arms without independently stretching markings to the full arm width.
- Updated German and English PDF guides with new screenshots and corrected startup/version information.

## Validation

539 local omsi-app tests passed (9 optional local probes ignored) before release metadata changes; engine build and starter self-test passed. Chris confirmed marking fitting, undo, save/reload and full-width terrain support with save/reload on the Merge test map. Windows packaging and Linux/Windows CI remain the release gate. No Apple package is offered.
