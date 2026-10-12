# Editor 0.7.23-pre – einzelne KI-Pfade auswählen, löschen und bearbeiten

## Deutsch

Diese Testversion basiert weiterhin auf openOMSI 0.2.27. Sie ergänzt die vorhandenen
Editor-Werkzeuge um die Bearbeitung einzelner Fahrwege. Die bisherigen PDF-Inhalte bleiben
erhalten; die KI-Anleitung für diesen Stand ist auf den Seiten 24 bis 31 ergänzt.

### Einen Fahrweg löschen

1. Den Editor öffnen und **KI-Pfade auswählen / bearbeiten** wählen. Die Schaltfläche
   steht sowohl im Objektmodus als auch im Spline-Modus zur Verfügung. Es muss zuvor
   keine Straße ausgewählt werden.
2. Einen blauen oder orangefarbenen Fahrweg anklicken. Der ausgewählte Pfad wird lila
   hervorgehoben. Das Fenster zeigt Objekt/Straße, Element-ID, Pfadnummer und Richtung.
3. Liegen mehrere Wege übereinander, mit **Tab** oder **Nächster überlagerter Pfad**
   durch die beim Anklicken erkannten Wege wechseln. Vor dem Löschen die Hervorhebung
   und die Fahrtrichtung kontrollieren.
4. **Entf** oder **KI-Pfad löschen** verwenden. Nur dieser Fahrweg in dieser Richtung
   wird entfernt. Eine Gegenrichtung, andere Fahrwege und das sichtbare Kartenobjekt
   bleiben erhalten. Bei einem Kreuzungsarm können mehrere Fahrwege betroffen sein;
   diese einzeln auswählen und löschen.
5. **Strg+S** speichern, das Fenster schließen und die Karte mit der vorhandenen
   Neuladefunktion neu laden. Erst dann verwendet der KI-Verkehr das geänderte Netz.

**Strg+Z** macht die Änderung rückgängig, **Strg+Y** stellt sie wieder her. Solange
der gerade gelöschte Pfad ausgewählt ist, stellt die Schaltfläche **KI-Pfad
wiederherstellen** ihn ebenfalls wieder her. Nach Neuladen beginnt eine neue
Bearbeitungssitzung; ihre Rückgängig-Historie enthält die vorherige Sitzung nicht.

### Einen ursprünglichen oder gespeicherten Fahrweg korrigieren

1. Den einzelnen Fahrweg wie oben auswählen. Die Bearbeitungsgriffe erscheinen direkt;
   bei Bedarf **Bogen bearbeiten** einschalten.
2. Die beiden grünen Punkte mit der linken Maustaste ziehen. Damit wird die innere
   Form des Pfads verändert. Den lila Anfangsgriff oder gelben Endgriff ziehen, um
   Anfang bzw. Ende zu verschieben oder den Pfad zu verlängern.
3. **Bogenbearbeitung beenden** wählen. **Ursprünglichen Bogen wiederherstellen**
   setzt Form, verschobene Endpunkte und gespeicherte Höhenkorrektur zurück, ohne den
   Fahrweg zu löschen.
4. Speichern und die Karte neu laden.

Die Funktion gilt für ursprüngliche Straßenpfade, ursprüngliche Pfade in
Kreuzungsobjekten sowie gespeicherte, vom Editor erzeugte Fahrwege. Auch sehr kurze Pfade mit nur zwei Ausgangspunkten können gebogen werden.

### Speicherung und Grenzen

- Änderungen werden als `editor_lane_*`-Regeln an der betreffenden Objekt- oder
  Spline-Instanz in der Kartenkachel gespeichert. Gemeinsam verwendete `.sco`- und
  `.sli`-Dateien werden nicht verändert. Pfadnummern und bestehende Verkehrsregeln
  bleiben erhalten.
- Gelöschte Wege werden beim Aufbau des openOMSI-Verkehrsnetzes ausgelassen. Die
  Änderung betrifft also auch den Verkehr und nicht nur die blaue Anzeige.
- Diese Regeln benötigen **openOMSI mit dieser Erweiterung**. Das ursprüngliche
  OMSI 2 und ältere Editor-Versionen übernehmen diese Löschung/Formkorrektur nicht.
  Es handelt sich noch nicht um einen Export geänderter Objekt-/Spline-Varianten
  für OMSI 2.
- Einzelspieler, normale OMSI-Karten und Kartenkacheln der Version 14. Durch aktive
  Chrono-Szenarien hinzugefügte/geänderte Kartenelemente werden abgewiesen, damit
  keine zeitabhängigen Fahrwege versehentlich in die Grundkarte übernommen werden.
- Die Bearbeitung aktualisiert den laufenden KI-Verkehr erst nach Neuladen. Eine
  Unterteilung eines Pfads ist hier nicht enthalten. Verschobene Endpunkte benötigen
  passende Anschlüsse und Fahrtrichtungen; nach dem Neuladen kontrollieren.
- Vorhandene Dateisicherungen des Editors bleiben aktiv. Für den ersten praktischen
  Test eine Kartenkopie verwenden.

## English

This candidate remains based on openOMSI 0.2.27. The existing 0.7.22 PDF contents are
preserved; pages 24 to 31 add the current AI-path workflow.

### Delete an individual path

1. Open **Select / edit AI paths** from the object or spline editor. Selecting a road
   first is not required.
2. Click a blue or orange driving path. The selected path turns pink. The panel shows
   its object/road, instance ID, path number and direction.
3. Use **Tab** or **Next overlapping path** to cycle through paths detected by the
   click. Check the highlight and direction before deleting.
4. Press **Delete** or click **Delete AI path**. This removes only that path direction
   on that instance. The road/junction object, other paths and the opposite direction
   remain. A junction arm may involve several paths; delete those individually.
5. Press **Ctrl+S**, close the panel and reload the map to rebuild AI traffic.

**Ctrl+Z** undoes, **Ctrl+Y** redoes. **Restore AI path** restores the currently selected
deleted path. Reloading starts a new editing session and resets its undo history.

### Edit an original or saved curve

Select a path to show its handles; enable **Edit curve** if needed. Drag the two green
handles with the left mouse button to reshape the interior. Drag the purple start or
yellow end handle to move or extend the path. Choose **Finish curve editing**, save and
reload. **Restore original curve** resets shape, endpoint offsets and saved height correction
without deleting the path. Original spline paths, junction-object paths and saved editor-generated paths
are supported. Very short paths with only two original samples can also be bent.

### Persistence and scope

Changes are saved as per-instance, per-path, per-direction `editor_lane_*` rules in
the map tile. Shared SCO/SLI definitions, path indices and unrelated traffic rules
are preserved. Deleted lanes are excluded when openOMSI builds the traffic network.

**These rules require openOMSI with this extension. Original OMSI 2 and older editor
versions do not apply them.** This feature does not yet export modified SCO/SLI
variants for OMSI 2. It supports single-player, ordinary OMSI maps and version 14
tiles. Active Chrono additions/changes are rejected to protect the base map. Splitting
paths is not included. Moved endpoints need compatible connections and travel directions;
check the traffic network after reloading. Test on a map copy first.

## Pfadfix 3 – Fahrbahnhöhe, freie Bogenpunkte und verschiebbare Pfadenden

**Deutsch:** Neue zusätzliche KI-Verbindungen werden entlang ihrer Kurve auf die
Fahrbahnflächen der geladenen Karte gelegt. Das Höhenprofil wird in der erzeugten
KI-Spline-Datei gespeichert und beim Neuladen wieder verwendet. Fehlt unter einem
Teil der Kurve eine geladene Fahrbahn, wird die Verbindung nicht hinzugefügt und eine
Meldung angezeigt. Die grünen Griffe bleiben verfügbar, damit der Bogen korrigiert
werden kann. Beide Bogenpunkte sind frei in der Kartenansicht verschiebbar.

Mit **F9** die KI-Pfad-Auswahl öffnen, dann den Pfad direkt anklicken. Keine vorherige
Spline-Auswahl nötig. Grün verändert den Bogen, lila verschiebt den Anfang, gelb das
Ende. Beim Loslassen nahe einem eindeutigen passenden Anschluss rastet der Endpunkt
an dessen Position ein. Passende Fahrtrichtung und lückenlosen Anschluss anschließend
kontrollieren. Verschobene Pfadenden lösen die alte explizite Verknüpfung auf der
betroffenen Seite. Das gespeicherte Verkehrsnetz wird erst beim Neuladen aufgebaut.

Beim Ziehen eines gespeicherten Pfads wird die Fahrbahnhöhe neu erfasst, sofern die
gesamte innere Kurve über geladenen Fahrbahnflächen liegt. Für diese Korrektur werden
65 Höhenstützstellen mitgespeichert. Liegt ein Teil außerhalb der Fahrbahn, greift die
geometrische Straßenprofil-Korrektur; das ist keine Garantie für einen fahrbaren Weg.
Bereits fehlerhaft gespeicherte zusätzliche Verbindungen einmal bearbeiten oder neu
anlegen. Die Farbmarker werden zur Sichtbarkeit etwas über dem Pfad dargestellt.

**Strg+S speichern und Karte neu laden.** Die bisherigen PDF-Kapitel erklären den Stand
0.7.22-pre; das ergänzte KI-Kapitel auf den Seiten 24 bis 31 erklärt die neuen Funktionen.

**English:** New additional AI connections are projected onto the loaded map's road
surfaces along their curve. The height profile is baked into the generated AI spline
and reused after reloading. If part of the curve has no loaded road surface beneath
it, adding the connection is blocked with a message. Green handles remain available
so the curve can be repaired. Both green handles can be moved freely in the map view.

Press **F9**, then click the AI path directly. No preceding spline selection is needed.
Green reshapes the curve; purple moves the start; yellow moves the end. Releasing an
endpoint near a unique compatible connection snaps it to that position. Check travel
direction and continuity afterwards. Moving an endpoint clears its previous explicit
link on that side. The saved traffic network is rebuilt when the map is reloaded.

Dragging a saved path captures road height again when its entire interior lies above
loaded road surfaces. This correction is stored using 65 height samples. Otherwise,
the geometric road-profile correction is used; a driveable route is not guaranteed.
Edit or recreate previously saved faulty additional connections. Visual markers have
a small height offset for visibility. **Save with Ctrl+S and reload the map.** The
included PDFs preserve 0.7.22-pre content and include the complete AI section on pages 21 to 30.

## Bedienungsanleitung / User guide

- [Vollständige Bedienungsanleitung - Deutsch](openOMSI-Editor-Kurzanleitung.pdf)
- [Complete editor guide - English](openOMSI-Editor-Quick-Guide-EN.pdf)

Je 31 Seiten einschließlich KI-Teil: Auswahl, Bogen/Endpunkte, Löschen, neue Verbindungen, Speichern
und Test mit Verkehr. Bei vorhandenem Spielerbus wird Straßenverkehr um den Bus
erzeugt, nicht um die entfernte freie Kamera.

Praktischer Benutzercheck, 11.10.2026: Pfadfix3-Bearbeitung als funktionierend
bestätigt. Verkehr über die geänderte T-Kreuzung noch separat testen.

Die KI-Anleitung ist auch in beiden Haupt-PDFs enthalten (je 31 Seiten):
- [Deutsche Kurzanleitung](openOMSI-Editor-Kurzanleitung.pdf)
- [English quick guide](openOMSI-Editor-Quick-Guide-EN.pdf)

Die bisherigen 23 Seiten sind unverändert; neue PDF-Lesezeichen führen direkt zu
Werkzeugwahl, Pfadbearbeitung, zusätzlichen Verbindungen und Verkehrstest.

## Speicherfix 4 / Save shortcut fix 4

**Deutsch:** Strg+S funktioniert nun auch im Fenster für zusätzliche KI-Verbindungen
und im KI-Straßenwerkzeug. Beide Strg-Tasten werden erkannt; S wird beim Speichern
nicht als Kamerabewegung festgehalten. Das Verbindungsfenster bietet zusätzlich
„Verbindung hinzufügen + speichern (Strg+S)“. Eine vollständige, gültige Vorschau
wird dabei zuerst übernommen und dann gespeichert. Ungültige Vorschauen bleiben offen.
F9 verwirft eine noch offene Verbindung nicht mehr.
Nach dem Speichern die Karte neu laden, damit die KI das geänderte Netz verwendet.
Lila Auswahlpunkte zusätzlicher KI-Verbindungen zeigen nun den tatsächlichen Bogen.

**English:** Ctrl+S now also works in the additional-connection panel and road AI
tool. Both Control keys are tracked, and saving does not hold S as camera movement.
The connection panel also has a combined add-and-save button. Ctrl+S applies a
complete valid preview before saving. Invalid previews remain editable. F9 no longer
discards a pending connection preview. Reload the map after saving.

## Neuladefix 5 / Driving session reload 5

Deutsch: Beim Karten-Neuladen bleibt der Bus an seiner aktuellen Position mit seiner
Fahrtrichtung. Der Editor setzt ihn nicht mehr am ursprünglichen Einstiegspunkt ein.
Ein aktiver Linienauftrag samt Fahrt- und Haltestellenfortschritt bleibt erhalten;
freie Fahrt bleibt freie Fahrt. Spielzeit, gespeicherte Bus-Skriptwerte und Anzeigen
werden übernommen. KI und Fahrgäste werden neu aufgebaut.
Zum Testen: Bus abstellen, Karte bearbeiten, speichern, Karte neu laden, anschließend
in die Fahreransicht wechseln und an derselben Stelle weiterfahren.

English: Reloading the map now restores the bus at its current location and heading.
The active duty and its trip/stop progress are retained; free driving stays free driving.
The current clock, bus script values and display images are restored. AI traffic and
passengers are rebuilt. Park, edit and save the map, reload it, then return to the cab.

Validation: 835 library tests passed, 0 failed, 15 ignored; release build successful.
User confirmed successful map reload in free driving and with an active duty.

## Aktuelle Anleitungen / Current manuals

Beide Haupt-PDFs: 31 Seiten. Begrüßung auf Seite 1, anklickbares
Inhaltsverzeichnis auf Seite 2. Eigentliche Bedienungsanleitung ab Seite 3.
Spline-Editor: Seite 12; ergänzende Werkzeugleiste: Seite 13.
KI-Bedienung: Seiten 21-30. Version, Start und Updates: letzte Seite 31.
Aktuelle Screenshots, Lesezeichen, Seitenzahlen und Seitenverweise angepasst.
Separate KI-Anleitungen unverändert: je sieben Seiten.

Both main PDFs: 31 pages. Welcome on page 1, clickable contents on page 2.
Instructions start on page 3. Spline editor: page 12; toolbar: page 13.
AI workflow: pages 21-30. Version and startup: final page 31.
Bookmarks, numbering and cross-references updated in German and English.


## KI-Fenster auf Englisch / AI panels in English

KI-Fahrwege entlang der Straße und zusätzliche KI-Verbindung nutzen nun englische
Ausgangstexte mit deutschen Übersetzungen. Beschriftungen, Schalter, Bedienhinweise,
Vorschauwerte und Fehlermeldungen beachten die Spracheinstellung.
Die deutsche Bedienung und die Funktionen bleiben erhalten. 838 Editor-Tests bestanden.
Die neuen englischen Screenshots sind in den englischen Anleitungen integriert.

The road AI-path tool and additional-connection panel now use English source keys
with German translations, including controls, instructions, formatted preview
counts and errors. 838 editor tests passed. The English screenshots are included in the English manuals.

## Englische Anleitung: Original-Screenshots

Alle 19 neuen Original-Screenshots des Benutzers den passenden Themen zugeordnet.
Englische Hauptanleitung: 31 Seiten; separate englische KI-Anleitung: 7 Seiten.
Vorherige deutsche Bildschichten entfernt, Beschreibungen auf englische
Schaltflächen abgestimmt. Werkzeugleisten, Baukästen, Gelände, Kataloge,
KI-Fahrwege, Pfadauswahl und Zusatzverbindung aktualisiert.
Inhaltsverzeichnis, Lesezeichen und Seitenzahlen geprüft.
Reiter KI-Pfade im Katalog-Original bleibt erkennbar; englische Erklärung ordnet ihn zu.
Neue Originalbilder unter docs/screenshots/en. Deutsche PDFs unverändert.

## Zusammengeführte Bedienungsanleitungen

Die deutsche und die englische Hauptanleitung enthalten jeweils den vollständigen KI-Teil auf Seiten 21 bis 30. Separate KI-PDFs und deren Textauszüge werden nicht mehr mitgeliefert; sie bleiben im lokalen Archiv erhalten.
