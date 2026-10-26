import { ChangeDetectionStrategy, Component, computed, inject, input } from '@angular/core';
import { ActionService } from '../../core/actions/action.service';
import { Frame } from '../../core/editor/editor.models';
import { locale } from '../../core/i18n/i18n';
import { RecordingStore } from '../recording/recording.store';
import { RECORDING_TEXTS } from '../recording/recording-texts';
import { EDITOR_TEXTS } from './editor-texts';

/**
 * The quiet status bar (design 010 E1): position and selection on the left, file facts on the
 * right. Language, encoding and line endings are buttons – they run the same actions as the
 * palette.
 */
@Component({
  selector: 'app-status-bar',
  template: `
    @if (frame(); as f) {
      <span class="item">{{ position() }}</span>
      @if (selection(); as s) {
        <span class="item">{{ s }}</span>
      }
      @if (f.overwrite) {
        <span class="item strong">{{ t.overwrite }}</span>
      }
      @if (recording()) {
        <span class="item rec" [title]="rt.recTitle">● {{ rt.rec }}</span>
      }
      @if (playback(); as p) {
        <span class="item strong" [title]="rt.playTitle">{{ p }}</span>
      }
      <span class="fill"></span>
      @if (f.wrap) {
        <span class="item">{{ longUnwrapped() ? t.wrap + ' – ' + t.longUnwrapped : t.wrap }}</span>
      }
      <span class="item">{{ t.lines(count()) }}</span>
      @if (f.info.fileSize > 0) {
        <span class="item">{{ size() }}</span>
      }
      <button
        type="button"
        class="item btn-link"
        [title]="t.changeLanguage"
        (click)="run('view.language')"
      >
        {{ f.language ?? t.plainText }}
      </button>
      <button
        type="button"
        class="item btn-link"
        [title]="t.changeEncoding"
        (click)="run('file.encoding')"
      >
        {{ f.info.encoding }}{{ f.info.bom ? ' ' + t.bom : '' }}
      </button>
      <button
        type="button"
        class="item btn-link"
        [title]="t.changeLineEndings"
        (click)="run('file.lineEnding')"
      >
        {{ f.info.mixedLineEndings ? t.mixed : f.info.lineEnding.toUpperCase() }}
      </button>
    }
  `,
  styleUrl: './status-bar.css',
  changeDetection: ChangeDetectionStrategy.OnPush,
  host: { role: 'status', '[attr.aria-label]': 't.status' },
})
export class StatusBar {
  readonly frame = input<Frame | null>(null);
  protected readonly t = EDITOR_TEXTS;
  protected readonly rt = RECORDING_TEXTS;
  private readonly actions = inject(ActionService);
  private readonly rec = inject(RecordingStore);

  /** "● REC" while this document records (design 015 E11). */
  protected readonly recording = computed(() => {
    const f = this.frame();
    return !!f && this.rec.recording().has(f.info.id);
  });

  /** "▶ 37 %" / "❚❚ 37 %" while a playback types here. */
  protected readonly playback = computed(() => {
    const f = this.frame();
    const p = this.rec.playback();
    if (!f || !p || p.docId !== f.info.id) return '';
    const pct = p.total ? Math.floor((p.done * 100) / p.total) : 0;
    return `${p.paused ? '❚❚' : '▶'} ${this.rt.progress(pct)}`;
  });

  protected readonly position = computed(() => {
    const f = this.frame();
    if (!f) return '';
    const c = f.carets[f.primary];
    return this.t.position(f.caretLine + 1, (c?.cell ?? 0) + 1);
  });

  protected readonly selection = computed(() => {
    const f = this.frame();
    if (!f) return '';
    if (f.carets.length > 1) return this.t.carets(f.carets.length);
    return f.selChars > 0 ? this.t.selected(f.selChars, f.selLines) : '';
  });

  protected readonly count = computed(() =>
    (this.frame()?.info.lineCount ?? 0).toLocaleString(locale()),
  );

  protected readonly size = computed(() => formatSize(this.frame()?.info.fileSize ?? 0));

  protected readonly longUnwrapped = computed(() => !!this.frame()?.lines.some((l) => l.long));

  protected run(id: string): void {
    this.actions.run(id);
  }
}

/** 1.2 KB, 34 MB … (decimal, like the file managers). */
export function formatSize(bytes: number): string {
  const units = ['B', 'KB', 'MB', 'GB'];
  let v = bytes;
  let i = 0;
  while (v >= 1000 && i < units.length - 1) {
    v /= 1000;
    i++;
  }
  const n = i === 0 || v >= 100 ? Math.round(v) : Math.round(v * 10) / 10;
  return `${n.toLocaleString(locale())} ${units[i]}`;
}
