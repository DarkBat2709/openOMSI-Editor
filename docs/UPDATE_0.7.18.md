# Editor 0.7.18-pre – temporäre Diagnose von Hängern

Die KI-Werkzeuge bleiben wie in 0.7.17. Diese Version ergänzt abschaltbare
Zeitmessungen. Der normale Start aktiviert sie nicht. Es ist kein zusätzliches
Hilfsprogramm erforderlich. Die Diagnose behebt noch keine Verzögerungen.

## Installation (Linux x64)

ZIP unter `/home/chris/Projekte/openOMSI-Editor` entpacken. Danach:

```bash
cd "/home/chris/Projekte/openOMSI-Editor/openomsi-source-0.2.27-editor-0.7.18-pre"
bash INSTALLIEREN.sh
```

Erst nach **FERTIG** die neue Installation starten. Der Installer baut das
Programm auf deinem Rechner; das ZIP enthält Quellcode, keine fertige Binary.
Vorhandene Installationen werden nicht überschrieben.

## Ein Testlauf mit Diagnose

```bash
bash "/home/chris/Projekte/openOMSI-Editor/openOMSI-Editor-0.7.18-pre/start-editor-diagnose.sh" "/home/chris/Games/openOMSI-0.1.1541-linux-x64"
```

Die problematische KI-Bearbeitung wiederholen. Beim nächsten Hänger die ungefähre
Uhrzeit und Aktion merken (Ziehen, Loslassen, Übernehmen, Kamera bewegen).
Danach die `game.log` dieses Laufs vor einem erneuten Start sichern und schicken.
Der Diagnosemodus schreibt in dieselbe normale Log; er überträgt keine Daten.

`EDITOR-DIAG enabled` bestätigt den Diagnosemodus. `completed` zeigt einen
Arbeitsschritt ab 250 ms. Wiederholte Meldungen derselben Kategorie werden auf
eine pro fünf Sekunden begrenzt. `waiting` meldet einen noch laufenden Schritt
ab zwei Sekunden durch einen unabhängigen Hintergrundthread und dann alle
fünf Sekunden. Die ID verbindet waiting mit dem späteren completed-Eintrag.
Einzelne Wartezeiten beim Laden oder Grafiktreiber sind nicht automatisch Fehler.

Gemessen werden Mausziehen/Einrasten, KI-Vorschau (mit Spline-ID, Kachel,
Punktzahl und Zielzahl), Lesen der bearbeiteten Kartendaten, Splineketten,
vorhandene KI-Pfade, Pfadanzeige, Kachelaktualisierung, Bildberechnung,
Verkehrssimulation, Szenerieskripte, Plugins sowie Grafik-Ausgabe.
Überlappende Zeiten sind verschachtelte Schritte, sie dürfen nicht addiert werden.
Die Überwachung liefert Eingrenzung, keinen Thread-Stacktrace. Bei einem
kompletten Systemstillstand oder blockierter Log-Ausgabe kann auch sie aussetzen.

## Wieder ohne Diagnose starten

```bash
bash "/home/chris/Projekte/openOMSI-Editor/openOMSI-Editor-0.7.18-pre/start-editor.sh" "/home/chris/Games/openOMSI-0.1.1541-linux-x64"
```

Das Diagnose-Skript setzt `OPENOMSI_EDITOR_DIAGNOSTICS=1` nur für seinen eigenen
Prozess und dessen Kindprozess, nicht dauerhaft in der Shell oder den Einstellungen.
Ohne diese Variable gibt es keinen Watchdog und keine Zeitmessungen. Die normale
Log bleibt aktiv. Die Diagnose kann nach der Fehlersuche wieder aus dem Quellcode
entfernt werden; ein Rückbau ist zum Abschalten nicht erforderlich.
