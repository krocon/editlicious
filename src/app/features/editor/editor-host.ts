import { InjectionToken, Signal } from '@angular/core';
import { inject } from '@angular/core';
import { McpStore } from '../mcp/mcp.store';
import { RecordingStore } from '../recording/recording.store';

/**
 * What the editor view takes from its surroundings (design 025 E7). The app gives the desktop
 * stores (root defaults); the embedded editor (`<ed-editor>`, `<editlicious-editor>`) gives
 * `null`/`true` per instance – no recording, no AI, no file.
 */
export const EDITOR_PLAYBACK = new InjectionToken<RecordingStore | null>('EDITOR_PLAYBACK', {
  providedIn: 'root',
  factory: () => inject(RecordingStore),
});

/** The AI connection shown in the status bar (design 016 E9); `null` = none. */
export const EDITOR_MCP = new InjectionToken<{ connections: Signal<number> } | null>('EDITOR_MCP', {
  providedIn: 'root',
  factory: () => inject(McpStore),
});

/** Embedded in a web page: no file behind the text (no "reopen with encoding"). */
export const EDITOR_EMBEDDED = new InjectionToken<boolean>('EDITOR_EMBEDDED', {
  providedIn: 'root',
  factory: () => false,
});

/**
 * The system clipboard in a browser (design 025 E6). The app leaves it out: there the engine
 * writes it itself (`arboard`).
 */
export interface ClipboardBridge {
  /**
   * Called in the same keystroke as the copy: `done` resolves when the engine has the text;
   * `html`: "copy as HTML" (text and HTML).
   */
  write(docId: number, html: boolean, done: Promise<unknown>): void;
  /** The text to paste, `null` = nothing (or not allowed). */
  read(): Promise<string | null>;
}

export const EDITOR_CLIPBOARD = new InjectionToken<ClipboardBridge | null>('EDITOR_CLIPBOARD', {
  providedIn: 'root',
  factory: () => null,
});
