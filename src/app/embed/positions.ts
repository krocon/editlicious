import { Pos } from '../core/editor/editor.models';

/**
 * A position in the text as the page gives and gets it: line and column, both 1-based. The
 * column counts characters as Unicode code points within the line (an emoji is one, a combining
 * mark is one of its own, the line break none) – not UTF-16 units (design 027 E5).
 */
export interface EdPos {
  line: number;
  ch: number;
}

/** A selected range in document order with its text. */
export interface EdSelection {
  text: string;
  start: EdPos;
  end: EdPos;
}

/** 1-based position of the page → the engine's 0-based one. */
export function enginePos(p: EdPos | null | undefined): Pos {
  return {
    line: Math.max(0, Math.round(Number(p?.line) || 1) - 1),
    ch: Math.max(0, Math.round(Number(p?.ch) || 1) - 1),
  };
}

export function pagePos(p: Pos): EdPos {
  return { line: p.line + 1, ch: p.ch + 1 };
}

/** `a` is at or before `b`. */
export function before(a: Pos, b: Pos): boolean {
  return a.line < b.line || (a.line === b.line && a.ch <= b.ch);
}
