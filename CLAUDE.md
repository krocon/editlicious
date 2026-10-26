# CLAUDE.md – editlicious (Tauri)

> Leitfaden für Claude (und Menschen) zur Arbeit an **editlicious**: einem ruhigen, tastaturgesteuerten Texteditor
> für macOS, Windows und Linux, der **sehr große Dateien** (Millionen Zeilen, Zeilen mit Millionen Zeichen) öffnet
> und bearbeitet.
> Autor: Marc Kronberg · Repo: https://github.com/krocon/editlicious · MIT
>
> Neuauflage des Java/Swing-Editors **editlicious** (`~/IdeaProjects/java-k-editor/Editor`, fachliche Vorlage,
> `docs/legacy-analysis.md`) als Tauri 2 + Angular 22 (zoneless, Signals) + Rust. Architektur wie
> **comic-commander-tauri** und **files-and-folders-tauri** (gleicher Autor, MIT, öffentlich auf GitHub).

---

## 1. Stand

- Prompt-Liste und Status: `prompts/README.md`; Auftrag: `prompts/000-start.md`; Regeln: `prompts/000-arbeitsweise.md`.
- Erledigte Prompts samt Design-Dokument liegen in `prompts/done/`; Kürzel wie „002 E4“ in Code und Doku verweisen auf
  Entscheidungen dort.
- **Arbeitsweise (Marc, 26.10.2026):** Prompts selbständig nacheinander, nach jedem Prompt Commit + Push auf `main`.

## 2. Befehle (Root, pnpm 10 via `packageManager`, Node ≥ 22.22.3, Rust stable ≥ 1.93)

| Zweck | Befehl |
|---|---|
| Install | `pnpm install` |
| App starten (Dev) | `pnpm tauri dev` (Angular-Dev-Server auf **Port 1440** + Fenster) |
| Nur Frontend im Browser | `pnpm start` (kein Backend → Browser-Modus mit In-Memory-Dokumenten ab 002) |
| Frontend-Tests | `pnpm test` (Vitest + jsdom über `ng test`), `pnpm test:watch` |
| Lint / Format | `pnpm lint`, `pnpm format`, `pnpm format:check` |
| Frontend-Build | `pnpm build` |
| Rust | `pnpm rust:fmt`, `pnpm rust:lint` (clippy `-D warnings`), `pnpm rust:test` (ganzer Workspace inkl. `ed-core`) |
| Messungen | `cd src-tauri && cargo test -p ed-core --release -- --ignored --nocapture measure_` |
| Alles prüfen | `pnpm check` |
| Release | `pnpm version:set X.Y.Z`, `pnpm release:notes`, `pnpm notices` + `pnpm licenses:report` (nach Abhängigkeitsänderungen!); Pipeline testen: Workflow „Release“ Modus `dry-run` (kein Release) – Ablauf `docs/release.md` |
| Webseite | `pnpm site:test`; Demo-Engine `pnpm site:demo` (braucht wasm32-Ziel) bzw. Workflow „Site demo“ |
| Video rendern | `editlicious render <skript> -o out.mp4 …` bzw. `cargo run -p ed-video --release -- …`; Golden-Images neu: `UPDATE_GOLDEN=1 cargo test -p ed-video` |
| MCP-Sidecar | `pnpm mcp:dev` (Debug neben der Debug-App), `pnpm mcp:build [triple]` (Release nach `src-tauri/binaries/`) |
| Debug-App ohne Bundle | `pnpm tauri build --debug --no-bundle` → `src-tauri/target/debug/editlicious` |
| Icons neu erzeugen | `pnpm tauri icon src-tauri/icons/source/app-icon.svg -o src-tauri/icons` (danach `android/`, `ios/` löschen) |

## 3. Layout

```
src/                          Angular 22 (zoneless, Signals, OnPush, Standalone)
  styles.css                  Design-Tokens light/dark (einzige Stelle für Farben/Größen), Editor-Tokens --ed-*
  app/app.ts, app.config.ts   Root + Fassaden-Bindung
  app/core/backend/call.ts    einzige Stelle mit invoke() (+ isTauri-Prüfung, Fehler → FsError)
  app/core/errors/            FsError, toFsError, describeFsError (Codes = Rust error.rs)
  app/core/about/             AppInfo, AboutApi
  app/core/backend/desktop.ts DESKTOP-Token (Tauri da?)
  app/core/editor/            EditorApi (open, editor_command → Frame, save …), DTOs = Rust editor/model.rs
  app/core/actions/           Aktionen (aus comic-commander 008): action-catalog (Ids, Kontexte global/editor,
                              Editor-Aktionen mit Engine-Befehl), keymap.{base,mac,other}.json, ActionService,
                              KeyboardDispatcher (der eine keydown-Listener), shortcut, platform (OS-Token)
  app/core/workspace/         WorkspaceApi: Dateidialoge, session.json, Titel, Schließen, Drag & Drop
  app/features/editor/        EditorView (zeichnet Frames, Scrollleisten, Maus, IME-Textfeld, registriert die
                              Editor-Aktionen)
  app/features/documents/     DocumentsStore (offene Dateien, Öffnen/Speichern/Schließen, Sitzung, Änderungen von
                              außen, Beenden), TabBar
  app/features/find/          FindStore (Muster, Optionen, Verlauf), FindBar, GotoBar
  app/shared/ask/             AskService + AskDialog (Fragen als Promise)
  app/shared/pick/            PickService + PickDialog (filterbare Auswahl: Sprache, später Encoding)
  app/core/menu/              MenuApi (set_app_menu, menu-action), KeymapApi (keymap.json)
  app/features/menus/         menu-model (rein), MenuService (nativ/HTML), MenuBar (APG, versteckt bis Alt),
                              CommandPalette, ShortcutsDialog, ChromeStore (F1, Menüleiste, Vollbild)
  app/core/recording/         RecordingApi (Aufnahme starten/stoppen/speichern, prüfen, abspielen, steuern)
  app/features/recording/     RecordingStore (record.*-Aktionen, Wiedergabe-Zustand, gepushte Frames), Texte
  app/core/mcp/               McpApi (KI-Steuerung an/aus, Aktionen/aktives Dokument melden, Ereignisse)
  app/features/mcp/           McpStore (führt aus, was die KI verlangt: Tabs, Aktionen, Wiedergabe, Status)
  app/features/about/         About-Dialog
  app/core/i18n/              translatable(), Sprache (Signal), System-Sprache, catalogs.spec (Vollständigkeit)
  app/core/settings/          Settings-DTO (= Rust settings.rs), SettingsApi
  app/features/settings/      SettingsStore (laden, sofort anwenden, gebündelt speichern, Zoom), SettingsDialog
  app/shared/dialog/          Modal (Fokusfalle, Esc), DialogService (aus comic-commander/files-and-folders)
  testing/                    Fakes – nie im Produktivcode importieren
public/                       favicon.png, logo.svg (Schriftzug, Yellowtail-Pfade), icon.svg (App-Icon)
src-tauri/                    Rust (Tauri 2), Crate `editlicious`, Lib `ed_lib`
  src/lib.rs                  Modulübersicht, Builder, Commands
  src/commands.rs             dünne Command-Schicht, Test command_rules
  src/app_menu.rs             natives macOS-Menü aus dem Frontend-Modell (ohne vordefiniertes Edit-Menü)
  src/keymap_file.rs          keymap.json laden
  src/session_file.rs         session.json (offene Dateien, zuletzt benutzt, Tab-Modus)
  src/settings.rs             settings.json (Einstellungen, nachsichtig laden, Werte begrenzen)
  src/export.rs               „Als Video exportieren …“ (Thread, Fortschritt, Abbruch)
  src/mcp.rs                  KI-Steuerung: Brücken-Server (127.0.0.1 + Token) und Werkzeuge auf den Sitzungen
  src/playback.rs             Aufnahmen/Skripte lesen und speichern, Wiedergabe-Thread (Frames über Channel)
  src/model.rs                DTOs der App (serde camelCase); `document`, `error` kommen aus ed-core
  core/                       Crate `ed-core` (Workspace-Mitglied): Editor-Engine ohne Tauri
    src/error.rs              FsError { code, message, path }
    src/document/             Rope, load (Encoding/BOM/Zeilenenden), save (atomar), history (Undo), registry, model
    src/highlight.rs          Syntax-Hervorhebung (syntect, Checkpoints, Hintergrund-Aufholen, Stilklassen)
    src/search.rs             Suchen/Ersetzen in Zeilenblöcken (regex), Zählen auf Schnappschuss
    src/editor/               Sitzung (Cursor, Befehle, Scroll, Frames), layout (Zellen, Grapheme), words,
                              ops (Sortieren, Groß/klein, Zeilen, Blockauswahl, mehrere Cursor, Marker),
                              recording (*.edrec.jsonl), script (*.edscript), playback (Zeitachse, Tempo),
                              colors (Farbwerte erkennen/umrechnen), expand (Doppelklick-Auswahl wächst),
                              hints (Rechenausdruck am Caret, Hinweis und Vorschläge)
    src/math.rs               Rechen-Parser (Vorlage MathParser): Rangfolge, Funktionen, Zahlenformate
    testdata/document.cases.json  datengetriebene Bearbeitungsfälle; math.cases.json Rechenfälle
  mcp/                        Crate `ed-mcp` (Workspace): MCP-Sidecar `editlicious-mcp` (stdio), Tools, Brücke
  video/                      Crate `ed-video` (Workspace): Video-Export (Zeitachse, Rasterizer, Farben aus
                              src/styles.css, ffmpeg/PNG, CLI `editlicious render`), fonts/, testdata/ (Golden-Images)
  tauri.sidecar.conf.json     nur im Release: Sidecar als `bundle.externalBin` (`pnpm mcp:build` vorher)
  src/fs_atomic.rs, config_file.rs  atomar schreiben / Konfig laden (aus files-and-folders)
  icons/source/app-icon.svg   Quelle des App-Icons
  tauri.windows.conf.json, windows/open-with.nsh   Windows: „Öffnen mit“ per NSIS-Hook statt Standard-Zuordnung
  linux/editlicious.desktop   .desktop-Vorlage (Exec mit %F, MimeType text/plain)
docs/                         architecture.md (Entscheidungen, Commands, Abhängigkeiten), legacy-analysis.md,
                              scripts.md (Aufnahmen und Tipp-Skripte), mcp.md (KI-Steuerung) – für Nutzer
prompts/                      Feature-Prompts (Index README.md), done/
site/                         Werbe-Webseite (index.html ohne JS) + demo/ (Engine als WebAssembly, pkg/ vom Workflow)
  src-tauri/wasm/             Crate `ed-wasm` (wasm-bindgen-Bindung von ed-core, außerhalb des Workspaces)
```

## 4. Architekturregeln (verbindlich)

- **Rust ist die Editor-Engine, die UI rendert nur** (Marc, 26.10.2026; Design 002 E13): Text (Rope), Carets,
  Auswahl, Navigation, Bearbeitungsbefehle, Scrollstand und Layout in Zellen leben im Crate `ed-core`
  (`src-tauri/core`, ohne Tauri, WASM-fähig). Die UI übersetzt Eingaben in Befehle und zeichnet Frames (sichtbare
  Zeilen bzw. Spaltenfenster). Nichts, was proportional zur Dateigröße ist, geht über die IPC-Grenze – auch nicht
  beim Kopieren großer Auswahlen, Suchen oder Sortieren. Ausnahme: IME-Komposition bis zum Abschluss.
- **Angular bleibt die UI** (Entscheidung 26.10.2026): WebAssembly nur für die Webseiten-Demo; die Engine läuft
  nativ (große Dateien, Threads, kein 4-GB-Limit, kein Kopieren der Datei über die Brücke).
- **Frontend ruft das Backend nur über typisierte Fassaden** (`AboutApi`, `DocumentApi` …, in `app/core/`);
  `invoke()` steht nur in `core/backend/call.ts`. Tests und der Browser-Modus ersetzen die Fassade.
- **DTO-Vertrag**: Rust `model.rs` ↔ TS `*.models.ts` von Hand synchron; jede Änderung auf beiden Seiten + Test.
  Fehler immer als `FsError { code: snake_case, message, path }`; neue Codes in `error.rs`, `fs-error.ts` und
  `describeFsError()`. Im UI nie Rusts `message` zeigen.
- **Pfade sind opake Strings** aus dem Backend; kein Split/Join mit `/` oder `\` im Frontend.
- **Zustand**: Signals; Stores je Ansicht ohne `providedIn: 'root'`; Globales in Root-Services. `OnPush` überall,
  kein zone.js. Veraltete Antworten verwerfen (Revision/Sequenz).
- **Commands nie synchron**: `async fn` + `spawn_blocking` für blockierende Arbeit (`command_rules`-Test). Logik in
  reinen Modulen ohne Tauri, Command-Schicht dünn. Lange Arbeit mit Fortschritt über `tauri::ipc::Channel` + Abbruch.
- **Speichern ist heilig**: atomar (Temp-Datei + rename), Rechte erhalten, Symlinks auflösen, nie halb geschriebene
  Dateien; Encoding, BOM und Zeilenende bleiben wie geladen, außer der Nutzer ändert sie.
- **Sicherheit**: keine Tauri-Plugins mit breitem Dateisystem- oder Shell-Zugriff für die WebView; Dialoge nur über
  die Rust-API; CSP strikt (`style-src 'unsafe-inline'` wegen Angulars Laufzeit-Styles). Dateiinhalte nie loggen.
- **Tastatur nur über Aktionen** (seit 004): jede Funktion ist eine Aktion in `action-catalog.ts` (Editor-Aktionen
  tragen ihren Engine-Befehl), Kürzel stehen in `keymap.*.json`, der einzige globale `keydown`-Listener ist der
  `KeyboardDispatcher`. Widgets mit eigener Tastatur nach APG (Tabs, Menüs) behalten Element-Handler und rufen
  `preventDefault`.
- **Wenig Ablenkung**: neue sichtbare Bedienelemente nur mit Begründung; Funktionen gehören in Aktionen,
  Befehlspalette und (ausblendbares) Menü.
- **Neue Abhängigkeiten** (npm oder crates) nur mit kurzer Begründung in `docs/architecture.md`. Keine
  UI-Frameworks; Styling mit CSS (natives Nesting) + Tokens aus `src/styles.css` – kein SCSS.
- **Texte** nur aus Katalogen (`*-texts.ts` über `translatable()` aus `core/i18n`, Englisch = Quelle, Deutsch
  daneben; Aktionsnamen deutsch in `action-catalog.de.ts`). `catalogs.spec.ts` prüft Vollständigkeit.
- **Barrierefreiheit**: sichtbarer Fokus, alles per Tastatur, Dialoge mit `role="dialog"`/`aria-modal` und
  Fokusfalle, Live-Region für Statusmeldungen, Kontraste in light und dark.

## 5. Tests & Verifikation

- **Rust**: Unit-Tests neben dem Code (`#[cfg(test)]`), Dateisystem mit `tempfile`; Unicode (Emoji, CJK,
  kombinierende Zeichen), alle Zeilenenden, BOMs, Legacy-Encodings, große Dateien (ignored-Tests mit Messung).
- **Angular**: Vitest (`*.spec.ts` neben dem Code), Stores und Komponenten mit Fakes/In-Memory-Dokumenten.
- **Ende-zu-Ende**: echte App starten und die Akzeptanzkriterien durchspielen; Testdateien per Generator, nie in
  echten Nutzerdaten schreiben.
- Jeder Bugfix bekommt zuerst einen roten Test.

## 6. Git & Zusammenarbeit

- **Commits direkt auf `main`, Pushen erlaubt** (Marc, 26.10.2026) – Conventional Commits (`feat:`, `fix:`,
  `docs:`, `test:`, `chore:`), nach jedem Prompt mit grünem `pnpm check`.
- Kein Force-Push, kein `git reset --hard`, keine Releases ohne ausdrückliche Anweisung.

## 7. Lizenzen & Referenzen

- Eigene Lizenz: **MIT**.
- **comic-commander-tauri**, **files-and-folders-tauri** (MIT, gleicher Autor): Code darf übernommen werden – mit
  Herkunftsvermerk im Modulkopf und den zugehörigen Tests.
- Schriftzug im Logo: Pfade aus *Yellowtail* (Astigmatic, Apache License 2.0 – google/fonts `apache/yellowtail`; Text in THIRD-PARTY-NOTICES.txt).

## 8. Arbeitsumgebung für Claude (Remote-Sitzung)

- Das Repo liegt auf Marcs Mac (`~/WebstormProjects/editlicious`). Gebaut, getestet und gepusht wird in der
  **Cloud-Umgebung** (Klon über `add_repo` mit Schreibrecht; Push auf `main` funktioniert, Stand 26.10.2026).
- Die Vorlage liegt auf dem Mac (`~/IdeaProjects/java-k-editor/Editor`) – per `device_bash` lesen. Die
  Schwesterprojekte sind öffentlich: `git clone --depth 1 https://github.com/krocon/comic-commander-tauri`.
- Container einrichten: `apt-get install -y libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev
  librsvg2-dev librsvg2-bin xdotool imagemagick xvfb`, `pnpm install`, `cd src-tauri && CARGO_INCREMENTAL=0 cargo build --tests`.
- Ende-zu-Ende: `pnpm tauri build --debug --no-bundle`, `Xvfb :99 &`, `DISPLAY=:99 src-tauri/target/debug/editlicious &`,
  Screenshot `DISPLAY=:99 import -window root shot.png`; `GTK_THEME=Adwaita:dark` schaltet dunkel. `xdotool type`
  kann keine Mehrbyte-Zeichen – solche Texte per Datei.
