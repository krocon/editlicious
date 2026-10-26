# Prompt 001: Grundgerüst (Walking Skeleton)

> Lies zuerst [000-arbeitsweise.md](000-arbeitsweise.md). Braucht: –

## Ziel

Ein lauffähiges, geprüftes Grundgerüst wie bei comic-commander-tauri: **Tauri 2 + Angular 22 (zoneless, Signals,
Standalone, OnPush) + Rust**, pnpm 10, Vitest, ESLint, Prettier, `pnpm check`, GitHub-CI. Das Fenster zeigt eine
ruhige, leere Editorfläche mit Platzhaltertext und – per Kürzel – einen About-Dialog mit Version aus dem Backend.

## Umfang

- Projektdateien analog comic-commander: `package.json` (Skripte inkl. `check`), `angular.json` (Port **1440**),
  `tsconfig*.json`, `eslint.config.js`, `.prettierrc`, `.editorconfig`, `.gitattributes`, `.gitignore`.
- Rust-Crate `editlicious` (Lib `ed_lib`): `lib.rs`, `commands.rs` (dünn, `command_rules`-Test), `error.rs`
  (`FsError { code, message, path }`), `about.rs`, `model.rs`, `fs_atomic.rs` + `config_file.rs` (übernommen),
  `tauri.conf.json` (Identifier `io.github.krocon.editlicious`, strenge CSP), `capabilities/default.json`.
- Frontend: `core/backend/call.ts` (einziges `invoke`), `core/errors/fs-error.ts`, `core/about/`, `styles.css` mit
  Design-Tokens hell/dunkel, `shared/dialog/` (Modal + DialogService aus comic-commander), `testing/` (Fakes).
- App-Icon (SVG-Quelle unter `src-tauri/icons/source/`, alle Größen erzeugt) und Logo `public/logo.svg`.
- Doku: `CLAUDE.md`, `README.md`, `docs/architecture.md`, `docs/legacy-analysis.md` (fachliche Analyse des
  Java-Editors: Aktionen, Kürzel, Optionen, Farbstile, Todo-Liste), `LICENSE` (MIT).
- CI: `.github/workflows/ci.yml` (Checks, Rust auf 3 OS).

## Akzeptanz

- `pnpm check` grün, `pnpm tauri build --debug --no-bundle` baut, App startet unter Xvfb und zeigt das Fenster.
- `pnpm start` im Browser zeigt die Oberfläche mit Hinweis „desktop backend is not available“ statt eines Fehlers.
