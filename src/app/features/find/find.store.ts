import { computed, Injectable, signal } from '@angular/core';
import { Query } from '../../core/editor/editor.models';

/** Entries kept per history (same as Rust `session_file::MAX_HISTORY`). */
const MAX_HISTORY = 20;

/**
 * The find/replace bar's state, shared by all tabs (design 006): pattern, options, the
 * replacement and their histories (kept in the session file by the `DocumentsStore`).
 */
@Injectable({ providedIn: 'root' })
export class FindStore {
  readonly open = signal(false);
  readonly replaceOpen = signal(false);
  readonly pattern = signal('');
  readonly replacement = signal('');
  readonly regex = signal(false);
  readonly caseSensitive = signal(false);
  readonly wholeWord = signal(false);
  readonly findHistory = signal<string[]>([]);
  readonly replaceHistory = signal<string[]>([]);
  /** Bumped when the find field should take the focus (Mod+F while open). */
  readonly focusRequest = signal(0);

  readonly query = computed<Query>(() => ({
    pattern: this.pattern(),
    regex: this.regex(),
    caseSensitive: this.caseSensitive(),
    wholeWord: this.wholeWord(),
  }));

  remember(): void {
    push(this.findHistory, this.pattern());
  }

  rememberReplacement(): void {
    push(this.replaceHistory, this.replacement());
  }
}

function push(h: ReturnType<typeof signal<string[]>>, value: string): void {
  if (!value) return;
  h.update((list) => [value, ...list.filter((x) => x !== value)].slice(0, MAX_HISTORY));
}

/** Step through a history from the field's current value (↑ = older). */
export function stepHistory(list: string[], current: string, older: boolean): string | null {
  if (list.length === 0) return null;
  const i = list.indexOf(current);
  if (older) return list[Math.min(list.length - 1, i + 1)] ?? null;
  if (i <= 0) return null;
  return list[i - 1];
}
