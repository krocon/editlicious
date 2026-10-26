import { ChangeDetectionStrategy, Component, computed, inject, signal } from '@angular/core';
import { ACTIONS } from '../../core/actions/action-catalog';
import { ActionService } from '../../core/actions/action.service';
import { formatChord } from '../../core/actions/shortcut';
import { DialogService } from '../../shared/dialog/dialog.service';
import { Modal } from '../../shared/dialog/modal';
import { matches } from './command-palette';
import { MENUS_TEXTS } from './menus-texts';

/**
 * All shortcuts at a glance (legacy „Show Keystrokes…“, Ctrl+Shift+K): every action with its
 * chords of this OS, filterable. Overrides from `keymap.json` are included.
 */
@Component({
  selector: 'app-shortcuts-dialog',
  imports: [Modal],
  template: `
    <app-modal [heading]="t.shortcutsTitle" width="40rem" height="80vh" (closed)="dialogs.close()">
      <input
        class="input"
        type="search"
        data-autofocus
        spellcheck="false"
        [attr.aria-label]="t.shortcutsFilter"
        [placeholder]="t.shortcutsFilter"
        (input)="query.set($any($event.target).value)"
      />
      <div class="list" tabindex="0" role="region" [attr.aria-label]="t.shortcutsTitle">
        <table>
          <tbody>
            @for (r of rows(); track r.id) {
              <tr>
                <td>{{ r.label }}</td>
                <td class="keys">
                  @for (k of r.keys; track k) {
                    <kbd>{{ k }}</kbd>
                  }
                </td>
              </tr>
            }
          </tbody>
        </table>
      </div>
      <p class="hint">{{ t.shortcutsHint }}</p>
    </app-modal>
  `,
  styles: `
    .list {
      flex: 1;
      min-height: 0;
      overflow: auto;
    }
    table {
      width: 100%;
      border-collapse: collapse;
    }
    td {
      padding: 3px var(--space-2);
      border-bottom: 1px solid var(--color-border);
    }
    .keys {
      text-align: right;
      white-space: nowrap;
    }
    kbd + kbd {
      margin-left: var(--space-1);
    }
    .hint {
      margin: 0;
      font-size: var(--font-size-small);
      color: var(--color-text-muted);
    }
  `,
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class ShortcutsDialog {
  protected readonly dialogs = inject(DialogService);
  private readonly actions = inject(ActionService);
  protected readonly t = MENUS_TEXTS;
  protected readonly query = signal('');

  private readonly all = computed(() =>
    ACTIONS.map((def) => ({
      id: def.id,
      label: this.actions.label(def.id),
      keys: this.actions.chords(def.id).map((c) => formatChord(c, this.actions.os)),
    })).filter((r) => r.keys.length > 0),
  );

  protected readonly rows = computed(() =>
    this.all().filter(
      (r) =>
        matches(r, this.query()) ||
        r.keys.some((k) => k.toLowerCase().includes(this.query().toLowerCase())),
    ),
  );
}
