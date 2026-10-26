/**
 * Mirror of Rust `ed_core::document::model` and `ed_core::editor::model` (designs 002, 003).
 * Rust is the editor engine – the UI sends commands and draws frames (design 002 E13).
 */

export interface Pos {
  line: number;
  ch: number;
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
  | { type: 'configure'; tabWidth?: number; insertSpaces?: boolean }
  | { type: 'scroll'; lines: number; cells: number }
  | { type: 'scrollTo'; top?: number; left?: number }
  | { type: 'move'; to: Motion; select?: boolean }
  | { type: 'selectAll' }
  | { type: 'goTo'; line: number; ch: number; select?: boolean }
  | { type: 'click'; line: number; cell: number; count?: number; extend?: boolean }
  | { type: 'drag'; line: number; cell: number }
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
  | { type: 'paste' };

export type RunKind = 'text' | 'wide' | 'tab' | 'ctrl';

export interface Run {
  /** First cell, absolute in the line. */
  cell: number;
  width: number;
  kind: RunKind;
  text: string;
}

export interface FrameLine {
  line: number;
  runs: Run[];
  /** Selected cell ranges `[from, to)`. */
  sel: [number, number][];
  selEol: boolean;
  width: number;
  long: boolean;
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
  caretLine: number;
  caretCh: number;
  selChars: number;
  selLines: number;
  overwrite: boolean;
  tabWidth: number;
  insertSpaces: boolean;
  notice: string | null;
}
