# Design 001: Grundgerüst

Status: umgesetzt · Datum: 26.10.2026

## Entscheidungen

- **E1 Gerüst aus comic-commander übernommen** (statt `create-tauri-app`): gleiche Skripte, Lint-/Format-Regeln,
  `call.ts`, `FsError`, `Modal`/`DialogService`, `fs_atomic.rs`, `config_file.rs`, CI. Herkunft steht in den
  Modulköpfen.
- **E2 Port 1440**, Crate `editlicious`, Lib `ed_lib`, Identifier `io.github.krocon.editlicious`.
- **E3 Kein festes Theme beim Start:** anders als comic-commander folgt editlicious dem System (`prefers-color-scheme`);
  `data-theme` erzwingt hell/dunkel (Einstellung in 009).
- **E4 Logo:** Schreibschrift-Schriftzug wie die alte Version, als Pfade aus *Yellowtail* (OFL) erzeugt
  (`public/logo.svg`, `docs/wordmark.svg`); App-Icon neu gezeichnet (goldene Feder schreibt eine blaue Linie,
  macOS-Raster 824 px / Radius 185.4) – das alte Füller-Icon stammt aus einem Icon-Set und wird nicht übernommen.
- **E5 Editor-Tokens `--ed-*`** in `styles.css` schon jetzt (Hintergrund, Zeilennummern, Auswahl, Caret, Suche),
  damit 003 nur noch Tokens verwendet.
- **E6 Kein `version:check`/`notices` im CI** bis Prompt 012 die Skripte liefert.

## Verträge

- Command `get_app_info() -> AppInfo { name, version, os, arch }`.
- Fehlercodes: `not_found`, `permission_denied`, `not_a_directory`, `is_a_directory`, `invalid_path`, `io`
  (+ Frontend `backend_unavailable`, `unknown`).

## Tests

- Rust: `error` (snake_case, io-Mapping), `about`, `command_rules`, übernommene `fs_atomic`/`config_file`-Tests.
- Frontend: `fs-error.spec.ts`, `app.spec.ts` (About zeigt Version aus der Fassade).

## Fortschritt

- [x] Gerüst, Icon, Logo, Doku, CI · `pnpm check` grün · Debug-Build unter Xvfb gestartet
