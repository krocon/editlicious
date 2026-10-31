import { Injectable } from '@angular/core';
import { listen } from '@tauri-apps/api/event';
import { getCurrentWebview } from '@tauri-apps/api/webview';
import { call } from '../backend/call';
import { PathCompletion, SessionFile } from './workspace.models';

/**
 * The app around the editor (design 004): native file dialogs, the session file, window title,
 * closing and dropped files. Tests use `FakeWorkspaceApi`.
 */
export abstract class WorkspaceApi {
  /** `near`: a file whose folder the dialog starts in. `null` = cancelled. */
  abstract pickOpenFiles(near?: string | null): Promise<string[] | null>;
  abstract pickSavePath(name: string, near?: string | null): Promise<string | null>;
  /** "Open path…" (prompt 024): the text the field starts with – the folder of `near`. */
  abstract openPathStart(near?: string | null): Promise<string>;
  /** The typed text (`~`, quotes, `file://`, relative to `near`) → an existing file. */
  abstract resolveOpenPath(input: string, near?: string | null): Promise<string>;
  /** Folder entries matching the typed text. */
  abstract completeOpenPath(input: string, near?: string | null): Promise<PathCompletion>;
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
  /** Files the system hands over while the app runs ("open with", a second start). */
  abstract onOpenFiles(fn: (paths: string[]) => void): Promise<() => void>;
}

@Injectable()
export class TauriWorkspaceApi extends WorkspaceApi {
  pickOpenFiles(near?: string | null): Promise<string[] | null> {
    return call<string[] | null>('pick_open_files', { near: near ?? null });
  }

  pickSavePath(name: string, near?: string | null): Promise<string | null> {
    return call<string | null>('pick_save_path', { name, near: near ?? null });
  }

  openPathStart(near?: string | null): Promise<string> {
    return call<string>('open_path_start', { near: near ?? null });
  }

  resolveOpenPath(input: string, near?: string | null): Promise<string> {
    return call<string>('resolve_open_path', { input, near: near ?? null });
  }

  completeOpenPath(input: string, near?: string | null): Promise<PathCompletion> {
    return call<PathCompletion>('complete_open_path', { input, near: near ?? null });
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

  onOpenFiles(fn: (paths: string[]) => void): Promise<() => void> {
    return listen<string[]>('open-files', (e) => fn(e.payload));
  }

  onFilesDropped(fn: (paths: string[]) => void): Promise<() => void> {
    return getCurrentWebview().onDragDropEvent((e) => {
      if (e.payload.type === 'drop' && e.payload.paths.length > 0) fn(e.payload.paths);
    });
  }
}
