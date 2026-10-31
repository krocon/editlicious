/**
 * The custom element `<editlicious-editor>` (design 025 E10): the embeddable editor as a
 * standard web component (shadow DOM, zoneless). Build: `pnpm build:element`.
 *
 * ```html
 * <script type="module" src="editlicious-editor.js"></script>
 * <editlicious-editor language="Rust" scheme="one"></editlicious-editor>
 * ```
 */
import { createCustomElement, NgElementConstructor } from '@angular/elements';
import { provideZonelessChangeDetection } from '@angular/core';
import { createApplication } from '@angular/platform-browser';
import { Command, MatchCount, Query } from './app/core/editor/editor.models';
import { EditliciousEditor, EMIT_DOM_EVENTS, FACADE } from './app/embed/editlicious-editor';
import { initEditlicious } from './app/embed/engine';

export { initEditlicious };
export type { EdCaret, EdChange } from './app/embed/editlicious-editor';

const TAG = 'editlicious-editor';

/** The facade of an element – once Angular has created it (after `connectedCallback`). */
function facade(el: HTMLElement): Promise<EditliciousEditor> {
  return new Promise((resolve) => {
    const look = () => {
      const f = (el as unknown as Record<symbol, EditliciousEditor | undefined>)[FACADE];
      if (f) resolve(f);
      else requestAnimationFrame(look);
    };
    look();
  });
}

/** The methods of `<editlicious-editor>` on top of its properties (the inputs). */
function withMethods(base: NgElementConstructor<unknown>): CustomElementConstructor {
  return class EditliciousEditorElement extends (base as unknown as typeof HTMLElement) {
    getValue(): Promise<string> {
      return facade(this).then((f) => f.getValue());
    }
    getSlice(from: number, to: number): Promise<string> {
      return facade(this).then((f) => f.getSlice(from, to));
    }
    setValue(text: string): Promise<void> {
      return facade(this).then((f) => f.setValue(text));
    }
    loadText(text: string, options?: { language?: string }): Promise<void> {
      return facade(this).then((f) => f.loadText(text, options));
    }
    override focus(): void {
      void facade(this).then((f) => f.focus());
    }
    runCommand(cmd: Command): Promise<void> {
      return facade(this).then((f) => f.runCommand(cmd));
    }
    runAction(id: string): Promise<boolean> {
      return facade(this).then((f) => f.runAction(id));
    }
    find(pattern: string, options?: Partial<Omit<Query, 'pattern'>>): Promise<MatchCount> {
      return facade(this).then((f) => f.find(pattern, options));
    }
    goTo(line: number, ch?: number): Promise<void> {
      return facade(this).then((f) => f.goTo(line, ch));
    }
  };
}

/** Defines `<editlicious-editor>` once (a second bundle on the page leaves it alone). */
export async function defineEditlicious(): Promise<void> {
  if (customElements.get(TAG)) return;
  const app = await createApplication({
    providers: [provideZonelessChangeDetection(), { provide: EMIT_DOM_EVENTS, useValue: true }],
  });
  const element = createCustomElement(EditliciousEditor, { injector: app.injector });
  if (!customElements.get(TAG)) customElements.define(TAG, withMethods(element));
}

void defineEditlicious();
