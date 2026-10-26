import { Injectable } from '@angular/core';
import { RecordingApi } from '../app/core/recording/recording-api';
import {
  ExportEvent,
  ExportOptions,
  PlaybackAction,
  PlaybackCheck,
  PlaybackEvent,
  PlaybackSource,
  RecordingInfo,
  Tempo,
} from '../app/core/recording/recording.models';

/** In-memory recording facade for tests: remembers calls, lets the test send events. */
@Injectable()
export class FakeRecordingApi extends RecordingApi {
  readonly calls: string[] = [];
  info: RecordingInfo | null = { steps: 3, durationMs: 1500, dropped: 0 };
  checkAnswer: PlaybackCheck = { kind: 'script', steps: 3, durationMs: 900, otherText: false };
  lastPlay: { id: number; source: PlaybackSource; tempo: Tempo | null } | null = null;
  emit: ((e: PlaybackEvent) => void) | null = null;

  start(id: number): Promise<void> {
    this.calls.push(`start:${id}`);
    return Promise.resolve();
  }
  stop(id: number): Promise<RecordingInfo | null> {
    this.calls.push(`stop:${id}`);
    return Promise.resolve(this.info);
  }
  save(id: number, path: string, script: boolean): Promise<void> {
    this.calls.push(`save:${id}:${path}:${script}`);
    return Promise.resolve();
  }
  check(): Promise<PlaybackCheck> {
    return Promise.resolve(this.checkAnswer);
  }
  play(
    id: number,
    source: PlaybackSource,
    tempo: Tempo | null,
    onEvent: (e: PlaybackEvent) => void,
  ): Promise<void> {
    this.lastPlay = { id, source, tempo };
    this.emit = onEvent;
    return Promise.resolve();
  }
  control(id: number, action: PlaybackAction): Promise<void> {
    this.calls.push(`${action}:${id}`);
    return Promise.resolve();
  }
  lastExport: { id: number; source: PlaybackSource; options: ExportOptions } | null = null;
  exportEmit: ((e: ExportEvent) => void) | null = null;
  exportVideo(
    id: number,
    source: PlaybackSource,
    options: ExportOptions,
    onEvent: (e: ExportEvent) => void,
  ): Promise<void> {
    this.lastExport = { id, source, options };
    this.exportEmit = onEvent;
    return Promise.resolve();
  }
  cancelExport(): Promise<void> {
    this.calls.push('cancelExport');
    return Promise.resolve();
  }
}
