# Editor 0.8.6-pre — saving and map compatibility

## Deutsch

- Neu gesetzte Objekte aus Katalog, Zwischenablage und Mehrfachsetzen können über
  vorhandene Kachelgrenzen bewegt und zuverlässig gespeichert werden. Frühere
  Speicherpositionen werden bereinigt, damit keine Duplikate zurückbleiben.
  Weltposition, Drehung, Beschriftung und geländerelative Höhe bleiben erhalten.
- Geländeanweisungen von Objekten und Anhängen werden nicht mehr versehentlich
  auf den vorherigen Straßen-Spline angewendet.
- Alte verknüpfte Straßen-Splines schließen beim Laden am tatsächlichen Ende des
  Vorgängers an, einschließlich Kurvenrichtung und Höhenänderung. Basiskacheln und
  Chrono-Ergänzungen berücksichtigen jeweils ihr eigenes Dateiformat.
- Fehlt die Einstiegspunktliste einer Karte, werden Einstiegspunkte aus gewöhnlichen
  platzierten Objekten mit aktivem `[entrypoint]` erkannt. Vorhandene Listen haben
  Vorrang; die Quelldateien werden durch die Erkennung nicht umgeschrieben.

Weiterhin auf Basis von **openOMSI 0.2.50**. Dies ist kein Wechsel auf 0.3.33.
Die Windows-Startkorrektur aus 0.8.5-pre mit Microsofts DirectX-Shader-Compiler bleibt
enthalten. Alte Kachelformate werden durch diese Änderung nicht zum Bearbeiten
freigeschaltet. Die bisherigen DE/EN-PDF-Anleitungen gelten weiterhin.

Die lokalen Prüfungen umfassten 543 Editor-Tests und 127 Karten-, Welt- und
Launcher-Tests ohne Fehler. Chris bestätigte die vorgeschlagenen praktischen
Tests am 18. Oktober 2026 als erfolgreich. Einzelne ältere Testkarten wurden
dabei nicht protokolliert. Windows- und Linux-CI prüfen den Veröffentlichungsstand
zusätzlich; der Windows-Lauf enthält den DirectX-12-Renderer-Test.

Das vollständige Paket in einen neuen Ordner entpacken. Vor dem Wechsel die Karte
speichern und den laufenden Editor normal beenden. Eigene Karten bleiben im
bisherigen Inhaltsordner. Bei Problemen bitte die vollständige `game.log`, die
Editorversion und den betroffenen Kartennamen angeben.

## English

- Newly placed objects from the catalogue, clipboard and repeated placement can
  cross existing tile boundaries and save to their current tile. Former saved
  placements are removed to prevent duplicates. World position, rotation, labels
  and terrain-relative height are preserved.
- Terrain instructions belonging to objects or attachments no longer affect the
  preceding road spline accidentally.
- Linked splines in older maps now start at the actual end of the preceding road,
  including its tangent and elevation. Base tiles and chrono additions are resolved
  according to their respective file versions.
- Maps without an explicit entry-point list discover entry points from ordinary
  placed objects whose definitions contain an active `[entrypoint]`. Existing lists
  take precedence; discovery does not rewrite the map files.

The engine base remains **openOMSI 0.2.50**, not 0.3.33. The Windows startup fix
from 0.8.5-pre, including Microsoft's DirectX shader compiler, is retained. Existing
editing restrictions for old tile formats remain. The German and English PDF
guides remain applicable.

Local validation passed 543 editor tests and 127 map, world and launcher tests.
Chris reported the proposed practical tests successful on 18 October 2026;
individual older test maps were not recorded. Windows and Linux CI additionally
validate the release source, including the Windows DirectX 12 renderer regression.

Extract the complete package into a fresh folder. Save your map and close the
editor normally before switching versions. Keep your existing content folder.
For problems, provide the full `game.log`, editor version and affected map name.
