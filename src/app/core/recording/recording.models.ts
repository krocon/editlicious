import { Frame } from '../editor/editor.models';

/** DTOs of recordings and playback (design 015). Mirror: Rust `src-tauri/src/playback.rs`,
 * `ed-core/src/editor/playback.rs`. */

/** How fast a playback types (Rust `Tempo`). */
export type Tempo =
  | { mode: 'original' }
  | { mode: 'instant' }
  | { mode: 'factor'; factor: number }
  | { mode: 'even'; ms: number }
  | { mode: 'human'; cpm: number; seed: number; typos: number };

/** What to play. */
export type PlaybackSource = { kind: 'path'; path: string } | { kind: 'last' };

export interface RecordingInfo {
  steps: number;
  durationMs: number;
  dropped: number;
}

export interface PlaybackCheck {
  kind: 'recording' | 'script';
  steps: number;
  durationMs: number;
  /** Recorded on another text – ask before playing. */
  otherText: boolean;
}

export type PlaybackEvent =
  | { kind: 'frame'; frame: Frame }
  | { kind: 'progress'; done: number; total: number; paused: boolean }
  /** `reason`: `finished`, `stopped` or an error code. */
  | { kind: 'done'; reason: string };

export type PlaybackAction = 'pause' | 'resume' | 'stop' | 'step';

/** Video export (design 017; Rust `export.rs`). */
export interface ExportOptions {
  /** `.mp4`/`.mov`/`.webm` (ffmpeg) or a folder for PNG frames. */
  path: string;
  width: number;
  height: number;
  fps?: number;
  /** Colour scheme of the app (`paper`, `ink`, …). */
  theme: string;
  fontSize?: number | null;
  chrome?: 'none' | 'plain' | 'mac';
  best?: boolean;
  tempo?: Tempo | null;
  /** Empty = `ffmpeg` on the PATH. */
  ffmpeg?: string;
}

export type ExportEvent =
  | { kind: 'progress'; done: number; total: number }
  /** `reason`: `finished`, `cancelled` or an error code. */
  | { kind: 'done'; reason: string };
