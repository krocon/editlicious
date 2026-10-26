import { RecordingApi } from './core/recording/recording-api';
import { FakeRecordingApi } from '../testing/fake-recording-api';
import { SettingsApi } from './core/settings/settings-api';
import { FakeSettingsApi } from '../testing/fake-settings-api';
import { TestBed } from '@angular/core/testing';
import { App } from './app';
import { AboutApi } from './core/about/about-api';
import { DESKTOP } from './core/backend/desktop';
import { EditorApi } from './core/editor/editor-api';
import { FakeAboutApi } from '../testing/fake-about-api';
import { FakeEditorApi } from '../testing/fake-editor-api';
import { WorkspaceApi } from './core/workspace/workspace-api';
import { FakeWorkspaceApi } from '../testing/fake-workspace-api';
import { OS } from './core/actions/platform';
import { ActionService } from './core/actions/action.service';

function setup(desktop: boolean, paths: string[] = []): FakeEditorApi {
  const api = new FakeEditorApi();
  api.paths = paths;
  TestBed.configureTestingModule({
    imports: [App],
    providers: [
      { provide: SettingsApi, useClass: FakeSettingsApi },
      { provide: AboutApi, useClass: FakeAboutApi },
      { provide: EditorApi, useValue: api },
      { provide: RecordingApi, useClass: FakeRecordingApi },
      { provide: WorkspaceApi, useClass: FakeWorkspaceApi },
      { provide: DESKTOP, useValue: desktop },
      { provide: OS, useValue: 'other' },
    ],
  });
  return api;
}

describe('App', () => {
  it('without the desktop backend: welcome screen with About', async () => {
    setup(false);
    const fixture = TestBed.createComponent(App);
    await fixture.whenStable();
    const el = fixture.nativeElement as HTMLElement;
    expect(el.querySelector('app-editor-view')).toBeNull();
    el.querySelector<HTMLButtonElement>('button.about')!.click();
    await fixture.whenStable();
    expect(el.querySelector('[role="dialog"]')?.textContent).toContain('0.0.0-test');
  });

  it('opens the file from the command line', async () => {
    const api = setup(true, ['/tmp/a.txt']);
    const fixture = TestBed.createComponent(App);
    await fixture.whenStable();
    expect(api.opened).toEqual(['/tmp/a.txt']);
    expect(fixture.nativeElement.querySelector('app-editor-view')).not.toBeNull();
  });

  it('opens files the system hands over while running (design 012 E3)', async () => {
    const api = setup(true);
    const fixture = TestBed.createComponent(App);
    await fixture.whenStable();
    const ws = TestBed.inject(WorkspaceApi) as FakeWorkspaceApi;
    ws.openFiles?.(['/x/b.log']);
    await fixture.whenStable();
    await new Promise((r) => setTimeout(r));
    expect(api.opened).toEqual(['/x/b.log']);
  });

  it('starts with an empty document', async () => {
    setup(true);
    const fixture = TestBed.createComponent(App);
    await fixture.whenStable();
    expect(fixture.nativeElement.querySelector('app-editor-view')).not.toBeNull();
  });

  it('keeps the menu bar hidden until Alt and runs actions from the palette', async () => {
    setup(true);
    const fixture = TestBed.createComponent(App);
    await fixture.whenStable();
    const el = fixture.nativeElement as HTMLElement;
    expect(el.querySelector('app-menu-bar')).toBeNull();
    const actions = TestBed.inject(ActionService);
    actions.run('app.focusMenu');
    await fixture.whenStable();
    expect(el.querySelector('[role="menubar"]')).not.toBeNull();
    actions.run('view.toggleChrome');
    actions.run('app.palette');
    await fixture.whenStable();
    expect(el.querySelector('[role="combobox"]')).not.toBeNull();
    const options = [...el.querySelectorAll('[role="option"]')].map((o) => o.textContent ?? '');
    expect(options.some((o) => o.includes('Save') && o.includes('Ctrl+S'))).toBe(true);
    expect(options.some((o) => o.includes('Cursor left'))).toBe(false);
  });

  it('lists the keyboard shortcuts', async () => {
    setup(true);
    const fixture = TestBed.createComponent(App);
    await fixture.whenStable();
    TestBed.inject(ActionService).run('app.shortcuts');
    await fixture.whenStable();
    const text = (fixture.nativeElement as HTMLElement).querySelector(
      '[role="dialog"]',
    )!.textContent;
    expect(text).toContain('Undo');
    expect(text).toContain('Ctrl+Z');
  });

  it('F12 switches the colour scheme on the document', async () => {
    setup(true);
    const fixture = TestBed.createComponent(App);
    await fixture.whenStable();
    const actions = TestBed.inject(ActionService);
    actions.run('view.nextScheme');
    await fixture.whenStable();
    expect(document.documentElement.getAttribute('data-theme')).toBe('light');
    actions.run('view.scheme.night');
    await fixture.whenStable();
    expect(document.documentElement.getAttribute('data-scheme')).toBe('night');
    actions.run('view.scheme.auto');
    await fixture.whenStable();
    expect(document.documentElement.hasAttribute('data-theme')).toBe(false);
  });

  it('changes the language through the picker', async () => {
    const api = setup(true);
    const fixture = TestBed.createComponent(App);
    await fixture.whenStable();
    await new Promise((r) => setTimeout(r));
    await fixture.whenStable();
    TestBed.inject(ActionService).run('view.language');
    await new Promise((r) => setTimeout(r));
    await fixture.whenStable();
    const el = fixture.nativeElement as HTMLElement;
    const options = [...el.querySelectorAll('[role="option"]')].map((o) => o.textContent?.trim());
    expect(options).toEqual(['Plain text', 'JavaScript', 'Rust']);
    (el.querySelectorAll('[role="option"]')[2] as HTMLElement).click();
    await new Promise((r) => setTimeout(r));
    expect(api.sent('setLanguage')).toEqual([{ type: 'setLanguage', name: 'Rust' }]);
  });
});
