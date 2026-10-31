import { inject, Injectable } from '@angular/core';
import { ClipboardApi } from '../clipboard/clipboard-api';

/** A field the user types into (not the editor's hidden input, not a checkbox or button). */
export function isTextField(el: Element | null): boolean {
  if (!el) return false;
  // the editor's hidden input is no text field: all its keys are actions of the engine
  if (el instanceof HTMLElement && el.dataset['editorInput'] !== undefined) return false;
  if (el instanceof HTMLTextAreaElement || el instanceof HTMLSelectElement) return true;
  if (el instanceof HTMLInputElement) {
    return !['checkbox', 'radio', 'button', 'submit', 'reset', 'range', 'file', 'color'].includes(
      el.type,
    );
  }
  return (el as HTMLElement).isContentEditable === true;
}

/**
 * Editor actions whose shortcuts a focused text field keeps (find field, "Open path…", settings):
 * ⌘V there pastes into the field, not into the document.
 */
export const TEXT_FIELD_ACTIONS: ReadonlySet<string> = new Set([
  'edit.cut',
  'edit.copy',
  'edit.paste',
  'edit.selectAll',
  'edit.undo',
  'edit.redo',
]);

type Field = HTMLInputElement | HTMLTextAreaElement;

function editableField(el: Element | null): Field | null {
  if (!isTextField(el)) return null;
  return el instanceof HTMLInputElement || el instanceof HTMLTextAreaElement ? el : null;
}

/** `document.execCommand` keeps the field's own undo; jsdom and old engines lack it. */
function exec(command: string, value?: string): boolean {
  try {
    return (
      typeof document.execCommand === 'function' && document.execCommand(command, false, value)
    );
  } catch {
    return false;
  }
}

/**
 * Runs {@link TEXT_FIELD_ACTIONS} on the focused text field. Needed for the native macOS menu:
 * its accelerators (⌘V …) arrive as menu actions and never reach the webview as keys.
 */
@Injectable({ providedIn: 'root' })
export class TextFieldEdit {
  private readonly clipboard = inject(ClipboardApi);

  /** `true` = the focused element is a text field and took the action. */
  run(id: string, target: Element | null = document.activeElement): boolean {
    if (!TEXT_FIELD_ACTIONS.has(id)) return false;
    const el = editableField(target);
    if (!el) return isTextField(target); // a <select> or contenteditable: nothing for the document
    switch (id) {
      case 'edit.paste':
        void this.paste(el);
        break;
      case 'edit.copy':
        void this.copy(el, false);
        break;
      case 'edit.cut':
        void this.copy(el, true);
        break;
      case 'edit.selectAll':
        el.select();
        break;
      case 'edit.undo':
        exec('undo');
        break;
      case 'edit.redo':
        exec('redo');
        break;
    }
    return true;
  }

  private async paste(el: Field): Promise<void> {
    const raw = await this.clipboard.readText().catch(() => null);
    if (!raw) return;
    // a one-line field: line breaks at the ends go, inner ones become spaces
    const text =
      el instanceof HTMLInputElement
        ? raw.replace(/^[\r\n]+|[\r\n]+$/g, '').replace(/\r\n|\r|\n/g, ' ')
        : raw;
    if (!text || !el.isConnected) return;
    el.focus();
    if (exec('insertText', text)) return;
    const start = el.selectionStart ?? el.value.length;
    const end = el.selectionEnd ?? start;
    el.setRangeText(text, start, end, 'end');
    el.dispatchEvent(new Event('input', { bubbles: true }));
  }

  private async copy(el: Field, cut: boolean): Promise<void> {
    if (el instanceof HTMLInputElement && el.type === 'password') return;
    const start = el.selectionStart ?? 0;
    const end = el.selectionEnd ?? 0;
    if (end <= start) return;
    await this.clipboard.writeText(el.value.slice(start, end)).catch(() => undefined);
    if (!cut || el.readOnly) return;
    if (exec('delete')) return;
    el.setRangeText('', start, end, 'end');
    el.dispatchEvent(new Event('input', { bubbles: true }));
  }
}
