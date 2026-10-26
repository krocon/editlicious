import { McpApi } from '../../core/mcp/mcp-api';
import { FakeMcpApi } from '../../../testing/fake-mcp-api';
import { TestBed } from '@angular/core/testing';
import { ActionService } from '../../core/actions/action.service';
import { OS } from '../../core/actions/platform';
import { docInfo, textFrame } from '../../../testing/fake-editor-api';
import { formatSize, StatusBar } from './status-bar';
import { EditorApi } from '../../core/editor/editor-api';
import { RecordingApi } from '../../core/recording/recording-api';
import { SettingsApi } from '../../core/settings/settings-api';
import { WorkspaceApi } from '../../core/workspace/workspace-api';
import { FakeEditorApi } from '../../../testing/fake-editor-api';
import { FakeRecordingApi } from '../../../testing/fake-recording-api';
import { FakeSettingsApi } from '../../../testing/fake-settings-api';
import { FakeWorkspaceApi } from '../../../testing/fake-workspace-api';
import { RecordingStore } from '../recording/recording.store';

const PROVIDERS = [
  { provide: OS, useValue: 'other' },
  { provide: EditorApi, useClass: FakeEditorApi },
  { provide: RecordingApi, useClass: FakeRecordingApi },
  { provide: SettingsApi, useClass: FakeSettingsApi },
  { provide: McpApi, useClass: FakeMcpApi },
  { provide: WorkspaceApi, useClass: FakeWorkspaceApi },
];

describe('StatusBar', () => {
  it('shows position, selection and file facts; the buttons run actions', async () => {
    TestBed.configureTestingModule({
      imports: [StatusBar],
      providers: PROVIDERS,
    });
    const fixture = TestBed.createComponent(StatusBar);
    const frame = textFrame(['hello'], {
      caretLine: 4,
      carets: [{ line: 4, cell: 7 }],
      selChars: 12,
      selLines: 2,
      language: 'Rust',
      overwrite: true,
      info: docInfo({
        encoding: 'windows-1252',
        lineEnding: 'crlf',
        lineCount: 12345,
        fileSize: 1_234_567,
      }),
    });
    fixture.componentRef.setInput('frame', frame);
    await fixture.whenStable();
    const text = (fixture.nativeElement as HTMLElement).textContent!.replace(/\s+/g, ' ');
    expect(text).toContain('Ln 5, Col 8');
    expect(text).toContain('12 selected (2 lines)');
    expect(text).toContain('OVR');
    expect(text).toContain('12,345 lines');
    expect(text).toContain('1.2 MB');
    expect(text).toContain('Rust');
    expect(text).toContain('windows-1252');
    expect(text).toContain('CRLF');

    const actions = TestBed.inject(ActionService);
    const ran: string[] = [];
    vi.spyOn(actions, 'run').mockImplementation((id) => {
      ran.push(id);
      return true;
    });
    const buttons = (fixture.nativeElement as HTMLElement).querySelectorAll('button');
    buttons.forEach((b) => b.click());
    expect(ran).toEqual(['view.language', 'file.encoding', 'file.lineEnding']);
  });

  it('formats sizes', () => {
    expect(formatSize(999)).toBe('999 B');
    expect(formatSize(1500)).toBe('1.5 KB');
    expect(formatSize(123_456_789)).toBe('123 MB');
  });
});

describe('StatusBar while recording and playing (design 015 E11)', () => {
  it('shows REC and the progress of a playback', async () => {
    TestBed.configureTestingModule({ imports: [StatusBar], providers: PROVIDERS });
    const rec = TestBed.inject(RecordingStore);
    const fixture = TestBed.createComponent(StatusBar);
    fixture.componentRef.setInput('frame', textFrame(['x'], { info: docInfo({ id: 3 }) }));
    rec.recording.set(new Set([3]));
    await fixture.whenStable();
    const el = fixture.nativeElement as HTMLElement;
    expect(el.querySelector('.rec')?.textContent).toContain('REC');
    rec.recording.set(new Set());
    rec.playback.set({ docId: 3, done: 37, total: 100, paused: false });
    await fixture.whenStable();
    expect(el.textContent).toContain('▶ 37 %');
    expect(el.querySelector('.rec')).toBeNull();
  });
});
