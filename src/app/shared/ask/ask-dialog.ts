import { ChangeDetectionStrategy, Component, inject } from '@angular/core';
import { Modal } from '../dialog/modal';
import { AskService } from './ask.service';

/** Shows the current question of the {@link AskService}. */
@Component({
  selector: 'app-ask-dialog',
  imports: [Modal],
  template: `
    @if (ask.current(); as q) {
      <app-modal
        [heading]="q.title"
        dialogRole="alertdialog"
        describedBy="ask-message"
        width="30rem"
        (closed)="ask.answer(null)"
      >
        <p id="ask-message" class="message">{{ q.message }}</p>
        <div class="actions">
          @for (b of q.buttons; track b.id) {
            <button
              type="button"
              class="btn"
              [class.primary]="b.primary"
              [attr.data-autofocus]="b.primary ? '' : null"
              (click)="ask.answer(b.id)"
            >
              {{ b.label }}
            </button>
          }
        </div>
      </app-modal>
    }
  `,
  styles: `
    .message {
      margin: 0;
      line-height: 1.5;
      overflow-wrap: anywhere;
    }
    .actions {
      display: flex;
      justify-content: flex-end;
      gap: var(--space-2);
    }
  `,
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class AskDialog {
  protected readonly ask = inject(AskService);
}
