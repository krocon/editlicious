import { Injectable } from '@angular/core';
import { Channel } from '@tauri-apps/api/core';
import { call } from '../backend/call';
import {
  Command,
  DiskState,
  DocInfo,
  Frame,
  LoadEvent,
  MatchCount,
  Query,
  SaveRequest,
} from './editor.models';

/** A running load: cancel it with {@link EditorApi.cancelLoad}. */
export interface Loading {
  loadId: number;
  done: Promise<DocInfo>;
}

/**
 * Typed facade of the editor engine (designs 002/003). Components and stores never call
 * `invoke()`; tests use `FakeEditorApi`.
 */
export abstract class EditorApi {
  abstract open(path: string, onProgress?: (e: LoadEvent) => void): Loading;
  abstract cancelLoad(loadId: number): Promise<void>;
  abstract newDocument(): Promise<DocInfo>;
  abstract close(id: number): Promise<void>;
  abstract info(id: number): Promise<DocInfo>;
  abstract command(id: number, cmd: Command): Promise<Frame>;
  abstract save(id: number, req?: SaveRequest): Promise<DocInfo>;
  abstract startupPaths(): Promise<string[]>;
  abstract reload(id: number): Promise<DocInfo>;
  abstract diskState(id: number): Promise<DiskState>;
  /** Counts on a snapshot; a newer count cancels an older one (`cancelled`). */
  abstract countMatches(id: number, query: Query): Promise<MatchCount>;
}

let loadSeq = 0;

@Injectable()
export class TauriEditorApi extends EditorApi {
  open(path: string, onProgress?: (e: LoadEvent) => void): Loading {
    const loadId = ++loadSeq;
    const onEvent = new Channel<LoadEvent>();
    onEvent.onmessage = (e) => onProgress?.(e);
    return { loadId, done: call<DocInfo>('open_document', { path, loadId, onEvent }) };
  }

  cancelLoad(loadId: number): Promise<void> {
    return call<void>('cancel_load', { loadId });
  }

  newDocument(): Promise<DocInfo> {
    return call<DocInfo>('new_document');
  }

  close(id: number): Promise<void> {
    return call<void>('close_document', { id });
  }

  info(id: number): Promise<DocInfo> {
    return call<DocInfo>('document_info', { id });
  }

  command(id: number, cmd: Command): Promise<Frame> {
    return call<Frame>('editor_command', { id, cmd });
  }

  save(id: number, req: SaveRequest = {}): Promise<DocInfo> {
    return call<DocInfo>('save_document', { id, req });
  }

  startupPaths(): Promise<string[]> {
    return call<string[]>('startup_paths');
  }

  reload(id: number): Promise<DocInfo> {
    return call<DocInfo>('reload_document', { id });
  }

  diskState(id: number): Promise<DiskState> {
    return call<DiskState>('disk_state', { id });
  }

  countMatches(id: number, query: Query): Promise<MatchCount> {
    return call<MatchCount>('count_matches', { id, query });
  }
}
