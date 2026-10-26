# Design 011: Webseite und Live-Demo

## Entscheidungen

- **E1 Eine statische Seite** (`site/index.html`, Englisch) wie bei comic-commander und files-and-folders: CSS
  inline, kein JavaScript, keine Webfonts, keine fremden Anfragen; hell/dunkel nach System. Erbe der alten Seite
  (legacy §10): Akzentrot `#e00034` (dunkel `#ff4d6d`), Schriftzug (Yellowtail-Pfade inline), Claim-Ton
  „Still tastier than chocolate“, FAQ „Why editlicious?“. Hauptbotschaft neu: große Dateien, lange Zeilen, Ruhe.
- **E2 Abschnitte**: Hero (Logo, Claim, zwei Knöpfe Download/Demo, Screenshot), Messwert-Kacheln (aus
  `docs/architecture.md`), zwei Kapitel mit je zwei Bild-Text-Paaren (große Dateien; ruhig + Tastatur), Karten
  für weitere Funktionen, Demo-Hinweis, Kürzel-Tabelle, Download-Tabelle („planned“ bis 012), FAQ, Fuß mit
  Impressum-Link auf krocon.de. Nur Aussagen, die der Stand hält (Download-Pakete als geplant markiert).
- **E3 Bilder**: echte Fensteraufnahmen der App (Debug-Build unter Xvfb, 1100 × 760, Englisch) in fünf
  Farbschemata/Situationen; `width`/`height` gesetzt (kein Layout-Springen), `loading="lazy"` unterhalb des
  Heros, aussagekräftige Alt-Texte. Test prüft Existenz, Maße und Alt-Text.
- **E4 Live-Demo** (`site/demo/`): Crate `ed-wasm` (`src-tauri/wasm`, außerhalb des Cargo-Workspaces, eigene
  Lock-Datei) bindet `ed-core` mit Feature `fancy` (reines Rust statt Oniguruma) über `wasm-bindgen` an:
  `Editor.command(json) → json` – dieselben Befehle und Frames wie `editor_command` der App. `demo.js` zeichnet
  Frames (Umbruch, Mehrfach-Cursor, Auswahl, Treffer) und übersetzt Tasten/Maus – keine Textlogik. Knöpfe laden
  eine Million Zeilen bzw. eine Zeile mit fünf Millionen Zeichen, um die Engine zu zeigen.
- **E5 Bauen**: Im Container fehlt das wasm32-Ziel (static.rust-lang.org gesperrt) – gebaut wird im Workflow
  **Site demo** (rust-toolchain mit wasm32, wasm-bindgen in der Version der Lock-Datei über
  `taiki-e/install-action`), der `site/demo/pkg` bei Änderungen zurück auf `main` committet (GITHUB_TOKEN,
  löst keine weiteren Läufe aus). Lokal: `pnpm site:demo`. Für WebAssembly nutzt `ed-core` `web-time::Instant`
  (std::time::Instant gibt es dort nicht) und holt die Hervorhebung ohne Hintergrund-Thread nach (cfg).

## Tests

- `pnpm site:test` (node:test): Bilder vorhanden, Maße und Alt-Text, keine Skripte und keine fremden Server auf
  der Hauptseite, Anker vorhanden, Demo-Seite mit Engine.
- `ed-wasm`: nativer Test der Bindung (Befehle rein, Frames raus, Zwischenablage, Fehler, Sprachen).
- Ende-zu-Ende: Seite mit Playwright hell/dunkel/mobil (kein horizontales Scrollen bei 390 px), Demo im Chromium
  (Tippen, Millionen Zeilen, lange Zeile).
