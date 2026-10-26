import { Injectable } from '@angular/core';
import { WorkspaceApi } from '../app/core/workspace/workspace-api';
import { SessionFile } from '../app/core/workspace/workspace.models';

/** Test double of {@link WorkspaceApi}. Never imported by app code. */
@Injectable()
export class FakeWorkspaceApi extends WorkspaceApi {
  session: SessionFile = {
    version: 1,
    files: [],
    active: 0,
    recent: [],
    tabs: 'auto',
    findHistory: [],
    replaceHistory: [],
  };
  savedSessions: SessionFile[] = [];
  openAnswer: string[] | null = null;
  saveAnswer: string | null = null;
  titles: string[] = [];
  quitCalls = 0;

  pickOpenFiles(): Promise<string[] | null> {
    return Promise.resolve(this.openAnswer);
  }
  pickSavePath(): Promise<string | null> {
    return Promise.resolve(this.saveAnswer);
  }
  loadSession(): Promise<SessionFile> {
    return Promise.resolve(this.session);
  }
  saveSession(session: SessionFile): Promise<void> {
    this.savedSessions.push(session);
    return Promise.resolve();
  }
  setTitle(title: string): Promise<void> {
    this.titles.push(title);
    return Promise.resolve();
  }
  takeOverClose(): Promise<void> {
    return Promise.resolve();
  }
  quit(): Promise<void> {
    this.quitCalls++;
    return Promise.resolve();
  }
  fullscreen = false;
  toggleFullscreen(): Promise<boolean> {
    this.fullscreen = !this.fullscreen;
    return Promise.resolve(this.fullscreen);
  }
  onCloseRequested(): Promise<() => void> {
    return Promise.resolve(() => undefined);
  }
  onFilesDropped(): Promise<() => void> {
    return Promise.resolve(() => undefined);
  }

  /** The listener of `open-files` (tests call it). */
  openFiles: ((paths: string[]) => void) | null = null;

  onOpenFiles(fn: (paths: string[]) => void): Promise<() => void> {
    this.openFiles = fn;
    return Promise.resolve(() => (this.openFiles = null));
  }
}
