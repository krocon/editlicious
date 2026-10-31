import {
  afterRenderEffect,
  ChangeDetectionStrategy,
  Component,
  ElementRef,
  inject,
  signal,
  viewChild,
} from '@angular/core';
import { describeFsError, toFsError } from '../../core/errors/fs-error';
import { WorkspaceApi } from '../../core/workspace/workspace-api';
import { PathEntry } from '../../core/workspace/workspace.models';
import { DialogService } from '../../shared/dialog/dialog.service';
import { Modal } from '../../shared/dialog/modal';
import { DOC_TEXTS } from './documents-texts';
import { DocumentsStore } from './documents.store';

/**
 * "Open path…" (prompt 024): type or paste a path – `~/Library/…/x.json`, a quoted path, a
 * `file://` URL, a name next to the active file – instead of clicking through the native
 * dialog. The folder entries matching what was typed so far are listed below (APG combobox +
 * listbox); Tab completes like a shell. Rust expands and checks the path (paths are opaque here).
 */
@Component({
  selector: 'app-open-path-dialog',
  imports: [Modal],
  template: `
    <app-modal [heading]="t.openPathTitle" width="40rem" (closed)="close()">
      <input
        #field
        class="input path"
        type="text"
        role="combobox"
        aria-autocomplete="list"
        aria-controls="open-path-list"
        aria-describedby="open-path-hint open-path-error"
        spellcheck="false"
        autocomplete="off"
        autocapitalize="off"
        data-autofocus
        [attr.aria-expanded]="entries().length > 0"
        [attr.aria-label]="t.openPathTitle"
        [attr.aria-activedescendant]="active() >= 0 ? 'open-path-' + active() : null"
        [attr.aria-invalid]="error() ? true : null"
        [placeholder]="t.openPathPlaceholder"
        (input)="onInput()"
        (keydown)="onKey($event)"
      />
      <ul id="open-path-list" role="listbox" class="list" [attr.aria-label]="t.openPathEntries">
        @for (e of entries(); track e.value; let i = $index) {
          <!-- eslint-disable-next-line @angular-eslint/template/click-events-have-key-events, @angular-eslint/template/interactive-supports-focus -->
          <li
            role="option"
            [id]="'open-path-' + i"
            [attr.aria-selected]="i === active()"
            [class.active]="i === active()"
            [class.dir]="e.dir"
            (pointerdown)="$event.preventDefault()"
            (click)="choose(e)"
          >
            {{ e.label }}
          </li>
        }
        @if (more()) {
          <li class="more" aria-disabled="true">{{ t.openPathMore }}</li>
        }
      </ul>
      <p id="open-path-error" class="error" role="alert">{{ error() }}</p>
      <div class="actions">
        <span id="open-path-hint" class="hint">{{ t.openPathHint }}</span>
        <button type="button" class="btn primary" [disabled]="busy()" (click)="open()">
          {{ t.openPathOpen }}
        </button>
      </div>
    </app-modal>
  `,
  styles: `
    .path {
      width: 100%;
      font-family: var(--font-editor);
    }
    .list {
      max-height: 40vh;
      margin: var(--space-2) 0 0;
      padding: 0;
      overflow: auto;
      list-style: none;
      font-family: var(--font-editor);
      &:empty {
        display: none;
      }
    }
    li {
      padding: 2px var(--space-2);
      border-radius: 4px;
      white-space: pre;
      overflow: hidden;
      text-overflow: ellipsis;
      cursor: default;
      &.dir {
        font-weight: 600;
      }
      &.active {
        color: var(--color-accent-text);
        background: var(--color-accent);
      }
      &.more {
        color: var(--color-text-muted);
        font-family: var(--font-ui);
        font-style: italic;
      }
    }
    .error {
      margin: var(--space-2) 0 0;
      color: var(--color-error);
      overflow-wrap: anywhere;
      &:empty {
        display: none;
      }
    }
    .actions {
      display: flex;
      align-items: center;
      justify-content: space-between;
      gap: var(--space-2);
      margin-top: var(--space-3);
    }
    .hint {
      color: var(--color-text-muted);
      font-size: var(--font-size-small);
    }
  `,
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class OpenPathDialog {
  protected readonly t = DOC_TEXTS;
  private readonly ws = inject(WorkspaceApi);
  private readonly store = inject(DocumentsStore);
  private readonly dialogs = inject(DialogService);
  private readonly field = viewChild.required<ElementRef<HTMLInputElement>>('field');
  protected readonly entries = signal<PathEntry[]>([]);
  protected readonly more = signal(false);
  /** The chosen entry; `-1` = none (Enter opens the typed text). */
  protected readonly active = signal(-1);
  protected readonly error = signal('');
  protected readonly busy = signal(false);
  private common: string | null = null;
  /** Answers to older inputs are dropped (CLAUDE.md §4). */
  private seq = 0;
  private readonly near = this.store.active()?.info.path ?? null;

  constructor() {
    void this.start();
    const host = inject<ElementRef<HTMLElement>>(ElementRef).nativeElement;
    afterRenderEffect(() => {
      const i = this.active();
      if (i >= 0) host.querySelector(`#open-path-${i}`)?.scrollIntoView?.({ block: 'nearest' });
    });
  }

  private async start(): Promise<void> {
    const text = await this.ws.openPathStart(this.near).catch(() => '');
    const el = this.field().nativeElement;
    if (el.value) return; // the user was faster
    el.value = text;
    el.select(); // typing or pasting replaces the suggested folder
    await this.complete();
  }

  protected onInput(): void {
    this.error.set('');
    void this.complete();
  }

  private async complete(): Promise<void> {
    const seq = ++this.seq;
    const text = this.field().nativeElement.value;
    const c = await this.ws
      .completeOpenPath(text, this.near)
      .catch(() => ({ entries: [], common: null, more: false }));
    if (seq !== this.seq) return;
    this.entries.set(c.entries);
    this.more.set(c.more);
    this.common = c.common;
    this.active.set(-1);
  }

  /** Puts a text in the field (caret at the end) and lists what matches it. */
  private setText(text: string): Promise<void> {
    const el = this.field().nativeElement;
    el.value = text;
    el.setSelectionRange(text.length, text.length);
    this.error.set('');
    return this.complete();
  }

  protected onKey(e: KeyboardEvent): void {
    const n = this.entries().length;
    switch (e.key) {
      case 'ArrowDown':
        if (n) this.active.update((i) => (i + 1) % n);
        break;
      case 'ArrowUp':
        if (n) this.active.update((i) => (i <= 0 ? n - 1 : i - 1));
        break;
      case 'Tab': {
        if (e.shiftKey) return; // leave the field backwards
        const chosen = this.entries()[this.active()];
        const next = chosen?.value ?? this.common ?? (n === 1 ? this.entries()[0].value : null);
        if (next === null) return; // nothing to complete: Tab moves on
        e.stopPropagation(); // not the focus trap of the modal
        void this.setText(next);
        break;
      }
      case 'Enter': {
        const chosen = this.entries()[this.active()];
        if (chosen) this.choose(chosen);
        else void this.open();
        break;
      }
      default:
        return;
    }
    e.preventDefault();
  }

  /** A folder goes into the field, a file opens. */
  protected choose(e: PathEntry): void {
    if (e.dir) void this.setText(e.value);
    else void this.open(e.value);
  }

  protected async open(text = this.field().nativeElement.value): Promise<void> {
    if (this.busy()) return;
    this.busy.set(true);
    try {
      const path = await this.ws.resolveOpenPath(text, this.near);
      this.dialogs.close();
      await this.store.openPaths([path]);
    } catch (err) {
      this.error.set(describeFsError(toFsError(err)));
      this.field().nativeElement.focus();
    } finally {
      this.busy.set(false);
    }
  }

  protected close(): void {
    this.dialogs.close();
  }
}
