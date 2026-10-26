import { TestBed } from '@angular/core/testing';
import { ActionService } from '../../core/actions/action.service';
import { OS } from '../../core/actions/platform';
import { docInfo, textFrame } from '../../../testing/fake-editor-api';
import { formatSize, StatusBar } from './status-bar';

describe('StatusBar', () => {
  it('shows position, selection and file facts; the buttons run actions', async () => {
    TestBed.configureTestingModule({
      imports: [StatusBar],
      providers: [{ provide: OS, useValue: 'other' }],
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
