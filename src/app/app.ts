import { ChangeDetectionStrategy, Component, inject, signal } from '@angular/core';
import { AboutDialog } from './features/about/about-dialog';
import { EditorView } from './features/editor/editor-view';
import { EditorApi } from './core/editor/editor-api';
import { DESKTOP } from './core/backend/desktop';
import { describeFsError, toFsError } from './core/errors/fs-error';
import { DialogService } from './shared/dialog/dialog.service';

/**
 * Root: one editor surface over the whole window (tabs, menus and the rest follow in 004/005).
 * Nothing but the text should catch the eye (prompt 000-start: "wenig Ablenkung").
 */
@Component({
  selector: 'app-root',
  imports: [AboutDialog, EditorView],
  template: `
    @if (docId(); as id) {
      @if (progress() !== null) {
        <div class="progress" [style.width.%]="progress()! * 100"></div>
      }
      <app-editor-view class="editor" [docId]="id" />
    } @else {
      <main class="welcome">
        <img class="logo" src="logo.svg" alt="editlicious" />
        <p class="hint">{{ message() }}</p>
        <button type="button" class="btn quiet about" (click)="dialogs.show('about')">About</button>
      </main>
    }
    @switch (dialogs.open()) {
      @case ('about') {
        <app-about-dialog />
      }
    }
  `,
  styleUrl: './app.css',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class App {
  protected readonly dialogs = inject(DialogService);
  protected readonly docId = signal<number | null>(null);
  protected readonly progress = signal<number | null>(null);
  protected readonly message = signal('A calm text editor for huge files.');
  private readonly api = inject(EditorApi);

  constructor() {
    if (inject(DESKTOP)) {
      void this.start();
    }
  }

  /** The first file of the command line, else an empty document (more files: prompt 004). */
  private async start(): Promise<void> {
    try {
      const [path] = await this.api.startupPaths();
      if (path) {
        const loading = this.api.open(path, (e) => this.progress.set(e.loaded / (e.total || 1)));
        const info = await loading.done;
        this.progress.set(null);
        this.docId.set(info.id);
      } else {
        this.docId.set((await this.api.newDocument()).id);
      }
    } catch (e) {
      this.progress.set(null);
      this.message.set(describeFsError(toFsError(e)));
      if (this.docId() === null) this.docId.set((await this.api.newDocument()).id);
    }
  }
}
