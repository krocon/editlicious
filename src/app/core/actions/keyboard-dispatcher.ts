import { inject, Injectable } from '@angular/core';
import { ActionContext } from './action-catalog';
import { ActionService } from './action.service';
import { chordOf } from './shortcut';

/** Where the keyboard is: contexts in order of preference, or `null` (a dialog owns it). */
export type ContextProvider = (target: Element | null) => ActionContext[] | null;

const SLIDER_KEYS = new Set([
  'ArrowLeft',
  'ArrowRight',
  'ArrowUp',
  'ArrowDown',
  'Home',
  'End',
  'PageUp',
  'PageDown',
]);

function isTextField(el: Element | null): boolean {
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
 * The one `keydown` listener of the app (taken over from comic-commander design 008, CLAUDE.md §4): event → chord → action of
 * the first context that has one. Widgets with their own keys (menus, lists, the modal's focus
 * trap) handle theirs first and call `preventDefault`; text fields keep everything except
 * `Mod` shortcuts (E5); buttons keep Space/Enter, sliders their arrows.
 */
@Injectable({ providedIn: 'root' })
export class KeyboardDispatcher {
  private readonly actions = inject(ActionService);
  private provider: ContextProvider = () => ['global'];
  private readonly fallbacks = new Map<ActionContext, (e: KeyboardEvent) => boolean>();
  private attached: Document | null = null;

  setContextProvider(p: ContextProvider): void {
    this.provider = p;
  }

  /** Keys no action took. */
  setFallback(ctx: ActionContext, fn: (e: KeyboardEvent) => boolean): void {
    this.fallbacks.set(ctx, fn);
  }

  /** Removes `fn` – only if it is still the one set (a newer view may have replaced it). */
  clearFallback(ctx: ActionContext, fn: (e: KeyboardEvent) => boolean): void {
    if (this.fallbacks.get(ctx) === fn) this.fallbacks.delete(ctx);
  }

  attach(doc: Document): void {
    if (this.attached) return;
    this.attached = doc;
    doc.addEventListener('keydown', (e) => this.handle(e));
    doc.addEventListener('keyup', (e) => this.keyup(e));
    doc.addEventListener('contextmenu', (e) => this.contextMenu(e));
  }

  /**
   * The webview's own menu (Back, Reload, Inspect Element) has no place in the app – WebKitGTK
   * shows it on Shift+F10 even after our book menu opened. Text fields keep it (copy, paste).
   */
  contextMenu(e: MouseEvent): void {
    const target = e.target instanceof Element ? e.target : null;
    if (!isTextField(target)) e.preventDefault();
  }

  /** Alt pressed and released alone (Windows/Linux) opens the hidden menu bar (design 005 E2). */
  private altAlone = false;

  keyup(e: KeyboardEvent): void {
    const alone = this.altAlone;
    this.altAlone = false;
    if (e.key !== 'Alt' || !alone || this.actions.os === 'mac') return;
    if (this.provider(e.target instanceof Element ? e.target : null) === null) return;
    if (this.actions.run('app.focusMenu')) e.preventDefault();
  }

  handle(e: KeyboardEvent): void {
    this.altAlone = e.key === 'Alt' && !e.ctrlKey && !e.metaKey && !e.shiftKey && !e.repeat;
    if (e.defaultPrevented || e.isComposing) return;
    const target = e.target instanceof Element ? e.target : null;
    const contexts = this.provider(target);
    if (!contexts) return;
    // a menu widget owns its keys
    if (target?.closest('[role="menu"],[role="menubar"],[role="listbox"]')) return;
    const chord = chordOf(e, this.actions.os);
    if (!chord) return;
    const hasMod =
      chord.startsWith('Mod+') || (this.actions.os === 'mac' && chord.startsWith('Ctrl+'));
    // F-keys (F10 menu bar, F11 full screen) are no typing either
    if (isTextField(target) && !hasMod && !/^(Shift\+)?F\d{1,2}$/.test(chord)) return;
    if (target instanceof HTMLButtonElement && (e.key === ' ' || e.key === 'Enter')) return;
    if (target instanceof HTMLInputElement && target.type === 'range' && SLIDER_KEYS.has(e.key)) {
      return;
    }
    const keymap = this.actions.keymap();
    for (const ctx of contexts) {
      const id = keymap.byContext[ctx].get(chord);
      if (!id || !this.actions.available(id)) continue;
      e.preventDefault();
      e.stopPropagation();
      if (e.repeat && !this.actions.def(id)?.repeat) return;
      this.actions.run(id);
      return;
    }
    for (const ctx of contexts) {
      const fb = this.fallbacks.get(ctx);
      if (fb?.(e)) {
        e.preventDefault();
        e.stopPropagation();
        return;
      }
    }
  }
}
