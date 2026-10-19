# Editor 0.8.8.1 — KI-Verkehr-Hotfix / AI traffic hotfix

## Deutsch

Änderungen gegenüber 0.8.8-pre, weiterhin auf Basis von openOMSI 0.3.33:

- Frei gezeichnete, erkannte Kreisverkehr-Fahrwege erhalten eine feste Ausfahrtsplanung. Das reduziert zusätzliche Runden.
- Zufälliger KI-Verkehr bevorzugt andere erreichbare Ausfahrten gegenüber der Rückfahrt zur eigenen Zufahrt. Wenn keine Alternative offen ist, bleibt der Rückweg möglich. Einzelne Rückfahrten sind weiterhin möglich; das Verhalten wird weiter getestet.
- Beim Löschen eines Kreisverkehr-Objekts verschwinden seine KI-Fahrwege sofort aus Editoranzeige und Auswahl. Beim Wiederherstellen erscheinen sie wieder. Unabhängige KI-Splines bleiben erhalten. Die Verkehrssimulation wird nach Speichern und Neuladen aktualisiert.
- Der Updater unterstützt künftig vierteilige Hotfix-Versionsnummern.

**Installation:** 0.8.8-pre erkennt vierteilige Versionsnummern noch nicht automatisch. Diesen Hotfix bitte manuell herunterladen und vollständig in einen neuen Ordner entpacken. Die bisherigen PDFs für 0.8.8-pre gelten weiterhin und sind enthalten. Die automatische Fahrwegerzeugung wurde in diesem Hotfix nicht geändert.

## English

Changes since 0.8.8-pre; engine base remains openOMSI 0.3.33:

- Detected hand-drawn roundabouts receive a committed exit route to reduce repeated laps.
- Random traffic prefers another reachable exit over returning to its incoming arm, retaining the return when no alternative is open. Occasional returns remain possible; further testing continues.
- Deleting a roundabout object immediately hides its AI paths from the editor display and selection. Restoring the object restores the display. Independent AI splines remain intact. Save and reload to update the traffic simulation.
- The updater now accepts four-part hotfix version numbers.

**Installation:** 0.8.8-pre does not automatically recognize four-part versions. Download this hotfix manually and extract the complete package into a new folder. The existing 0.8.8-pre PDF guides remain applicable and are included. Automatic path generation is unchanged.
