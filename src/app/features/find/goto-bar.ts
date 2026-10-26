import {
  afterNextRender,
  ChangeDetectionStrategy,
  Component,
  ElementRef,
  output,
  viewChild,
} from '@angular/core';
import { FIND_TEXTS } from './find-texts';

/** `"120"`, `"120:5"`, `"120,5"` → 0-based line and character; `null` = not a position. */
export function parseGoto(text: string): { line: number; ch: number } | null {
  const m = /^\s*(\d+)\s*(?:[:,]\s*(\d+))?\s*$/.exec(text);
  if (!m) return null;
  const line = Math.max(1, Number(m[1])) - 1;
  const ch = m[2] ? Math.max(1, Number(m[2])) - 1 : 0;
  return { line, ch };
}

/** Go to line (design 006 E7): a small field at the top of the editor; Enter jumps. */
@Component({
  selector: 'app-goto-bar',
  template: `
    <div class="bar">
      <label class="label" for="goto-input">{{ t.gotoLabel }}</label>
      <input
        #field
        id="goto-input"
        class="input"
        type="text"
        inputmode="numeric"
        autocomplete="off"
        [placeholder]="t.gotoPlaceholder"
        (keydown)="onKey($event)"
      />
    </div>
  `,
  styles: `
    :host {
      position: absolute;
      z-index: 5;
      top: 6px;
      left: 50%;
      transform: translateX(-50%);
      font-family: var(--font-ui);
      font-size: var(--font-size-small);
      line-height: normal;
    }
    .bar {
      display: flex;
      align-items: center;
      gap: var(--space-2);
      padding: var(--space-1) var(--space-2);
      background: var(--color-surface);
      border: 1px solid var(--color-border);
      border-radius: var(--radius);
      box-shadow: var(--shadow-popup);
    }
    .input {
      width: 10rem;
      height: 26px;
    }
  `,
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class GotoBar {
  protected readonly t = FIND_TEXTS;
  readonly go = output<{ line: number; ch: number }>();
  readonly closed = output<void>();
  private readonly field = viewChild.required<ElementRef<HTMLInputElement>>('field');

  constructor() {
    afterNextRender(() => this.field().nativeElement.focus());
  }

  protected onKey(e: KeyboardEvent): void {
    if (e.key === 'Escape') {
      e.preventDefault();
      this.closed.emit();
    } else if (e.key === 'Enter') {
      e.preventDefault();
      const p = parseGoto(this.field().nativeElement.value);
      if (p) this.go.emit(p);
      else this.closed.emit();
    }
  }
}
