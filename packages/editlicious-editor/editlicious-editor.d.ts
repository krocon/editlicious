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

/** Detail of `ed-caret`: 1-based line and column of the primary caret. */
export interface EdCaret {
  line: number;
  ch: number;
  selChars: number;
  selLines: number;
  carets: number;
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
  | 'amber';

export interface EditliciousEditorElement extends HTMLElement {
  /** Text to show; set again only when it differs from what was set last. */
  value: string | null;
  /** Grammar (`Rust`), alias (`TypeScript`, `Kotlin`) or file name (`app.ts`, `.kt`); empty = plain text. */
  language: string;
  scheme: ColorScheme;
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
