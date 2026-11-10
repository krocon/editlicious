# editlicious-editor

A calm, keyboard-driven code editor as a **web component** – built for **very large texts**: a million lines or a
single line with millions of characters stay as quick as a short file. The engine is the same Rust core that drives
the [editlicious](https://krocon.de/products/editlicious/) desktop app, compiled to WebAssembly and running in a web
worker, so even sorting or replacing in 100 MB never blocks your page.

```html
<script type="module" src="https://cdn.jsdelivr.net/npm/editlicious-editor/editlicious-editor.js"></script>

<editlicious-editor language="Rust" scheme="one" style="height: 400px"></editlicious-editor>
```

That is all: no framework, no build step, no global CSS. Try it live:
[krocon.de/products/editlicious/demo](https://krocon.de/products/editlicious/demo/).

## Why editlicious-editor

- **Large texts are the normal case.** Only the visible lines ever reach the page; typing in a 100 MB document stays
  O(log n). The full text crosses into JavaScript only when you ask for it (`getValue()`).
- **Isolated.** Every editor lives in its own shadow root with its own colours, keyboard, find state and settings.
  It never touches `<html>`, `<body>`, global styles or global key handlers – two editors with different schemes sit
  side by side and share one WebAssembly module.
- **Complete editing.** Syntax highlighting for many languages, multiple carets and block selection, find & replace
  (regex, case, whole word), go to line, sort lines, change case, duplicate/move/join lines, word markers, colour
  swatches for `#rrggbb`/`rgb()`/`hsl()`, inline calculator hints (`255/8=` → `31.875`), word completion
  (<kbd>Ctrl</kbd>+<kbd>Space</kbd>), undo/redo, copy as HTML.
- **13 colour schemes** – `paper`, `glass`, `sand`, `harvest`, `chalk` (light), `one`, `moss`, `mocha`, `moon`,
  `night`, `graphite`, `ink`, `amber` (dark) – or `auto` following `prefers-color-scheme`. Or bring your own:
  every colour is a documented CSS variable, `scheme="none"` leaves only yours.
- **Read-only mode** for log and JSON viewers: navigate, select, search, copy and mark – nothing changes.
- **Typing scripts** play scripted edits in real time – for tutorials and product videos.

## Install

```bash
npm install editlicious-editor
```

```js
import 'editlicious-editor'; // defines <editlicious-editor>
```

The package is an ES module next to its engine files. Your bundler or server must serve these files **from the same
folder** (they are found relative to `editlicious-editor.js`):

| File | What |
|---|---|
| `editlicious-editor.js` | the custom element (Angular inside, bundled) |
| `worker-*.js` | the engine's web worker |
| `ed_wasm.js`, `ed_wasm_bg.wasm` | the Rust engine as WebAssembly |

Serve `.wasm` as `application/wasm`. If your setup moves the files apart, tell the editor where they are before
the first editor appears:

```js
import { initEditlicious } from 'editlicious-editor';
await initEditlicious({ wasmUrl: '/assets/ed_wasm_bg.wasm', glueUrl: '/assets/ed_wasm.js' });
```

With a Content Security Policy, allow `script-src 'wasm-unsafe-eval'` and `worker-src 'self'`; without workers, use
`initEditlicious({ worker: false })` (the engine then runs on the main thread).

## Attributes and properties

Every attribute is also a property (`el.wordWrap = true`). Properties may be set before the element is defined.

| Attribute | Property | Values | Default |
|---|---|---|---|
| `value` | `value` | the text | empty |
| `language` | `language` | grammar (`Rust`, `Python`, `SQL` …), alias (`TypeScript`, `Kotlin`) or file name (`app.ts`, `.kt`) | plain text |
| `scheme` | `scheme` | `auto`, one of the 13 schemes, or `none` (only your own colours – see [Styling](#styling)) | `auto` |
| `color-scheme` | `colorScheme` | `light`, `dark` – system colours and form controls with `scheme="none"` | the system's |
| `readonly` | `readOnly` | boolean | `false` |
| `word-wrap` | `wordWrap` | boolean | `false` |
| `line-numbers` | `lineNumbers` | `absolute`, `relative`, `off` | `absolute` |
| `status-bar` | `statusBar` | boolean | `false` |
| `tab-width` | `tabWidth` | 1–16 | `4` |
| `insert-spaces` | `insertSpaces` | `auto` (detect), `true`, `false` | `auto` |
| `font-size` | `fontSize` | px | `14` |
| `font-family` | `fontFamily` | CSS font list | monospace stack |
| `indent-guides` | `indentGuides` | boolean | `true` |
| `color-swatches` | `colorSwatches` | boolean | `true` |
| `overview` | `overview` | boolean (marks of matches beside the scroll bar) | `true` |
| `math-hints` | `mathHints` | boolean | `true` |
| `word-suggest` | `wordSuggest` | boolean (suggest while typing: words, and your [provider](#suggestions-from-your-page)) | `false` |
| `completion-sources` | `completionSources` | `both`, `provider`, `words` – with a provider: its entries, the words of the text, or both | `both` |
| – | `completionProvider` | a function `(ctx) => items` – see [Suggestions from your page](#suggestions-from-your-page) | `null` |
| – | `snippets` | `[{ prefix, body, description? }]` – see [Snippets](#snippets) | `null` |
| `ui-language` | `uiLanguage` | `auto`, `en`, `de` | `auto` |

Boolean attributes follow HTML: present = `true` (`<editlicious-editor readonly word-wrap>`).

`value` only replaces the text when it differs from the value set last – for big texts prefer `setValue(text)`.

## Methods

```js
const ed = document.querySelector('editlicious-editor');

await ed.setValue(text);                  // replace the text (clean, no undo back)
await ed.loadText(text, { language: 'app.ts' });
const all = await ed.getValue();          // the whole text – only when you ask
const part = await ed.getSlice(10, 20);   // lines 10..20 (1-based, inclusive)
const sel = await ed.getSelection();      // { text, start: { line, ch }, end: { line, ch } }
const all = await ed.getSelections();     // every cursor / block selection, in document order
const where = await ed.getRange({ line: 3, ch: 1 }, { line: 5, ch: 12 });
await ed.replaceRange(sel.start, sel.end, format(sel.text)); // one undo step
await ed.replaceSelection('NOW()');
await ed.insertSnippet('SELECT ${1:*} FROM ${2:table};$0'); // Tab goes from placeholder to placeholder
await ed.goTo(120, 5);                    // line 120, column 5
const { total, current } = await ed.find('TODO', { wholeWord: true });
await ed.runAction('edit.sortAsc');       // any action of the editor
await ed.runCommand({ type: 'selectAll' });
ed.focus();
await ed.playScript('type "hello\\n"\npause 500\ntype "world"', 1.5);
ed.stopScript();
```

### Positions

A position is `{ line, ch }`, both **1-based**, as in `goTo()` and `ed-caret`. `ch` counts characters as **Unicode
code points** within the line: an emoji is one character (in JavaScript `'👍'.length` is 2), a combining mark counts
on its own, and a line break – also `\r\n` – is not part of any line. Texts come back with the line breaks the
document has. To turn `ch` into a UTF-16 index of a line's text: `[...lineText].slice(0, ch - 1).join('').length`.

`getSelection()` and `getRange()` cost as much as the range is long, not as the document is: a small selection in
a text of five million lines is instant; selecting everything hands you everything (at most 64 MB).

## Suggestions from your page

The editor knows nothing about your language's names – your page does. Give it a provider and its entries appear in
the editor's own suggestion list, sorted, filtered while typing and inserted on <kbd>Enter</kbd> or <kbd>Tab</kbd>:

```js
ed.completionProvider = async (ctx) => {
  // ctx: position, lineText (the line up to the caret), word, wordStart, trigger, triggerCharacter, signal
  const afterFrom = /\bFROM\s+\w*$/i.test(ctx.lineText);
  const rows = await fetchNames(afterFrom ? 'tables' : 'columns', { signal: ctx.signal });
  return rows.map((r) => ({
    label: r.name,
    kind: r.kind,                  // your own class: 'table', 'column', …
    detail: r.type,                // at the right
    documentation: r.comment,      // below the list while the entry is active
    insertText: r.quoted,          // default: the label
  }));
};
ed.completionProvider.triggerCharacters = ['.'];
```

- **When it is asked:** <kbd>Ctrl</kbd>+<kbd>Space</kbd>, a typed trigger character, and – with `word-suggest` –
  while typing, from two characters of a word.
- **It never blocks typing.** The provider may take as long as it likes; when the text or the caret changes
  meanwhile, `ctx.signal` aborts and a late answer is dropped – no flicker, no stale list.
- **An array is complete:** typing on filters it in the editor without asking again (50 000 entries stay fluid –
  only ten rows are on screen). Return `{ items, incomplete: true }` to be asked on every key.
- **More text only on request:** `await ctx.getRange(start, end)` or `ctx.getSlice(fromLine, toLine)`.
- **`insertText` with placeholders:** set `snippet: true` (`'COUNT(${1:*})$0'`), see [Snippets](#snippets).
- **With the editor's own word completion:** `completion-sources="both"` (default) lists your entries first, then
  the words of the text; `provider` only yours; `words` only the editor's.
- **Icons and colours of kinds:** the first letter of `kind` is the icon. Style it from your page:

  ```css
  editlicious-editor::part(icon-table) { color: white; background: #b58900; }
  editlicious-editor::part(item-column) { font-style: italic; }
  ```

Without a provider the editor behaves as before.

## Snippets

`insertSnippet(template)` inserts a text with placeholders as one undo step. <kbd>Tab</kbd> and
<kbd>Shift</kbd>+<kbd>Tab</kbd> go from placeholder to placeholder, <kbd>Esc</kbd> or leaving the placeholder ends
it.

| In the template | Meaning |
|---|---|
| `$1`, `${1:default}` | placeholder 1, with a default text |
| the same number twice | typed together at every place |
| `${1\|one,two\|}` | a choice (the first option is inserted) |
| `$0` | where the caret ends (default: behind the snippet) |
| `$NAME`, `${NAME:default}` | a variable: yours (`{ vars: { NAME: '…' } }`), or `SELECTION`, `DATE`, `TIME`, `CURRENT_YEAR`, `CURRENT_MONTH`, `CURRENT_DATE`, `CURRENT_HOUR`, `CURRENT_MINUTE`, `CURRENT_SECOND` |
| `\$`, `\}`, `\\` | the characters themselves |

```js
await ed.insertSnippet('SELECT ${1:*} FROM $TABLE WHERE ${2:1=1};$0', { vars: { TABLE: 'orders' } });
await ed.insertSnippet('/* $SELECTION */', { range: { start, end } });

// by prefix: in the suggestion list, and Tab right behind the prefix expands it
ed.snippets = [{ prefix: 'sel', body: 'SELECT ${1:*} FROM ${2:table};$0', description: 'select' }];
```

New lines of a template take the indentation of the line it is inserted in; a tab is the editor's indentation.

## Events

Events bubble and cross the shadow boundary (`composed`).

| Event | `detail` |
|---|---|
| `ed-ready` | – (the engine session is up) |
| `ed-change` | `{ revision, dirty, lineCount, charCount }` – never the text itself |
| `ed-caret` | `{ line, ch, selChars, selLines, carets, selStart, selEnd }` (1-based; `selStart`/`selEnd`: the primary selection in document order, both the caret when nothing is selected) |

```js
ed.addEventListener('ed-change', async (e) => {
  if (needsSaving(e.detail.revision)) save(await ed.getValue());
});
```

## Styling

The element is `display: block` with a default height of `24rem`; give it the height you want.

**Every colour of the editor is a CSS custom property** – set it on the element (any selector that matches it) and
it wins over the chosen `scheme`; what you do not set comes from the scheme. Changes apply at once, also at run
time.

```css
editlicious-editor {
  height: 60vh;
  border: 1px solid #ccc;
  --ed-bg: #fffdf7;
  --syn-keyword: #a626a4;
}
```

### Your own colour schemes

A page with its own themes maps its variables once; the editor then follows every theme switch of the page (the
mapping is resolved on the element, so changing `--app-bg` on `<html>` repaints the editor immediately):

```html
<editlicious-editor scheme="none" color-scheme="dark"></editlicious-editor>
```

```css
editlicious-editor {
  --ed-bg: var(--app-bg);
  --ed-text: var(--app-text);
  --color-accent: var(--app-accent);
  --syn-keyword: var(--app-keyword);
  --syn-string: var(--app-string);
}
```

`scheme="none"` switches every built-in scheme off: colours you do not set are derived from `--ed-bg` and
`--ed-text` (the system colours `Canvas`/`CanvasText` when those are unset) – a coherent, monochrome editor, no
colour of a built-in scheme. `color-scheme="light"` or `"dark"` tells the browser which system colours and form
controls to use (default: both, following the system); it only acts together with `scheme="none"`.

Variables must be set **on the element** (directly, by a rule, or mapped as above): the editor defines its tokens on
its own host, so equally named variables on `<html>` alone do not reach it.

### Colour variables

| Variable | Colours | Example |
|---|---|---|
| `--ed-bg` | background of the text area | `#fffdf7` |
| `--ed-text` | plain text | `#2a2d33` |
| `--ed-gutter-bg` | background of the line number gutter | `#f4f2ec` |
| `--ed-gutter-text` | line numbers | `#b3b0a6` |
| `--ed-gutter-text-active` | line number of the caret line | `#5d6068` |
| `--ed-current-line` | background of the caret line | `rgb(0 0 0 / 4%)` |
| `--ed-selection` | selection while the editor has focus | `rgb(47 111 223 / 22%)` |
| `--ed-selection-inactive` | selection without focus | `rgb(0 0 0 / 9%)` |
| `--ed-caret` | the caret | `#2f6fdf` |
| `--ed-whitespace` | tab arrows | `rgb(0 0 0 / 18%)` |
| `--ed-indent-guide` | indentation guides | `rgb(0 0 0 / 7%)` |
| `--ed-search-match` | search matches | `rgb(255 196 0 / 38%)` |
| `--ed-search-current` | the selected search match | `rgb(255 140 0 / 55%)` |
| `--ed-scrollbar` | scroll bar thumbs | `rgb(0 0 0 / 22%)` |
| `--ed-scrollbar-hover` | scroll bar thumb under the mouse | `rgb(0 0 0 / 38%)` |
| `--ed-overview-match` | marks of search matches beside the scroll bar (default: `--ed-search-current`) | `orange` |
| `--ed-overview-marker` | marks of word markers beside the scroll bar (default: `--ed-marker-0`) | `hotpink` |
| `--ed-control` | control characters (default: `--color-warning`) | `#8a5a00` |
| `--ed-swatch-border` | outline of the colour swatches in the gutter (default: `--ed-gutter-text`) | `#999` |
| `--ed-checker-light` | light squares behind transparent colour swatches | `#ffffff` |
| `--ed-checker-dark` | dark squares behind transparent colour swatches | `#c8c8c8` |
| `--ed-marker-0` … `--ed-marker-9` | the ten word markers (translucent) | `rgb(255 99 132 / 30%)` |
| `--syn-comment` | comments (italic) | `#8a8f98` |
| `--syn-string` | strings | `#2f7d32` |
| `--syn-keyword` | keywords | `#8e44ad` |
| `--syn-number` | numbers | `#b5532b` |
| `--syn-constant` | constants (`true`, `null`, escapes) | `#b5532b` |
| `--syn-type` | types and classes | `#1f6fb2` |
| `--syn-function` | function names | `#0b6e8c` |
| `--syn-tag` | tags (HTML, XML) | `#1f6fb2` |
| `--syn-attribute` | attributes, properties | `#8a6d00` |
| `--syn-variable` | identifiers (default: the text colour) | `#2a2d33` |
| `--syn-operator` | operators | `#6b6f78` |
| `--syn-punct` | punctuation, brackets | `#6b6f78` |
| `--syn-heading` | headings (Markdown) | `#1f6fb2` |
| `--syn-code` | inline code (Markdown) | `#b5532b` |
| `--syn-link` | links (underlined) | `#2f6fdf` |
| `--syn-invalid` | invalid code (wavy underline) | `#b42318` |
| `--syn-inserted` | inserted lines (diff) | `#2f7d32` |
| `--syn-deleted` | deleted lines (diff) | `#b42318` |
| `--syn-changed` | changed lines (diff) | `#8a6d00` |
| `--syn-log-error` | log level error | `#c62828` |
| `--syn-log-warning` | log level warning | `#b26a00` |
| `--syn-log-info` | log level info | `#1f6fb2` |
| `--syn-log-debug` | log level debug | `#8a8f98` |
| `--color-bg` | background behind dialogs' fields, status bar | `#f7f6f2` |
| `--color-surface` | find bar, go-to-line, suggestion list, hints, dialogs | `#ffffff` |
| `--color-surface-2` | buttons, key caps | `#eeece6` |
| `--color-text` | text of bars, lists and dialogs | `#22252b` |
| `--color-text-muted` | secondary text (counts, kinds, status bar) | `#6b6f78` |
| `--color-border` | borders of bars, lists, fields | `#d9d6cc` |
| `--color-accent` | the active suggestion, primary buttons, switched-on options | `#2f6fdf` |
| `--color-accent-text` | text on the accent colour | `#ffffff` |
| `--color-focus` | focus ring | `#2f6fdf` |
| `--color-error` | invalid search pattern, errors | `#b42318` |
| `--color-warning` | warnings | `#8a5a00` |
| `--color-backdrop` | veil behind a dialog | `rgb(20 18 12 / 40%)` |
| `--color-hover` | hovered buttons and list entries | `#e9e6dd` |
| `--shadow-popup` | shadow of lists, hints and dialogs (a `box-shadow` value) | `0 12px 32px rgb(0 0 0 / 18%)` |

The font is set with the attributes `font-family` and `font-size` (or, while `font-family` is not set, with
`--font-editor` on the element).

## Keyboard

The editor's shortcuts only act while focus is inside it; keys it does not use reach your page unchanged.
<kbd>Ctrl/⌘</kbd>+<kbd>F</kbd> find, <kbd>F3</kbd> next match, <kbd>Ctrl</kbd>+<kbd>G</kbd> (macOS <kbd>⌘</kbd>+<kbd>L</kbd>) go to line,
<kbd>Ctrl/⌘</kbd>+<kbd>Z</kbd>/<kbd>Shift</kbd>+<kbd>Z</kbd> undo/redo, <kbd>Ctrl</kbd>+<kbd>Space</kbd>
complete a word, <kbd>Alt</kbd>+<kbd>Z</kbd> word wrap, <kbd>Ctrl/⌘</kbd>+<kbd>+</kbd>/<kbd>-</kbd> zoom. Full list:
[shortcuts](https://krocon.de/products/editlicious/#keys).

## Typing scripts

`playScript(script, tempo?)` types a script into the editor in real time (`tempo` 2 = twice as fast):

```text
tempo human cpm=320 seed=42
type "fn main() {\n"
pause 600
type "    println!(\"hi\");\n}"
goto 1:4
select 1:4 1:8
type "start"
```

Commands: `type`, `pause`, `goto`, `select`, `move`, `find`, `newline`, `backspace`, `undo`, `copy`, `paste`, `do
{json}` and more – see [docs/scripts.md](https://github.com/krocon/editlicious/blob/main/docs/scripts.md)
(`type @file` is not available in the browser). Copy and paste inside a script use the editor's own clipboard,
never the user's.

## Angular

Inside Angular you can use the custom element with `CUSTOM_ELEMENTS_SCHEMA`:

```ts
import 'editlicious-editor';

@Component({
  schemas: [CUSTOM_ELEMENTS_SCHEMA],
  template: `<editlicious-editor [value]="code" language="TypeScript" (ed-change)="changed($event)" />`,
})
export class CodeView { /* … */ }
```

## Browser support

Current Chrome, Edge, Firefox and Safari (ES2022 modules, WebAssembly, module workers). Copying writes `text/plain`
and – for *copy as HTML* – `text/html` via the asynchronous clipboard API.

## Size

SIZE_TABLE

The WebAssembly engine is loaded once per page, however many editors there are.

## License

MIT © Marc Kronberg. The bundle contains Angular (MIT) and Rust crates under MIT, Apache-2.0 and similar licences –
see `THIRD-PARTY-NOTICES.txt`. Source: [github.com/krocon/editlicious](https://github.com/krocon/editlicious).
