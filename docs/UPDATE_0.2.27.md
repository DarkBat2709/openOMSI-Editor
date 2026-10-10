# Editor 0.7.11-pre — openOMSI 0.2.27

## Deutsch

Dieser Entwicklungsstand führt den Editor 0.7.10-pre (Commit `04497112`)
mit dem offiziellen openOMSI-Tag `v0.2.27` zusammen. Die gemeinsame Basis ist
`v0.2.20`. Der Editor bleibt ein eigener Build mit `standalone-editor`;
offizielle Spielupdates ersetzen ihn nicht.

### Anpassungen

- Editor-Eingabefelder und Plugin-Texteingabe bleiben erhalten.
- Fotomodus und Karteneditor sind getrennte Bedienmodi. Den Karteneditor zuerst
  schließen, bevor der Fotomodus geöffnet wird. Dabei werden keine Änderungen
  verworfen; zum dauerhaften Speichern weiterhin die Editor-Speicherfunktion nutzen.
- Die neue Maussteuerung berücksichtigt weiterhin den aktiven Karteneditor.
- Beim Karten-Neuladen werden auch die neuen Umgebungsgeräusche, Plugin-Ereignisse,
  Plugin-Soundreferenzen und gehaltenen Objekt-Schalter zurückgesetzt.
- Die zusätzlichen Editor-Datenfelder bleiben auch in neuen Upstream-Tests erhalten.
- GitHub prüft jetzt auch `update/**`-Branches und Pull Requests auf `editor-preview`.
- Versionsanzeige: Editor 0.7.11-pre, Basis 0.2.27.

### Installation des Vorbereitungspakets (Linux x64)

1. Editor und Spiel schließen. ZIP in einen eigenen Ordner entpacken.
2. Dort `bash install-update.sh` ausführen. Optional den bisherigen
   Quellordner angeben: `bash install-update.sh "/Pfad/zum/Quellprojekt"`.
3. Der bisherige Quellordner muss auf `04497112` stehen und unverändert sein.
   Andernfalls bricht das Skript ab; keine Änderungen verwerfen oder zurücksetzen.
4. Das Paket legt einen separaten Git-Arbeitsordner auf `update/openomsi-0.2.27`
   an. Dieser teilt Git-Objekte mit dem bisherigen Quellprojekt; letzteres behalten.
5. Es führt Python-Prüfungen, `cargo check`, Workspace-Tests und den Release-Build aus.
   Erst danach entsteht der separate Ordner `openOMSI-Editor-0.7.11-pre`.
6. Im neuen Editor-Ordner starten:

   ```bash
   bash start-editor.sh "/Pfad/zum/openOMSI-Spielinhaltsordner"
   ```

Es werden keine Dateien auf GitHub hochgeladen, keine bestehenden Editoren ersetzt
und keine Karten kopiert oder verändert. Der neue Starter greift auf den angegebenen
Spielinhaltsordner zu; beim späteren Bearbeiten werden Karten dort gespeichert.
Zum ersten Test deshalb eine Kartenkopie auswählen. Die Rust-Build-Abhängigkeiten
müssen wie beim bisherigen Editor installiert sein. Fehler stehen in
`update-0.7.11-pre.log` neben dem Installationsskript.

### Vor einer Veröffentlichung manuell prüfen

- Neue und vorhandene Karte laden; freie Kamera und Werkzeugauswahl bedienen.
- Objekte und Splines setzen, bewegen, drehen und löschen; rückgängig machen.
- Kreuzung und Kreisverkehr mit Zufahrten drehen/neigen und zurücksetzen.
- Text in Katalog, Objektbeschriftung und Baukasten eingeben; DE/EN wechseln.
- Gelände verändern; speichern und neu laden, Kamera und Umgebungsgeräusche prüfen.
- Gespeicherte Karte in offiziellem openOMSI 0.2.27 öffnen.
- Karteneditor schließen, Fotomodus öffnen/beenden, Karteneditor erneut öffnen.
- Linux- und Windows-Prüfungen auf GitHub abwarten; Windows interaktiv separat testen.

Die vorhandenen PDF-Anleitungen beschreiben 0.7.10-pre. Dieses Dokument beschreibt
die Änderungen des Integrationsstands. Ein erfolgreiches Kompilieren allein ist
keine Bestätigung aller Spiel- und Speicherabläufe.

## English

This candidate merges editor commit `04497112` with upstream `v0.2.27`, preserving
both parents in Git history. Editor fields coexist with plugin typing; photo mode
must be opened after closing the map editor. Map reload also clears the new audio,
plugin and scenery-input state. CI covers update branches and pull requests.

The Linux installer creates a separate worktree and a new installation, after
validation and release build only. It never pushes, resets existing branches or
overwrites the old editor. Keep the original source repository: the worktree shares
its Git objects. Test map copies, saving/reloading and photo/editor transitions
before merging or publishing. Windows gameplay remains a separate validation gate.
