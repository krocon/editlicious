import {
  afterRenderEffect,
  ChangeDetectionStrategy,
  Component,
  computed,
  ElementRef,
  inject,
  signal,
} from '@angular/core';
import { Modal } from '../dialog/modal';
import { PickService } from './pick.service';

const key = (s: string) => s.normalize('NFC').toLowerCase();

/** The list of the {@link PickService}: type to filter, ↑/↓, Enter (APG combobox + listbox). */
@Component({
  selector: 'app-pick-dialog',
  imports: [Modal],
  template: `
    @if (svc.request(); as r) {
      <app-modal [heading]="r.title" width="28rem" (closed)="svc.answer(null)">
        <input
          class="input"
          type="text"
          role="combobox"
          aria-autocomplete="list"
          aria-expanded="true"
          aria-controls="pick-list"
          spellcheck="false"
          autocomplete="off"
          data-autofocus
          [attr.aria-label]="r.title"
          [attr.aria-activedescendant]="items().length ? 'pick-' + active() : null"
          [placeholder]="r.placeholder"
          (input)="onInput($event)"
          (keydown)="onKey($event)"
        />
        <ul id="pick-list" role="listbox" class="list" [attr.aria-label]="r.title">
          @for (o of items(); track o.id; let i = $index) {
            <!-- eslint-disable-next-line @angular-eslint/template/click-events-have-key-events, @angular-eslint/template/interactive-supports-focus -->
            <li
              role="option"
              [id]="'pick-' + i"
              [attr.aria-selected]="i === active()"
              [class.active]="i === active()"
              [class.current]="o.id === r.current"
              (pointerdown)="$event.preventDefault()"
              (click)="svc.answer(o.id)"
            >
              {{ o.label }}
            </li>
          }
        </ul>
      </app-modal>
    }
  `,
  styles: `
    .list {
      max-height: 50vh;
      margin: 0;
      padding: 0;
      overflow: auto;
      list-style: none;
    }
    li {
      padding: 4px var(--space-2);
      border-radius: 4px;
      cursor: default;
      &.active {
        color: var(--color-accent-text);
        background: var(--color-accent);
      }
      &.current:not(.active) {
        font-weight: 600;
      }
    }
  `,
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class PickDialog {
  protected readonly svc = inject(PickService);
  private readonly query = signal('');
  protected readonly active = signal(0);
  protected readonly items = computed(() => {
    const r = this.svc.request();
    if (!r) return [];
    const words = key(this.query()).split(/\s+/).filter(Boolean);
    return r.options.filter((o) => words.every((w) => key(o.label).includes(w)));
  });

  constructor() {
    const host = inject<ElementRef<HTMLElement>>(ElementRef).nativeElement;
    const r = this.svc.request();
    const i = r?.options.findIndex((o) => o.id === r.current) ?? -1;
    this.active.set(Math.max(0, i));
    afterRenderEffect(() => {
      host.querySelector(`#pick-${this.active()}`)?.scrollIntoView?.({ block: 'nearest' });
    });
  }

  protected onInput(e: Event): void {
    this.query.set((e.target as HTMLInputElement).value);
    this.active.set(0);
  }

  protected onKey(e: KeyboardEvent): void {
    const n = this.items().length;
    switch (e.key) {
      case 'ArrowDown':
        if (n) this.active.update((i) => (i + 1) % n);
        break;
      case 'ArrowUp':
        if (n) this.active.update((i) => (i - 1 + n) % n);
        break;
      case 'Enter': {
        const o = this.items()[this.active()];
        if (o) this.svc.answer(o.id);
        break;
      }
      default:
        return;
    }
    e.preventDefault();
  }
}
