# Design 012: Release und Distribution

Übernommen aus comic-commander-tauri (Prompt 012) bzw. files-and-folders-tauri (Design 017); Abläufe in
`docs/release.md`.

## Entscheidungen

- **E1 Eine Version**: `package.json` ist die Quelle; `tauri.conf.json` verweist darauf, `Cargo.toml`/`Cargo.lock`
  hält `scripts/version.mjs` gleich (`pnpm version:set|check`, Teil von `pnpm check` und `release.yml`).
  Release Notes aus Conventional Commits (`pnpm release:notes`), Lizenzhinweise `THIRD-PARTY-NOTICES.txt`
  (`pnpm notices`, im Paket über `bundle.resources`, `notices:check` in `check` und Release).
- **E2 Dateizuordnung nur als „Öffnen mit“** – ein Texteditor soll sich anbieten, nicht Standard werden:
  macOS `fileAssociations` mit Rang `Alternate`; Linux `MimeType=text/plain`; Windows: Tauris Zuordnungen würden
  den Standard umstellen, daher dort aus (`tauri.windows.conf.json`, JSON-Merge ersetzt das Array) und ein
  NSIS-Hook (`windows/open-with.nsh`) trägt die App in `Applications\editlicious.exe` + `OpenWithList` ein (HKCU,
  passend zu `installMode: currentUser`), Deinstallation räumt auf. Liste: Text, Logs, Daten, Konfiguration und
  verbreitete Quelltexte.
- **E3 Dateien zur laufenden App**: macOS liefert Finder-Dateien als `RunEvent::Opened` (nicht als argv);
  Windows/Linux starten ein zweites Mal – `tauri-plugin-single-instance` reicht argv an die laufende App (neue
  Abhängigkeit, offizielles Plugin). Beides landet in `files_opened`: vor dem Start der Oberfläche in einer
  Warteschlange (die `startup_paths` mitliefert), danach als Ereignis `open-files`. Die App lauscht schon vor
  `startup_paths` und öffnet früh gekommene Dateien nach dem Start.
- **E4 Pakete**: dmg/app (universal, macOS 12+), msi + NSIS (currentUser, en/de, feste WiX-`upgradeCode`),
  deb/rpm/AppImage (Ubuntu 22.04 im Workflow). `release.yml`: Tag `v*` oder manuell → Entwurf, `0.x` als
  Pre-release; Signierung vorbereitet (Secrets optional, ohne → unsigniert).
- **E5 Schriftlizenz**: Yellowtail (Schriftzug) steht unter **Apache-2.0** (google/fonts `apache/yellowtail`),
  nicht OFL wie bisher notiert – korrigiert in README, CLAUDE.md, Webseite; Text in den Lizenzhinweisen.

## Tests

- `scripts/*.test.mjs` (Version, Release Notes, Lizenzhinweise, Webseite) über `pnpm scripts:test`.
- Rust: `paths_from_args` (Optionen, relative Pfade, fehlende Dateien), `command_rules` (Anzahl Commands).
- Angular: Dateien aus `open-files` werden geöffnet.
- Ende-zu-Ende im Container: `.deb` gebaut, installiert, `.desktop` mit `MimeType`, App startet; Zweitstart mit
  Datei öffnet sie in der laufenden App.
