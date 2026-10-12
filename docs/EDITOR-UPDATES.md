# Getrennte Updates für Spiel und Editor

Die lokale Editorfassung bietet unter **Einstellungen → Allgemein → Updates** zwei getrennte Bereiche mit **Jetzt prüfen**. Eine Prüfung lädt keine neue Programmversion herunter und installiert nichts. Wenn eine neuere Version gefunden wird, erscheinen die Versionsnummern, der Zielordner, **Herunterladen und installieren** und der Link zu den Versionshinweisen.

- **openOMSI · separate Spielinstallation** prüft die offiziellen, veröffentlichten Spielversionen von `openOMSI-org/openOMSI`. Verglichen wird die Version der separaten Programmdatei im ausgewählten openOMSI-Spielinhaltsordner. Der normale Spiel-Updater darf den Editorordner nicht ersetzen. Ist dort keine separate Spielinstallation vorhanden, zeigt die Prüfung einen Hinweis; sie legt keine zusätzliche Installation an.
- **openOMSI Editor · Editorpaket** prüft die veröffentlichten Pakete von `DarkBat2709/openOMSI-Editor`, einschließlich `-pre`-Versionen. Verglichen wird die Editorversion aus `VERSION-EDITOR.txt`. Eine gleich alte oder ältere Veröffentlichung ersetzt die lokale Fassung nicht. Derzeit werden Editorupdates für das veröffentlichte Linux-x64-Paket unterstützt.

Die Editorfassung installiert diese Updates bewusst manuell. Die ursprüngliche automatische Installation des offiziellen Updaters bleibt in Editorbuilds gesperrt. `OMSI_NO_UPDATE=1` im Startskript verhindert weiterhin dessen automatische Hintergrundprüfung; die beiden manuellen Schaltflächen funktionieren trotzdem.

## Was getrennt bleibt

Das Editorpaket enthält seine eigene openOMSI-Engine. Beim Editorupdate werden Editor und mitgelieferte Engine als zusammengehöriges Paket aktualisiert. Ein Update der separaten Spielinstallation ändert diese Engine nicht. Spieler, die das normale Spiel zusätzlich installiert haben, haben somit zwei Programmdateien, können aber denselben Karten- und Modbestand nutzen.

Die Versionsnummer der offiziellen Spielinstallation ist unabhängig von der Editorversion und von der openOMSI-Basis des Editors. Ein neueres Spiel garantiert keine Kompatibilität neu gespeicherter Inhaltsformate mit einer älteren Editorengine. Karten vor der Bearbeitung mit einer neueren Version sichern. Ein Programmbackup ersetzt kein Kartenbackup.

## Installation und Sicherungen

Vor einer Installation die laufende Fahrt beenden. Während Download und Installation bleibt ein Fortschrittsdialog geöffnet. Nach einem Editorupdate startet der Editor erneut; ein Spielupdate startet keine zusätzliche Spielinstanz und beendet den Editor nicht.

Updatepakete werden anhand ihrer SHA-256-Prüfsumme überprüft. Editorarchive müssen zum Release passen und dürfen keine Links, unsicheren Pfade oder fremden Inhaltsdateien enthalten. Die ersetzten Programmdateien werden im jeweiligen Installationsordner unter `.editor-update-backups/<alte-Version>-<Zeitstempel>/` gesichert. Die Installation nutzt den vorhandenen Austausch mit Rückabwicklung bei Fehlern. Karten, Busse und andere Mods werden nicht als Programmdateien ersetzt. Das persönliche `start.sh` bleibt erhalten.

Bei Bedarf können die gesicherten Programmdateien nach dem vollständigen Beenden von Spiel und Editor in denselben Installationsordner zurückkopiert werden. Die Datei `RECOVERY.txt` im Sicherungsordner nennt den Zielordner. Die Sicherungen belegen Platz und werden nicht automatisch gelöscht.

Diese Funktion ist eine lokale Erweiterung. Bereits veröffentlichte Editorpakete erhalten sie erst mit einem neuen Release.
