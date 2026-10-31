import { Injectable } from '@angular/core';
import { DocInfo } from '../app/core/editor/editor.models';
import { EmbedEngineApi } from '../app/embed/wasm-editor-api';
import { docInfo, FakeEditorApi } from './fake-editor-api';

/** Test double of the embedded editor's engine (design 025): texts in memory, no WebAssembly. */
@Injectable()
export class FakeEmbedEngineApi extends FakeEditorApi implements EmbedEngineApi {
  texts = new Map<number, string>();
  languagesSet: { id: number; spec: string }[] = [];
  closed: number[] = [];
  /** How often the whole text went over the boundary. */
  textReads = 0;
  private next = 100;

  createText(text: string, language: string | null): Promise<DocInfo> {
    const id = this.next++;
    this.texts.set(id, text);
    if (language) this.languagesSet.push({ id, spec: language });
    const info = docInfo({ id, lineCount: text.split('\n').length, charCount: text.length });
    this.infos.set(id, info);
    return Promise.resolve(info);
  }

  setText(id: number, text: string): Promise<void> {
    this.texts.set(id, text);
    return Promise.resolve();
  }

  setLanguage(id: number, spec: string): Promise<string | null> {
    this.languagesSet.push({ id, spec });
    return Promise.resolve(spec || null);
  }

  text(id: number): Promise<string> {
    this.textReads++;
    return Promise.resolve(this.texts.get(id) ?? '');
  }

  slice(id: number, from: number, to: number): Promise<string> {
    const lines = (this.texts.get(id) ?? '').split('\n');
    return Promise.resolve(lines.slice(from, to).join('\n'));
  }

  clipboard(): Promise<{ text: string | null; html: string | null }> {
    return Promise.resolve({ text: 'copied', html: null });
  }

  override close(id?: number): Promise<void> {
    if (id !== undefined) this.closed.push(id);
    return Promise.resolve();
  }
}
