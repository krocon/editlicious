# Prompt 025: Einbettbare Editor-Komponente – Angular & Web Component (`<editlicious-editor>`)

> Lies zuerst [000-arbeitsweise.md](000-arbeitsweise.md). Braucht: 003, 005, 006, 008–011, 018–021, 023.
> Bewusste Erweiterung von Entscheidung A5 (`docs/architecture.md`, `CLAUDE.md` §4): Die Desktop-App nutzt weiterhin
> unverändert die native Rust-Engine über Tauri-IPC; **zusätzlich** entsteht aus derselben Angular-UI und `ed-wasm`
> eine einbettbare Editor-Komponente (**Variante B**: Angular-Standalone-Komponente **und** Custom Element
> `<editlicious-editor>` über `@angular/elements`).

## Ziel

`editlicious` lässt sich als eigenständige, mehrinstanzfähige Komponente in beliebige Web-Anwendungen einbetten –
wahlweise direkt in Angular (`<ed-editor>`) oder frameworkunabhängig als Standard-Web-Component
(`<editlicious-editor>`) mit Shadow DOM. Im Inneren arbeitet dieselbe Rust-Engine (`ed-core` als WebAssembly über
`ed-wasm`) mit derselben virtualisierten Render-Ansicht (`EditorView`, `FindBar`, `GotoBar`, optional `StatusBar`) wie
in der Desktop-App: Millionen Zeilen, Zeilen mit Millionen Zeichen, 13 Farbschemata, Mehrfach-Cursor, Suchen/Ersetzen,
Wort-Marker, Farbfelder, Rechen-Hinweise und Wort-Vervollständigung – aus **einer einzigen UI-Codebasis**.

## Umfang

1. **Engine & WebAssembly-Bindung ausbauen (`ed-core`, `src-tauri/wasm`):**
   - **Schreibschutz (`readOnly`) in `ed-core`:** Optionaler Sitzungs-Zustand (z. B. über `Command::Configure { read_only }`),
     der alle textverändernden Befehle ablehnt, während Navigation, Auswahl, Kopieren („Als HTML kopieren“ inklusive),
     Suche, Gehe-zu-Zeile, Wort-Marker und Rechen-Hinweise am Caret uneingeschränkt funktionieren (wichtig für
     Log-/JSON-Viewer im Web).
   - **`ed-wasm` vervollständigen:** Alles ergänzen, was `EditorView` und `StatusBar` über `EditorApi` benötigen und in
     `ed-core` bereits vorhanden ist: `info()`, `set_text(text)` (Sitzungstext ersetzen), `line_ending` setzen,
     `count_matches(query_json)`, `count_marker()`, `overview(buckets)`, `encodings()` sowie HTML-Ablage im
     `MemoryClipboard` (`clipboard_html()` neben `clipboard()`), damit „Als HTML kopieren“ samt deckenden Wort-Markern
     (019) auch im Browser funktioniert.
   - **Spracherkennung ohne Dateipfad:** Sprache entweder per Grammatikname (`"Rust"`, `"TypeScript"`) oder über einen
     virtuellen Dateinamen/Endung (`"app.ts"`, `".kt"`) setzen können, sodass auch TypeScript/Kotlin/Angular-Lexika aus
     023 greifen.
   - **Container-Hinweis (wie 011 E5):** Native Bindungstests laufen mit
     `cargo test --manifest-path src-tauri/wasm/Cargo.toml`. Fehlt im Container das Target `wasm32-unknown-unknown`,
     baut lokal `pnpm site:demo` (bzw. das erweiterte WASM-Build-Skript) und in CI der Workflow `.github/workflows/site-demo.yml`.

2. **`WasmEditorApi` & Web-Zwischenablage (`src/app/core/editor/`):**
   - Neue `EditorApi`-Implementierung auf Basis von `ed-wasm` mit **einmaliger, geteilter, verzögerter
     WASM-Initialisierung** (`initEditlicious({ wasmUrl? })`, Standard relativ zu `import.meta.url`), die von beliebig
     vielen Editor-Instanzen auf derselben Seite gemeinsam genutzt wird.
   - Im Design bewerten und entscheiden: Ausführung der WASM-Sitzungen in einem **Web Worker** (da `EditorApi` ohnehin
     asynchron ist, blockieren dann auch `countMatches`, `overview`, `replaceAll` oder `sortLines` auf 100 MB nie den
     UI-Thread – als Einzeldatei/Inline-Worker baubar) gegenüber Main-Thread-Ausführung, insbesondere im Zusammenspiel
     mit der Browser-Zwischenablage.
   - **Browser-Clipboard:** `copy`, `cut` und `copyHtml` im Browser sauber über `ClipboardEvent.clipboardData` bzw.
     `navigator.clipboard.write` (`text/plain` + `text/html`) anbinden; in Tauri bleibt `arboard` unberührt.

3. **Mehrinstanz-Fähigkeit & Entkopplung von Desktop-Singletons:**
   - **Zwei oder mehr `<editlicious-editor>`-Instanzen auf einer Seite** müssen komplett unabhängig voneinander laufen:
     - `ActionService`, `FindStore` und ein instanzlokaler Einstellungs-/Ansichtszustand werden pro eingebetteter
       Editor-Komponente bereitgestellt (`providers` auf der Komponente), ohne das Verhalten der Tauri-Shell (`App`) zu
       brechen.
     - `KeyboardDispatcher`: in der Komponente **nur am Host-Element** lauschen, niemals global auf `document` – und
       niemals global `contextmenu` auf der Host-Seite unterdrücken. Nicht behandelte Tasten (z. B. Browser-Kürzel)
       dürfen die einbettende Seite nicht stören.
     - Desktop-Abhängigkeiten in `EditorView` sauber ablösen bzw. optional machen (`DocumentsStore` beim
       Encoding-Wechsel, `RecordingStore`, `PickService` für Sprach-/Zeilenende-Auswahl innerhalb der Komponente).
     - Statische ARIA-IDs in `editor-view.html` (`math-hint`, `completion-list`, `completion-N`) pro Instanz eindeutig
       machen, damit ARIA-Beziehungen (`aria-controls`, `aria-activedescendant`, `aria-describedby`) sowohl bei
       mehreren Angular-Komponenten im Light DOM als auch innerhalb jedes Shadow Roots gültig und kollisionsfrei sind.
     - **Lebenszyklus & Speicher:** Beim Entfernen (`DestroyRef` / `disconnectedCallback`) Sitzung in WASM freigeben
       (`close`/`free`), Observer und Timer abräumen; erneutes Einhängen (`connectedCallback` kann mehrfach feuern)
       idempotent behandeln.

4. **Kapselung der Styles & 13 Farbschemata (Shadow DOM & Light DOM):**
   - Die Komponente darf **niemals** `document.documentElement` (`data-theme`, `data-scheme`, `lang`,
     `--editor-font-size`, `--font-editor`) oder `body` der Host-Seite verändern.
   - Alle 13 Farbschemata (`paper`, `glass`, `sand`, `harvest`, `chalk`, `one`, `moss`, `mocha`, `moon`, `night`,
     `graphite`, `ink`, `amber`) sowie `auto` (`prefers-color-scheme`) müssen **je Instanz** über `:host` funktionieren
     (z. B. ein heller und ein dunkler Editor nebeneinander auf derselben Seite).
   - **Kein `:host-context()` verwenden** (in Firefox und Safari nicht unterstützt). Stattdessen Token-Selektoren über
     `:host` / `:host([data-scheme='…'])` / `:host([data-theme='…'])` und vererbbare CSS-Custom-Properties
     (`--ed-*`, `--syn-*`, `--color-*`) aufbauen, sodass Konsumenten Farben und Schriften von außen überschreiben
     können. `:host` erhält `display: block` und eine sinnvolle Standard-/Mindesthöhe; Formular-Controls im Shadow DOM
     (`FindBar`, `GotoBar`, `StatusBar`) erben `font: inherit`.
   - **Eine Quelle der Wahrheit bewahren:** `src-tauri/video/src/theme.rs` liest `src/styles.css` per `include_str!`
     zur Bauzeit (`cargo test -p ed-video` muss weiterhin alle 13 Schemata und Kontraste prüfen). Farbwerte nicht
     duplizieren.

5. **Öffentliche API von `<ed-editor>` (Angular) und `<editlicious-editor>` (Custom Element):**
   - **Eine gemeinsame Fassaden-Komponente** (`EditliciousEditor`):
     - In Angular direkt als Standalone-Komponente importierbar (Signal-Inputs/-Outputs; `ControlValueAccessor` im
       Design prüfen, nur mit Schutz gegen Volltext-Serialisierung riesiger Dokumente).
     - Über einen separaten Einstiegspunkt (`src/element.ts`) mit `@angular/elements` (`createCustomElement`,
       `ViewEncapsulation.ShadowDom`, zoneless `createApplication`) als `<editlicious-editor>` registriert.
   - **Inputs / HTML-Attribute** (mit `booleanAttribute` / `numberAttribute`, damit sowohl HTML-Attribute wie
     `<editlicious-editor word-wrap readonly tab-width="2">` als auch JS-Properties `el.wordWrap = true` synchron
     funktionieren; keine globalen HTML-Attribute wie `title`, `lang`, `style` überschatten):
     - `value` (Text setzen; für sehr große Texte zusätzlich `setValue(text)` / `loadText(text, options?)`)
     - `language` (Sprachname oder Dateiname/Endung, leer = Plain Text)
     - `scheme` (`'auto'` | eines der 13 Farbschemata)
     - `readOnly` / `readonly` (Boolean, Standard `false`)
     - `wordWrap` / `word-wrap` (Boolean, Standard `false`)
     - `lineNumbers` / `line-numbers` (`'absolute' | 'relative' | 'off'`, Standard `'absolute'`)
     - `statusBar` / `status-bar` (Boolean, Standard `false` – wenig Ablenkung, bei Bedarf zuschaltbar)
     - `tabWidth` / `tab-width`, `insertSpaces` / `insert-spaces` (`'auto' | true | false`), `fontSize` / `font-size`,
       `fontFamily` / `font-family`
     - `indentGuides` / `indent-guides`, `colorSwatches` / `color-swatches`, `overview`, `mathHints` / `math-hints`,
       `wordSuggest` / `word-suggest`, `uiLanguage` / `ui-language` (`'auto' | 'en' | 'de'`, lokal ohne Mutation von
       `<html lang>`)
   - **Events / Outputs & Methoden:**
     - **Kein Volltext-Kopieren pro Tastendruck:** `textChange` / `ed-change` feuert mit leichtgewichtigen Metadaten
       (`{ revision, dirty, lineCount, charCount }`); der Volltext wird nur gezielt über `getValue(): Promise<string>`
       (bzw. `getSlice(fromLine, toLine)`) über die WASM-Grenze geholt, damit Tippen in 100-MB-Dokumenten in O(log n)
       bleibt.
     - Weitere Events: `caretChange` / `ed-caret` (`{ line, ch, selChars, selLines, carets }`), `ready` / `ed-ready`.
     - Methoden am Element / an der Komponente: `getValue()`, `setValue(text)`, `focus()`, `runCommand(cmd)`,
       `runAction(actionId)`, `find(pattern, options?)`, `goTo(line, ch?)` sowie optional `playScript(edscript, tempo?)`
       (nutzt die vorhandene Skript-Timeline aus 015 / `ed-wasm`).

6. **Build, Bundle, Demo & Lizenzen:**
   - Eigenes Build-Target in `angular.json` (z. B. `element`, Einstieg `src/element.ts`, `outputHashing: "none"` oder
     sauberes ES-Modul-Paket unter `dist/element/`) + Skript `pnpm build:element`, ohne den Tauri-Build (`pnpm build`)
     zu vergrößern. Bundle-Budgets für das Element sinnvoll festlegen und tatsächliche Größen (JS + WASM, roh und
     gzip) im Design-/Abschlussbericht dokumentieren.
   - Neue Abhängigkeit `@angular/elements` (passend zu Angular 22, MIT): in `docs/architecture.md` begründen und
     **`pnpm notices` + `pnpm licenses:report`** ausführen, damit `pnpm check` grün bleibt.
   - Eigenständige Beispiel-/Testseite (z. B. `site/embed/index.html`, reines HTML ohne Framework), die das gebaute
     Custom Element einbindet und **zwei Editoren nebeneinander** zeigt (unterschiedliche Farbschemata und Sprachen,
     Schreibschutz-Umschalter, 1-Mio.-Zeilen-Generator, Event-Log).

## Regeln und Randfälle

- **Große Dateien bleiben der Maßstab:** Kein `rope.to_string()` bei jedem `input`-, Scroll- oder Caret-Ereignis; eine
  1-Mio.-Zeilen-Datei und eine 5-Mio.-Zeichen-Zeile müssen sich in `<editlicious-editor>` genauso verzögerungsfrei
  bearbeiten und durchsuchen lassen wie in der Live-Demo.
- **Null Nebenwirkungen auf die Host-Seite:** Weder `<html>`/`<body>`-Attribute oder globale CSS-Regeln noch globale
  `document`-Event-Listener (`contextmenu`, `keydown`) dürfen außerhalb von `<editlicious-editor>` wirken.
- **Keine Regression in Tauri & Video-Export:** Die Desktop-App (`pnpm build`, `cargo test --workspace` inkl.
  `ed-video`) verhält sich unverändert; gemeinsame UI-Komponenten werden nicht dupliziert.
- **Upgrade-Sicherheit des Custom Elements:** Attribute und Properties funktionieren unabhängig davon, ob sie vor oder
  nach `customElements.define('editlicious-editor', …)` gesetzt werden.

## Tests

- **Rust (`ed-core`, `ed-wasm`, `ed-video`):**
  - `readOnly` in `ed-core`: Schreibbefehle (`type`, `insert`, `backspace`, `replace`, `sortLines`, `applyCompletion` …)
    werden abgelehnt, Lese-/Ansichtsbefehle (`move`, `select`, `click`, `find`, `copy`, `copyHtml`, `toggleMarker` …)
    funktionieren.
  - `ed-wasm`: native Tests für `info`, `set_text`, `count_matches`, `count_marker`, `overview`, `clipboard_html`,
    Spracherkennung und `readOnly`.
  - `ed-video`: `cargo test -p ed-video` grün (alle 13 Farbschemata aus Stylesheet weiterhin lesbar).
- **Angular / Custom Element (Vitest):**
  - `<ed-editor>` mit `WasmEditorApi` (bzw. Fake für schnelle Unit-Tests + echter Integrationslauf gegen die Engine):
    Inputs (`value`, `language`, `scheme`, `readOnly`, `wordWrap`, `lineNumbers`, `statusBar`), Methoden (`getValue`,
    `setValue`, `goTo`, `runAction`), Events (`textChange` ohne Volltext-Kopie, `caretChange`).
  - **Mehrinstanz-Test:** Zwei `<ed-editor>`-Komponenten gleichzeitig im selben Dokument – Strg+F / Tippen /
    Farbschema-Wechsel in Editor A lässt Editor B völlig unberührt; `document.documentElement` bleibt unverändert;
    ARIA-IDs kollidieren nicht.
  - Teardown-Test: Zerstören der Komponente schließt die Sitzung in der `EditorApi`.
- **Ende-zu-Ende:**
  - `pnpm build:element` erzeugt das Custom-Element-Bundle; die statische Testseite mit zwei `<editlicious-editor>`-Tags
    lädt im headless Browser (bzw. Prüfskript analog `scripts/site.test.mjs`), erlaubt Tippen, Suchen, Kopieren,
    Schreibschutz und das Laden von 1 Mio. Zeilen ohne Konsolenfehler.
  - Desktop-App (`pnpm check` + Debug-Build) bleibt grün.

## Akzeptanzkriterien

- Auf einer gewöhnlichen, frameworkfreien HTML-Seite reicht ein `<script type="module">` plus
  `<editlicious-editor language="Rust" scheme="one"></editlicious-editor>`, um einen voll funktionsfähigen
  `editlicious`-Editor mit Shadow-DOM-Kapselung einzubetten.
- In einer Angular-App kann `<ed-editor [value]="…" [language]="'TypeScript'" [readOnly]="true" />` direkt als
  Standalone-Komponente genutzt werden.
- Zwei `<editlicious-editor>`-Instanzen mit verschiedenem Farbschema (`paper` und `night`) auf derselben Seite teilen
  sich einmalig das geladene WASM-Modul, beeinflussen weder einander noch das Styling/Tastaturverhalten der Host-Seite
  und bewältigen jeweils 1 Mio. Zeilen bzw. eine 5-Mio.-Zeichen-Zeile flüssig.
- `pnpm check` läuft vollständig grün durch (inkl. Lizenz- und Notice-Prüfung für `@angular/elements`).

