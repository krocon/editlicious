# Design 025: Einbettbare Editor-Komponente – `<ed-editor>` und `<editlicious-editor>`

> Prompt: [025-web-component.md](025-web-component.md) (von Marc als „024“ angelegt; umbenannt, weil 024 schon
> „Pfad öffnen“ ist). Stand 31.10.2026. Fortschritt über mehrere Sitzungen: [025-web-component.progress.md](025-web-component.progress.md).

## Entscheidungen

### Engine (Rust)

- **E1 Schreibschutz in der Sitzung**: eigener Befehl `setReadOnly { readOnly }` (Ansichtsbefehl, wird nicht
  aufgezeichnet) setzt `Session.read_only` – nicht als Feld von `configure`, weil `Configure` an 17 Stellen voll
  ausgeschrieben gebaut wird.
  `Command::is_edit()` (neben `is_view_only()`) zählt die textverändernden Befehle auf: `type`, `insert`,
  `newline`, `backspace`, `delete`, `deleteWord*`, `indent`, `outdent`, `undo`, `redo`, `cut`, `paste`, `replace`,
  `replaceAll`, `sortLines`, `changeCase`, `duplicateLines`, `deleteLines`, `moveLines`, `joinLines`,
  `trimTrailingWhitespace`, `convertIndentation`, `setLineEnding`, `convertColor`, `applyCompletion`, `evaluate`.
  Dazu `complete` (setzt evtl. einen Einzelkandidaten ein, 023 E7) – die Rechen-*Hinweise* am Caret bleiben. Abgelehnt wird mit dem neuen Fehlercode **`read_only`**
  (`error.rs`, `fs-error.ts`, `describeFsError`) – vor jeder Änderung, also ohne Teilwirkung. Erlaubt bleiben
  Navigation, Auswahl, Klicks, Mehrfach-Cursor, `copy`, `copyHtml`, `copyResult`, `find`, `goTo`, Marker,
  `setLanguage`, `toggleOverwrite`. `Frame.readOnly` meldet den Zustand (UI: kein Caret-Blinken nötig, Status).
  Allgemein in `ed-core`, nicht nur in `ed-wasm`: auch die App könnte ihn später nutzen (z. B. Log-Modus).
- **E2 `ed-wasm` vervollständigt** – `Editor` bekommt, was `EditorApi` braucht und `ed-core` schon kann:
  `info()` (DocInfo-JSON), `set_text(text)` (neues `Document::from_text`, Sitzung über `replace_document`, Cursor
  an den Anfang; Undo-Verlauf neu), `count_matches(query_json)` (auf dem Such-Schnappschuss wie
  `commands::count_matches`), `count_marker()`, `overview(buckets)`, `clipboard_html()` (die Zwischenablage merkt
  sich HTML **und** Text: neuer `WebClipboard { text, html }` in `ed-wasm` statt `MemoryClipboard`), dazu
  freie Funktion `encodings()`. Die Logik von Zählen/Übersicht liegt bisher in der Tauri-Command-Schicht; sie
  wandert als reine Funktionen nach `ed-core` (`search::count_on`, `editor::overview`), damit App und WASM dieselbe
  nutzen (keine Duplikate).
- **E3 Sprache ohne Pfad**: `Editor::set_language_spec(spec)` und Konstruktor-Argument verstehen
  (1) Grammatiknamen ohne Groß/klein (`"rust"` → `Rust`), (2) Aliasse `TypeScript`/`ts` → virtuelle Datei
  `editor.ts`, `Kotlin` → `editor.kt`, (3) Datei- oder Endungsnamen (`app.ts`, `.kt`) → `highlight::detect` +
  Lexikon über `complete::key_for` (023 E6). Dafür bekommt die Sitzung einen **virtuellen Dateinamen**
  (`Session::set_file_name`, setzt `doc.path`; in WASM wird nie gespeichert). Leer/unbekannt = Klartext.

### Frontend

- **E4 Worker statt Main-Thread** (bewertet): Ein **dedizierter Module-Worker** (`engine.worker.ts`) hält das
  eine WASM-Modul und alle Sitzungen der Seite; `WasmEditorApi` spricht ihn über `postMessage` mit Anfrage-Ids an.
  Gründe: `EditorApi` ist ohnehin asynchron; `countMatches`, `overview`, `replaceAll`, `sortLines` oder
  `set_text` auf 100 MB blockieren nie das Tippen in der Host-Seite; Frames sind klein (JSON-Strings, sichtbare
  Zeilen) – das Kopieren über `postMessage` kostet Mikrosekunden. Nachteil Zwischenablage (E6) ist lösbar, weil
  ein `ClipboardItem` ein Promise annehmen darf. Die Befehlsverteilung (`engine-host.ts`) ist eine reine Funktion
  über einem Modul-Objekt; der Worker und ein **Main-Thread-Fallback** (`initEditlicious({ worker: false })`,
  CSP ohne `worker-src`, Tests) nutzen sie gleichermaßen. Der Worker wird von Angulars Builder als eigene Datei
  (`worker-*.js`) neben dem Element gebaut (`new Worker(new URL('./engine.worker', import.meta.url))`) – kein
  Inline-Blob, damit CSP-freundlich.
- **E5 Geteilte, verzögerte Initialisierung**: `initEditlicious({ wasmUrl?, worker? })` gibt ein einmaliges
  Promise zurück; jede Komponente ruft es implizit beim ersten Gebrauch. Standard-`wasmUrl` relativ zum Bundle
  (`new URL('./ed_wasm_bg.wasm', import.meta.url)`). Zweiter Aufruf mit anderen Optionen nach dem Start: ignoriert
  (Warnung). Glue-Code: `site/demo/pkg/ed_wasm.js` (vom Workflow „Site demo“ gebaut und eingecheckt, 011 E5) –
  dieselbe Datei für Demo und Element, keine zweite WASM-Kopie im Repo.
- **E6 Browser-Zwischenablage** (`ClipboardBridge`-Token, optional in `EditorView`; Tauri: nicht gesetzt, `arboard`
  unverändert): nach `copy`/`cut`/`copyHtml` schreibt die Brücke im selben Tastendruck
  `navigator.clipboard.write([new ClipboardItem({ 'text/plain': Promise, 'text/html'?: Promise })])` – das
  Promise löst nach dem Engine-Befehl auf (Safari verlangt den synchronen Aufruf in der Geste, Chrome/Firefox
  nehmen Promises an); Fallback `writeText` nach dem Befehl. Einfügen per Tastatur (`edit.paste`):
  `navigator.clipboard.readText()` → `insert`; das Kontextmenü-Einfügen nutzt weiter `ClipboardEvent` (`onPaste`).
  Kopiert wird immer nur, was die Engine liefert (Auswahl begrenzt wie in der App, 002 E11).
- **E7 Mehrinstanz** – die Einbettung stellt pro Komponente bereit (`providers`): `ActionService`,
  `KeyboardDispatcher`, `FindStore`, `DialogService`, `PickService` und die Ansichtseinstellungen. Die Desktop-App
  bekommt dieselben Klassen weiter aus dem Root-Injector – Verhalten unverändert.
  - **Ansichtseinstellungen**: neue abstrakte Klasse `ViewSettings` (`settings`, `fontFamily`, `update`, `zoom`,
    `cycleLineNumbers`); Root-Standard ist der `SettingsStore` (Factory). `EditorView`, `StatusBar`, `FindBar`
    injizieren nur noch `ViewSettings`. Die Einbettung liefert `EmbeddedSettings` (nur Signale, kein Backend,
    kein `document`, Sprache instanzlokal).
  - **`KeyboardDispatcher.attach(target, { contextMenu })`**: die App hängt ihn wie bisher an `document`
    (mit Kontextmenü-Sperre); die Einbettung nur ans Host-Element, **ohne** Kontextmenü-Sperre. Nicht behandelte
    Tasten laufen ungestört weiter zur Seite (kein `stopPropagation` ohne Aktion).
  - **Optionale Desktop-Teile** über Tokens mit Root-Factory: `EDITOR_PLAYBACK` (Standard `RecordingStore`,
    Einbettung `null`), `EDITOR_EMBEDDED` (Standard `false`): ohne Datei kein „Encoding wechseln“ (Aktion
    `file.encoding` wird dort nicht registriert); `DocumentsStore` wird nur in der App per `Injector` geholt.
  - **ARIA-IDs**: jede `EditorView` bekommt einen Präfix `ed<N>-` (Zähler je Seite) für `math-hint`,
    `completion-list`, `completion-<i>` – eindeutig in Light DOM und Shadow Roots.
  - **Lebenszyklus**: `DestroyRef` der Fassade schließt die Sitzung (`api.close`), Worker-Seite gibt sie frei
    (`free()`). `@angular/elements` zerstört die Komponente erst nach `disconnectedCallback` + 10 ms; ein
    erneutes Einhängen davor behält sie (idempotent). Ziehen (Maus) hängt `mousemove`/`mouseup` nur **während**
    eines Ziehens an `document` und entfernt sie danach – nötig, damit Ziehen über den Rand hinaus funktioniert.
- **E8 Styles & Farbschemata**: `scripts/host-theme.mjs` erzeugt aus `src/styles.css` die Datei
  `src/app/embed/host-theme.css` (eingecheckt, `--check` in `pnpm scripts:test`): `:root` → `:host`,
  `:root[data-scheme='x']` → `:host([data-scheme='x'])`, `:root[data-theme='dark']` → `:host([data-theme='dark'])`,
  `:root:not([data-theme='light'])` → `:host(:not([data-theme='light']))`, `body` → `:host`; `html, body` und die
  `[data-preview]`-Selektoren fallen weg. Damit bleibt `styles.css` die einzige Quelle (`ed-video` liest sie
  weiter per `include_str!`), kein `:host-context()`. Dieselbe Datei funktioniert mit `ShadowDom` (Element) und
  `Emulated` (Angular-Komponente: `:host(...)` → `[_nghost-…]…`). Die Fassade setzt `data-scheme`/`data-theme`
  über `schemeAttributes()` (wie die App auf `<html>`), Schrift über `--editor-font-size`/`--font-editor` auf dem
  Host. Konsumenten überschreiben Tokens von außen (`editlicious-editor { --ed-bg: … }` gewinnt über `:host`).
  `:host { display: block; min-height: 12rem; height: 24rem; contain: content }`.
- **E9 Fassade `EditliciousEditor`** (`src/app/embed/editlicious-editor.ts`, Selektor `ed-editor`, Standalone,
  `OnPush`): Signal-Inputs mit `booleanAttribute`/`numberAttribute`; Aliasse in Kebab-Case für HTML-Attribute
  (`@angular/elements` leitet Attribute aus Input-Namen ab: `wordWrap` ↔ `word-wrap`). `readOnly` heißt im Element
  zusätzlich `readonly` (Alias-Input). **Kein `ControlValueAccessor`** (bewertet): Formulare würden bei jeder
  Änderung den Volltext lesen; wer das will, ruft `getValue()` beim Absenden. `value` setzt den Text nur, wenn er
  sich vom zuletzt gesetzten unterscheidet (kein Rücklauf).
  Outputs: `textChange` (`{ revision, dirty, lineCount, charCount }`), `caretChange`
  (`{ line, ch, selChars, selLines, carets }`), `ready`. Im Element heißen die Events `ed-change`, `ed-caret`,
  `ed-ready` (Output-Aliasse – `@angular/elements` nimmt den Alias als Eventnamen).
  Methoden: `getValue()`, `getSlice(from, to)`, `setValue(text)`, `loadText(text, { language? })`, `focus()`,
  `runCommand(cmd)`, `runAction(id)`, `find(pattern, options?)`, `goTo(line, ch?)`, `playScript(text, tempo?)`.
- **E10 Element-Einstieg** `src/element.ts`: `createApplication({ providers: [provideZonelessChangeDetection()…] })`,
  `createCustomElement(EditliciousEditorElement, …)`, `customElements.define('editlicious-editor', …)` nur wenn noch
  nicht definiert. `EditliciousEditorElement` erbt die Fassade mit `encapsulation: ShadowDom`. Upgrade-Sicherheit:
  `@angular/elements` übernimmt vor dem Upgrade gesetzte Properties (`initializeInputs`) und Attribute.
- **E11 Build**: Ziel `element` in `angular.json` (Einstieg `src/element.ts`, kein `index.html`, keine globalen
  Styles, `outputHashing: none`, Ausgabe `dist/element/browser`), Assets: `site/demo/pkg/ed_wasm_bg.wasm`.
  `pnpm build:element`. Budgets nur für dieses Ziel. Die Desktop-App importiert nichts aus `src/app/embed/` –
  `pnpm build` wächst nicht.
- **E12 Demo** `site/embed/index.html` (reines HTML): zwei Editoren (`paper` + Rust, `night` + TypeScript),
  Schreibschutz-Umschalter, 1-Mio.-Zeilen-Generator, Event-Log. Lädt `./editlicious-editor.js` (Kopie aus
  `dist/element`, vom Skript `pnpm build:element` dorthin gelegt; nicht eingecheckt).

## Verträge

```ts
initEditlicious(o?: { wasmUrl?: string | URL; worker?: boolean }): Promise<void>
interface EdChange { revision: number; dirty: boolean; lineCount: number; charCount: number }
interface EdCaret { line: number; ch: number; selChars: number; selLines: number; carets: number }
```
Worker-Protokoll: `{ id, op, args }` → `{ id, ok, value | error: FsError }`; `op` ∈ `open`, `close`, `command`,
`info`, `setText`, `text`, `slice`, `countMatches`, `countMarker`, `overview`, `languages`, `encodings`,
`clipboard`, `setClipboard`, `setLanguage`, `playScript`.

## Tests

- Rust: `read_only` (Schreibbefehle → `read_only`, Text unverändert; Lesebefehle ok; `complete` setzt nichts ein),
  `count_on`/`overview` nach dem Umzug (bestehende Command-Tests bleiben), `ed-wasm` nativ: `info`, `set_text`,
  `count_matches`, `count_marker`, `overview`, `clipboard_html`, Sprachauflösung, `readOnly`.
- Frontend: `engine-host` gegen ein Fake-Modul; `WasmEditorApi` mit Fake-Host; `EditliciousEditor` mit
  `FakeEditorApi` (Inputs, Methoden, Events ohne Volltext), Mehrinstanz (zwei Komponenten: Strg+F/Tippen/Schema
  in A lässt B und `<html>` unberührt, ARIA-IDs eindeutig), Teardown schließt die Sitzung, `host-theme`-Prüfung.
- Ende-zu-Ende: `pnpm build:element`, Playwright gegen `site/embed/` (Tippen, Suchen, Kopieren, Schreibschutz,
  1 Mio. Zeilen, keine Konsolenfehler).
