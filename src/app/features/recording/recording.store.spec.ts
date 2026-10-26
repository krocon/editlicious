import { DestroyRef } from '@angular/core';
import { TestBed } from '@angular/core/testing';
import { ActionService } from '../../core/actions/action.service';
import { OS } from '../../core/actions/platform';
import { EditorApi } from '../../core/editor/editor-api';
import { RecordingApi } from '../../core/recording/recording-api';
import { SettingsApi } from '../../core/settings/settings-api';
import { WorkspaceApi } from '../../core/workspace/workspace-api';
import { AskService } from '../../shared/ask/ask.service';
import { PickService } from '../../shared/pick/pick.service';
import { FakeEditorApi, textFrame } from '../../../testing/fake-editor-api';
import { FakeRecordingApi } from '../../../testing/fake-recording-api';
import { FakeSettingsApi } from '../../../testing/fake-settings-api';
import { FakeWorkspaceApi } from '../../../testing/fake-workspace-api';
import { DocumentsStore } from '../documents/documents.store';
import { RecordingStore } from './recording.store';

async function setup() {
  const api = new FakeRecordingApi();
  const ws = new FakeWorkspaceApi();
  TestBed.configureTestingModule({
    providers: [
      { provide: SettingsApi, useClass: FakeSettingsApi },
      { provide: EditorApi, useClass: FakeEditorApi },
      { provide: RecordingApi, useValue: api },
      { provide: WorkspaceApi, useValue: ws },
      { provide: OS, useValue: 'other' },
    ],
  });
  const docs = TestBed.inject(DocumentsStore);
  await docs.start();
  const store = TestBed.inject(RecordingStore);
  const actions = TestBed.inject(ActionService);
  store.registerActions(actions, TestBed.inject(DestroyRef));
  const id = docs.activeId()!;
  return { api, ws, docs, store, actions, id };
}

const flush = () => new Promise((r) => setTimeout(r, 0));

describe('RecordingStore (design 015)', () => {
  afterEach(() => vi.restoreAllMocks());

  it('starts and stops a recording of the active document', async () => {
    const { api, store, actions, docs, id } = await setup();
    expect(actions.enabled('record.save')).toBe(false);
    actions.run('record.toggle');
    await flush();
    expect(api.calls).toEqual([`start:${id}`]);
    expect(store.isRecording(id)).toBe(true);
    expect(actions.checked('record.toggle')).toBe(true);
    expect(actions.enabled('record.play')).toBe(false);
    actions.run('record.toggle');
    await flush();
    expect(store.isRecording(id)).toBe(false);
    expect(docs.message()).toContain('3');
    expect(actions.enabled('record.save')).toBe(true);
    expect(actions.enabled('record.replay')).toBe(true);
  });

  it('plays a file: other text asks, the tempo is chosen, frames and progress arrive', async () => {
    const { api, ws, store, id } = await setup();
    ws.openAnswer = ['/tmp/demo.edscript'];
    api.checkAnswer = { kind: 'recording', steps: 3, durationMs: 9, otherText: true };
    const asked = vi.spyOn(TestBed.inject(AskService), 'ask').mockResolvedValue('play');
    vi.spyOn(TestBed.inject(PickService), 'pick').mockResolvedValue('human');
    await store.playFile();
    expect(asked).toHaveBeenCalledOnce();
    expect(api.lastPlay).toEqual({
      id,
      source: { kind: 'path', path: '/tmp/demo.edscript' },
      tempo: { mode: 'human', cpm: 330, seed: 1, typos: 0 },
    });
    expect(store.isPlaying(id)).toBe(true);
    api.emit!({ kind: 'progress', done: 1, total: 4, paused: false });
    expect(store.playback()).toEqual({ docId: id, done: 1, total: 4, paused: false });
    const f = textFrame(['x']);
    api.emit!({ kind: 'frame', frame: f });
    expect(store.pushed()).toBe(f);
    await store.pauseResume();
    expect(api.calls.at(-1)).toBe(`pause:${id}`);
    expect(store.playback()?.paused).toBe(true);
    await store.stopPlayback();
    expect(api.calls.at(-1)).toBe(`stop:${id}`);
    api.emit!({ kind: 'done', reason: 'stopped' });
    expect(store.playback()).toBeNull();
  });

  it('does not play when the question about another text is declined', async () => {
    const { api, ws, store } = await setup();
    ws.openAnswer = ['/tmp/r.edrec.jsonl'];
    api.checkAnswer = { kind: 'recording', steps: 3, durationMs: 9, otherText: true };
    vi.spyOn(TestBed.inject(AskService), 'ask').mockResolvedValue('cancel');
    await store.playFile();
    expect(api.lastPlay).toBeNull();
  });

  it('saves the last recording as a recording or a script', async () => {
    const { api, ws, store, id } = await setup();
    ws.saveAnswer = '/tmp/out.edscript';
    await store.save(true);
    expect(api.calls).toEqual([`save:${id}:/tmp/out.edscript:true`]);
  });

  it('shows an error text when a playback ends with an error', async () => {
    const { api, store, docs } = await setup();
    await store.play({ kind: 'last' }, { mode: 'instant' });
    api.emit!({ kind: 'done', reason: 'recording_unsupported' });
    expect(docs.message()).toContain('cannot be played');
  });
});
