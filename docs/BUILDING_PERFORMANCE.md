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
