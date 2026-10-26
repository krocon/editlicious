import { DestroyRef, Injectable, effect, inject, signal, untracked } from '@angular/core';
import { ACTIONS } from '../../core/actions/action-catalog';
import { ActionService } from '../../core/actions/action.service';
import { language } from '../../core/i18n/i18n';
import { McpApi, McpEvent, McpInfo } from '../../core/mcp/mcp-api';
import { DocumentsStore } from '../documents/documents.store';
import { RecordingStore } from '../recording/recording.store';
import { SettingsStore } from '../settings/settings.store';

/**
 * AI control in the UI (design 016 E8/E9): turns the bridge on and off with the setting, tells
 * the engine the active document and the actions, and carries out what the AI asks – new tabs,
 * switching, drawing again, running actions, showing its playbacks.
 */
@Injectable({ providedIn: 'root' })
export class McpStore {
  private readonly api = inject(McpApi);
  private readonly docs = inject(DocumentsStore);
  private readonly rec = inject(RecordingStore);
  private readonly settings = inject(SettingsStore);
  private readonly actions = inject(ActionService);

  readonly info = signal<McpInfo>({ enabled: false, connections: 0, sidecar: null });
  readonly connections = signal(0);
  private started = false;

  /** Starts following the setting and the events (once, from the app root). */
  start(destroyRef: DestroyRef): void {
    if (this.started) return;
    this.started = true;
    void this.api
      .onEvent((e) => this.onEvent(e))
      .then((off) => destroyRef.onDestroy(off))
      .catch(() => undefined);
    effect(() => {
      const on = this.settings.settings().mcp;
      untracked(() => void this.enable(on));
    });
    effect(() => {
      const id = this.docs.activeId();
      untracked(() => void this.api.setActive(id).catch(() => undefined));
    });
    // the actions the AI may run: everything in the palette, labels in the UI language
    effect(() => {
      language();
      this.actions.registered();
      untracked(() => {
        const list: [string, string][] = ACTIONS.filter(
          (a) => a.palette !== false && !a.id.startsWith('mcp.'),
        ).map((a) => [a.id, this.actions.label(a.id)]);
        void this.api.setActions(list).catch(() => undefined);
      });
    });
    this.actions.registerAll(
      {
        'mcp.disconnect': {
          run: () => void this.disconnect(),
          enabled: () => this.connections() > 0 || !!this.rec.playback()?.ai,
        },
      },
      destroyRef,
    );
  }

  async enable(on: boolean): Promise<void> {
    try {
      const info = await this.api.enable(on);
      this.info.set(info);
      this.connections.set(info.connections);
    } catch {
      this.info.update((i) => ({ ...i, enabled: false }));
    }
  }

  async refresh(): Promise<void> {
    const info = await this.api.info().catch(() => null);
    if (info) {
      this.info.set(info);
      this.connections.set(info.connections);
    }
  }

  async disconnect(): Promise<void> {
    await this.api.disconnect().catch(() => undefined);
  }

  /** The line for Claude Code. */
  command(): string {
    const p = this.info().sidecar;
    return p ? `claude mcp add editlicious -- "${p}"` : '';
  }

  private onEvent(e: McpEvent): void {
    switch (e.kind) {
      case 'opened':
        this.docs.adopt(e.info, e.activate);
        break;
      case 'activate':
        this.docs.activate(e.id);
        break;
      case 'changed':
        if (this.docs.activeId() === e.id) this.docs.reloaded.update((n) => n + 1);
        break;
      case 'action':
        this.docs.activate(e.id);
        // let the editor of that document come up before its action runs
        setTimeout(() => this.actions.run(e.action));
        break;
      case 'recording':
        this.rec.setRecording(e.id, e.on);
        break;
      case 'playback':
        this.rec.external(e.id, e.event);
        break;
      case 'status':
        this.connections.set(e.connections);
        this.info.update((i) => ({ ...i, connections: e.connections }));
        break;
    }
  }
}
