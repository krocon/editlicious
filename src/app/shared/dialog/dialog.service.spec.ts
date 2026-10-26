import { TestBed } from '@angular/core/testing';
import { DialogService } from './dialog.service';

describe('DialogService', () => {
  it('gives the focus back – after a chain of dialogs to where it was before the first', async () => {
    const editor = document.createElement('textarea');
    const other = document.createElement('input');
    document.body.append(editor, other);
    editor.focus();
    const d = TestBed.inject(DialogService);
    d.show('pick');
    other.focus(); // the dialog's own input
    d.close();
    d.show('pick'); // right away the next question
    await new Promise((r) => setTimeout(r));
    expect(document.activeElement).toBe(other);
    d.close();
    await new Promise((r) => setTimeout(r));
    expect(document.activeElement).toBe(editor);
    editor.remove();
    other.remove();
  });
});
