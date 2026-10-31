import { inject, Injectable } from '@angular/core';
import { ClipboardBridge } from '../features/editor/editor-host';
import { EmbedEngineApi } from './wasm-editor-api';

/**
 * The page's clipboard for the embedded editor (design 025 E6). The write starts in the same
 * keystroke (Safari wants that) with a `ClipboardItem` whose data arrive when the engine has
 * them; browsers without promise items get `writeText` afterwards.
 */
@Injectable()
export class BrowserClipboard implements ClipboardBridge {
  private readonly api = inject(EmbedEngineApi);

  write(docId: number, html: boolean, done: Promise<unknown>): void {
    const clip = typeof navigator !== 'undefined' ? navigator.clipboard : undefined;
    if (!clip) return;
    const data = done.then(() => this.api.clipboard(docId));
    const blob = (kind: 'text' | 'html', type: string) =>
      data.then((d) => new Blob([d[kind] ?? ''], { type }));
    try {
      if (typeof ClipboardItem === 'undefined') throw new Error('no ClipboardItem');
      const items: Record<string, Promise<Blob>> = { 'text/plain': blob('text', 'text/plain') };
      if (html) items['text/html'] = blob('html', 'text/html');
      clip.write([new ClipboardItem(items)]).catch(() => this.fallback(data));
    } catch {
      this.fallback(data);
    }
  }

  private fallback(data: Promise<{ text: string | null }>): void {
    void data
      .then((d) => (d.text !== null ? navigator.clipboard?.writeText(d.text) : undefined))
      .catch(() => undefined);
  }

  async read(): Promise<string | null> {
    try {
      return (await navigator.clipboard?.readText()) ?? null;
    } catch {
      return null;
    }
  }
}
