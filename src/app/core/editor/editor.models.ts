/**
 * Mirror of Rust `ed_core::document::model` and `ed_core::editor::model` (designs 002, 003).
 * Rust is the editor engine – the UI sends commands and draws frames (design 002 E13).
 */

export interface Pos {
  line: number;
  ch: number;
}

/** A selection; `anchor` = `head` is a caret (Rust `Sel`). */
export interface Sel {
  anchor: Pos;
  head: Pos;
}

export type LineEnding = 'lf' | 'crlf' | 'cr';

export interface DocInfo {
  id: number;
  path: string | null;
  name: string;
  encoding: string;
  bom: boolean;
  lineEnding: LineEnding;
  mixedLineEndings: boolean;
  decodeErrors: boolean;
  lineCount: number;
  charCount: number;
  fileSize: number;
  revision: number;
  dirty: boolean;
  canUndo: boolean;
  canRedo: boolean;
}

/** Did the file change on disk? (`disk_state`, design 004 E6) */
export interface DiskState {
  changed: boolean;
  missing: boolean;
}

export interface LoadEvent {
  kind: 'progress';
  loaded: number;
  total: number;
}

export interface SaveRequest {
  path?: string;
  encoding?: string;
  bom?: boolean;
  /** Overwrite although the file changed on disk (the user said so). */
  force?: boolean;
}

export type Motion =
  | 'left'
  | 'right'
  | 'up'
  | 'down'
  | 'wordLeft'
  | 'wordRight'
  | 'lineStart'
  | 'lineEnd'
  | 'pageUp'
  | 'pageDown'
  | 'docStart'
  | 'docEnd';

/** Commands of the editor session (`ed_core::editor::model::Command`, tag `type`). */
export type Command =
  | { type: 'frame' }
  | { type: 'resize'; lines: number; cols: number }
  | {
      type: 'configure';
      tabWidth?: number;
      insertSpaces?: boolean;
      detect?: boolean;
      wrap?: boolean;
      colors?: boolean;
      expandWeights?: Record<string, number>;
    }
  | { type: 'setLineEnding'; lineEnding: LineEnding }
  | { type: 'copyHtml'; lineNumbers: boolean; styles: string[]; base: string; gutter: string }
  | { type: 'scroll'; lines: number; cells: number }
  | { type: 'scrollTo'; top?: number; left?: number }
  | { type: 'move'; to: Motion; select?: boolean }
  | { type: 'selectAll' }
  | { type: 'goTo'; line: number; ch: number; select?: boolean }
  | {
      type: 'click';
      line: number;
      cell: number;
      count?: number;
      extend?: boolean;
      add?: boolean;
    }
  | { type: 'drag'; line: number; cell: number; column?: boolean }
  | { type: 'type'; text: string }
  | { type: 'insert'; text: string }
  | { type: 'newline' }
  | { type: 'backspace' }
  | { type: 'delete' }
  | { type: 'deleteWordLeft' }
  | { type: 'deleteWordRight' }
  | { type: 'indent' }
  | { type: 'outdent' }
  | { type: 'toggleOverwrite' }
  | { type: 'undo' }
  | { type: 'redo' }
  | { type: 'copy' }
  | { type: 'cut' }
  | { type: 'paste' }
  | { type: 'find'; query: Query; backwards?: boolean; incremental?: boolean }
  | { type: 'clearSearch' }
  | { type: 'replace'; query: Query; replacement: string }
  | { type: 'replaceAll'; query: Query; replacement: string }
  | { type: 'sortLines'; options: SortOptions }
  | { type: 'changeCase'; to: CaseChange }
  | { type: 'duplicateLines' }
  | { type: 'deleteLines' }
  | { type: 'moveLines'; up: boolean }
  | { type: 'joinLines' }
  | { type: 'trimTrailingWhitespace' }
  | { type: 'convertIndentation'; spaces: boolean }
  | { type: 'blockMove'; dx: number; dy: number }
  | { type: 'addNextOccurrence' }
  | { type: 'singleCursor' }
  | { type: 'toggleMarker' }
  | { type: 'clearMarkers' }
  | { type: 'setLanguage'; name: string | null }
  /** One cursor per entry, the last one primary (design 015 E2). */
  | { type: 'select'; sels: Sel[] }
  | { type: 'convertColor' };

/** A search (`ed_core::search::Query`, design 006). */
export interface Query {
  pattern: string;
  regex: boolean;
  caseSensitive: boolean;
  wholeWord: boolean;
}

/** `count_matches`: all matches and the number of the selected one (0 = none). */
export interface MatchCount {
  total: number;
  current: number;
}

export interface SortOptions {
  descending?: boolean;
  mode?: 'natural' | 'text' | 'numeric';
  ignoreCase?: boolean;
  unique?: boolean;
}

export type CaseChange = 'upper' | 'lower' | 'title' | 'toggle';

export type RunKind = 'text' | 'wide' | 'tab' | 'ctrl';

export interface Run {
  /** First cell, absolute in the line. */
  cell: number;
  width: number;
  kind: RunKind;
  text: string;
  /** Style class, index into {@link STYLE_CLASSES} (0 = plain). */
  style: number;
}

/** Mirror of Rust `highlight::CLASSES` (design 008). */
export const STYLE_CLASSES = [
  '',
  'comment',
  'string',
  'keyword',
  'number',
  'constant',
  'type',
  'function',
  'tag',
  'attribute',
  'variable',
  'operator',
  'punct',
  'heading',
  'bold',
  'italic',
  'code',
  'link',
  'invalid',
  'inserted',
  'deleted',
  'changed',
  'log-error',
  'log-warning',
  'log-info',
  'log-debug',
] as const;

export interface FrameLine {
  line: number;
  runs: Run[];
  /** Selected cell ranges `[from, to)`. */
  sel: [number, number][];
  selEol: boolean;
  width: number;
  long: boolean;
  /** Search matches `[from, to, kind]` in cells; kind 1 = the selected one. */
  marks: [number, number, number][];
  /** Marked words `[from, to, colour 0…9]` in cells. */
  markers: [number, number, number][];
  /** Width of the leading whitespace in cells; blank lines: the line above's. */
  indent: number;
  /** Rows from the top of the view to this line (negative: partly scrolled away). */
  row: number;
  /** Soft wrap: first cells of the rows after the first. */
  wraps: number[];
  /** Colour values `[from, to, rgba]`: characters, colour 0xRRGGBBAA (design 018). */
  colors: [number, number, number][];
}

/** Where a caret is drawn: rows from the view top, cell within its row. */
export interface CaretXY {
  row: number;
  x: number;
}

export interface CaretPos {
  line: number;
  cell: number;
}

export interface Frame {
  seq: number;
  info: DocInfo;
  top: number;
  left: number;
  firstLine: number;
  lines: FrameLine[];
  maxWidth: number;
  carets: CaretPos[];
  /** Where each caret is drawn (same order). */
  caretXy: CaretXY[];
  /** Index of the primary caret. */
  primary: number;
  caretLine: number;
  caretCh: number;
  selChars: number;
  selLines: number;
  overwrite: boolean;
  tabWidth: number;
  insertSpaces: boolean;
  notice: string | null;
  search: boolean;
  /** More than one cursor. */
  multi: boolean;
  /** Language of the highlighting, `null` = plain text. */
  language: string | null;
  /** The highlighting of the view is approximate (far jump; exact soon). */
  approxHighlight: boolean;
  selectionText: string | null;
  /** Soft wrap is on. */
  wrap: boolean;
  /** Changes with search and markers (the overview asks again). */
  deco: number;
  /** The gutter shows colour swatches (stays once a colour showed, design 018). */
  colorColumn: boolean;
  /** Colour value at the primary caret, `#rrggbb[aa]`. */
  caretColor: string | null;
}
