import { Injectable } from '@angular/core';
import { EditorApi, Loading } from '../core/editor/editor-api';
import {
  Command,
  DiskState,
  DocInfo,
  Frame,
  MatchCount,
  Query,
} from '../core/editor/editor.models';
import { FsError } from '../core/errors/fs-error';
import { engine, EngineCall } from './engine';

/**
 * The engine as the embedded editor needs it (design 025 E4): an {@link EditorApi} plus text in
 * and out. One per page (`WasmEditorApi` on the shared WebAssembly engine); tests give a fake.
 */
@Injectable({ providedIn: 'root', useFactory: () => new WasmEditorApi() })
export abstract class EmbedEngineApi extends EditorApi {
  /** A new session on `text`; `language` = grammar, alias or file name (design 025 E3). */
  abstract createText(text: string, language: string | null): Promise<DocInfo>;
  /** Replaces the whole text (clean, no undo back). */
  abstract setText(id: number, text: string): Promise<void>;
  /** The language shown afterwards (`null` = plain text). */
  abstract setLanguage(id: number, spec: string): Promise<string | null>;
  /** The whole text – only on request. */
  abstract text(id: number): Promise<string>;
  /** Lines `from..to` (0-based, end exclusive). */
  abstract slice(id: number, from: number, to: number): Promise<string>;
  /** What the last copy put on the engine's clipboard. */
  abstract clipboard(id: number): Promise<{ text: string | null; html: string | null }>;
}

function unsupported(what: string): Promise<never> {
  const e: FsError = {
    code: 'invalid_request',
    message: `${what}: not in the browser`,
    path: null,
  };
  return Promise.reject(e);
}

/**
 * {@link EditorApi} on the engine as WebAssembly (design 025 E4): the embedded editor's sessions
 * live in the shared engine (worker). No files: `open`, `save`, `reload` are refused; text comes
 * in with {@link createText} / {@link setText}.
 */
@Injectable()
export class WasmEditorApi extends EmbedEngineApi {
  private call(op: string, ...args: unknown[]): Promise<unknown> {
    return engine().then((c: EngineCall) => c(op, ...args));
  }

  /** A new session on `text`; `language` = grammar, alias or file name (design 025 E3). */
  async createText(text: string, language: string | null): Promise<DocInfo> {
    const id = (await this.call('open', text, language)) as number;
    return this.info(id);
  }

  /** Replaces the whole text (clean, no undo back). */
  setText(id: number, text: string): Promise<void> {
    return this.call('setText', id, text).then(() => undefined);
  }

  /** The language shown afterwards (`null` = plain text). */
  setLanguage(id: number, spec: string): Promise<string | null> {
    return this.call('setLanguage', id, spec) as Promise<string | null>;
  }

  /** The whole text – only on request. */
  text(id: number): Promise<string> {
    return this.call('text', id) as Promise<string>;
  }

  /** Lines `from..to` (end exclusive). */
  slice(id: number, from: number, to: number): Promise<string> {
    return this.call('slice', id, from, to) as Promise<string>;
  }

  /** What the last copy put on the engine's clipboard. */
  clipboard(id: number): Promise<{ text: string | null; html: string | null }> {
    return this.call('clipboard', id) as Promise<{ text: string | null; html: string | null }>;
  }

  open(): Loading {
    return { loadId: 0, done: unsupported('open') };
  }

  cancelLoad(): Promise<void> {
    return Promise.resolve();
  }

  newDocument(): Promise<DocInfo> {
    return this.createText('', null);
  }

  close(id: number): Promise<void> {
    return this.call('close', id).then(() => undefined);
  }

  // the binding numbers every session 1 – the page's session id is the engine host's
  async info(id: number): Promise<DocInfo> {
    const info = JSON.parse((await this.call('info', id)) as string) as DocInfo;
    return { ...info, id };
  }

  async command(id: number, cmd: Command): Promise<Frame> {
    const f = JSON.parse((await this.call('command', id, JSON.stringify(cmd))) as string) as Frame;
    f.info.id = id;
    return f;
  }

  save(): Promise<DocInfo> {
    return unsupported('save');
  }

  startupPaths(): Promise<string[]> {
    return Promise.resolve([]);
  }

  reload(): Promise<DocInfo> {
    return unsupported('reload');
  }

  diskState(): Promise<DiskState> {
    return unsupported('diskState');
  }

  async countMatches(id: number, query: Query): Promise<MatchCount> {
    return JSON.parse(
      (await this.call('countMatches', id, JSON.stringify(query))) as string,
    ) as MatchCount;
  }

  countMarker(id: number): Promise<number | null> {
    return this.call('countMarker', id) as Promise<number | null>;
  }

  async listLanguages(): Promise<string[]> {
    return JSON.parse((await this.call('languages')) as string) as string[];
  }

  async listEncodings(): Promise<string[]> {
    return JSON.parse((await this.call('encodings')) as string) as string[];
  }

  overview(id: number, buckets: number): Promise<number[]> {
    return this.call('overview', id, buckets) as Promise<number[]>;
  }
}
