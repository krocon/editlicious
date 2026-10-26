import { Injectable } from '@angular/core';
import { listen } from '@tauri-apps/api/event';
import { call } from '../backend/call';
import { DocInfo } from '../editor/editor.models';
import { PlaybackEvent } from '../recording/recording.models';

/** AI control as the settings dialog shows it (Rust `McpInfo`, design 016). */
export interface McpInfo {
  enabled: boolean;
  connections: number;
  /** The sidecar program next to the app, `null` = missing. */
  sidecar: string | null;
}

/** What the AI tools tell the UI (design 016 E8). */
export type McpEvent =
  | { kind: 'opened'; info: DocInfo; activate: boolean }
  | { kind: 'activate'; id: number }
  | { kind: 'changed'; id: number }
  | { kind: 'action'; id: number; action: string }
  | { kind: 'recording'; id: number; on: boolean }
  | { kind: 'playback'; id: number; event: PlaybackEvent }
  | { kind: 'status'; connections: number };

/** Typed facade of AI control (design 016). Tests use `FakeMcpApi`. */
export abstract class McpApi {
  abstract enable(enabled: boolean): Promise<McpInfo>;
  abstract info(): Promise<McpInfo>;
  abstract setActions(actions: [string, string][]): Promise<void>;
  abstract setActive(id: number | null): Promise<void>;
  abstract disconnect(): Promise<void>;
  /** All AI events; returns the function that stops listening. */
  abstract onEvent(fn: (e: McpEvent) => void): Promise<() => void>;
}

const EVENTS = [
  'opened',
  'activate',
  'changed',
  'action',
  'recording',
  'playback',
  'status',
] as const;

@Injectable()
export class TauriMcpApi extends McpApi {
  enable(enabled: boolean): Promise<McpInfo> {
    return call<McpInfo>('mcp_enable', { enabled });
  }

  info(): Promise<McpInfo> {
    return call<McpInfo>('mcp_info');
  }

  setActions(actions: [string, string][]): Promise<void> {
    return call<void>('mcp_set_actions', { actions });
  }

  setActive(id: number | null): Promise<void> {
    return call<void>('mcp_set_active', { id });
  }

  disconnect(): Promise<void> {
    return call<void>('mcp_disconnect');
  }

  async onEvent(fn: (e: McpEvent) => void): Promise<() => void> {
    const offs = await Promise.all(
      EVENTS.map((kind) =>
        listen<object>(`mcp-${kind}`, (e) => fn({ kind, ...e.payload } as McpEvent)),
      ),
    );
    return () => offs.forEach((off) => off());
  }
}
