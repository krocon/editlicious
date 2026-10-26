import {
  afterNextRender,
  ChangeDetectionStrategy,
  Component,
  computed,
  DestroyRef,
  effect,
  ElementRef,
  inject,
  input,
  signal,
  untracked,
  viewChild,
} from '@angular/core';
import { EditorApi } from '../../core/editor/editor-api';
import { Command, Frame, FrameLine, Run } from '../../core/editor/editor.models';
import { describeFsError, toFsError } from '../../core/errors/fs-error';
import { detectOs, keyToCommand } from './key-commands';
import { EDITOR_TEXTS } from './editor-texts';

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
  templateUrl: './editor-view.html',
  styleUrl: './editor-view.css',
  changeDetection: ChangeDetectionStrategy.OnPush,
  host: {
    '[class.focused]': 'focused()',
    '(wheel)': 'onWheel($event)',
  },
})
export class EditorView {
  /** The engine session to show. */
  readonly docId = input.required<number>();

  protected readonly texts = EDITOR_TEXTS;
  protected readonly frame = signal<Frame | null>(null);
  protected readonly metrics = signal<Metrics>({ cw: 8, lh: 22 });
  protected readonly size = signal({ width: 0, height: 0 });
  protected readonly focused = signal(false);
  protected readonly composing = signal(false);
  protected readonly status = signal('');

  private readonly api = inject(EditorApi);
  private readonly host = inject<ElementRef<HTMLElement>>(ElementRef);
  private readonly input = viewChild.required<ElementRef<HTMLTextAreaElement>>('input');
  private readonly measure = viewChild.required<ElementRef<HTMLElement>>('measure');
  private readonly os = detectOs();
  private readonly destroyRef = inject(DestroyRef);

  private chain: Promise<unknown> = Promise.resolve();
  private pendingScroll: { lines: number; cells: number } | null = null;
  private pendingDrag: { line: number; cell: number } | null = null;
  private frameRequested = false;
  private dragging: { autoscroll?: ReturnType<typeof setInterval>; x: number; y: number } | null =
    null;

  // ---------------------------------------------------------------- geometry

  protected readonly gutterWidth = computed(() => {
    const lines = this.frame()?.info.lineCount ?? 1;
    const digits = Math.max(3, String(lines).length);
    return (digits + 2) * this.metrics().cw;
  });

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
    const caretCell = f?.carets[0]?.cell ?? 0;
    const extent = Math.max(f?.maxWidth ?? 0, caretCell + 1);
    if (!f || extent <= cols - 1) return null;
    const range = Math.max(1, extent - cols + 2);
    const thumb = Math.max(MIN_THUMB, (w * cols) / (extent + 2));
    return { thumb, pos: ((w - thumb) * Math.min(f.left, range)) / range, track: w, range };
  });

  /** Primary caret in px – the textarea sits there (IME window near the caret). */
  protected readonly caretXY = computed(() => {
    const f = this.frame();
    const c = f?.carets[0];
    if (!f || !c) return { x: 0, y: 0 };
    return { x: this.x(c.cell), y: this.y(c.line) };
  });

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
    this.destroyRef.onDestroy(() => this.stopDrag());
  }

  focus(): void {
    this.input().nativeElement.focus({ preventScroll: true });
  }

  /** Runs a command from outside (menus, actions in prompt 005). */
  run(cmd: Command): Promise<unknown> {
    return this.send(cmd);
  }

  // ---------------------------------------------------------------- rendering helpers

  protected x(cell: number): number {
    const f = this.frame();
    return (cell - (f?.left ?? 0)) * this.metrics().cw;
  }

  protected y(line: number): number {
    const f = this.frame();
    return (line - (f?.top ?? 0)) * this.metrics().lh;
  }

  protected isCurrent(line: FrameLine): boolean {
    const f = this.frame();
    return !!f && f.selChars === 0 && f.carets.some((c) => c.line === line.line);
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
          if (f.notice) this.status.set(this.texts.notice(f.notice));
        }
      } catch (e) {
        this.status.set(describeFsError(toFsError(e)));
      }
    });
    return this.chain;
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
        void this.send({ type: 'drag', line: d.line, cell: d.cell });
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

  protected onKeydown(e: KeyboardEvent): void {
    if (this.composing() || e.isComposing) return;
    const cmd = keyToCommand(e, this.os);
    if (!cmd) return;
    e.preventDefault();
    void this.send(cmd);
  }

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

  private pointAt(clientX: number, clientY: number): { line: number; cell: number } {
    const r = this.host.nativeElement.getBoundingClientRect();
    const f = this.frame();
    const { cw, lh } = this.metrics();
    return {
      line: (f?.top ?? 0) + (clientY - r.top) / lh,
      cell: (f?.left ?? 0) + (clientX - r.left - this.textLeft()) / cw,
    };
  }

  protected onTextMousedown(e: MouseEvent, gutter = false): void {
    if (e.button !== 0) return;
    e.preventDefault();
    this.focus();
    const p = this.pointAt(e.clientX, e.clientY);
    const count = gutter ? 3 : Math.min(3, Math.max(1, e.detail));
    void this.send({ type: 'click', line: p.line, cell: p.cell, count, extend: e.shiftKey });
    this.startDrag(e);
  }

  private startDrag(e: MouseEvent): void {
    this.stopDrag();
    this.dragging = { x: e.clientX, y: e.clientY };
    const move = (ev: MouseEvent) => {
      if (!this.dragging) return;
      this.dragging.x = ev.clientX;
      this.dragging.y = ev.clientY;
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
    this.pendingDrag = this.pointAt(this.dragging.x, this.dragging.y);
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
