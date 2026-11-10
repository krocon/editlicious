/**
 * Types of `<editlicious-editor>` (prompt 025) – written by hand next to the bundle.
 * Importing the module defines the element.
 */

/** Options of {@link initEditlicious}. */
export interface EditliciousInit {
  /** Where `ed_wasm_bg.wasm` is; default: next to `editlicious-editor.js`. */
  wasmUrl?: string | URL;
  /** Where the binding's glue code `ed_wasm.js` is; default: next to `editlicious-editor.js`. */
  glueUrl?: string | URL;
  /** Run the engine in a web worker (default `true`); `false`: on the main thread. */
  worker?: boolean;
}

/** Detail of `ed-change`: what changed – never the text itself. */
export interface EdChange {
  revision: number;
  dirty: boolean;
  lineCount: number;
  charCount: number;
}

/**
 * A position in the text: line and column, both 1-based. The column counts characters as
 * **Unicode code points** within the line – an emoji is one, a combining mark is one of its own,
 * the line break (also `\r\n`) is none. It is not a UTF-16 index: the UTF-16 offset of `ch` in a
 * line's text is `[...lineText].slice(0, ch - 1).join('').length`.
 */
export interface EdPos {
  line: number;
  ch: number;
}

/** A selected range in document order with its text (line breaks as they are in the text). */
export interface EdSelection {
  text: string;
  start: EdPos;
  end: EdPos;
}

/** Detail of `ed-caret`: 1-based line and column of the primary caret. */
export interface EdCaret {
  line: number;
  ch: number;
  selChars: number;
  selLines: number;
  carets: number;
  /** The primary selection in document order; both equal the caret when nothing is selected. */
  selStart: EdPos;
  selEnd: EdPos;
}

/** One suggestion of a {@link CompletionProvider}. */
export interface CompletionItem {
  /** What the list shows (and what is inserted without `insertText`). */
  label: string;
  insertText?: string;
  /** `insertText` is a snippet with placeholders (`${1:name}`, `$0` – see `insertSnippet`). */
  snippet?: boolean;
  /**
   * A class of your own (`table`, `column` …). Its first letter is the entry's icon; style it
   * with `editlicious-editor::part(icon-table)` and the row with `::part(item-table)`.
   */
  kind?: string;
  /** Shown at the right (a type, a schema). */
  detail?: string;
  /** Shown below the list while the entry is active. */
  documentation?: string;
  /** Order in the list (default: `label`). */
  sortText?: string;
  /** What typing filters on (default: `label`). */
  filterText?: string;
  /** The text to replace (default: the word begun, up to the caret). */
  range?: { start: EdPos; end: EdPos };
}

/** What a {@link CompletionProvider} is asked with – a few characters, never the whole text. */
export interface CompletionContext {
  /** The caret. */
  position: EdPos;
  /** The line up to the caret (its last 2000 characters at most). */
  lineText: string;
  /** The word begun left of the caret (empty after a blank or a dot). */
  word: string;
  wordStart: EdPos;
  /** `invoke`: Ctrl+Space; `type`: while typing; `character`: a trigger character was typed. */
  trigger: 'invoke' | 'type' | 'character';
  triggerCharacter: string | null;
  /** Aborted as soon as the answer is no longer wanted (typed on, caret moved). */
  signal: AbortSignal;
  /** Text around the caret – only when you ask for it. */
  getRange(start: EdPos, end: EdPos): Promise<string>;
  /** Lines `from..to` (1-based, inclusive). */
  getSlice(from: number, to: number): Promise<string>;
}

/**
 * The answer: the entries, or `null` for none. An array is complete – typing on filters it
 * without asking again; `{ items, incomplete: true }` is asked again on every key.
 */
export type CompletionResult =
  | CompletionItem[]
  | { items: CompletionItem[]; incomplete?: boolean }
  | null
  | undefined;

export interface CompletionProviderFunction {
  (ctx: CompletionContext): CompletionResult | Promise<CompletionResult>;
  /** Characters that ask the provider when typed (`.`, a blank …). */
  triggerCharacters?: string | string[];
}

/** Suggestions the page provides: a function, or an object with the function. */
export type CompletionProvider =
  | CompletionProviderFunction
  | { provideCompletions: CompletionProviderFunction; triggerCharacters?: string | string[] };

/** A snippet offered by its prefix (in the suggestion list; Tab behind the prefix expands it). */
export interface Snippet {
  prefix: string;
  /** The template (see {@link EditliciousEditorElement.insertSnippet}); an array is its lines. */
  body: string | string[];
  description?: string;
}

/** Options of {@link EditliciousEditorElement.insertSnippet}. */
export interface SnippetOptions {
  /** The text to replace (default: the selection). */
  range?: { start: EdPos; end: EdPos };
  /** Values of the template's variables: `{ TABLE: 'orders' }` for `$TABLE`. */
  vars?: Record<string, string>;
}

/** A marker the page puts on the text (`setMarkers`). */
export interface Marker {
  start: EdPos;
  end: EdPos;
  /**
   * `error`, `warning`, `info`, `hint`: a wavy (hint: dotted) underline in `--ed-mark-error` …
   * Any other name is a class of your own: its text is tinted with `--ed-mark-<class>`.
   * Default `info`.
   */
  severity?: 'error' | 'warning' | 'info' | 'hint' | (string & {});
  /** Shown as a tooltip: under the mouse, and when the caret is in the marker. */
  message?: string;
  /** A symbol in the gutter of its first line: `true` = a dot, or one or two characters. */
  gutter?: boolean | string;
  /** Tint the whole lines it covers. */
  lineBackground?: boolean;
  /**
   * When text inside the marker changes: `track` (default) – it grows and shrinks with its text
   * and goes when nothing of it is left; `remove` – any change inside removes it. Text inserted
   * or deleted before a marker always just moves it.
   */
  onEdit?: 'track' | 'remove';
}

/** A marker where it is now (detail of `ed-marker-click`, `getMarkers()`). */
export interface MarkerInfo {
  owner: string;
  /** Its index in the list given to `setMarkers(owner, …)`. */
  index: number;
  start: EdPos;
  end: EdPos;
  severity: string;
  message: string | null;
}

/** Options of {@link EditliciousEditorElement.find}. */
export interface FindOptions {
  regex?: boolean;
  caseSensitive?: boolean;
  wholeWord?: boolean;
}

/** Answer of {@link EditliciousEditorElement.find}: number of matches, 1-based current one. */
export interface MatchCount {
  total: number;
  current: number;
}

/** An engine command, e.g. `{ type: 'selectAll' }` or `{ type: 'sortLines', … }`. */
export interface EditorCommand {
  type: string;
  [key: string]: unknown;
}

export type ColorScheme =
  | 'auto'
  | 'paper'
  | 'glass'
  | 'sand'
  | 'harvest'
  | 'chalk'
  | 'one'
  | 'moss'
  | 'mocha'
  | 'moon'
  | 'night'
  | 'graphite'
  | 'ink'
  | 'amber'
  /** No built-in scheme: only the CSS variables the page sets (README „Styling“). */
  | 'none';

export interface EditliciousEditorElement extends HTMLElement {
  /** Text to show; set again only when it differs from what was set last. */
  value: string | null;
  /** Grammar (`Rust`), alias (`TypeScript`, `Kotlin`) or file name (`app.ts`, `.kt`); empty = plain text. */
  language: string;
  scheme: ColorScheme;
  /** With `scheme: 'none'`: light or dark system colours and form controls; `null` = the system's. */
  colorScheme: 'light' | 'dark' | null;
  readOnly: boolean;
  wordWrap: boolean;
  lineNumbers: 'absolute' | 'relative' | 'off';
  statusBar: boolean;
  tabWidth: number;
  insertSpaces: 'auto' | boolean;
  fontSize: number;
  fontFamily: string;
  indentGuides: boolean;
  colorSwatches: boolean;
  overview: boolean;
  mathHints: boolean;
  wordSuggest: boolean;
  uiLanguage: 'auto' | 'en' | 'de';
  /**
   * Suggestions from the page (property only). Asked on Ctrl+Space, at its `triggerCharacters`
   * and – with `wordSuggest` – while typing (from two characters of a word). It never blocks
   * typing; an answer that comes after the text or the caret changed is dropped.
   */
  completionProvider: CompletionProvider | null;
  /** With a provider: `both` = its entries, then the words of the text; or only one of them. */
  completionSources: 'both' | 'provider' | 'words';
  /** Snippets by prefix (property only). */
  snippets: Snippet[] | null;

  /** The whole text – crosses into the page only when asked for. */
  getValue(): Promise<string>;
  /** Lines `from..to` (1-based, inclusive). */
  getSlice(from: number, to: number): Promise<string>;
  /**
   * The primary selection: its text and where it starts and ends. Nothing selected: empty text,
   * `start` = `end` = the caret.
   */
  getSelection(): Promise<EdSelection>;
  /** Every selection (several cursors, block selection) in document order. */
  getSelections(): Promise<EdSelection[]>;
  /**
   * The text between two positions (clamped into the text, either order). The cost grows with
   * the length of the range, not of the document: a small range of a huge text is instant, a
   * huge range is a huge string (more than 64 MB is refused with `selection_too_large`).
   */
  getRange(start: EdPos, end: EdPos): Promise<string>;
  /**
   * Replaces the text between two positions as **one** undo step; the same text changes nothing.
   * Cursors and selections stay where they are in the text.
   */
  replaceRange(start: EdPos, end: EdPos, text: string): Promise<void>;
  /** Replaces every selection by `text` (inserts it at a caret) as one undo step. */
  replaceSelection(text: string): Promise<void>;
  /**
   * Replaces all markers of `owner` (`'sql-error'`, `'running'` …); other owners' stay. Markers
   * follow the text when it is edited. Not an edit: no undo step, works read-only.
   * `setValue()` and `loadText()` drop all markers. Only the markers in view are drawn –
   * 100 000 of them do not slow down typing or scrolling.
   */
  setMarkers(owner: string, markers: Marker[]): Promise<void>;
  /** Removes the markers of `owner`; without an owner all of them. */
  clearMarkers(owner?: string): Promise<void>;
  /** The markers where they are now (after edits), of one owner or all, in document order. */
  getMarkers(owner?: string): Promise<MarkerInfo[]>;
  /**
   * Inserts a snippet in place of the selection (or `range`) as one undo step and selects its
   * first placeholder; Tab / Shift+Tab go on, Escape or leaving the placeholder ends it.
   * Syntax: `$1`, `${1:default}`, `${1|one,two|}`, `$0` (where the caret ends); the same number
   * in several places is typed together; `$NAME` / `${NAME:default}` are variables (`vars`, and
   * built in: `SELECTION`, `DATE`, `TIME`, `CURRENT_YEAR`, `CURRENT_MONTH`, `CURRENT_DATE`,
   * `CURRENT_HOUR`, `CURRENT_MINUTE`, `CURRENT_SECOND`); `\$`, `\}` and `\\` are the characters
   * themselves. New lines take the indentation of the line, tabs the editor's indentation.
   */
  insertSnippet(template: string, options?: SnippetOptions | { start: EdPos; end: EdPos }): Promise<void>;
  /** Replaces the whole text (clean, no undo back into the old one). */
  setValue(text: string): Promise<void>;
  /** A new text, optionally in another language. */
  loadText(text: string, options?: { language?: string }): Promise<void>;
  focus(): void;
  runCommand(cmd: EditorCommand): Promise<void>;
  /** Runs an action of the catalog (`find.open`, `edit.sortLines` …); `false` = not available. */
  runAction(id: string): Promise<boolean>;
  /** Selects the next match and returns how many there are. */
  find(pattern: string, options?: FindOptions): Promise<MatchCount>;
  /** Puts the caret at `line`, column `ch` (both 1-based). */
  goTo(line: number, ch?: number): Promise<void>;
  /** Plays a typing script (`*.edscript`) in real time; `tempo` 2 = twice as fast. */
  playScript(script: string, tempo?: number): Promise<void>;
  stopScript(): void;

  addEventListener<K extends keyof EditliciousEditorEventMap>(
    type: K,
    listener: (this: EditliciousEditorElement, ev: EditliciousEditorEventMap[K]) => unknown,
    options?: boolean | AddEventListenerOptions,
  ): void;
  addEventListener(
    type: string,
    listener: EventListenerOrEventListenerObject,
    options?: boolean | AddEventListenerOptions,
  ): void;
}

export interface EditliciousEditorEventMap extends HTMLElementEventMap {
  'ed-ready': CustomEvent<null>;
  'ed-change': CustomEvent<EdChange>;
  'ed-caret': CustomEvent<EdCaret>;
  'ed-marker-click': CustomEvent<MarkerInfo>;
}

/** Loads the engine once for the whole page (optional – the first editor does it too). */
export function initEditlicious(options?: EditliciousInit): Promise<void>;

/** Defines `<editlicious-editor>` (done on import; a second call does nothing). */
export function defineEditlicious(): Promise<void>;

declare global {
  interface HTMLElementTagNameMap {
    'editlicious-editor': EditliciousEditorElement;
  }
}
