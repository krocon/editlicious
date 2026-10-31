import { Injectable } from '@angular/core';
import { EditorApi, Loading } from '../app/core/editor/editor-api';
import {
  Command,
  DiskState,
  DocInfo,
  Frame,
  FrameLine,
  LineEnding,
  LoadEvent,
  MatchCount,
  Query,
  SaveRequest,
} from '../app/core/editor/editor.models';

/** A DocInfo with defaults. */
export function docInfo(over: Partial<DocInfo> = {}): DocInfo {
  return {
    id: 1,
    path: null,
    name: 'Untitled',
    encoding: 'UTF-8',
    bom: false,
    lineEnding: 'lf',
    mixedLineEndings: false,
    decodeErrors: false,
    lineCount: 1,
    charCount: 0,
    fileSize: 0,
    revision: 0,
    dirty: false,
    canUndo: false,
    canRedo: false,
    ...over,
  };
}

/** A frame of plain text lines (one text run each), caret at the start. */
export function textFrame(lines: string[], over: Partial<Frame> = {}): Frame {
  const frameLines: FrameLine[] = lines.map((text, line) => ({
    line,
    runs: text ? [{ cell: 0, width: text.length, kind: 'text', text, style: 0 }] : [],
    sel: [],
    selEol: false,
    width: text.length,
    long: false,
    marks: [],
    markers: [],
    indent: 0,
    row: line,
    wraps: [],
    colors: [],
  }));
  return {
    seq: 1,
    info: docInfo({ lineCount: lines.length }),
    top: 0,
    left: 0,
    firstLine: 0,
    lines: frameLines,
    maxWidth: Math.max(0, ...lines.map((l) => l.length)),
    carets: [{ line: 0, cell: 0 }],
    caretXy: [{ row: 0, x: 0 }],
    primary: 0,
    caretLine: 0,
    caretCh: 0,
    selChars: 0,
    selLines: 0,
    overwrite: false,
    tabWidth: 4,
    insertSpaces: true,
    notice: null,
    search: false,
    multi: false,
    language: null,
    approxHighlight: false,
    selectionText: null,
    wrap: false,
    deco: 0,
    colorColumn: false,
    caretColor: null,
    markerWords: [],
    caretMarker: null,
    hint: null,
    completion: null,
    readOnly: false,
    ...over,
  };
}

/**
 * Test double of {@link EditorApi}: records every command and answers with `nextFrame`.
 * Never imported by app code.
 */
@Injectable()
export class FakeEditorApi extends EditorApi {
  commands: { id: number; cmd: Command }[] = [];
  nextFrame: Frame = textFrame(['hello', 'world']);
  paths: string[] = [];
  opened: string[] = [];
  saved: { id: number; req: SaveRequest }[] = [];
  private nextId = 1;

  open(path: string, onProgress?: (e: LoadEvent) => void): Loading {
    this.opened.push(path);
    onProgress?.({ kind: 'progress', loaded: 1, total: 1 });
    const id = this.nextId++;
    const info = docInfo({ id, path, name: path });
    this.infos.set(id, info);
    return { loadId: id, done: Promise.resolve(info) };
  }

  cancelLoad(): Promise<void> {
    return Promise.resolve();
  }

  /** What the fake engine knows about each document. */
  infos = new Map<number, DocInfo>();

  newLineEndings: (LineEnding | null)[] = [];

  newDocument(lineEnding: LineEnding | null = null): Promise<DocInfo> {
    this.newLineEndings.push(lineEnding);
    const info = docInfo({ id: this.nextId++, lineEnding: lineEnding ?? 'lf' });
    this.infos.set(info.id, info);
    return Promise.resolve(info);
  }

  /** Next answer of `save` (an error to throw, or nothing). */
  saveError: unknown = null;

  close(): Promise<void> {
    return Promise.resolve();
  }

  info(id: number): Promise<DocInfo> {
    return Promise.resolve(this.infos.get(id) ?? docInfo({ id }));
  }

  command(id: number, cmd: Command): Promise<Frame> {
    this.commands.push({ id, cmd });
    const info = this.infos.get(id) ?? { ...this.nextFrame.info, id };
    return Promise.resolve({ ...this.nextFrame, info });
  }

  save(id: number, req: SaveRequest = {}): Promise<DocInfo> {
    this.saved.push({ id, req });
    if (this.saveError) {
      const e = this.saveError;
      this.saveError = null;
      return Promise.reject(e);
    }
    const path = req.path ?? `/doc${id}`;
    return Promise.resolve(docInfo({ id, path, name: path.split('/').pop()!, dirty: false }));
  }

  startupPaths(): Promise<string[]> {
    return Promise.resolve(this.paths);
  }

  disk = new Map<number, DiskState>();
  reloads: number[] = [];

  reloadEncodings: (string | undefined)[] = [];

  /** Next answer of `reload` (an error to throw, or nothing). */
  reloadError: unknown = null;

  reload(id: number, encoding?: string): Promise<DocInfo> {
    this.reloads.push(id);
    if (this.reloadError) {
      const e = this.reloadError;
      this.reloadError = null;
      return Promise.reject(e);
    }
    this.reloadEncodings.push(encoding);
    return Promise.resolve(docInfo({ id, path: `/doc${id}`, name: `doc${id}` }));
  }

  diskState(id: number): Promise<DiskState> {
    return Promise.resolve(this.disk.get(id) ?? { changed: false, missing: false });
  }

  matchCount: MatchCount = { total: 0, current: 0 };
  counted: Query[] = [];

  countMatches(_id: number, query: Query): Promise<MatchCount> {
    this.counted.push(query);
    return Promise.resolve(this.matchCount);
  }

  encodings = ['UTF-8', 'windows-1252'];
  listEncodings(): Promise<string[]> {
    return Promise.resolve(this.encodings);
  }

  overviewAnswer: number[] = [];
  overviews: number[] = [];
  overview(_id: number, buckets: number): Promise<number[]> {
    this.overviews.push(buckets);
    return Promise.resolve(this.overviewAnswer);
  }

  languages = ['JavaScript', 'Rust'];
  listLanguages(): Promise<string[]> {
    return Promise.resolve(this.languages);
  }

  /** Commands of a type, for assertions. */
  markerCount: number | null = null;
  countMarker(): Promise<number | null> {
    return Promise.resolve(this.markerCount);
  }

  sent(type: Command['type']): Command[] {
    return this.commands.map((c) => c.cmd).filter((c) => c.type === type);
  }
}
