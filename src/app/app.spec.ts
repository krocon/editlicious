import { TestBed } from '@angular/core/testing';
import { App } from './app';
import { AboutApi } from './core/about/about-api';
import { DESKTOP } from './core/backend/desktop';
import { EditorApi } from './core/editor/editor-api';
import { FakeAboutApi } from '../testing/fake-about-api';
import { FakeEditorApi } from '../testing/fake-editor-api';
import { WorkspaceApi } from './core/workspace/workspace-api';
import { FakeWorkspaceApi } from '../testing/fake-workspace-api';

function setup(desktop: boolean, paths: string[] = []): FakeEditorApi {
  const api = new FakeEditorApi();
  api.paths = paths;
  TestBed.configureTestingModule({
    imports: [App],
    providers: [
      { provide: AboutApi, useClass: FakeAboutApi },
      { provide: EditorApi, useValue: api },
      { provide: WorkspaceApi, useClass: FakeWorkspaceApi },
      { provide: DESKTOP, useValue: desktop },
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

  it('starts with an empty document', async () => {
    setup(true);
    const fixture = TestBed.createComponent(App);
    await fixture.whenStable();
    expect(fixture.nativeElement.querySelector('app-editor-view')).not.toBeNull();
  });
});
