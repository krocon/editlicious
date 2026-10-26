import { Injectable } from '@angular/core';
import { McpApi, McpEvent, McpInfo } from '../app/core/mcp/mcp-api';

/** In-memory AI control for tests: `emit` plays the backend. */
@Injectable()
export class FakeMcpApi extends McpApi {
  enabled = false;
  connections = 0;
  sidecar: string | null = '/opt/editlicious/editlicious-mcp';
  actions: [string, string][] = [];
  active: number | null = null;
  disconnects = 0;
  private listener: ((e: McpEvent) => void) | null = null;

  enable(enabled: boolean): Promise<McpInfo> {
    this.enabled = enabled;
    return this.info();
  }
  info(): Promise<McpInfo> {
    return Promise.resolve({
      enabled: this.enabled,
      connections: this.connections,
      sidecar: this.sidecar,
    });
  }
  setActions(actions: [string, string][]): Promise<void> {
    this.actions = actions;
    return Promise.resolve();
  }
  setActive(id: number | null): Promise<void> {
    this.active = id;
    return Promise.resolve();
  }
  disconnect(): Promise<void> {
    this.disconnects++;
    return Promise.resolve();
  }
  onEvent(fn: (e: McpEvent) => void): Promise<() => void> {
    this.listener = fn;
    return Promise.resolve(() => (this.listener = null));
  }
  emit(e: McpEvent): void {
    this.listener?.(e);
  }
}
