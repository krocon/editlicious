import { computed, inject, Injectable, signal } from '@angular/core';
import { Command, Pos } from '../core/editor/editor.models';
import {
  ExternalCompletion,
  ExternalCompletionView,
  ExternalList,
} from '../features/editor/editor-host';
import { before, EdPos, enginePos, pagePos } from './positions';
import { EmbedEngineApi, EngineCompletionContext } from './wasm-editor-api';

/** One suggestion of the page (design 028 E3). */
export interface EdCompletionItem {
  /** What the list shows (and what is inserted without `insertText`). */
  label: string;
  insertText?: string;
  /** `insertText` is a snippet: `${1:name}`, `$0` … (design 030). */
  snippet?: boolean;
  /** A class of your own (`table`, `column` …): the icon's letter, `::part(icon-<kind>)`. */
  kind?: string;
  /** Shown at the right (a type, a schema). */
  detail?: string;
  /** Shown beside the list while the entry is active. */
  documentation?: string;
  /** Order in the list (default: `label`). */
  sortText?: string;
  /** What typing filters on (default: `label`). */
  filterText?: string;
  /** The text to replace (default: the word begun up to the caret). */
  range?: { start: EdPos; end: EdPos };
}

/** What the provider gets (design 028 E2) – a few characters, never the whole text unasked. */
export interface EdCompletionContext {
  /** The caret. */
  position: EdPos;
  /** The line up to the caret (its last 2000 characters at most). */
  lineText: string;
  /** The word begun left of the caret (empty after a blank or a dot). */
  word: string;
  wordStart: EdPos;
  /** `invoke`: asked for (Ctrl+Space); `type`: while typing; `character`: a trigger character. */
  trigger: 'invoke' | 'type' | 'character';
  triggerCharacter: string | null;
  /** Aborted when the answer is no longer wanted (typed on, moved away). */
  signal: AbortSignal;
  /** Text around the caret, on request. */
  getRange(start: EdPos, end: EdPos): Promise<string>;
  /** Lines `from..to` (1-based, inclusive). */
  getSlice(from: number, to: number): Promise<string>;
}

export type EdCompletionResult =
  EdCompletionItem[] | { items: EdCompletionItem[]; incomplete?: boolean } | null | undefined;

export interface EdCompletionProviderFn {
  (ctx: EdCompletionContext): EdCompletionResult | Promise<EdCompletionResult>;
  /** Characters that ask the provider when typed (`.`, a blank …). */
  triggerCharacters?: string | string[];
}

export type EdCompletionProvider =
  | EdCompletionProviderFn
  | { provideCompletions: EdCompletionProviderFn; triggerCharacters?: string | string[] };

/** A snippet the page offers by a prefix (design 030 E6). */
export interface EdSnippet {
  prefix: string;
  /** The template; an array is its lines. */
  body: string | string[];
  description?: string;
}

/** Where suggestions come from when there is a provider (design 028 E5). */
export type CompletionSources = 'both' | 'provider' | 'words';

/** Rows of the list on screen; the rest scrolls (50 000 entries cost ten rows). */
export const COMPLETION_ROWS = 10;
/** Typing asks from this many characters of a word (like the word completion). */
const AUTO_MIN = 2;

interface Trigger {
  kind: 'invoke' | 'type' | 'character';
  character: string | null;
}

interface Prepared {
  item: EdCompletionItem;
  /** Lower case of what typing filters on. */
  filter: string;
  kind: string;
  detail: string;
}

interface Open {
  line: number;
  wordFrom: number;
  cell: number;
  all: Prepared[];
  complete: boolean;
  trigger: Trigger;
  shown: Prepared[];
  active: number;
  top: number;
}

/** The letters and digits of a kind – safe as a part name. */
export function kindName(kind: string | undefined): string {
  return (kind ?? '')
    .toLowerCase()
    .replace(/[^a-z0-9-]+/g, '-')
    .replace(/^-+|-+$/g, '');
}

/** The body of a snippet as one template. */
export function snippetBody(s: EdSnippet): string {
  return Array.isArray(s.body) ? s.body.join('\n') : String(s.body ?? '');
}

/** The date and time variables of snippets (`$CURRENT_YEAR`, `$DATE` …) at `now`. */
export function dateVars(now = new Date()): Record<string, string> {
  const two = (n: number) => String(n).padStart(2, '0');
  const date = `${now.getFullYear()}-${two(now.getMonth() + 1)}-${two(now.getDate())}`;
  const time = `${two(now.getHours())}:${two(now.getMinutes())}:${two(now.getSeconds())}`;
  return {
    CURRENT_YEAR: String(now.getFullYear()),
    CURRENT_MONTH: two(now.getMonth() + 1),
    CURRENT_DATE: two(now.getDate()),
    CURRENT_HOUR: two(now.getHours()),
    CURRENT_MINUTE: two(now.getMinutes()),
    CURRENT_SECOND: two(now.getSeconds()),
    DATE: date,
    TIME: time,
  };
}

/**
 * Entries that match `word`, in their order: first those that start with it, then those that
 * contain it (case-insensitive). One pass – 50 000 entries take about a millisecond.
 */
export function filterItems<T extends { filter: string }>(all: T[], word: string): T[] {
  const w = word.toLowerCase();
  if (!w) return all;
  const starts: T[] = [];
  const inner: T[] = [];
  for (const p of all) {
    const i = p.filter.indexOf(w);
    if (i === 0) starts.push(p);
    else if (i > 0) inner.push(p);
  }
  return starts.concat(inner);
}

function prepare(item: EdCompletionItem): Prepared {
  const label = String(item.label ?? '');
  return {
    item: { ...item, label },
    filter: String(item.filterText ?? label).toLowerCase(),
    kind: kindName(item.kind),
    detail: String(item.detail ?? ''),
  };
}

/** The provider's entries in their order (`sortText`, else `label`; stable). */
function sorted(items: EdCompletionItem[]): Prepared[] {
  const keyed = items
    .filter((it) => it && typeof it.label === 'string' && it.label !== '')
    .map((it, i) => ({ p: prepare(it), key: String(it.sortText ?? it.label).toLowerCase(), i }));
  keyed.sort((a, b) => (a.key < b.key ? -1 : a.key > b.key ? 1 : a.i - b.i));
  return keyed.map((k) => k.p);
}

/**
 * Suggestions the page provides (design 028): asks the provider, keeps its answer, filters while
 * the user types and inserts the chosen entry. One per editor; the view draws {@link list} in its
 * suggestion list and sends the keys here. Nothing here ever waits in the way of typing: every
 * change of the text or the caret makes an older answer worthless (it is dropped when it comes).
 */
@Injectable()
export class ProviderCompletion implements ExternalCompletion {
  readonly provider = signal<EdCompletionProvider | null>(null);
  readonly sources = signal<CompletionSources>('both');
  readonly snippets = signal<EdSnippet[]>([]);
  /** Ask while typing (the editor's "suggest while typing"). */
  readonly auto = signal(false);

  /** There is something to ask besides the engine's own word list. */
  readonly on = computed(
    () => (this.provider() !== null && this.sources() !== 'words') || this.snippets().length > 0,
  );
  readonly list = signal<ExternalList | null>(null);

  private readonly api = inject(EmbedEngineApi);
  private view: ExternalCompletionView | null = null;
  /** Counts everything that makes a running request stale. */
  private seq = 0;
  private abort: AbortController | null = null;
  /** The request on its way: where it was asked and why. */
  private asked: { line: number; wordFrom: number; trigger: Trigger } | null = null;
  private open: Open | null = null;

  bind(view: ExternalCompletionView): void {
    this.view = view;
  }

  private fn(): EdCompletionProviderFn | null {
    const p = this.provider();
    if (!p || this.sources() === 'words') return null;
    if (typeof p === 'function') return p;
    return typeof p.provideCompletions === 'function' ? (ctx) => p.provideCompletions(ctx) : null;
  }

  private triggers(): string {
    const p = this.provider();
    const t =
      (p as { triggerCharacters?: string | string[] } | null)?.triggerCharacters ??
      (p && typeof p !== 'function' ? p.provideCompletions?.triggerCharacters : undefined);
    return Array.isArray(t) ? t.join('') : typeof t === 'string' ? t : '';
  }

  // ------------------------------------------------------------------ what the view tells

  after(cmd: Command): void {
    if (!this.on()) return;
    switch (cmd.type) {
      case 'type': {
        const ch = Array.from(cmd.text).at(-1) ?? '';
        if (ch && this.triggers().includes(ch)) {
          void this.update({ kind: 'character', character: ch });
        } else if (this.open || this.asked) {
          void this.update(null, ch);
        } else if (this.auto()) {
          void this.update({ kind: 'type', character: ch });
        }
        return;
      }
      case 'backspace':
        if (this.open || this.asked) void this.update(null);
        return;
      // the view only: the list stays
      case 'frame':
      case 'resize':
      case 'configure':
      case 'scroll':
      case 'scrollTo':
      case 'copy':
      case 'copyHtml':
      case 'setReadOnly':
        return;
      default:
        this.close();
    }
  }

  invoke(): void {
    this.open = null;
    void this.update({ kind: 'invoke', character: null });
  }

  close(): void {
    this.seq++;
    this.abort?.abort();
    this.abort = null;
    this.asked = null;
    this.open = null;
    this.list.set(null);
  }

  move(delta: number): void {
    const o = this.open;
    const n = o?.shown.length ?? 0;
    if (!o || !n) return;
    o.active = (o.active + delta + n) % n;
    if (o.active < o.top) o.top = o.active;
    if (o.active >= o.top + COMPLETION_ROWS) o.top = o.active - COMPLETION_ROWS + 1;
    this.publish();
  }

  scroll(rows: number): void {
    const o = this.open;
    if (!o) return;
    const max = Math.max(0, o.shown.length - COMPLETION_ROWS);
    o.top = Math.min(max, Math.max(0, o.top + rows));
    o.active = Math.min(o.top + COMPLETION_ROWS - 1, Math.max(o.top, o.active));
    this.publish();
  }

  accept(index?: number): void {
    const o = this.open;
    const view = this.view;
    const p = o?.shown[index ?? o.active];
    if (!o || !p || !view) return;
    const id = view.docId();
    this.close();
    // in the order of the keys: what was typed before the choice is part of the word
    void view.enqueue(async () => {
      const ctx = await this.api.completionContext(id, false);
      if (!ctx.single || ctx.line !== o.line || ctx.ch < o.wordFrom) return;
      let from: Pos = { line: o.line, ch: o.wordFrom };
      let to: Pos = { line: ctx.line, ch: ctx.ch };
      const range = p.item.range;
      if (range) {
        from = enginePos(range.start);
        const end = enginePos(range.end);
        if (before(to, end)) to = end;
      }
      const text = String(p.item.insertText ?? p.item.label);
      const cmd: Command = p.item.snippet
        ? { type: 'insertSnippet', template: text, from, to, vars: dateVars() }
        : { type: 'replaceRange', from, to, text, caret: true };
      await this.api.command(id, cmd);
    });
  }

  /** A snippet's prefix stands right before the caret: the command that expands it (Tab). */
  async tabCommand(): Promise<Command | null> {
    const view = this.view;
    const snippets = this.snippets();
    if (!view || !snippets.length) return null;
    const ctx = await this.api.completionContext(view.docId(), false);
    if (!ctx.single) return null;
    const word = wordOf(ctx);
    const s = word ? snippets.find((x) => x.prefix === word) : undefined;
    if (!s) return null;
    return {
      type: 'insertSnippet',
      template: snippetBody(s),
      from: { line: ctx.line, ch: ctx.wordFrom },
      to: { line: ctx.line, ch: ctx.ch },
      vars: dateVars(),
    };
  }

  // ------------------------------------------------------------------ asking and filtering

  /**
   * `fresh`: why to ask (Ctrl+Space, a trigger character, typing); `null`: the text changed
   * under an open list or a running request – go on with it while the caret stays in its word.
   */
  private async update(fresh: Trigger | null, typed = ''): Promise<void> {
    const view = this.view;
    if (!view) return;
    const seq = ++this.seq;
    const id = view.docId();
    const fn = this.fn();
    let ctx: EngineCompletionContext;
    try {
      ctx = await this.api.completionContext(id, this.sources() !== 'provider' || !fn);
    } catch {
      return;
    }
    if (seq !== this.seq) return;
    if (!ctx.single) return this.close();
    const word = wordOf(ctx);
    const o = this.open;
    const prev = o ?? this.asked;
    const same = prev !== null && prev.line === ctx.line && prev.wordFrom === ctx.wordFrom;
    let trigger = fresh ?? (same && prev ? prev.trigger : null);
    if (!trigger) {
      // typed on into another word: that is typing, if typing asks at all
      if (!this.auto()) return this.close();
      trigger = { kind: 'type', character: typed };
    }
    if (o && same) {
      // typed on in the same word: the entries we have are filtered right away
      this.show(o, word);
      if (o.complete) return;
    } else if (o) {
      this.open = null;
      this.list.set(null);
    }
    if (trigger.kind === 'type' && Array.from(word).length < AUTO_MIN) {
      if (!(o && same)) this.close();
      return;
    }
    // ask again; an answer still on its way is no longer wanted
    this.abort?.abort();
    const ctrl = (this.abort = new AbortController());
    let items: Prepared[] = [];
    let complete = true;
    this.asked = { line: ctx.line, wordFrom: ctx.wordFrom, trigger };
    if (fn) {
      let result: EdCompletionResult = null;
      try {
        result = await fn(this.pageContext(id, ctx, word, trigger, ctrl.signal));
      } catch (e) {
        if (!ctrl.signal.aborted) console.error('editlicious: the completion provider failed', e);
      }
      if (seq !== this.seq || ctrl.signal.aborted) return;
      const list = Array.isArray(result) ? result : (result?.items ?? []);
      complete = Array.isArray(result) || !result?.incomplete;
      items = sorted(Array.isArray(list) ? list : []);
    }
    const all = this.withLocal(items, ctx);
    const next: Open = {
      line: ctx.line,
      wordFrom: ctx.wordFrom,
      cell: ctx.wordCell,
      all,
      complete,
      trigger,
      shown: [],
      active: 0,
      top: 0,
    };
    this.asked = null;
    this.open = next;
    this.show(next, word);
  }

  /** The provider's entries, then the page's snippets, then the words of the text (E5). */
  private withLocal(items: Prepared[], ctx: EngineCompletionContext): Prepared[] {
    const all = items.slice();
    for (const s of this.snippets()) {
      if (!s?.prefix) continue;
      const body = snippetBody(s);
      all.push(
        prepare({
          label: s.prefix,
          insertText: body,
          snippet: true,
          kind: 'snippet',
          detail: s.description ?? '',
          documentation: body,
        }),
      );
    }
    if (ctx.words.length) {
      const have = new Set(all.map((p) => p.item.label));
      for (const [w, keyword] of ctx.words) {
        if (!have.has(w)) all.push(prepare({ label: w, kind: keyword ? 'keyword' : 'word' }));
      }
    }
    return all;
  }

  private pageContext(
    id: number,
    ctx: EngineCompletionContext,
    word: string,
    trigger: Trigger,
    signal: AbortSignal,
  ): EdCompletionContext {
    return {
      position: pagePos({ line: ctx.line, ch: ctx.ch }),
      lineText: ctx.before,
      word,
      wordStart: pagePos({ line: ctx.line, ch: ctx.wordFrom }),
      trigger: trigger.kind,
      triggerCharacter: trigger.character,
      signal,
      getRange: (start, end) => this.api.rangeText(id, enginePos(start), enginePos(end)),
      getSlice: (from, to) => this.api.slice(id, Math.max(0, from - 1), Math.max(0, to)),
    };
  }

  private show(o: Open, word: string): void {
    o.shown = filterItems(o.all, word);
    o.active = 0;
    o.top = 0;
    if (o.shown.length === 0) {
      // nothing fits: no list – but a complete answer is kept for a backspace
      this.list.set(null);
      if (!o.complete || !word) this.open = this.open === o ? null : this.open;
      return;
    }
    this.publish();
  }

  private publish(): void {
    const o = this.open;
    if (!o || !o.shown.length) return this.list.set(null);
    const active = o.shown[o.active];
    this.list.set({
      line: o.line,
      cell: o.cell,
      total: o.shown.length,
      top: o.top,
      active: o.active,
      items: o.shown.slice(o.top, o.top + COMPLETION_ROWS).map((p) => ({
        label: p.item.label,
        kind: p.kind,
        detail: p.detail,
      })),
      doc: String(active?.item.documentation ?? ''),
    });
  }
}

/** The word begun left of the caret. */
function wordOf(ctx: EngineCompletionContext): string {
  const tail = Array.from(ctx.before);
  const n = ctx.ch - ctx.wordFrom;
  return n > 0 ? tail.slice(-n).join('') : '';
}
