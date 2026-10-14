# Baukomfort und Performance - lokale Testfassung

Stand: 14.10.2026. Entwicklungszweig codex/editor-building-performance.
Update 0.8.2-pre nach bestandenen praktischen Tests am 14.10.2026 von Chris beauftragt. Historische Einträge unten dokumentieren den jeweiligen damaligen Freigabestand.

## Umgesetzt

- Auswahlfilter in der Spline-Werkzeugbox: alle Splines, Straßen, Gehwege, Markierungen, Sonstige / unbekannt. Anklicken schaltet weiter. Der Objektmodus bleibt separat.
- Der Filter wirkt auf den nächsten Klick und dessen Tab-Trefferliste. Eine bestehende Auswahl oder Änderung wird beim Umschalten nicht verworfen. Während einer Verbindungsvorschau bleibt deren bisherige Bedienung bestehen.
- Straßen werden anhand von Fahrzeugpfaden erkannt, Gehwege anhand von Fußgängerpfaden ohne Fahrzeugpfade. Eine Straße mit integriertem Gehweg zählt als Straße. Markierungen ohne Pfade werden anhand eindeutiger Wörter im Dateinamen erkannt. Fehlende Metadaten und uneindeutige Assets bleiben unter Sonstige beziehungsweise Alle erreichbar. Das ist keine vollständige automatische Klassifikation aller Fremdassets.
- Ausgeschlossene Splinekategorien werden vor der Erzeugung von Auswahlmarkern übersprungen. Die ungefilterte Auswahl behält ihr bisheriges Verhalten.
- Bei OPENOMSI_EDITOR_DIAGNOSTICS=1 erscheinen alle fünf Sekunden EDITOR-PERF-Zusammenfassungen: Anzahl, Mittelwert und Maximum abgeschlossener Schritte sowie eine obere Histogrammgrenze für das 95. Perzentil. Es werden keine Einzelbildlisten dauerhaft gespeichert.
- Die Splineauswahl wird getrennt nach Filter gemessen. Bestehende Messstellen umfassen unter anderem Frame-Verarbeitung, Rendering, KI-Vorschau, Mausziehen und Kartenaktualisierung. Zeiten verschachtelter Schritte dürfen nicht addiert werden. Frame-Verarbeitungszeiten sind keine direkte FPS-Messung.
- Die bisherigen Warnungen ab 250 ms und der unabhängige Hänger-Wächter bleiben aktiv. Ohne Diagnose-Start bleibt die zusätzliche Messung ausgeschaltet.

## Prüfstand

- Die lokale öffentliche Installation meldet nach der Paketkorrektur Editor 0.8.1-pre.
- Automatische Tests prüfen Kategorien, überlagerte Treffer, Tab, unveränderte Kartendaten und die Beibehaltung der Auswahl beim Filterwechsel.
- Diagnose-Tests prüfen kurze Messungen, Histogramm und Rücksetzen des Zeitfensters.
- Zusätzlicher Geländetest: Straße über zwei Kacheln, gleiche Randhöhen und vollständiges Rückgängig beider Kacheln.
- Bestehende Tests decken unter anderem Werkzeuglayout, verschiebbare Fenster, Geländegrenzen, Speichern, Neuladen und Verbindungen ab.
- Noch offen: tatsächliche Darstellung und Bedienung der neuen Filterbox im Spiel sowie praktische Bausitzungen auf kleiner und großer Karte. Automatische Tests ersetzen diese Prüfung nicht.

## Praktischer Vergleich

Vorgesehen sind zuerst die Vorführkarte, danach die große Merge-Karte. Jeweils eine Kartenkopie verwenden. Beide Durchgänge mit demselben Bus, Standort, Blickwinkel, Wetter, Verkehr und denselben Grafikeinstellungen durchführen; währenddessen nicht kompilieren.

1. Diagnose-Testversion starten. Karte vollständig laden lassen, dann 60 Sekunden ohne Bearbeitung beobachten.
2. 20-mal dieselbe Straße zwischen Markierungen/Gehwegen auswählen, zuerst mit Alle, dann mit Straßen. Filter und Uhrzeit notieren.
3. Einen Spline zehnmal verschieben und zurücknehmen. Ruckler während des Ziehens und nach dem Loslassen getrennt notieren.
4. Dasselbe mit einer Haltestelle; anschließend Gelände auf einer kleinen Fläche formen und texturieren.
5. Gelände an Straße an einer geraden, einer gekrümmten und einer kachelübergreifenden Straße prüfen. Rückgängig, Speichern und Neuladen kontrollieren.
6. Zwischen Objekten, Splines und KI-Fahrwegen wechseln; bestehende Auswahl, offene Vorschau und ungespeicherte Änderungen prüfen.
7. Speichern, Karte neu laden und die Fahrt fortsetzen. Fahrgäste, Buszustand und KI-Verbindungen kontrollieren.
8. Mindestens 30 Minuten wiederholt bauen und Speicherverbrauch sowie Verzögerungen beobachten. game.log vor dem nächsten Programmstart sichern.

Danach die EDITOR-PERF-Abschnitte vergleichen und den größten reproduzierbaren Engpass auswählen. Noch keine gemessene FPS- oder Laufzeitverbesserung auf einer echten Karte behaupten.

## Weiter offen

- Anschlussvorschau und verständlichere Ursachen bei unpassenden Straßenprofilen.
- Vorschau der betroffenen Geländerasterpunkte.
- Gruppenbearbeitung zusammengehörender Objekte mit gemeinsamem Rückgängig.
- Katalogfavoriten und zuletzt verwendete Bauteile.
- Vollständige VM-Neuinstallations- und Spieltests bleiben für v0.9 vorgesehen.

## Erster Merge-Durchgang und Korrektur (14.10.2026)

Das gesicherte Log der Sitzung 09:42–09:46 UTC betrifft Lemmental_Krefrath_Merge_Test. Die Auswahl funktioniert dort grundsätzlich. Für die fehlende Auswahl auf der Vorführkarte enthält dieses Log keinen Nachweis.

- Fehler behoben: BS_StrMarkierung_WeissGestrichelt_M/S und BS_StrMarkierung_Weiss_S wurden als Sonstige eingestuft. Der Dateinamenfilter erkennt jetzt auch das Wort StrMarkierung; Fahrzeug-/Fußgängerpfade haben weiterhin Vorrang. Regressionstest mit den tatsächlich ausgewählten Assetnamen.
- Gemessene Auswahlvorgänge vor der Korrektur: ungefähr 80–108 ms. Neue Teilmessungen für Einsammeln, Kandidaten, Filter, Randpunkte und Strahlprüfung grenzen den Engpass beim nächsten Durchgang ein.
- Beim Einsammeln wird der Vergleich mit aktiven Chrono-Splines nach ID vorgruppiert, statt jeden Basisspline mit allen aktiven Splines zu vergleichen. Vollständige Datensatzgleichheit und Kandidaten mit doppelter ID bleiben berücksichtigt. Geladene Kacheln werden bei der Auswahl über eine Hash-Menge geprüft.
- Beim Verschieben wurden object_rows_refresh-Spitzen bis rund 93 ms gemessen. Dieser zweite Engpass ist noch offen.
- Die tatsächliche Zeitersparnis der Änderungen muss im nächsten praktischen Durchgang gemessen werden; kein FPS-Versprechen.

## Zweiter Merge-Durchgang und nächste Testfassung

Gemessene, nach Aufrufzahlen gewichtete Mittelwerte: Alle Splines 101,060 → 47,434 ms (7/14 Aufrufe), Straßen 90,994 → 45,118 ms (7/21). Unterschiedliche Klicks/Standorte, kein kontrollierter Benchmark. Markierungen wurden im passenden Filter erfolgreich ausgewählt. Einsammeln der Daten: 46,927 ms bei 55 Aufrufen; Strahlprüfung: 0,529 ms. Objektgruppen: 88,193 ms im Mittel, 159,694 ms Maximum bei 66 Aktualisierungen.

Nächste lokale Optimierung:
- Erfolgreich eingelesene Spline-Kacheln werden innerhalb derselben Inhaltsgeneration und Chrono-Auswahl wiederverwendet. Nicht gelesene/fehlgeschlagene Kacheln werden erneut versucht. Ausgeladene Kacheln werden beim Abgleich aus dem Cache entfernt. Neue World-Instanzen starten leer; Kartenneuladen/Inhaltsänderungen und andere Chrono-Verzeichnisse verwerfen den Lesestatus. Änderungen und Rückgängig bleiben in den bestehenden Live-Overlays.
- Objektgruppen teilen pro Aktualisierung einen gemeinsamen aktuellen Kachel- und Straßenstand. Planung aller Gruppen bleibt vor Installation ihrer Ergebnisse; Fehlerbehandlung und stabile Objekt-IDs bleiben erhalten. Es gibt noch kein Überspringen vermeintlich unbetroffener Gruppen.
- Regression: mehrere verschobene Straßen/Objektgruppen gegen unabhängige Planung, unveränderte IDs, neu eingelesene Disk-Splines nach Inhaltswechsel und Erhalt ungespeicherter Änderungen. Separater Cache-Test für Inhaltsgeneration, Chrono-Wechsel, ausgeladene Kacheln und Wiederholung nicht erfolgreicher Lesevorgänge.
- Praktische Zeitersparnis dieser zweiten Optimierung noch offen. Nächster Durchgang: erster Klick nach Laden getrennt von wiederholten Klicks, Verschieben/Rückgängig, Speichern/Neuladen und Auswahl danach.

## Dritter Merge-Durchgang, Hintergrundvorbereitung und Mehrfachauswahl

Neue Messung: Auswahl Alle 17,853 ms, Straßen 5,208 ms, Markierungen 5,782 ms im Mittel. Einsammeln wiederverwendeter Daten meist 0,002–0,003 ms. Objektgruppen-Aktualisierung 52,211 ms im Mittel, 96,526 ms Maximum. Einmalige Initialisierung noch 1414 ms, gesamte entsprechende Aktualisierung 1511 ms. Unterschiedliche Klickzahlen/Standorte; kein kontrollierter Benchmark.

Lokale nächste Testfassung:
- Die erste Vorbereitung der Objektgruppen läuft in einem CPU-Hintergrundthread für die aktuelle World. Geänderte Kacheln bleiben bis zum Abschluss vorgemerkt; erst dann werden Gruppen auf Basis der aktuellen Änderungen berechnet und Kacheln aktualisiert. Fehler werden zurückgemeldet; eine verlassene World kann keine neue Sitzung verändern. Während der Vorbereitung wird Speichern mit einem Hinweis abgewiesen.
- Strg+Klick fügt einen Spline zur Gruppe hinzu oder entfernt ihn. Normaler Klick auf ein Gruppenmitglied erhält die Gruppe zum Ziehen, auf einen anderen Spline ersetzt er sie; Klick ins Leere hebt die Auswahl auf. Tab kehrt zur Einzelauswahl zurück. Auswahlfilter gelten weiterhin pro Klick; für Straße plus Markierungen/Gehweg empfiehlt sich Alle Splines.
- Alle Gruppenmitglieder erhalten Marker; die Statusanzeige nennt die Anzahl. I/J/K/L und U/O sowie Mausziehen bewegen die Gruppe mit identischer Verschiebung. Ein Mauszug ergibt einen gemeinsamen Rückgängig-Schritt. Andere Geometriewerkzeuge benötigen Einzelauswahl.
- Mitglieder dürfen auf unterschiedlichen Kacheln liegen, bleiben aber jeweils innerhalb ihrer ursprünglichen Kachelgrenzen. Wird eine Grenze verletzt oder fehlt ein Mitglied, bewegt sich keines. Bestehende Verknüpfungen/IDs werden beibehalten; Anschlüsse an nicht mitbewegte Nachbarn müssen anschließend geprüft werden.
- Tests: Gruppenauswahl, Entnahme und Leerklick; gemeinsame Verschiebung/Höhe; abgelehnte Grenzüberschreitung ohne Teiländerung; mehrere Mausbewegungen mit einem Undo; Speichern auf zwei Kacheln bei unveränderten Originaldateien; asynchrone Vorbereitung bei erhaltener Dirty-Warteschlange. DE/EN-Bedienhinweise ergänzt.

Veröffentlichung ausdrücklich erst nach praktischen Tests und Freigabe von Chris. Nur lokale Bautest-Installation aktualisieren, kein Push und kein Release-Upload.

## Korrektur Strg+Z im Spline-Modus

Der praktische Test meldete nicht funktionierendes Strg+Z. Der bisherige Eingabepfad wertete physische US-Tastenpositionen aus; auf QWERTZ entspricht die beschriftete Z-Taste physisch KeyY. Der Spline-Arbeitsbereich verarbeitet seine Z/Y-Verlaufskürzel jetzt anhand des logischen Buchstabens nach den modalen Eingabefeldern. Strg+Y wird nicht versehentlich als Undo behandelt. Wiederholungen werden unterdrückt; es wird keine umbelegte gehaltene Taste in den Eingabestatus eingefügt. Andere Editorbereiche bleiben in dieser Korrektur unverändert. Ein erfolgreicher Spline-Undo protokolliert die Anzahl zurückgesetzter Splines und Geländekacheln. Layout-Test für QWERTZ/QWERTY ergänzt; praktischen Gruppentest nach Neustart wiederholen.

## Praktisch bestätigt: Gruppenverschiebung speichern und neu laden

14.10.2026, Testfassung 6dccb33d, Merge-Testkarte: Chris bestätigt, dass die verschobenen Splines nach Speichern und Neuladen an den geänderten Positionen geblieben sind. Log: um 12:33:56 Uhr neun Kartendateien gespeichert, vor Neuladen um 12:34:00 Uhr erneut geschrieben, Neuladen um 12:34:03 Uhr abgeschlossen. Um 12:32:13 und 12:32:14 Uhr jeweils Undo für drei Splines. Damit Gruppen-Undo und Persistenz praktisch bestätigt. Der Reload erfolgte ohne Spielerbus, Dienst oder Fahrgäste; deren Erhalt ist durch diesen Durchgang nicht geprüft. Sitzung regulär beendet. Keine Veröffentlichung autorisiert. Sicherung vor diesem Test: /home/chris/Projekte/openOMSI-Editor/Sicherungen/Merge-Karte_vor-Gruppentest_2026-10-14_12-30-06.

## Praktischer Fahrgast-/Diensttest am Lemmentaler Hauptbahnhof

14.10.2026: Chris bestätigt Linie geladen, Fahrgäste aufgenommen, Spline am Hauptbahnhof geändert und weitergefahren. Log der Testfassung: RVL-Umlauf 100092, Fahrt 473_LZH8_LAB3, MB O530 Facelift Gelenkbus. Speichern um 12:50:40 Uhr; vollständiges Kartenneuladen 12:50:43–12:50:51 Uhr. Abschluss: bus=true, duty=true, passengers=21, waiting=146. Protokollierte Motordrehzahl, Motorstatus und Fehlerflags sowie Geschwindigkeit vor/nach Neuladen identisch. Reguläres Sitzungsende, Karriereprotokoll 0 crashes und 0 hurt. Keine ERROR-Zeilen.

Grenzen/Performance: vollständiges Neuladen rund 8 Sekunden (darunter ein Frame-Schritt von 5,9 Sekunden); Hintergrundvorbereitung der Objektreihen rund 1,66 Sekunden; Objektgruppenaktualisierung 62 ms Mittel/122 ms Maximum bei sieben Aufrufen. Lauf bestätigt Weiterfahrt und Wiederherstellung der Fahrgäste/Dienstzustände für diesen Fall, keinen vollständigen Langzeit- oder Aussteigetest. Log gesichert unter tmp/editor-performance/driving-passengers-spline-test.log. Weiterhin keine Veröffentlichung freigegeben.

Ergänzende Nutzerbestätigung vom 14.10.2026: Nach dem Neuladen und Weiterfahren erfolgte der Fahrgastwechsel an der nächsten Haltestelle normal. Damit ist auch der Anschlusscheck Ein-/Aussteigen für diesen praktischen Durchgang bestanden. Grundlage ist Chris' Beobachtung; hierfür wurde kein zusätzlicher Loglauf übermittelt. Keine Freigabe zur Veröffentlichung daraus ableiten.

## Tests 1–3 praktisch bestanden

14.10.2026: Chris bestätigt größere Gruppen, Auswahl hinzufügen/entfernen/aufheben und Verhalten an Kachelgrenzen als erfolgreich. Gesicherter Loglauf 13:00–13:13 Uhr, Build 6dccb33d: maximal 50 ausgewählte Splines angezeigt; Undo um 13:11:28 Uhr für 25 Splines, danach für einen und drei Splines. Drei Grenzüberschreitungen mit „nothing moved“ abgelehnt. Auswahl Alle: 211 Aufrufe, gewichtetes Mittel 11,41 ms, Maximum 14,371 ms. Objektgruppenaktualisierung: sieben Aufrufe, Mittel 54,79 ms, Maximum 70,763 ms. Keine ERROR-/Panikzeilen, reguläres Sitzungsende. Visuelle Korrektheit der Auswahl/Grenzbehandlung durch Nutzer bestätigt, nicht allein aus dem Log abgeleitet. Log: tmp/editor-performance/group-tests-1-3.log. Als nächste praktische Checks offen: Gelände mit Undo und gemischter längerer Bautest. Keine Veröffentlichung freigegeben.

## Test 4: Gelände-Undo auf QWERTZ korrigiert

Chris meldete nach Anheben und Texturieren „Keine Geländeänderung verfügbar“ bei Strg+Z. Ursache im Eingabepfad: Die bisherige logische Z/Y-Zuordnung galt nur für Splines. Im Gelände-Modus führte die physische KeyY-Position der beschrifteten Z-Taste zu Redo auf leerem Wiederholen-Verlauf. Korrektur erweitert die logische Verarbeitung auf aktives Gelände, weiterhin nach modalen Eingabefeldern und ohne zusätzliche gehaltene Tasten. Strg+Z=Undo, Strg+Y bzw. Strg+Umschalt+Z=Redo. Gemeinsame Terrain-Kürzelfunktion mit Regressionstest ergänzt; vorhandene Tests für Höhen- und Textur-Undo bleiben relevant. Praktischer Test 4 ist bis zum erneuten Nutzer-Test noch offen. Keine Veröffentlichung.

## Test 4: Video-Nachprüfung vom 14.10.2026, 13:24 Uhr

Aufnahme cinnamon-2026-10-14T132455+0200.webm und Log der Testfassung 6380b627 geprüft. Textur-Undo visuell bestätigt: aufgetragene Betonplatten verschwinden, Meldung „Geländeänderung rückgängig“, Verlauf schließlich 0 Rückgängig / 6 Wiederholen. Höhen-Undo und Redo werden damit noch nicht separat als bestanden gewertet. Log gesichert unter tmp/editor-performance/terrain-test-4-video.log.

Zum Ende um 13:27:22 Uhr Meldung „Section leaves its tile; create shorter sections first“. Diese stammt aus fit_terrain (Straße an Gelände/F6), nicht aus terrain_to_road (Gelände an Straße/Umschalt+F6). Ausgewählt waren ein Gehweg-Spline und anschließend 10mTerrain_Spline.sli. Gelände an Straße mit anschließendem Spline-Undo bleibt praktisch offen; dafür einen tatsächlichen Straßenspline wählen. Keine Veröffentlichung.

## Test 4: Gelände an Straße und Undo praktisch bestätigt

14.10.2026, Aufnahme cinnamon-2026-10-14T133429+0200.webm: Am Ende ist die Anpassung unter dem einzeln gewählten Seeburger-Straßenspline sichtbar. Log um 13:37:12 Uhr: „Terrain fitted under selected road only“. Um 13:37:29 Uhr bestätigt Spline-Undo die Wiederherstellung von 0 Splines und 1 Geländekachel. Damit Gelände an Straße und zugehöriges Undo für diesen Fall praktisch bestätigt. Vorherige Gruppenversuche wurden mit Einzelauswahl-Hinweis abgewiesen. Textur-Undo bereits im vorherigen Durchgang bestätigt; separater Höhenpinsel-Undo/Redo und gemischter längerer Bautest bleiben offen. Log: tmp/editor-performance/terrain-to-road-video-confirmed.log. Keine Veröffentlichung.

## Test 4 abgeschlossen: Höhenpinsel mit Verlauf

14.10.2026: Chris bestätigt den angeforderten Höhen-Test als erfolgreich. Aufnahme cinnamon-2026-10-14T134029+0200.webm zeigt „Berg heben“ und wechselnde Rückgängig-/Wiederholen-Zähler bei der Geländebearbeitung. Zusammen mit der Nutzerbestätigung sind Höhenbearbeitung und Verlauf für diesen praktischen Durchgang bestätigt. Textur-Undo und Gelände an Straße samt Undo wurden zuvor geprüft. Test 4 damit praktisch abgeschlossen; als nächster Schritt bleibt der gemischte Bautest über 20–30 Minuten mit Speichern/Neuladen und anschließender Weiterfahrt. Log gesichert unter tmp/editor-performance/terrain-height-confirmed.log. Keine Veröffentlichung autorisiert.

## Gemischter Bautest: Logprüfung, 14.10.2026 13:43–14:08

25 Minuten Laufzeit, Build 6380b627, reguläres Ende, keine ERROR-/Panikzeilen. Texturierung, Objektplatzierung/-bearbeitung und Straßenbau protokolliert; Straße an Gelände erzeugt 16 Abschnitte. Speichern um 14:02:02 (15 Dateien) und 14:05:28 (19 Dateien). Reload 14:07:36–14:07:44: bus=true, duty=false, passengers=0, waiting=41, protokollierte Busvariablen unverändert. Dieser Lauf enthält keine gefahrene Strecke/bediente Haltestelle; KI-Pfad-Bearbeitung und Gruppen-Undo nicht aus dem Log bestätigt. Frühere separate Tests bleiben gültig. Visueller Erhalt der Bauänderungen nach Reload und subjektive Bedienflüssigkeit noch von Chris zu bestätigen.

Gewichtete Messwerte: Spline-Auswahl 11,939 ms (4 Aufrufe), Objektgruppenaktualisierung 52,623 ms (30 Aufrufe), Maximum 110,610 ms. Frame-Mittel einschließlich Laden 7,069 ms; größte Pausen beim Start (7,348 s) und Reload (6,095 s), größte übrige erfasste Frame-Spitze 115,474 ms. Einmalige Hintergrundinitialisierung 1,547 s. Keine pauschale Ruckelfreiheit daraus ableiten. Inhaltshinweise: sechs fehlende Texturen, Windows-BBS-Plugin unter Linux nicht geladen. Log: tmp/editor-performance/mixed-building-25min.log. Keine Veröffentlichung.

## Gemischter Bautest abschließend bestätigt

14.10.2026: Chris bestätigt ausdrücklich, dass nach dem Neuladen alle gewünschten Bauänderungen vorhanden waren und die Bedienung während des Durchgangs flüssig blieb. Damit ist der gemischte 25-Minuten-Bautest für den geprüften Umfang bestanden. Zusammen mit den vorherigen Einzeltests sind die geplanten praktischen Bau-/Performance-Prüfungen 1–5 abgeschlossen. Dies ist keine allgemeine Fehlerfreiheit oder Windows-/VM-Freigabe; diese Plattformprüfungen bleiben wie vereinbart für v0.9 vorgesehen. Veröffentlichung weiterhin nur nach ausdrücklicher Freigabe, kein Upload/Push durch diese Testbestätigung.

Veröffentlichungsvorbereitung für 0.8.2-pre von Chris nach Abschluss der Tests ausdrücklich beauftragt (14.10.2026).
