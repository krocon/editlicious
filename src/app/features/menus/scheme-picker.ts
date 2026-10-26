import { NgTemplateOutlet } from '@angular/common';
import { ChangeDetectionStrategy, Component, inject } from '@angular/core';
import { ActionService } from '../../core/actions/action.service';
import { DialogService } from '../../shared/dialog/dialog.service';
import { Modal } from '../../shared/dialog/modal';
import { ChromeStore, previewAttributes, Scheme, SCHEMES } from './chrome.store';
import { MENUS_TEXTS } from './menus-texts';

/** Thumbnails per row (arrow up/down jump by a row). */
const COLUMNS = 4;

/**
 * The colour schemes as thumbnails, light to dark (prompt 021): an APG radio group – arrows
 * try a scheme right away, Enter or OK keeps it, Esc goes back to the one before. Each thumbnail
 * draws its scheme with `[data-preview]` from `styles.css`, the one place for colours.
 */
@Component({
  selector: 'app-scheme-picker',
  imports: [Modal, NgTemplateOutlet],
  template: `
    <app-modal [heading]="t.schemeTitle" width="46rem" (closed)="cancel()">
      <div class="grid" role="radiogroup" [attr.aria-label]="t.schemeTitle">
        @for (s of schemes; track s) {
          <div
            class="thumb"
            role="radio"
            [attr.aria-checked]="s === chrome.scheme()"
            [tabindex]="s === chrome.scheme() ? 0 : -1"
            [attr.data-autofocus]="s === chrome.scheme() ? '' : null"
            [attr.data-scheme-id]="s"
            (click)="choose(s, $event)"
            (keydown)="onKey($event)"
            (dblclick)="keep()"
          >
            @if (s === 'auto') {
              <span class="split" aria-hidden="true">
                <span class="preview" data-preview>
                  <ng-container *ngTemplateOutlet="mini" />
                </span>
                <span class="preview" data-preview data-dark>
                  <ng-container *ngTemplateOutlet="mini" />
                </span>
              </span>
            } @else {
              <span
                class="preview"
                aria-hidden="true"
                [attr.data-preview]="preview(s).preview"
                [attr.data-dark]="preview(s).dark"
              >
                <ng-container *ngTemplateOutlet="mini" />
              </span>
            }
            <span class="name">{{ label(s) }}</span>
          </div>
        }
      </div>
      <ng-template #mini>
        <span class="gutter"><span>1</span><span>2</span><span class="on">3</span></span>
        <span class="code">
          <span
            ><i class="kw">fn</i>&ngsp;<i class="fn">main</i><i class="pu">() {{ '{' }}</i></span
          >
          <span class="sel"
            >&nbsp;&nbsp;<i class="kw">let</i> n = <i class="nu">42</i><i class="pu">;</i></span
          >
          <span class="cur"
            >&nbsp;&nbsp;<i class="st">"hi"</i><b class="caret"></b>&ngsp;<i class="co"
              >// ok</i
            ></span
          >
        </span>
      </ng-template>
      <p class="hint">{{ t.schemeHint }}</p>
      <div class="actions">
        <button type="button" class="btn" (click)="cancel()">{{ t.cancel }}</button>
        <button type="button" class="btn primary" (click)="keep()">{{ t.ok }}</button>
      </div>
    </app-modal>
  `,
  styles: `
    .grid {
      display: grid;
      grid-template-columns: repeat(4, 1fr);
      gap: var(--space-3);
      overflow: auto;
      padding: 2px;
    }
    .thumb {
      display: flex;
      flex-direction: column;
      gap: var(--space-1);
      padding: var(--space-1);
      border: 2px solid transparent;
      border-radius: var(--radius);
      cursor: pointer;

      &[aria-checked='true'] {
        border-color: var(--color-accent);
      }
      &:hover .preview,
      &:hover .split {
        box-shadow: 0 0 0 1px var(--color-text-muted);
      }
    }
    .name {
      font-size: var(--font-size-small);
      text-align: center;
    }
    .preview,
    .split {
      display: flex;
      height: 72px;
      overflow: hidden;
      font-family: var(--font-editor);
      font-size: 10px;
      line-height: 16px;
      border: 1px solid var(--color-border);
      border-radius: 4px;
    }
    .preview {
      color: var(--ed-text);
      background: var(--ed-bg);
      padding: 6px 0;
    }
    .split {
      padding: 0;

      .preview {
        flex: 1;
        min-width: 0;
        border: 0;
        border-radius: 0;
      }
    }
    .gutter {
      display: flex;
      flex-direction: column;
      width: 18px;
      text-align: right;
      padding-right: 4px;
      color: var(--ed-gutter-text);
      background: var(--ed-gutter-bg);

      .on {
        color: var(--ed-gutter-text-active);
      }
    }
    .code {
      display: flex;
      flex-direction: column;
      flex: 1;
      padding-left: 6px;
      white-space: nowrap;

      i {
        font-style: normal;
      }
      .sel {
        background: var(--ed-selection);
      }
      .cur {
        background: var(--ed-current-line);
      }
    }
    .kw {
      color: var(--syn-keyword);
    }
    .fn {
      color: var(--syn-function);
    }
    .pu {
      color: var(--syn-punct);
    }
    .nu {
      color: var(--syn-number);
    }
    .st {
      color: var(--syn-string);
    }
    .co {
      color: var(--syn-comment);
    }
    .caret {
      display: inline-block;
      width: 2px;
      height: 12px;
      vertical-align: middle;
      background: var(--ed-caret);
    }
    .hint {
      margin: 0;
      font-size: var(--font-size-small);
      color: var(--color-text-muted);
    }
    .actions {
      display: flex;
      justify-content: flex-end;
      gap: var(--space-2);
    }
  `,
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class SchemePicker {
  protected readonly dialogs = inject(DialogService);
  protected readonly chrome = inject(ChromeStore);
  private readonly actions = inject(ActionService);
  protected readonly t = MENUS_TEXTS;
  protected readonly schemes = SCHEMES;
  /** The scheme before the dialog – Esc and Cancel go back to it. */
  private readonly before = this.chrome.scheme();

  protected preview(s: Exclude<Scheme, 'auto'>) {
    return previewAttributes(s);
  }

  protected label(s: Scheme): string {
    const l = this.actions.label(`view.scheme.${s}`).replace(/^[^:]+:\s*/, '');
    return l.charAt(0).toUpperCase() + l.slice(1);
  }

  protected choose(s: Scheme, e?: Event): void {
    this.chrome.scheme.set(s);
    (e?.currentTarget as HTMLElement | undefined)?.focus();
  }

  protected keep(): void {
    this.dialogs.close();
  }

  protected cancel(): void {
    this.chrome.scheme.set(this.before);
    this.dialogs.close();
  }

  protected onKey(e: KeyboardEvent): void {
    const i = SCHEMES.indexOf(this.chrome.scheme());
    const n = SCHEMES.length;
    let next: number;
    switch (e.key) {
      case 'ArrowRight':
        next = (i + 1) % n;
        break;
      case 'ArrowLeft':
        next = (i - 1 + n) % n;
        break;
      case 'ArrowDown':
        next = Math.min(n - 1, i + COLUMNS);
        break;
      case 'ArrowUp':
        next = Math.max(0, i - COLUMNS);
        break;
      case 'Home':
        next = 0;
        break;
      case 'End':
        next = n - 1;
        break;
      case 'Enter':
      case ' ':
        e.preventDefault();
        this.keep();
        return;
      default:
        return;
    }
    e.preventDefault();
    this.chrome.scheme.set(SCHEMES[next]);
    const host = (e.currentTarget as HTMLElement).parentElement;
    queueMicrotask(() =>
      host?.querySelector<HTMLElement>(`[data-scheme-id="${SCHEMES[next]}"]`)?.focus(),
    );
  }
}
