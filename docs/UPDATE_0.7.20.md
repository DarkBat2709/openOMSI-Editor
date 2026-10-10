# Editor 0.7.20-pre – Mausbewegungen bündeln

Diese Testversion übernimmt 0.7.19 und korrigiert einen möglichen Rückstau beim
Ziehen von KI-Freiformpunkten. Die Ursache der längeren Hänger ist noch nicht
abschließend bestätigt.

Bisher wurde für jedes CursorMoved-Ereignis sofort die vollständige KI-Vorschau
neu aufgebaut. Jetzt aktualisieren diese Ereignisse nur die aktuelle Mausposition
und merken eine ausstehende Bearbeitung vor. Vor der nächsten Bildberechnung
wird höchstens eine solche Bearbeitung ausgeführt. Beim Loslassen wird eine noch
ausstehende Position vor der Anschlusssuche verarbeitet. Zwischenpositionen werden
nicht nachträglich abgearbeitet; die zuletzt gemeldete Position bleibt maßgeblich.

Der Diagnosemodus meldet zusätzlich EDITOR-DIAG drag_queue bei mindestens 250 ms
Wartezeit einschließlich der Zahl gebündelter Mausereignisse. Langsame mouse_drag-
Messungen enthalten ebenfalls diese Angaben. Außerhalb des Diagnosemodus werden
keine zusätzlichen Diagnosemeldungen geschrieben. Der normale Start bleibt möglich.

## Installation

ZIP nach /home/chris/Projekte/openOMSI-Editor entpacken:

```bash
cd "/home/chris/Projekte/openOMSI-Editor/openomsi-source-0.2.27-editor-0.7.20-pre"
bash INSTALLIEREN.sh
```

Erst nach FERTIG:

```bash
bash "/home/chris/Projekte/openOMSI-Editor/openOMSI-Editor-0.7.20-pre/start-editor-diagnose.sh" "/home/chris/Games/openOMSI-0.1.1541-linux-x64"
```

Den problematischen längeren Ziehvorgang wiederholen und danach game.log sichern.
Falls es noch hängt: Uhrzeit sowie Angabe, ob während des Ziehens oder erst beim
Loslassen, helfen bei der Zuordnung. Die Diagnose kann anschließend durch normalen
Start mit start-editor.sh ausgeschaltet werden.

## Prüfstatus

Installer- und Shell-Syntax sowie Quellpaket-Prüfsummen und ZIP-Integrität geprüft.
Kein lokaler Rust-Build oder ausgeführter Rust-Test: Die Bauumgebung ist hier nicht
mehr vorhanden. Der Installer führt Cargo check, die bestehenden Regressionstests
und den Release-Build vor der Installation aus. Kein interaktiver Test auf der
Benutzerkarte. Alte Installationen werden nicht überschrieben. Bei Buildfehlern
update-0.7.20-pre.log schicken.
