import {
  ChangeDetectionStrategy,
  Component,
  effect,
  ElementRef,
  inject,
  input,
  output,
  untracked,
  viewChild,
} from '@angular/core';
import { FIND_TEXTS } from './find-texts';
import { FindStore, stepHistory } from './find.store';

/**
 * The quiet find/replace bar at the top right of the editor (design 006 E5). It only edits the
 * {@link FindStore}; the editor turns its events into engine commands. Keys in the fields:
 * Enter next, Shift+Enter previous, Alt+C/W/R options, ↑/↓ history, Escape closes.
 */
@Component({
  selector: 'app-find-bar',
  template: `
    <div class="bar" role="search" [attr.aria-label]="t.find">
      <div class="row">
        <button
          type="button"
          class="icon expand"
          [attr.aria-expanded]="store.replaceOpen()"
          [attr.aria-label]="t.toggleReplace"
          [title]="t.toggleReplace"
          (click)="store.replaceOpen.set(!store.replaceOpen())"
        >
          {{ store.replaceOpen() ? '▾' : '▸' }}
        </button>
        <input
          #findInput
          class="input field"
          type="text"
          spellcheck="false"
          autocomplete="off"
          [attr.aria-label]="t.find"
          [placeholder]="t.find"
          [value]="store.pattern()"
          [attr.aria-invalid]="invalid() ? 'true' : null"
          (input)="onPattern($event)"
          (keydown)="onFindKey($event)"
        />
        <button
          type="button"
          class="icon opt"
          [attr.aria-pressed]="store.caseSensitive()"
          [attr.aria-label]="t.caseSensitive"
          [title]="t.caseSensitive + ' (Alt+C)'"
          (click)="toggle('case')"
        >
          Aa
        </button>
        <button
          type="button"
          class="icon opt"
          [attr.aria-pressed]="store.wholeWord()"
          [attr.aria-label]="t.wholeWord"
          [title]="t.wholeWord + ' (Alt+W)'"
          (click)="toggle('word')"
        >
          <u>ab</u>
        </button>
        <button
          type="button"
          class="icon opt"
          [attr.aria-pressed]="store.regex()"
          [attr.aria-label]="t.regex"
          [title]="t.regex + ' (Alt+R)'"
          (click)="toggle('regex')"
        >
          .*
        </button>
        <span class="count" aria-live="polite">{{ count() }}</span>
        <button
          type="button"
          class="icon"
          [attr.aria-label]="t.prev"
          [title]="t.prev"
          (click)="findStep.emit({ backwards: true })"
        >
          ↑
        </button>
        <button
          type="button"
          class="icon"
          [attr.aria-label]="t.next"
          [title]="t.next"
          (click)="findStep.emit({ backwards: false })"
        >
          ↓
        </button>
        <button
          type="button"
          class="icon"
          [attr.aria-label]="t.close"
          [title]="t.close + ' (Esc)'"
          (click)="closed.emit()"
        >
          ×
        </button>
      </div>
      @if (store.replaceOpen()) {
        <div class="row">
          <span class="spacer"></span>
          <input
            #replaceInput
            class="input field"
            type="text"
            spellcheck="false"
            autocomplete="off"
            [attr.aria-label]="t.replace"
            [placeholder]="t.replace"
            [value]="store.replacement()"
            (input)="store.replacement.set($any($event.target).value)"
            (keydown)="onReplaceKey($event)"
          />
          <button type="button" class="btn small" (click)="replace.emit()">
            {{ t.replaceOne }}
          </button>
          <button type="button" class="btn small" (click)="replaceAll.emit()">
            {{ t.replaceAll }}
          </button>
        </div>
      }
    </div>
  `,
  styleUrl: './find-bar.css',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class FindBar {
  protected readonly store = inject(FindStore);
  protected readonly t = FIND_TEXTS;
  /** "3 of 12", "No matches" … */
  readonly count = input('');
  readonly invalid = input(false);
  /** Find the next/previous match; `incremental` while typing. */
  readonly findStep = output<{ backwards: boolean; incremental?: boolean }>();
  readonly replace = output<void>();
  readonly replaceAll = output<void>();
  readonly closed = output<void>();
  private readonly findInput = viewChild.required<ElementRef<HTMLInputElement>>('findInput');
  private readonly replaceInput = viewChild<ElementRef<HTMLInputElement>>('replaceInput');

  constructor() {
    effect(() => {
      this.store.focusRequest();
      const replace = this.store.replaceOpen();
      untracked(() =>
        setTimeout(() => {
          const el =
            (replace && this.store.pattern() ? this.replaceInput() : null) ?? this.findInput();
          el?.nativeElement.focus();
          el?.nativeElement.select();
        }),
      );
    });
  }

  protected onPattern(e: Event): void {
    this.store.pattern.set((e.target as HTMLInputElement).value);
    this.findStep.emit({ backwards: false, incremental: true });
  }

  protected toggle(which: 'case' | 'word' | 'regex'): void {
    const s = this.store;
    const sig = which === 'case' ? s.caseSensitive : which === 'word' ? s.wholeWord : s.regex;
    sig.set(!sig());
    this.findStep.emit({ backwards: false, incremental: true });
  }

  /** Option keys shared by both fields; `true` = handled. */
  private optionKey(e: KeyboardEvent): boolean {
    if (!e.altKey || e.ctrlKey || e.metaKey) return false;
    const k = e.code;
    if (k === 'KeyC') this.toggle('case');
    else if (k === 'KeyW') this.toggle('word');
    else if (k === 'KeyR') this.toggle('regex');
    else return false;
    return true;
  }

  protected onFindKey(e: KeyboardEvent): void {
    if (this.optionKey(e)) {
      e.preventDefault();
      return;
    }
    switch (e.key) {
      case 'Enter':
        this.store.remember();
        this.findStep.emit({ backwards: e.shiftKey });
        break;
      case 'Escape':
        this.closed.emit();
        break;
      case 'ArrowUp':
      case 'ArrowDown': {
        const v = stepHistory(this.store.findHistory(), this.store.pattern(), e.key === 'ArrowUp');
        if (v === null) return;
        this.store.pattern.set(v);
        this.findStep.emit({ backwards: false, incremental: true });
        break;
      }
      default:
        return;
    }
    e.preventDefault();
    e.stopPropagation();
  }

  protected onReplaceKey(e: KeyboardEvent): void {
    if (this.optionKey(e)) {
      e.preventDefault();
      return;
    }
    switch (e.key) {
      case 'Enter':
        this.store.rememberReplacement();
        if (e.ctrlKey || e.metaKey) this.replaceAll.emit();
        else this.replace.emit();
        break;
      case 'Escape':
        this.closed.emit();
        break;
      case 'ArrowUp':
      case 'ArrowDown': {
        const v = stepHistory(
          this.store.replaceHistory(),
          this.store.replacement(),
          e.key === 'ArrowUp',
        );
        if (v === null) return;
        this.store.replacement.set(v);
        break;
      }
      default:
        return;
    }
    e.preventDefault();
    e.stopPropagation();
  }
}
