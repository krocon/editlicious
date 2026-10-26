import { SettingsApi } from '../../core/settings/settings-api';
import { FakeSettingsApi } from '../../../testing/fake-settings-api';
import { TestBed } from '@angular/core/testing';
import { EditorApi } from '../../core/editor/editor-api';
import { WorkspaceApi } from '../../core/workspace/workspace-api';
import { AskService } from '../../shared/ask/ask.service';
import { docInfo, FakeEditorApi, textFrame } from '../../../testing/fake-editor-api';
import { FakeWorkspaceApi } from '../../../testing/fake-workspace-api';
import { SettingsStore } from '../settings/settings.store';
import { DocumentsStore } from './documents.store';

function setup() {
  const api = new FakeEditorApi();
  const ws = new FakeWorkspaceApi();
  TestBed.configureTestingModule({
    providers: [
      { provide: SettingsApi, useClass: FakeSettingsApi },
      { provide: EditorApi, useValue: api },
      { provide: WorkspaceApi, useValue: ws },
    ],
  });
  const store = TestBed.inject(DocumentsStore);
  const ask = TestBed.inject(AskService);
  const answers: string[] = [];
  const asked: string[] = [];
  vi.spyOn(ask, 'ask').mockImplementation((q) => {
    asked.push(q.title);
    return Promise.resolve(answers.shift() ?? q.cancel);
  });
  return { api, ws, store, answers, asked };
}

/** Dirty in the store (last frame) and in the engine (what `info` answers). */
function markDirty(store: DocumentsStore, id: number, api?: FakeEditorApi) {
  const d = store.docs().find((x) => x.id === id)!;
  const info = { ...d.info, dirty: true };
  store.onFrame(textFrame(['x'], { info }));
  const fake = api ?? (TestBed.inject(EditorApi) as FakeEditorApi);
  fake.infos.set(id, info);
}

describe('DocumentsStore', () => {
  it('starts with an empty document', async () => {
    const { store } = setup();
    await store.start();
    expect(store.docs().length).toBe(1);
    expect(store.active()?.info.path).toBeNull();
  });

  it('opens command-line files and replaces the untouched empty document', async () => {
    const { store, api } = setup();
    await store.start();
    await store.openPaths(['/a/one.txt', '/a/two.txt', '/a/one.txt']);
    expect(api.opened).toEqual(['/a/one.txt', '/a/two.txt']);
    expect(store.docs().map((d) => d.info.path)).toEqual(['/a/one.txt', '/a/two.txt']);
    expect(store.active()?.info.path).toBe('/a/one.txt');
    expect(store.recent()[0]).toBe('/a/one.txt');
    expect(store.tabsVisible()).toBe(true);
  });

  it('restores the last session with caret and scroll', async () => {
    const { store, api, ws } = setup();
    ws.session = {
      version: 1,
      files: [
        { path: '/s/a.txt', line: 5, ch: 2, top: 3, markers: [null, 'row'] },
        { path: '/s/b.txt', line: 0, ch: 0, top: 0 },
      ],
      active: 1,
      recent: ['/s/a.txt'],
      tabs: 'show',
      findHistory: ['x'],
      replaceHistory: [],
    };
    await store.start();
    expect(store.docs().length).toBe(2);
    expect(store.active()?.info.path).toBe('/s/b.txt');
    expect(api.sent('goTo')[0]).toEqual({ type: 'goTo', line: 5, ch: 2 });
    expect(api.sent('scrollTo')[0]).toEqual({ type: 'scrollTo', top: 3 });
    // markers come back and go into the session again (design 019)
    expect(api.sent('setMarkers')).toEqual([{ type: 'setMarkers', words: [null, 'row'] }]);
    const a = store.docs()[0];
    store.onFrame(textFrame(['x'], { info: a.info, markerWords: [null, 'row'] }));
    await store.persist();
    expect(ws.savedSessions.at(-1)?.files[0].markers).toEqual([null, 'row']);
    expect(ws.savedSessions.at(-1)?.files[1].markers).toBeUndefined();
    expect(store.tabsMode()).toBe('show');
  });

  it('follows the settings: no session restore, line ending of new files', async () => {
    const { store, api, ws } = setup();
    const settings = TestBed.inject(SettingsStore);
    settings.update({ restoreSession: false, newLineEnding: 'crlf' });
    ws.session = {
      version: 1,
      files: [{ path: '/s/a.txt', line: 0, ch: 0, top: 0 }],
      active: 0,
      recent: ['/s/a.txt'],
      tabs: 'auto',
      findHistory: [],
      replaceHistory: [],
    };
    await store.start();
    expect(api.opened).toEqual([]);
    expect(store.recent()).toEqual(['/s/a.txt']);
    expect(api.newLineEndings).toEqual(['crlf']);
    expect(store.active()?.info.lineEnding).toBe('crlf');
  });

  it('reads again or converts in another encoding (design 010 E2)', async () => {
    const { store, api, answers, asked } = setup();
    await store.start();
    await store.openPaths(['/e/a.txt']);
    const id = store.activeId()!;
    answers.push('reopen');
    await store.changeEncoding(id, 'windows-1252');
    expect(api.reloads).toEqual([id]);
    expect(api.reloadEncodings).toEqual(['windows-1252']);
    expect(store.reloaded()).toBe(1);
    answers.push('convert');
    await store.changeEncoding(id, 'UTF-16LE');
    expect(api.saved.at(-1)).toEqual({ id, req: { encoding: 'UTF-16LE' } });
    answers.push('cancel');
    await store.changeEncoding(id, 'KOI8-R');
    expect(api.saved.length).toBe(1);
    expect(asked.length).toBe(3);
  });

  it('asks before overwriting changes made on disk (design 013)', async () => {
    const { store, api, answers, asked } = setup();
    await store.start();
    await store.openPaths(['/d/a.log']);
    const id = store.activeId()!;
    api.saveError = { code: 'changed_on_disk', message: 'changed', path: '/d/a.log' };
    answers.push('overwrite');
    expect(await store.save(id)).toBe(true);
    expect(asked).toEqual(['File changed on disk']);
    expect(api.saved.map((x) => x.req)).toEqual([{}, { force: true }]);
  });

  it('warns once before saving characters that could not be read', async () => {
    const { store, api, answers, asked } = setup();
    await store.start();
    await store.openPaths(['/d/b.csv']);
    const d = store.active()!;
    store.onFrame(textFrame(['x'], { info: { ...d.info, decodeErrors: true } }));
    answers.push('cancel');
    expect(await store.save(d.id)).toBe(false);
    expect(api.saved.length).toBe(0);
    answers.push('save');
    expect(await store.save(d.id)).toBe(true);
    expect(asked).toEqual(['Unreadable characters', 'Unreadable characters']);
  });

  it('asks the engine whether a file is unsaved, not only the last frame', async () => {
    const { store, api, answers, asked } = setup();
    await store.start();
    const id = store.activeId()!;
    // a running replace-all: the engine knows, the frame does not yet
    api.infos.set(id, { ...store.active()!.info, dirty: true });
    answers.push('cancel');
    expect(await store.close()).toBe(false);
    expect(asked).toEqual(['Unsaved changes']);
  });

  it('asks before closing unsaved changes', async () => {
    const { store, answers, asked } = setup();
    await store.start();
    const id = store.activeId()!;
    markDirty(store, id);
    answers.push('cancel');
    expect(await store.close()).toBe(false);
    expect(asked).toEqual(['Unsaved changes']);
    answers.push('discard');
    expect(await store.close()).toBe(true);
    expect(store.docs().length).toBe(1); // a fresh empty document
    expect(store.activeId()).not.toBe(id);
  });

  it('saves new documents through "Save as" and remembers the path', async () => {
    const { store, api, ws } = setup();
    await store.start();
    ws.saveAnswer = null;
    expect(await store.save()).toBe(false);
    ws.saveAnswer = '/out/new file.txt';
    expect(await store.save()).toBe(true);
    expect(api.saved.at(-1)?.req).toEqual({ path: '/out/new file.txt' });
    expect(store.active()?.info.path).toBe('/out/new file.txt');
    expect(store.recent()).toContain('/out/new file.txt');
  });

  it('offers UTF-8 when a character cannot be stored', async () => {
    const { store, api, answers } = setup();
    await store.openPaths(['/a/legacy.txt']);
    api.saveError = { code: 'unmappable_character', message: '2:4' };
    answers.push('utf8');
    expect(await store.save()).toBe(true);
    expect(api.saved.map((s) => s.req.encoding)).toEqual([undefined, 'UTF-8']);
  });

  it('quits after asking, keeping the session', async () => {
    const { store, ws, answers } = setup();
    await store.openPaths(['/a/x.txt']);
    markDirty(store, store.activeId()!);
    answers.push('cancel');
    await store.quit();
    expect(ws.quitCalls).toBe(0);
    answers.push('discard');
    await store.quit();
    expect(ws.quitCalls).toBe(1);
    expect(ws.savedSessions.at(-1)?.files.map((f) => f.path)).toEqual(['/a/x.txt']);
  });

  it('reloads unchanged files changed on disk and asks for edited ones', async () => {
    const { store, api, answers, asked } = setup();
    await store.openPaths(['/a/one.txt', '/a/two.txt']);
    const [one, two] = store.docs();
    api.disk.set(one.id, { changed: true, missing: false });
    api.disk.set(two.id, { changed: true, missing: false });
    markDirty(store, two.id);
    answers.push('keep');
    await store.checkDisk();
    expect(api.reloads).toEqual([one.id]);
    expect(asked).toEqual(['File changed on disk']);
    expect(store.reloaded()).toBe(1);
  });

  it('switches and reorders tabs', async () => {
    const { store } = setup();
    await store.openPaths(['/a', '/b', '/c']);
    store.activate(store.docs()[0].id);
    store.next(-1);
    expect(store.active()?.info.path).toBe('/c');
    store.move(store.docs()[2].id, 0);
    expect(store.docs().map((d) => d.info.path)).toEqual(['/c', '/a', '/b']);
    expect(docInfo().name).toBe('Untitled');
  });
});
