# Editor 0.8.5-pre — Windows startup hotfix

## Deutsch

Das Windows-Paket enthält jetzt `dxcompiler.dll` und `dxil.dll` samt Lizenztexten aus Microsoft DirectXShaderCompiler v1.9.2609. Ohne diese Dateien konnte die DirectX-12-Shader-Erstellung auf den älteren FXC-Compiler zurückfallen und mit Stack Overflow abbrechen: kurz ein graues Fenster, danach Programmende.

In unserer Windows-11-VM war der Absturz mit 0.8.4-pre reproduzierbar. Nach Ergänzen der DLLs startete dieselbe Engine zweimal erfolgreich, zeigte Grundorf und die Editoroberfläche und ließ sich normal beenden. Auch die Kamerabedienung wurde bestätigt. Getestet wurde DirectX 12 mit Microsofts Software-Renderer; dies ist kein Leistungsnachweis und noch keine Bestätigung auf der gemeldeten RTX 5070 Ti. Die VM-OpenGL-Darstellung ist separat weiterhin nicht geklärt.

Bitte das komplette neue Windows-ZIP in einen neuen Ordner entpacken. Bei weiteren Problemen die vollständige game.log direkt nach dem Versuch sowie Windows-, Grafiktreiber-, Karten- und Editorversion melden. Keine einzelnen DLLs von fremden Downloadseiten beziehen.

Die Paketerstellung prüft jetzt beide DLLs und die Lizenztexte als Pflichtdateien. Der Microsoft-Download ist auf Version und SHA256 festgelegt; CI erhält zusätzlich einen DirectX-12-Renderer-Test. Kartenbau-Funktionen und Engine-Basis 0.2.50 bleiben unverändert. Die DE/EN-PDF-Anleitungen aus 0.8.4 gelten weiterhin.

## English

The Windows package now includes Microsoft's `dxcompiler.dll` and `dxil.dll`, with their licences, from DirectXShaderCompiler v1.9.2609. Without these libraries, DirectX 12 could fall back to the legacy FXC shader compiler and abort with a stack overflow shortly after opening a grey window.

The failure was reproduced in a Windows 11 VM with 0.8.4-pre. Adding the libraries allowed the same engine to start twice, display Grundorf and the editor UI, and exit normally. Camera operation was confirmed. These tests used Microsoft's DirectX 12 software renderer; performance and the originally reported RTX 5070 Ti still require separate confirmation. The VM's blank OpenGL display remains a separate unresolved issue.

Extract the complete new Windows ZIP into a fresh folder. If problems remain, send the full game.log immediately after the attempt, plus Windows, graphics driver, map and editor versions. Do not download individual DLLs from third-party DLL sites.

Packaging now requires both libraries and licence files. The Microsoft download is pinned and SHA256-verified; CI also runs a DirectX 12 renderer regression. Map editing features and engine base 0.2.50 are unchanged. The existing German and English 0.8.4 PDF guides remain applicable.
