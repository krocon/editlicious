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
