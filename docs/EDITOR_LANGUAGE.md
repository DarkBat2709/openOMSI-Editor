# Editor language switching — 0.7.7-pre

The editor uses the language selected in openOMSI's settings. Its labels, tooltips,
status messages and launcher map page use English source strings with German
translations in `crates/omsi-app/locales/editor.yml`. Existing translations are
reused from `app.yml`. Languages without an editor translation use the normal
openOMSI fallback; complete translations for all supported languages are not
included in this update.

Messages remain in their source language internally and are translated when
drawn, so an existing status can change language without reopening the map.
Complete messages are translated before wrapping, clipping or splitting HUD rows.
Native dialogs use the selected language when opened.

This update does not translate user-authored map names, sign text, asset files or
saved junction projects on disk. The roundabout builder was added in 0.7.8-pre; see `ROUNDABOUT_BUILDER.md`.

## Validation

`python3 scripts/editor/test_editor_i18n.py` needs Python 3 and rustc, with no
additional Python packages. It compiles the actual UI translator and checks the
editor tables, format arguments, repeated DE/EN switching, nested messages,
translation idempotence and fallback to existing translations/English. The
editor preview workflow runs this on Linux and Windows.

Before publishing, test in the running editor:

1. Select English in settings and return to the editor without restarting.
2. Check the object/spline tool panel, asset catalogue, terrain heights and
   texture painting, junction builder and spline connection check.
3. Change back to German. Keep a selection/status message and check that it
   changes too; check long text in narrow panels.
4. Open the reload dialog and cancel it. Check the translated buttons.
5. End the session and check the launcher's Editor page in both languages.
6. Check that typed map names and object text retain their contents.

The Linux installation script keeps the old program until its tests and release
build have succeeded and makes a backup before installing. It never publishes.
