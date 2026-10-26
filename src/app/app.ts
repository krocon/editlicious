import { DOCUMENT } from '@angular/common';
import {
  ChangeDetectionStrategy,
  Component,
  DestroyRef,
  inject,
  signal,
  viewChild,
} from '@angular/core';
import { ActionService } from './core/actions/action.service';
import { KeyboardDispatcher } from './core/actions/keyboard-dispatcher';
import { DESKTOP } from './core/backend/desktop';
import { WorkspaceApi } from './core/workspace/workspace-api';
import { AboutDialog } from './features/about/about-dialog';
import { DocumentsStore } from './features/documents/documents.store';
import { TabBar } from './features/documents/tab-bar';
import { EditorView } from './features/editor/editor-view';
import { AskDialog } from './shared/ask/ask-dialog';
import { DialogService } from './shared/dialog/dialog.service';

/**
 * Root: tabs (only when wanted) and the editor surface of the active file – nothing else
 * catches the eye (prompt 000-start: "wenig Ablenkung"). Menus, palette and the
 * distraction-free switches follow in prompt 005.
 */
@Component({
  selector: 'app-root',
  imports: [AboutDialog, AskDialog, EditorView, TabBar],
  template: `
    @if (store.loading(); as l) {
      <div class="progress" [style.width.%]="l.progress * 100"></div>
    }
    @if (desktop) {
      @if (store.tabsVisible()) {
        <app-tab-bar (closeTab)="closeTab($event)" (chosen)="focusEditor()" />
      }
      @if (store.activeId(); as id) {
        <app-editor-view
          class="editor"
          [docId]="id"
          [refresh]="store.reloaded()"
          (frameChange)="store.onFrame($event)"
        />
      }
    } @else {
      <main class="welcome">
        <img class="logo" src="logo.svg" alt="editlicious" />
        <p class="hint">A calm text editor for huge files.</p>
        <p class="hint">The editor runs in the desktop app (pnpm tauri dev).</p>
        <button type="button" class="btn quiet about" (click)="dialogs.show('about')">About</button>
      </main>
    }
    <div class="sr-only" aria-live="polite">{{ store.message() }}</div>
    @switch (dialogs.open()) {
      @case ('about') {
        <app-about-dialog />
      }
      @case ('ask') {
        <app-ask-dialog />
      }
    }
  `,
  styleUrl: './app.css',
  changeDetection: ChangeDetectionStrategy.OnPush,
  host: { '(window:focus)': 'onWindowFocus()' },
})
export class App {
  protected readonly dialogs = inject(DialogService);
  protected readonly store = inject(DocumentsStore);
  protected readonly desktop = inject(DESKTOP);
  protected readonly ready = signal(false);
  private readonly editor = viewChild(EditorView);
  private readonly ws = inject(WorkspaceApi);
  private readonly destroyRef = inject(DestroyRef);

  constructor() {
    const dispatcher = inject(KeyboardDispatcher);
    dispatcher.setContextProvider((target) => {
      if (this.dialogs.open() !== null) return null;
      return target?.closest('app-editor-view') ? ['editor', 'global'] : ['global'];
    });
    dispatcher.attach(inject(DOCUMENT));
    this.registerActions();
    if (this.desktop) void this.start();
  }

  private async start(): Promise<void> {
    await this.store.start();
    const destroy = this.destroyRef;
    const offClose = await this.ws.onCloseRequested(() => void this.store.quit()).catch(() => null);
    const offDrop = await this.ws
      .onFilesDropped((paths) => void this.store.openPaths(paths))
      .catch(() => null);
    destroy.onDestroy(() => {
      offClose?.();
      offDrop?.();
    });
  }

  private registerActions(): void {
    const s = this.store;
    const after = (p: Promise<unknown>) => void p.then(() => this.focusEditor());
    inject(ActionService).registerAll(
      {
        'file.new': { run: () => after(s.newDoc()) },
        'file.open': { run: () => after(s.openDialog()) },
        'file.save': { run: () => after(s.save()), enabled: () => s.active() !== null },
        'file.saveAs': { run: () => after(s.saveAs()), enabled: () => s.active() !== null },
        'file.saveAll': { run: () => after(s.saveAll()) },
        'file.close': { run: () => after(s.close()) },
        'file.closeAll': { run: () => after(s.closeAll()) },
        'file.next': { run: () => s.next(1), enabled: () => s.docs().length > 1 },
        'file.prev': { run: () => s.next(-1), enabled: () => s.docs().length > 1 },
        'app.about': { run: () => this.dialogs.show('about') },
        'app.quit': { run: () => void s.quit() },
      },
      this.destroyRef,
    );
  }

  protected closeTab(id: number): void {
    void this.store.close(id).then(() => this.focusEditor());
  }

  protected focusEditor(): void {
    setTimeout(() => this.editor()?.focus());
  }

  protected onWindowFocus(): void {
    if (this.desktop) void this.store.checkDisk();
  }
}
