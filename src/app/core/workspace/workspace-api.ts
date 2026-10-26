import { Injectable } from '@angular/core';
import { listen } from '@tauri-apps/api/event';
import { getCurrentWebview } from '@tauri-apps/api/webview';
import { call } from '../backend/call';
import { SessionFile } from './workspace.models';

/**
 * The app around the editor (design 004): native file dialogs, the session file, window title,
 * closing and dropped files. Tests use `FakeWorkspaceApi`.
 */
export abstract class WorkspaceApi {
  /** `near`: a file whose folder the dialog starts in. `null` = cancelled. */
  abstract pickOpenFiles(near?: string | null): Promise<string[] | null>;
  abstract pickSavePath(name: string, near?: string | null): Promise<string | null>;
  abstract loadSession(): Promise<SessionFile>;
  abstract saveSession(session: SessionFile): Promise<void>;
  abstract setTitle(title: string): Promise<void>;
  /** From now on the close button asks via `onCloseRequested`. */
  abstract takeOverClose(): Promise<void>;
  abstract quit(): Promise<void>;
  /** Returns the new state. */
  abstract toggleFullscreen(): Promise<boolean>;
  abstract onCloseRequested(fn: () => void): Promise<() => void>;
  abstract onFilesDropped(fn: (paths: string[]) => void): Promise<() => void>;
}

@Injectable()
export class TauriWorkspaceApi extends WorkspaceApi {
  pickOpenFiles(near?: string | null): Promise<string[] | null> {
    return call<string[] | null>('pick_open_files', { near: near ?? null });
  }

  pickSavePath(name: string, near?: string | null): Promise<string | null> {
    return call<string | null>('pick_save_path', { name, near: near ?? null });
  }

  loadSession(): Promise<SessionFile> {
    return call<SessionFile>('load_session');
  }

  saveSession(session: SessionFile): Promise<void> {
    return call<void>('save_session', { session });
  }

  setTitle(title: string): Promise<void> {
    return call<void>('set_title', { title });
  }

  takeOverClose(): Promise<void> {
    return call<void>('take_over_close');
  }

  quit(): Promise<void> {
    return call<void>('quit_app');
  }

  toggleFullscreen(): Promise<boolean> {
    return call<boolean>('toggle_fullscreen');
  }

  onCloseRequested(fn: () => void): Promise<() => void> {
    return listen('close-requested', () => fn());
  }

  onFilesDropped(fn: (paths: string[]) => void): Promise<() => void> {
    return getCurrentWebview().onDragDropEvent((e) => {
      if (e.payload.type === 'drop' && e.payload.paths.length > 0) fn(e.payload.paths);
    });
  }
}
