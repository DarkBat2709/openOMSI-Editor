# Straßenanschlüsse / Road connections — 0.7.10-pre

## Deutsch

Die Funktion gilt für Kreuzungen und Kreisverkehre aus dem Baukasten, deren
Projektdatei `junction.junction.json` beim Bauteil vorhanden ist.

### Eine Straße verbinden

1. Im Spline-Modus die passende Zufahrtsstraße auswählen.
2. In den Objektmodus wechseln und die gesetzte Kreuzung bzw. den Kreisverkehr auswählen.
3. Den passenden Baukasten öffnen und Zufahrt A/B/C/D wählen.
4. „Connect road to selected arm“ / Straße an gewählte Zufahrt anschließen drücken.
5. Die Vorschau kontrollieren und das gesetzte Bauteil aktualisieren. Erst dieser
   Schritt übernimmt die Verbindung. Schließen verwirft die Vorbereitung.
6. Mit Strg+S speichern. Zum Aktualisieren der KI-Fahrwege die Karte neu laden.

Auch bereits geneigte Bauteile können angeschlossen werden. Höhe, Steigung,
Querneigung und sichtbare Straßenränder werden beim Anschluss berücksichtigt.
Unterschiedliche Bordstein-/Profilformen können weiterhin einen passenden
Übergangsspline erfordern.

### Verbundene Straßen folgen dem Bauteil

Nach einer ausdrücklichen Verbindung folgen die Zufahrten beim Verschieben mit
der Maus oder I/J/K/L, beim Drehen mit N/M bzw. Mausrad, bei Höhenänderungen mit
U/O und beim Neigen mit Pos1/Ende bzw. Bild↑/Bild↓. Das entfernte Straßenende und
seine bestehende Verbindung bleiben erhalten. Bloß aneinanderliegende Straßen
werden nicht automatisch als verbunden erkannt.

Alle betroffenen Zufahrten werden vor der Änderung geprüft. Passt eine nicht,
werden weder das Bauteil noch die Zufahrten geometrisch verändert. Kleinere
Schritte, längere Zufahrten oder ein zusätzlicher Übergang helfen bei engen Kurven.

Rücktaste setzt zusammengehörige Lageänderungen und Zufahrten gemeinsam auf den
Stand vor der ersten solchen Änderung zurück. Separat bearbeitete Straßen werden
nicht ungefragt überschrieben. Nach Löschen das Bauteil erst mit Entf wiederherstellen.
Die Rücksetz-Historie gilt für die aktuelle Editorsitzung; die Geometrie und
Verbindungs-IDs werden mit der Karte gespeichert.

Wurde das Bauteil in einer alten Version bereits von seiner Straße wegbewegt,
die Straße auswählen und im Baukasten die passende Zufahrt erneut verbinden.
Weitere beschädigte Verbindungen müssen gegebenenfalls zuerst einzeln korrigiert werden.

### Grenzen

- Normale OMSI-Karten; keine Weltkoordinatenkarten.
- Keine automatische Verbindung zu beliebigen, nur nahe liegenden Straßen.
- Keine komplette Neuplanung eines Straßennetzes oder eines mehrteiligen Zufahrtsverlaufs.
- Vorhandene Grenzen bleiben: höchstens 50 m Anschlussabstand, 30° Richtungsabweichung,
  passendes Profil und ausreichend lange Straße für den Übergang.
- Nicht bearbeitbare Chrono-/Altformat-Straßen werden nicht geändert.
- KI-Fahrwege nach Speichern durch Neuladen aktualisieren.

Diese Anleitung ergänzt die bisherigen PDFs; die PDFs wurden nicht neu erstellt.

## English

Builder junctions and roundabouts with their companion `junction.junction.json`
can connect roads while tilted. Select the road in spline mode, switch to object
mode, select the placed object, open its builder, select an arm, prepare the road
connection, check the preview and update the placed object. Closing cancels the
preparation. Save with Ctrl+S and reload the map to refresh traffic paths.

Explicitly connected roads now follow moving, rotating, raising/lowering and
tilting the builder object. The far road endpoint and its existing connection
stay fixed. Merely touching roads are not considered connected.

Every affected road is fitted before applying the pose change. If any fit fails,
the object and road geometry remain unchanged. Use smaller movements, longer
approaches or a separate transition section when necessary. Different kerb shapes
can still require a matching spline profile.

Backspace restores the combined pose/road changes from before the first linked
pose edit. Separately edited roads are protected from an unexpected reset.
Restore deleted objects with Delete first. Undo history belongs to the current
editor session; geometry and connection IDs persist in the saved map.

For an already broken link from an older version, select its road and reconnect
the correct arm in the builder. Other broken links may need individual repair first.

Standard OMSI maps only. Existing fitting limits remain (50 m gap, 30° direction
mismatch, matching profiles and sufficient transition length). Non-editable Chrono
or old-format roads are protected. This does not redesign a complete road network.
This guide supplements the existing PDFs.
