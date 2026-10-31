import { Injectable, signal } from '@angular/core';

/** The modal dialogs of the app (one at a time). */
export type DialogKind =
  'about' | 'ask' | 'palette' | 'shortcuts' | 'pick' | 'settings' | 'scheme' | 'openPath';

/**
 * Which modal dialog is open. Remembers the focused element on `show` and gives the focus back
 * on `close` (as in files-and-folders `dialog.service.ts`).
 */
@Injectable({ providedIn: 'root' })
export class DialogService {
  readonly open = signal<DialogKind | null>(null);
  private returnFocus: HTMLElement | null = null;
  /** A focus return that has not happened yet (a dialog right after another keeps it). */
  private pending: { target: HTMLElement; timer: ReturnType<typeof setTimeout> } | null = null;

  show(kind: DialogKind): void {
    if (this.pending) {
      // one question after another (pick a source, then a size): the focus goes back to
      // where it was before the first one – not in between
      clearTimeout(this.pending.timer);
      this.returnFocus = this.pending.target;
      this.pending = null;
    } else if (this.open() === null) {
      const active = typeof document !== 'undefined' ? document.activeElement : null;
      this.returnFocus = active instanceof HTMLElement ? active : null;
    }
    this.open.set(kind);
  }

  close(options?: { restoreFocus?: boolean }): void {
    this.open.set(null);
    const target = options?.restoreFocus === false ? null : this.returnFocus;
    this.returnFocus = null;
    if (target?.isConnected) {
      const timer = setTimeout(() => {
        this.pending = null;
        target.focus();
      });
      this.pending = { target, timer };
    }
  }
}
