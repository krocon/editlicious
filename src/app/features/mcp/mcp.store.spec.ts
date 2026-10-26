import { DestroyRef, Injector, runInInjectionContext } from '@angular/core';
import { TestBed } from '@angular/core/testing';
import { ActionService } from '../../core/actions/action.service';
import { OS } from '../../core/actions/platform';
import { EditorApi } from '../../core/editor/editor-api';
import { McpApi } from '../../core/mcp/mcp-api';
import { RecordingApi } from '../../core/recording/recording-api';
import { SettingsApi } from '../../core/settings/settings-api';
import { WorkspaceApi } from '../../core/workspace/workspace-api';
import { docInfo, FakeEditorApi, textFrame } from '../../../testing/fake-editor-api';
import { FakeMcpApi } from '../../../testing/fake-mcp-api';
import { FakeRecordingApi } from '../../../testing/fake-recording-api';
import { FakeSettingsApi } from '../../../testing/fake-settings-api';
import { FakeWorkspaceApi } from '../../../testing/fake-workspace-api';
import { DocumentsStore } from '../documents/documents.store';
import { RecordingStore } from '../recording/recording.store';
import { SettingsStore } from '../settings/settings.store';
import { McpStore } from './mcp.store';

const flush = () => new Promise((r) => setTimeout(r, 0));

async function setup() {
  const api = new FakeMcpApi();
  TestBed.configureTestingModule({
    providers: [
      { provide: McpApi, useValue: api },
      { provide: SettingsApi, useClass: FakeSettingsApi },
      { provide: EditorApi, useClass: FakeEditorApi },
      { provide: RecordingApi, useClass: FakeRecordingApi },
      { provide: WorkspaceApi, useClass: FakeWorkspaceApi },
      { provide: OS, useValue: 'other' },
    ],
  });
  const docs = TestBed.inject(DocumentsStore);
  await docs.start();
  const settings = TestBed.inject(SettingsStore);
  await settings.load();
  const store = TestBed.inject(McpStore);
  runInInjectionContext(TestBed.inject(Injector), () => store.start(TestBed.inject(DestroyRef)));
  TestBed.tick();
  await flush();
  return { api, docs, settings, store, actions: TestBed.inject(ActionService) };
}

describe('McpStore (design 016)', () => {
  it('follows the setting, the active document and tells the actions', async () => {
    const { api, docs, settings, store } = await setup();
    expect(api.enabled).toBe(false);
    settings.update({ mcp: true });
    TestBed.tick();
    await flush();
    expect(api.enabled).toBe(true);
    expect(store.command()).toBe(
      'claude mcp add editlicious -- "/opt/editlicious/editlicious-mcp"',
    );
    expect(api.active).toBe(docs.activeId());
    expect(api.actions.some(([id]) => id === 'edit.sortAsc')).toBe(true);
    expect(api.actions.some(([id]) => id.startsWith('mcp.'))).toBe(false);
  });

  it('carries out what the AI asks: tabs, actions, playback, status', async () => {
    const { api, docs, actions } = await setup();
    const before = docs.docs().length;
    api.emit({
      kind: 'opened',
      info: docInfo({ id: 42, name: 'ai.txt', path: '/x/ai.txt' }),
      activate: true,
    });
    expect(docs.docs().length).toBe(before + 1);
    expect(docs.activeId()).toBe(42);
    api.emit({ kind: 'activate', id: docs.docs()[0].id });
    expect(docs.activeId()).toBe(docs.docs()[0].id);
    const reloaded = docs.reloaded();
    api.emit({ kind: 'changed', id: docs.activeId()! });
    expect(docs.reloaded()).toBe(reloaded + 1);
    const ran = vi.spyOn(actions, 'run');
    api.emit({ kind: 'action', id: 42, action: 'edit.sortAsc' });
    await flush();
    expect(docs.activeId()).toBe(42);
    expect(ran).toHaveBeenCalledWith('edit.sortAsc');
    const rec = TestBed.inject(RecordingStore);
    api.emit({ kind: 'playback', id: 42, event: { kind: 'frame', frame: textFrame(['x']) } });
    expect(rec.playback()?.ai).toBe(true);
    api.emit({ kind: 'playback', id: 42, event: { kind: 'done', reason: 'finished' } });
    expect(rec.playback()).toBeNull();
    api.emit({ kind: 'recording', id: 42, on: true });
    expect(rec.isRecording(42)).toBe(true);
    api.emit({ kind: 'status', connections: 1 });
    expect(actions.enabled('mcp.disconnect')).toBe(true);
    actions.run('mcp.disconnect');
    await flush();
    expect(api.disconnects).toBe(1);
  });
});
