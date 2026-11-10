import { InjectionToken, Signal } from '@angular/core';
import { Command } from '../../core/editor/editor.models';
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

/** The suggestion list of a completion the surroundings provide (design 028 E4). */
export interface ExternalList {
  /** Line and cell the list hangs below: the start of the word. */
  line: number;
  cell: number;
  /** All matching entries; `items` are the rows on screen, from entry `top`. */
  total: number;
  top: number;
  /** The chosen entry (index among all). */
  active: number;
  items: { label: string; kind: string; detail: string }[];
  /** Documentation of the chosen entry. */
  doc: string;
}

/** What a completion from outside needs of the view. */
export interface ExternalCompletionView {
  docId(): number;
  /** Runs `fn` in the order of the view's commands, then draws. */
  enqueue(fn: () => Promise<unknown>): Promise<unknown>;
}

/**
 * Suggestions from outside the engine (design 028): the embedded editor asks a provider of the
 * page. The view draws `list` like the engine's own list and passes the keys on.
 */
export interface ExternalCompletion {
  /** There is something to ask (else the engine's word completion does everything). */
  readonly on: Signal<boolean>;
  readonly list: Signal<ExternalList | null>;
  bind(view: ExternalCompletionView): void;
  /** A command ran (its frame is shown). */
  after(cmd: Command): void;
  /** "Show suggestions". */
  invoke(): void;
  move(delta: number): void;
  /** The mouse wheel over the list. */
  scroll(rows: number): void;
  accept(index?: number): void;
  close(): void;
  /** Tab: the command that expands the snippet prefix before the caret, `null` = none. */
  tabCommand(): Promise<Command | null>;
}

export const EDITOR_COMPLETION = new InjectionToken<ExternalCompletion | null>(
  'EDITOR_COMPLETION',
  { providedIn: 'root', factory: () => null },
);
