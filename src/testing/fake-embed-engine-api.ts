import { Injectable } from '@angular/core';
import { DocInfo, Pos } from '../app/core/editor/editor.models';
import { EmbedEngineApi, EngineSelections, ScriptStep } from '../app/embed/wasm-editor-api';
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

  /** What `selections` answers (0-based). */
  sels: EngineSelections = {
    primary: 0,
    ranges: [{ anchor: { line: 0, ch: 0 }, head: { line: 0, ch: 0 }, text: '' }],
  };
  ranges: { from: Pos; to: Pos }[] = [];

  selections(): Promise<EngineSelections> {
    return Promise.resolve(this.sels);
  }

  rangeText(_id: number, from: Pos, to: Pos): Promise<string> {
    this.ranges.push({ from, to });
    return Promise.resolve(`${from.line}:${from.ch}-${to.line}:${to.ch}`);
  }

  clipboard(): Promise<{ text: string | null; html: string | null }> {
    return Promise.resolve({ text: 'copied', html: null });
  }

  /** What a script put on the engine's clipboard. */
  clips: string[] = [];
  /** The steps `scriptSteps` answers. */
  steps: ScriptStep[] = [];

  setClipboard(_id: number, text: string): Promise<void> {
    this.clips.push(text);
    return Promise.resolve();
  }

  scriptSteps(): Promise<ScriptStep[]> {
    return Promise.resolve(this.steps);
  }

  override close(id?: number): Promise<void> {
    if (id !== undefined) this.closed.push(id);
    return Promise.resolve();
  }
}
