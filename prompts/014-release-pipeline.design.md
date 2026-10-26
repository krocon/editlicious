# Design 014: Release-Pipeline

Vorlage: comic-commander-tauri (Prompt 012, Release 1.0.1) und files-and-folders-tauri (Design 017); deren
`release.yml` ist seit 012 übernommen. Neu hier, damit die Pipeline ohne echtes Release prüfbar ist und mehr selbst
erledigt:

- **E1 Modi**: `workflow_dispatch` mit Auswahl `dry-run` (Standard) | `draft`; Tag `v*` = Entwurf. Im Probelauf
  baut `tauri-action` ohne `tagName` – es entsteht nichts auf der Release-Seite. CLAUDE.md §6 („keine Releases ohne
  Anweisung“) bleibt gewahrt, die Pipeline lässt sich trotzdem jederzeit vollständig testen.
- **E2 Pre-flight-Job**: Versionen/Tag, alle Skript-Tests, Release Notes (seit dem vorigen Tag, ohne den eigenen)
  mit festem Kopf/Fuß (Downloads, Prüfsummen, Hinweis „unsigniert“) → Ausgabe `body` → `releaseBody` des Entwurfs.
- **E3 Paketprüfung** `scripts/collect-packages.mjs` (+ Tests): sucht alle `…/release/bundle`-Ordner (auch
  `universal-apple-darwin`), prüft die erwarteten Arten je OS, kopiert sie (`.app` als `.app.tar.gz`), schreibt
  `SHA256SUMS-<os>.txt` und eine Tabelle in `GITHUB_STEP_SUMMARY`. Dazu OS-eigene Prüfungen: `.deb`-Inhalt und
  `.desktop` (Linux), `lipo`-Architekturen, Lizenzhinweise, `LSHandlerRank` (macOS).
- **E4 Prüfsummen-Job** führt die Teil-Dateien zu `SHA256SUMS.txt` zusammen, prüft mit `sha256sum -c` und hängt sie
  an den Entwurf (`gh release upload --clobber`).
- **E5 Lizenzbericht** `docs/licenses.md` (`scripts/licenses.mjs` aus files-and-folders) mit `--check`: veraltet
  oder ungeprüfte nicht-permissive Lizenz → Fehler; MPL-2.0-Crates von Tauri (cssparser, selectors …) als geprüft.
- **E6 Nicht gemacht**: echtes Release/Tag, Signier-Secrets, Updater, Paketmanager-Einträge (Comic Commander
  Prompt 014 „Distribution“ als Vorlage für einen späteren Prompt).

## Verifikation

Probelauf des Workflows auf GitHub (alle drei Runner) – Ergebnis im Abschlussbericht.
