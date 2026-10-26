import {
  afterNextRender,
  ChangeDetectionStrategy,
  Component,
  computed,
  DestroyRef,
  effect,
  ElementRef,
  inject,
  Injector,
  input,
  output,
  signal,
  untracked,
  viewChild,
} from '@angular/core';
import { EditorApi } from '../../core/editor/editor-api';
import { Command, Frame, FrameLine, LineEnding, Run } from '../../core/editor/editor.models';
import { DocumentsStore } from '../documents/documents.store';
import { describeFsError, toFsError } from '../../core/errors/fs-error';
import { ActionService } from '../../core/actions/action.service';
import { ACTIONS } from '../../core/actions/action-catalog';
import { EDITOR_TEXTS, noticeText } from './editor-texts';
import { FindBar } from '../find/find-bar';
import { FindStore } from '../find/find.store';
import { FIND_TEXTS } from '../find/find-texts';
import { GotoBar } from '../find/goto-bar';
import { PickService } from '../../shared/pick/pick.service';
import { MENUS_TEXTS } from '../menus/menus-texts';
import { SettingsStore } from '../settings/settings.store';

/** Cells of padding left of the text. */
const PAD_CELLS = 1;
/** Width of the quiet scrollbars in px. */
const BAR = 10;
const MIN_THUMB = 28;

interface Metrics {
  /** width of one cell */
  cw: number;
  /** height of one line */
  lh: number;
}

/**
 * The editor surface (design 003). It only renders frames of the Rust engine and turns
 * keyboard, mouse and wheel input into commands – no text logic here (design 002 E13).
 * Both directions are virtual: the frame holds the visible lines and only the visible cells.
 */
@Component({
  selector: 'app-editor-view',
  imports: [FindBar, GotoBar],
  templateUrl: './editor-view.html',
  styleUrl: './editor-view.css',
  changeDetection: ChangeDetectionStrategy.OnPush,
  host: {
    '[class.focused]': 'focused()',
    '[style.--lh.px]': 'metrics().lh',
    '[class.no-current]': '!settings.settings().highlightCurrentLine',
    '[class.no-tabs]': '!settings.settings().showTabs',
    '(wheel)': 'onWheel($event)',
  },
})
export class EditorView {
  /** The engine session to show. */
  readonly docId = input.required<number>();
  /** Changes when the document was replaced underneath (reload) – draw again. */
  readonly refresh = input(0);
  /** Every frame (dirty state, caret, scroll for tabs and the session). */
  readonly frameChange = output<Frame>();

  protected readonly texts = EDITOR_TEXTS;
  protected readonly frame = signal<Frame | null>(null);
  protected readonly metrics = signal<Metrics>({ cw: 8, lh: 22 });
  protected readonly size = signal({ width: 0, height: 0 });
  protected readonly focused = signal(false);
  protected readonly composing = signal(false);
  protected readonly status = signal('');
  protected readonly find = inject(FindStore);
  protected readonly settings = inject(SettingsStore);
  protected readonly gotoOpen = signal(false);
  protected readonly findCount = signal('');
  protected readonly findInvalid = signal(false);
  private countTimer: ReturnType<typeof setTimeout> | null = null;

  private readonly api = inject(EditorApi);
  private readonly host = inject<ElementRef<HTMLElement>>(ElementRef);
  private readonly input = viewChild.required<ElementRef<HTMLTextAreaElement>>('input');
  private readonly measure = viewChild.required<ElementRef<HTMLElement>>('measure');
  private readonly destroyRef = inject(DestroyRef);

  private chain: Promise<unknown> = Promise.resolve();
  private pendingScroll: { lines: number; cells: number } | null = null;
  private pendingDrag: { line: number; cell: number; column: boolean } | null = null;
  private frameRequested = false;
  private dragging: {
    autoscroll?: ReturnType<typeof setInterval>;
    x: number;
    y: number;
    column: boolean;
  } | null = null;

  // ---------------------------------------------------------------- geometry

  protected readonly lineNumbers = computed(() => this.settings.settings().lineNumbers);

  protected readonly gutterWidth = computed(() => {
    const mode = this.lineNumbers();
    if (mode === 'off') return this.metrics().cw;
    const lines = this.frame()?.info.lineCount ?? 1;
    const digits = Math.max(3, String(lines).length);
    return (digits + 2) * this.metrics().cw;
  });

  /** Gutter number of a line: absolute, or the distance to the caret line (design 009). */
  protected lineLabel(line: number, caretLine: number): string {
    if (this.lineNumbers() !== 'relative' || line === caretLine) return String(line + 1);
    return String(Math.abs(line - caretLine));
  }

  /** Cells of the indentation guides of a line (one per indent step, not at 0). */
  protected guides(line: FrameLine, tab: number): number[] {
    if (!this.settings.settings().indentGuides || line.indent < tab * 1) return [];
    const out: number[] = [];
    for (let c = tab; c < line.indent; c += tab) out.push(c);
    return out;
  }

  /** Left edge of the text in px (gutter + padding). */
  protected readonly textLeft = computed(() => this.gutterWidth() + PAD_CELLS * this.metrics().cw);

  private readonly viewLines = computed(() => this.size().height / this.metrics().lh);
  private readonly viewCols = computed(
    () => Math.max(1, this.size().width - this.textLeft() - BAR) / this.metrics().cw,
  );

  protected readonly vbar = computed(() => {
    const f = this.frame();
    const h = this.size().height;
    if (!f || f.info.lineCount <= Math.floor(this.viewLines())) return null;
    const total = f.info.lineCount + this.viewLines() - 1;
    const thumb = Math.max(MIN_THUMB, (h * this.viewLines()) / total);
    const range = Math.max(1, f.info.lineCount - 1);
    return { thumb, pos: ((h - thumb) * f.top) / range, track: h, range };
  });

  protected readonly hbar = computed(() => {
    const f = this.frame();
    const w = this.size().width - this.textLeft() - BAR;
    const cols = this.viewCols();
    const caretCell = f?.carets[f.primary]?.cell ?? 0;
    const caretLong = !!f?.lines.find((l) => l.line === f.carets[f.primary]?.line)?.long;
    // soft wrap: only long lines scroll sideways
    const extent =
      f?.wrap && !caretLong ? (f?.maxWidth ?? 0) : Math.max(f?.maxWidth ?? 0, caretCell + 1);
    if (!f || extent <= cols - 1) return null;
    const range = Math.max(1, extent - cols + 2);
    const thumb = Math.max(MIN_THUMB, (w * cols) / (extent + 2));
    return { thumb, pos: ((w - thumb) * Math.min(f.left, range)) / range, track: w, range };
  });

  /** Primary caret in px – the textarea sits there (IME window near the caret). */
  protected readonly caretXY = computed(() => {
    const f = this.frame();
    if (!f || !f.carets[f.primary]) return { x: 0, y: 0 };
    return this.caretPx(f.primary);
  });

  /** Search matches and markers in the scrollbar (design 010 E3). */
  protected readonly overview = signal<number[]>([]);
  protected readonly overviewTicks = computed(() => {
    const o = this.overview();
    const b = this.vbar();
    if (!b || o.length === 0) return [];
    const out: { top: number; match: boolean; marker: boolean }[] = [];
    o.forEach((bits, i) => {
      if (bits) {
        out.push({ top: (i * b.track) / o.length, match: !!(bits & 1), marker: !!(bits & 2) });
      }
    });
    return out;
  });
  private overviewTimer: ReturnType<typeof setTimeout> | null = null;

  constructor() {
    afterNextRender(() => {
      this.measureFont();
      const ro = new ResizeObserver(() => this.onResize());
      ro.observe(this.host.nativeElement);
      this.destroyRef.onDestroy(() => ro.disconnect());
      this.onResize();
      this.focus();
    });
    // a new session (tab switch): size + first frame
    effect(() => {
      this.docId();
      untracked(() => {
        this.frame.set(null);
        this.sendResize();
      });
    });
    // the gutter grows with the line count → fewer columns
    effect(() => {
      this.viewCols();
      untracked(() => this.sendResize());
    });
    effect(() => {
      if (this.refresh() > 0) untracked(() => void this.send({ type: 'frame' }));
    });
    // a new font or size: measure again (the resize effect above follows)
    effect(() => {
      const key = `${this.settings.settings().fontSize}/${this.settings.fontFamily()}`;
      untracked(() => {
        if (key && this.size().height > 0) setTimeout(() => this.measureFont());
      });
    });
    // the overview follows search, markers and edits – a little later (design 010 E3)
    effect(() => {
      const f = this.frame();
      const on = this.settings.settings().overview;
      const track = this.vbar()?.track ?? 0;
      const key =
        f && on && f.deco !== 0 && track > 0 ? `${f.info.revision}/${f.deco}/${track}` : '';
      untracked(() => this.overviewSoon(key, track));
    });
    // tab width and indentation for every session (design 009)
    effect(() => {
      this.docId();
      const s = this.settings.settings();
      untracked(
        () =>
          void this.send({
            type: 'configure',
            tabWidth: s.tabWidth,
            insertSpaces: s.indentWith !== 'tabs',
            detect: s.indentWith === 'auto',
            wrap: s.wordWrap,
          }),
      );
    });
    this.destroyRef.onDestroy(() => this.stopDrag());
    // the editor actions run engine commands (design 004 E1)
    const actions = inject(ActionService);
    for (const def of ACTIONS) {
      const cmd = def.cmd;
      if (cmd) actions.register(def.id, { run: () => void this.send(cmd) }, this.destroyRef);
    }
    this.registerFind(actions);
  }

  // ---------------------------------------------------------------- find (design 006)

  private registerFind(actions: ActionService): void {
    const f = this.find;
    actions.registerAll(
      {
        'find.open': { run: () => this.openFind(false) },
        'find.replace': { run: () => this.openFind(true) },
        'find.next': { run: () => this.findStep(false) },
        'find.prev': { run: () => this.findStep(true) },
        'edit.escape': {
          run: () => {
            if (f.open() || this.gotoOpen() || this.frame()?.search) this.closeFind();
            else void this.send({ type: 'singleCursor' });
          },
          enabled: () =>
            f.open() || this.gotoOpen() || !!this.frame()?.search || !!this.frame()?.multi,
        },
        'goto.line': { run: () => this.gotoOpen.set(true) },
        'view.language': { run: () => void this.chooseLanguage() },
        'file.encoding': { run: () => void this.chooseEncoding() },
        'file.lineEnding': { run: () => void this.chooseLineEnding() },
        'edit.copyHtml': { run: () => void this.copyHtml(false) },
        'edit.copyHtmlNumbers': { run: () => void this.copyHtml(true) },
      },
      this.destroyRef,
    );
  }

  private readonly picker = inject(PickService);

  /** "Change language…": the engine's grammars, plain text first. */
  private async chooseLanguage(): Promise<void> {
    const names = await this.api.listLanguages().catch((): string[] => []);
    const t = MENUS_TEXTS;
    const id = await this.picker.pick({
      title: t.language,
      placeholder: t.languagePlaceholder,
      options: [{ id: '', label: t.plainText }, ...names.map((n) => ({ id: n, label: n }))],
      current: this.frame()?.language ?? '',
    });
    if (id !== null) void this.send({ type: 'setLanguage', name: id || null });
    this.focus();
  }

  private readonly injector = inject(Injector);

  /** Status bar / palette: read again or convert in another encoding (design 010 E2). */
  private async chooseEncoding(): Promise<void> {
    const f = this.frame();
    if (!f) return;
    const names = await this.api.listEncodings().catch((): string[] => []);
    const enc = await this.picker.pick({
      title: this.texts.encoding,
      placeholder: this.texts.encodingPlaceholder,
      options: names.map((n) => ({ id: n, label: n })),
      current: f.info.encoding,
    });
    if (enc) await this.injector.get(DocumentsStore).changeEncoding(f.info.id, enc);
    this.focus();
  }

  /** Converts every line break (one undo step). */
  private async chooseLineEnding(): Promise<void> {
    const f = this.frame();
    if (!f) return;
    const t = this.texts;
    const le = await this.picker.pick({
      title: t.lineEndings,
      placeholder: t.lineEndingsPlaceholder,
      options: [
        { id: 'lf', label: t.lf },
        { id: 'crlf', label: t.crlf },
        { id: 'cr', label: t.cr },
      ],
      current: f.info.mixedLineEndings ? '' : f.info.lineEnding,
    });
    if (le) void this.send({ type: 'setLineEnding', lineEnding: le as LineEnding });
    this.focus();
  }

  private readonly probes = viewChild<ElementRef<HTMLElement>>('probes');

  /** "Copy as HTML": the colours of the view go along as inline styles (design 010 E5). */
  private async copyHtml(lineNumbers: boolean): Promise<void> {
    const box = this.probes()?.nativeElement;
    const css = (el: Element) => {
      const c = getComputedStyle(el);
      const parts = [`color:${c.color}`];
      if (c.fontStyle && c.fontStyle !== 'normal') parts.push(`font-style:${c.fontStyle}`);
      if (Number(c.fontWeight) >= 600) parts.push(`font-weight:${c.fontWeight}`);
      return parts.join(';');
    };
    const styles = box ? Array.from(box.children, (el) => css(el)) : [];
    const host = getComputedStyle(this.host.nativeElement);
    const root = getComputedStyle(this.host.nativeElement.ownerDocument.documentElement);
    const base = [
      `color:${host.color}`,
      `background:${host.backgroundColor}`,
      `font-family:${host.fontFamily.replace(/"/g, "'")}`,
      `font-size:${host.fontSize}`,
      'padding:8px',
    ].join(';');
    const gutter = `color:${root.getPropertyValue('--ed-gutter-text').trim() || 'gray'}`;
    await this.send({ type: 'copyHtml', lineNumbers, styles, base, gutter });
    if (!this.frame()?.notice) this.status.set(this.texts.copiedHtml);
  }

  /** Style classes of the highlighter (`highlight::CLASSES`) – probes for "copy as HTML". */
  protected readonly styleIds = Array.from({ length: 26 }, (_, i) => i);

  private openFind(replace: boolean): void {
    const f = this.find;
    const sel = this.frame()?.selectionText;
    if (sel) f.pattern.set(sel);
    f.open.set(true);
    if (replace) f.replaceOpen.set(true);
    f.focusRequest.update((n) => n + 1);
    if (f.pattern()) this.runFind({ backwards: false, incremental: true });
  }

  private findStep(backwards: boolean): void {
    if (!this.find.pattern()) {
      this.openFind(false);
      return;
    }
    this.find.remember();
    this.runFind({ backwards });
  }

  protected runFind(o: { backwards: boolean; incremental?: boolean }): void {
    const query = this.find.query();
    if (!query.pattern) {
      this.findCount.set('');
      this.findInvalid.set(false);
      void this.send({ type: 'clearSearch' });
      return;
    }
    void this.send({ type: 'find', query, backwards: o.backwards, incremental: !!o.incremental });
    this.countSoon();
  }

  protected onReplace(all: boolean): void {
    const query = this.find.query();
    if (!query.pattern) return;
    this.find.remember();
    this.find.rememberReplacement();
    const replacement = this.find.replacement();
    void this.send(
      all ? { type: 'replaceAll', query, replacement } : { type: 'replace', query, replacement },
    );
    this.countSoon();
  }

  protected closeFind(): void {
    this.find.open.set(false);
    this.gotoOpen.set(false);
    this.findCount.set('');
    void this.send({ type: 'clearSearch' });
    this.focus();
  }

  protected onGoto(p: { line: number; ch: number }): void {
    this.gotoOpen.set(false);
    void this.send({ type: 'goTo', line: p.line, ch: p.ch });
    this.focus();
  }

  protected closeGoto(): void {
    this.gotoOpen.set(false);
    this.focus();
  }

  /** Counting runs on a snapshot in Rust; after the find command has selected its match. */
  private countSoon(): void {
    if (this.countTimer) clearTimeout(this.countTimer);
    this.countTimer = setTimeout(() => {
      const id = this.docId();
      const query = this.find.query();
      void this.chain.then(async () => {
        try {
          const c = await this.api.countMatches(id, query);
          this.findInvalid.set(false);
          this.findCount.set(
            c.total === 0 ? FIND_TEXTS.noMatch : FIND_TEXTS.count(c.current, c.total),
          );
        } catch (e) {
          const err = toFsError(e);
          if (err.code === 'invalid_search') {
            this.findInvalid.set(true);
            this.findCount.set(FIND_TEXTS.invalid);
          } else if (err.code !== 'cancelled') {
            this.findCount.set('');
          }
        }
      });
    }, 120);
  }

  focus(): void {
    this.input().nativeElement.focus({ preventScroll: true });
  }

  /** Runs a command from outside (menus, actions in prompt 005). */
  run(cmd: Command): Promise<unknown> {
    return this.send(cmd);
  }

  // ---------------------------------------------------------------- rendering helpers

  /** Horizontal scroll of a line: wrapped lines never scroll sideways (design 010 E4). */
  private offset(line: FrameLine | undefined): number {
    const f = this.frame();
    if (!f) return 0;
    return f.wrap && line && !line.long ? 0 : f.left;
  }

  /** Row of a cell in a (wrapped) line. */
  private rowOf(line: FrameLine, cell: number): number {
    let r = 0;
    while (r < line.wraps.length && line.wraps[r] <= cell) r++;
    return r;
  }

  /** Position of a cell within its line box (x, and y of its row). */
  protected px(line: FrameLine, cell: number): { x: number; y: number } {
    const { cw, lh } = this.metrics();
    const r = this.rowOf(line, cell);
    const start = r ? line.wraps[r - 1] : 0;
    return { x: (cell - start - this.offset(line)) * cw, y: r * lh };
  }

  /** A cell range `[a, b)` as boxes, one per row. */
  protected segs(line: FrameLine, a: number, b: number): { x: number; y: number; w: number }[] {
    const cw = this.metrics().cw;
    const out: { x: number; y: number; w: number }[] = [];
    let from = a;
    while (from < b) {
      const r = this.rowOf(line, from);
      const end = Math.min(b, r < line.wraps.length ? line.wraps[r] : b);
      const p = this.px(line, from);
      out.push({ x: p.x, y: p.y, w: (end - from) * cw });
      from = end;
    }
    return out;
  }

  protected lineY(line: FrameLine): number {
    return line.row * this.metrics().lh;
  }

  protected lineHeight(line: FrameLine): number {
    return (line.wraps.length + 1) * this.metrics().lh;
  }

  /** Where caret `i` is drawn. */
  protected caretPx(i: number): { x: number; y: number } {
    const f = this.frame();
    const xy = f?.caretXy[i];
    if (!f || !xy) return { x: 0, y: 0 };
    const line = f.lines.find((l) => l.line === f.carets[i].line);
    const { cw, lh } = this.metrics();
    return { x: (xy.x - this.offset(line)) * cw, y: xy.row * lh };
  }

  protected isCurrent(line: FrameLine): boolean {
    const f = this.frame();
    return !!f && f.selChars === 0 && f.carets.some((c) => c.line === line.line);
  }

  private overviewKey = '';

  private overviewSoon(key: string, track: number): void {
    if (key === this.overviewKey) return;
    this.overviewKey = key;
    if (this.overviewTimer) clearTimeout(this.overviewTimer);
    if (!key) {
      this.overview.set([]);
      return;
    }
    const id = this.docId();
    this.overviewTimer = setTimeout(() => {
      this.api
        .overview(id, Math.max(1, Math.floor(track / 3)))
        .then((o) => {
          if (id === this.docId() && key === this.overviewKey) this.overview.set(o);
        })
        .catch(() => undefined);
    }, 300);
  }

  /** Wide characters are drawn one by one in boxes of two cells. */
  protected wideChars(run: Run): string[] {
    return Array.from(run.text);
  }

  // ---------------------------------------------------------------- command plumbing

  private send(cmd: Command): Promise<unknown> {
    const id = this.docId();
    this.chain = this.chain.then(async () => {
      try {
        const f = await this.api.command(id, cmd);
        if (id === this.docId()) {
          this.frame.set(f);
          this.frameChange.emit(f);
          if (f.approxHighlight) this.refreshSoon();
          if (f.notice) this.onNotice(f.notice);
        }
      } catch (e) {
        this.status.set(describeFsError(toFsError(e)));
      }
    });
    return this.chain;
  }

  private refreshTimer: ReturnType<typeof setTimeout> | null = null;

  /** Approximate highlighting: ask again until the background parse has caught up. */
  private refreshSoon(): void {
    if (this.refreshTimer) return;
    this.refreshTimer = setTimeout(() => {
      this.refreshTimer = null;
      void this.send({ type: 'frame' });
    }, 250);
  }

  private onNotice(notice: string): void {
    if (notice.startsWith('replaced:')) {
      this.status.set(FIND_TEXTS.replaced(Number(notice.slice(9))));
    } else if (notice === 'no_match') {
      this.findCount.set(FIND_TEXTS.noMatch);
      this.status.set(FIND_TEXTS.noMatch);
    } else if (notice.startsWith('trimmed:')) {
      this.status.set(this.texts.trimmed(Number(notice.slice(8))));
    } else if (notice === 'invalid_search') {
      this.findInvalid.set(true);
      this.findCount.set(FIND_TEXTS.invalid);
    } else {
      this.status.set(noticeText(notice));
    }
  }

  /** Scroll and drag events come faster than frames: one command per animation frame. */
  private schedule(): void {
    if (this.frameRequested) return;
    this.frameRequested = true;
    requestAnimationFrame(() => {
      this.frameRequested = false;
      if (this.pendingScroll) {
        const s = this.pendingScroll;
        this.pendingScroll = null;
        void this.send({ type: 'scroll', lines: s.lines, cells: s.cells });
      }
      if (this.pendingDrag) {
        const d = this.pendingDrag;
        this.pendingDrag = null;
        void this.send({ type: 'drag', line: d.line, cell: d.cell, column: d.column });
      }
    });
  }

  private measureFont(): void {
    const el = this.measure().nativeElement;
    const rect = el.getBoundingClientRect();
    const cw = rect.width / 64 || 8;
    const fontSize = parseFloat(getComputedStyle(el).fontSize) || 14;
    this.metrics.set({ cw, lh: Math.round(fontSize * 1.55) });
  }

  private onResize(): void {
    const r = this.host.nativeElement.getBoundingClientRect();
    this.size.set({ width: r.width, height: r.height });
  }

  private sendResize(): void {
    if (this.size().height === 0) return;
    void this.send({ type: 'resize', lines: this.viewLines(), cols: this.viewCols() });
  }

  // ---------------------------------------------------------------- keyboard & text input

  protected onInput(e: Event): void {
    if (this.composing()) return;
    const el = e.target as HTMLTextAreaElement;
    const text = el.value;
    el.value = '';
    if (text) void this.send({ type: 'type', text });
  }

  protected onCompositionStart(): void {
    this.composing.set(true);
  }

  protected onCompositionEnd(e: CompositionEvent): void {
    this.composing.set(false);
    const el = this.input().nativeElement;
    el.value = '';
    if (e.data) void this.send({ type: 'type', text: e.data });
  }

  /** Paste from a context menu: the text comes with the event. */
  protected onPaste(e: ClipboardEvent): void {
    e.preventDefault();
    const text = e.clipboardData?.getData('text/plain');
    if (text) void this.send({ type: 'insert', text });
  }

  protected onClipboard(e: ClipboardEvent, cut: boolean): void {
    e.preventDefault();
    void this.send({ type: cut ? 'cut' : 'copy' });
  }

  // ---------------------------------------------------------------- mouse

  /** A point as the engine wants it: top + rows down, cell (in the row when wrapped). */
  private pointAt(clientX: number, clientY: number): { line: number; cell: number } {
    const r = this.host.nativeElement.getBoundingClientRect();
    const f = this.frame();
    const { cw, lh } = this.metrics();
    const rows = (clientY - r.top) / lh;
    const at = f?.lines.find((l) => rows >= l.row && rows < l.row + l.wraps.length + 1);
    return {
      line: (f?.top ?? 0) + rows,
      cell: this.offset(at ?? f?.lines.at(-1)) + (clientX - r.left - this.textLeft()) / cw,
    };
  }

  protected onTextMousedown(e: MouseEvent, gutter = false): void {
    if (e.button !== 0) return;
    e.preventDefault();
    this.focus();
    const p = this.pointAt(e.clientX, e.clientY);
    const count = gutter ? 3 : Math.min(3, Math.max(1, e.detail));
    void this.send({
      type: 'click',
      line: p.line,
      cell: p.cell,
      count,
      extend: e.shiftKey,
      add: e.altKey && !gutter,
    });
    this.startDrag(e);
  }

  private startDrag(e: MouseEvent): void {
    this.stopDrag();
    this.dragging = { x: e.clientX, y: e.clientY, column: e.altKey };
    const move = (ev: MouseEvent) => {
      if (!this.dragging) return;
      this.dragging.x = ev.clientX;
      this.dragging.y = ev.clientY;
      this.dragging.column = ev.altKey;
      this.queueDrag();
    };
    const up = () => {
      this.stopDrag();
      document.removeEventListener('mousemove', move);
      document.removeEventListener('mouseup', up);
    };
    document.addEventListener('mousemove', move);
    document.addEventListener('mouseup', up);
    // outside the text: keep extending (and scrolling) while the mouse stays there
    this.dragging.autoscroll = setInterval(() => {
      const d = this.dragging;
      if (!d) return;
      const r = this.host.nativeElement.getBoundingClientRect();
      if (d.y < r.top || d.y > r.bottom || d.x < r.left + this.textLeft() || d.x > r.right) {
        this.queueDrag();
      }
    }, 50);
  }

  private queueDrag(): void {
    if (!this.dragging) return;
    this.pendingDrag = {
      ...this.pointAt(this.dragging.x, this.dragging.y),
      column: this.dragging.column,
    };
    this.schedule();
  }

  private stopDrag(): void {
    if (this.dragging?.autoscroll) clearInterval(this.dragging.autoscroll);
    this.dragging = null;
  }

  protected onWheel(e: WheelEvent): void {
    e.preventDefault();
    const { cw, lh } = this.metrics();
    const unit = e.deltaMode === 1 ? lh : e.deltaMode === 2 ? this.size().height : 1;
    let dx = e.deltaX * unit;
    let dy = e.deltaY * unit;
    if (e.shiftKey && dx === 0) {
      dx = dy;
      dy = 0;
    }
    const s = this.pendingScroll ?? { lines: 0, cells: 0 };
    s.lines += dy / lh;
    s.cells += dx / cw;
    this.pendingScroll = s;
    this.schedule();
  }

  // ---------------------------------------------------------------- scrollbars

  protected onThumbDown(e: MouseEvent, vertical: boolean): void {
    if (e.button !== 0) return;
    e.preventDefault();
    e.stopPropagation();
    const bar = vertical ? this.vbar() : this.hbar();
    const f = this.frame();
    if (!bar || !f) return;
    const start = vertical ? e.clientY : e.clientX;
    const startValue = vertical ? f.top : f.left;
    const perPx = bar.range / Math.max(1, bar.track - bar.thumb);
    const move = (ev: MouseEvent) => {
      const v = startValue + ((vertical ? ev.clientY : ev.clientX) - start) * perPx;
      void this.send(vertical ? { type: 'scrollTo', top: v } : { type: 'scrollTo', left: v });
    };
    const up = () => {
      document.removeEventListener('mousemove', move);
      document.removeEventListener('mouseup', up);
      this.focus();
    };
    document.addEventListener('mousemove', move);
    document.addEventListener('mouseup', up);
  }

  /** A click on the track jumps there. */
  protected onTrackDown(e: MouseEvent, vertical: boolean): void {
    if (e.button !== 0 || e.target !== e.currentTarget) return;
    e.preventDefault();
    const bar = vertical ? this.vbar() : this.hbar();
    if (!bar) return;
    const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
    const px = (vertical ? e.clientY - r.top : e.clientX - r.left) - bar.thumb / 2;
    const v = (px / Math.max(1, bar.track - bar.thumb)) * bar.range;
    void this.send(vertical ? { type: 'scrollTo', top: v } : { type: 'scrollTo', left: v });
    this.focus();
  }
}
