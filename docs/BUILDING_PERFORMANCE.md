# Baukomfort und Performance - lokale Testfassung

Stand: 14.10.2026. Entwicklungszweig codex/editor-building-performance.
Noch kein neues öffentliches Release. Die veröffentlichte Version bleibt 0.8.1-pre.

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
