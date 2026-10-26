import { Injectable, inject, signal } from '@angular/core';
import { DialogService } from '../dialog/dialog.service';

export interface PickRequest {
  title: string;
  placeholder: string;
  options: { id: string; label: string }[];
  /** Pre-selected id. */
  current?: string | null;
}

/**
 * Choosing one of many (language, encoding …) as a promise: a filterable list like the
 * command palette (design 008 E4). `null` = cancelled.
 */
@Injectable({ providedIn: 'root' })
export class PickService {
  private readonly dialogs = inject(DialogService);
  readonly request = signal<PickRequest | null>(null);
  private resolve: ((id: string | null) => void) | null = null;

  pick(r: PickRequest): Promise<string | null> {
    this.resolve?.(null);
    return new Promise((resolve) => {
      this.resolve = resolve;
      this.request.set(r);
      this.dialogs.show('pick');
    });
  }

  answer(id: string | null): void {
    const r = this.resolve;
    this.resolve = null;
    this.request.set(null);
    this.dialogs.close();
    r?.(id);
  }
}
