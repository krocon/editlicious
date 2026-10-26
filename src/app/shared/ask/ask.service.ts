import { Injectable, inject, signal } from '@angular/core';
import { DialogService } from '../dialog/dialog.service';

export interface AskButton {
  id: string;
  label: string;
  primary?: boolean;
}

export interface Question {
  title: string;
  message: string;
  buttons: AskButton[];
  /** Answer when the dialog is closed with Escape or the backdrop. */
  cancel: string;
}

/**
 * Questions that interrupt work ("save changes?") as a promise (design 004 E3). One question at
 * a time; it is shown by `AskDialog` through the `DialogService`.
 */
@Injectable({ providedIn: 'root' })
export class AskService {
  private readonly dialogs = inject(DialogService);
  readonly current = signal<Question | null>(null);
  private resolve: ((answer: string) => void) | null = null;
  private queue: Promise<unknown> = Promise.resolve();

  ask(q: Question): Promise<string> {
    const next = this.queue.then(
      () =>
        new Promise<string>((resolve) => {
          this.resolve = resolve;
          this.current.set(q);
          this.dialogs.show('ask');
        }),
    );
    this.queue = next;
    return next;
  }

  answer(id: string | null): void {
    const q = this.current();
    if (!q) return;
    const r = this.resolve;
    this.resolve = null;
    this.current.set(null);
    this.dialogs.close();
    r?.(id ?? q.cancel);
  }
}
