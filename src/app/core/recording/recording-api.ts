import { Injectable } from '@angular/core';
import { Channel } from '@tauri-apps/api/core';
import { call } from '../backend/call';
import {
  PlaybackAction,
  PlaybackCheck,
  PlaybackEvent,
  PlaybackSource,
  RecordingInfo,
  Tempo,
} from './recording.models';

/**
 * Typed facade of recording and playback (design 015). The engine records and plays; frames
 * of a playback arrive as events. Tests use `FakeRecordingApi`.
 */
export abstract class RecordingApi {
  abstract start(id: number): Promise<void>;
  /** `null` when no recording ran. */
  abstract stop(id: number): Promise<RecordingInfo | null>;
  /** The last recording as `*.edrec.jsonl` or (`script`) as `*.edscript`. */
  abstract save(id: number, path: string, script: boolean): Promise<void>;
  abstract check(id: number, source: PlaybackSource): Promise<PlaybackCheck>;
  /** Starts playing; events until `done`. `tempo` null = as the source says. */
  abstract play(
    id: number,
    source: PlaybackSource,
    tempo: Tempo | null,
    onEvent: (e: PlaybackEvent) => void,
  ): Promise<void>;
  abstract control(id: number, action: PlaybackAction): Promise<void>;
}

@Injectable()
export class TauriRecordingApi extends RecordingApi {
  start(id: number): Promise<void> {
    return call<void>('start_recording', { id });
  }

  stop(id: number): Promise<RecordingInfo | null> {
    return call<RecordingInfo | null>('stop_recording', { id });
  }

  save(id: number, path: string, script: boolean): Promise<void> {
    return call<void>('save_recording', { id, path, script });
  }

  check(id: number, source: PlaybackSource): Promise<PlaybackCheck> {
    return call<PlaybackCheck>('check_playback', { id, source });
  }

  play(
    id: number,
    source: PlaybackSource,
    tempo: Tempo | null,
    onEvent: (e: PlaybackEvent) => void,
  ): Promise<void> {
    const ch = new Channel<PlaybackEvent>();
    ch.onmessage = onEvent;
    return call<void>('play', { id, source, tempo, onEvent: ch });
  }

  control(id: number, action: PlaybackAction): Promise<void> {
    return call<void>('playback_control', { id, action });
  }
}
