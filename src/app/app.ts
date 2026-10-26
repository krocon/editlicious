import { DOCUMENT } from '@angular/common';
import {
  ChangeDetectionStrategy,
  Component,
  computed,
  DestroyRef,
  effect,
  inject,
  signal,
  viewChild,
} from '@angular/core';
import { MAX_RECENT_ACTIONS, MAX_SWITCH_ACTIONS } from './core/actions/action-catalog';
import { ActionHandler, ActionService } from './core/actions/action.service';
import { KeyboardDispatcher } from './core/actions/keyboard-dispatcher';
import { DESKTOP } from './core/backend/desktop';
import { WorkspaceApi } from './core/workspace/workspace-api';
import { TabsMode } from './core/workspace/workspace.models';
import { AboutDialog } from './features/about/about-dialog';
import { ABOUT_TEXTS } from './features/about/about-texts';
import { DocumentsStore } from './features/documents/documents.store';
import { TabBar } from './features/documents/tab-bar';
import { EditorView } from './features/editor/editor-view';
import { StatusBar } from './features/editor/status-bar';
import { Frame } from './core/editor/editor.models';
import { ChromeStore, Scheme, SCHEMES, schemeAttributes } from './features/menus/chrome.store';
import { PickDialog } from './shared/pick/pick-dialog';
import { CommandPalette } from './features/menus/command-palette';
import { MenuBar } from './features/menus/menu-bar';
import { MenuService } from './features/menus/menu.service';
import { ShortcutsDialog } from './features/menus/shortcuts-dialog';
import { AskDialog } from './shared/ask/ask-dialog';
import { DialogService } from './shared/dialog/dialog.service';
import { SettingsDialog } from './features/settings/settings-dialog';
import { SettingsStore } from './features/settings/settings.store';
import { RecordingStore } from './features/recording/recording.store';
import { McpStore } from './features/mcp/mcp.store';

/**
 * Root: (hidden) menu bar, tabs only when wanted, the editor surface of the active file –
 * nothing else catches the eye (prompt 000-start: "wenig Ablenkung"). Every function is an
 * action: keys, menus and the command palette run them.
 */
@Component({
  selector: 'app-root',
  imports: [
    AboutDialog,
    AskDialog,
    CommandPalette,
    EditorView,
    MenuBar,
    PickDialog,
    SettingsDialog,
    StatusBar,
    ShortcutsDialog,
    TabBar,
  ],
  template: `
    @if (store.loading(); as l) {
      <div class="progress" [style.width.%]="l.progress * 100"></div>
    }
    @if (!mac && chrome.menuVisible()) {
      <app-menu-bar (left)="chrome.menuOnce.set(false)" />
    }
    @if (desktop) {
      @if (tabsVisible()) {
        <app-tab-bar (closeTab)="closeTab($event)" (chosen)="focusEditor()" />
      }
      @if (store.activeId(); as id) {
        <app-editor-view
          class="editor"
          [docId]="id"
          [refresh]="store.reloaded()"
          (frameChange)="onFrame($event)"
        />
        @if (statusVisible()) {
          <app-status-bar [frame]="lastFrame()" />
        }
      }
    } @else {
      <main class="welcome">
        <img class="logo" src="logo.svg" alt="editlicious" />
        <p class="hint">{{ about.tagline }}</p>
        <p class="hint">{{ about.desktopOnly }}</p>
        <button type="button" class="btn quiet about" (click)="dialogs.show('about')">
          {{ about.about }}
        </button>
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
      @case ('palette') {
        <app-command-palette />
      }
      @case ('shortcuts') {
        <app-shortcuts-dialog />
      }
      @case ('pick') {
        <app-pick-dialog />
      }
      @case ('settings') {
        <app-settings-dialog />
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
  protected readonly chrome = inject(ChromeStore);
  protected readonly desktop = inject(DESKTOP);
  protected readonly about = ABOUT_TEXTS;
  private readonly actions = inject(ActionService);
  protected readonly mac = this.actions.os === 'mac';
  protected readonly tabsVisible = computed(() => this.store.tabsVisible() && !this.chrome.bare());
  protected readonly statusVisible = computed(
    () => this.chrome.statusBar() && !this.chrome.bare() && !!this.lastFrame(),
  );
  /** The last frame of the active editor (status bar). */
  protected readonly lastFrame = signal<Frame | null>(null);
  private readonly editor = viewChild(EditorView);
  private readonly menuBar = viewChild(MenuBar);
  private readonly ws = inject(WorkspaceApi);
  private readonly destroyRef = inject(DestroyRef);
  private readonly settings = inject(SettingsStore);

  constructor() {
    const dispatcher = inject(KeyboardDispatcher);
    dispatcher.setContextProvider((target) => {
      if (this.dialogs.open() !== null) return null;
      const view = target?.closest('app-editor-view');
      if (!view) return ['global'];
      // the open suggestion list takes the arrows, Enter, Tab and Esc first (prompt 020)
      return view.hasAttribute('data-completion')
        ? ['completion', 'editor', 'global']
        : ['editor', 'global'];
    });
    const doc = inject(DOCUMENT);
    dispatcher.attach(doc);
    // the colour scheme sets the chrome's light/dark and the editor colours (design 008 E6)
    effect(() => {
      const a = schemeAttributes(this.chrome.scheme());
      const root = doc.documentElement;
      if (a.theme) root.setAttribute('data-theme', a.theme);
      else root.removeAttribute('data-theme');
      if (a.scheme) root.setAttribute('data-scheme', a.scheme);
      else root.removeAttribute('data-scheme');
    });
    this.registerActions();
    inject(RecordingStore).registerActions(this.actions, this.destroyRef);
    if (this.desktop) inject(McpStore).start(this.destroyRef);
    void this.actions.loadOverrides();
    inject(MenuService).start();
    if (this.desktop) void this.start();
    else void this.settings.load();
  }

  private async start(): Promise<void> {
    await this.settings.load();
    // files from "open with" / a second start (design 012 E3): listen before the start files
    // are fetched, open them once the start is done
    let started = false;
    const early: string[] = [];
    const offOpen = await this.ws
      .onOpenFiles((paths) => {
        if (started) void this.store.openPaths(paths);
        else early.push(...paths);
      })
      .catch(() => null);
    await this.store.start();
    started = true;
    if (early.length) await this.store.openPaths(early);
    const offClose = await this.ws.onCloseRequested(() => void this.store.quit()).catch(() => null);
    const offDrop = await this.ws
      .onFilesDropped((paths) => void this.store.openPaths(paths))
      .catch(() => null);
    this.destroyRef.onDestroy(() => {
      offOpen?.();
      offClose?.();
      offDrop?.();
    });
  }

  private registerActions(): void {
    const s = this.store;
    const c = this.chrome;
    const st = this.settings;
    const after = (p: Promise<unknown>) => void p.then(() => this.focusEditor());
    const tabs = (mode: TabsMode): ActionHandler => ({
      run: () => s.tabsMode.set(mode),
      checked: () => s.tabsMode() === mode,
    });
    const scheme = (s: Scheme): ActionHandler => ({
      run: () => c.scheme.set(s),
      checked: () => c.scheme() === s,
    });
    const handlers: Record<string, ActionHandler> = {
      'view.nextScheme': { run: () => c.nextScheme() },
      ...Object.fromEntries(SCHEMES.map((s) => [`view.scheme.${s}`, scheme(s)])),
      'file.new': { run: () => after(s.newDoc()) },
      'file.open': { run: () => after(s.openDialog()) },
      'file.save': { run: () => after(s.save()), enabled: () => s.active() !== null },
      'file.saveAs': { run: () => after(s.saveAs()), enabled: () => s.active() !== null },
      'file.saveAll': { run: () => after(s.saveAll()) },
      'file.close': { run: () => after(s.close()) },
      'file.closeAll': { run: () => after(s.closeAll()) },
      'file.next': { run: () => s.next(1), enabled: () => s.docs().length > 1 },
      'file.prev': { run: () => s.next(-1), enabled: () => s.docs().length > 1 },
      'app.palette': { run: () => this.dialogs.show('palette') },
      'app.shortcuts': { run: () => this.dialogs.show('shortcuts') },
      'app.settings': { run: () => this.dialogs.show('settings') },
      'view.zoomIn': { run: () => st.zoom(1) },
      'view.zoomOut': { run: () => st.zoom(-1) },
      'view.zoomReset': { run: () => st.zoom(0) },
      'view.lineNumbers': { run: () => st.cycleLineNumbers() },
      'view.wordWrap': {
        run: () => st.update({ wordWrap: !st.settings().wordWrap }),
        checked: () => st.settings().wordWrap,
      },
      'view.statusBar': {
        run: () => c.statusBar.update((v) => !v),
        checked: () => c.statusBar(),
      },
      'view.overview': {
        run: () => st.update({ overview: !st.settings().overview }),
        checked: () => st.settings().overview,
      },
      'view.colorSwatches': {
        run: () => st.update({ colorSwatches: !st.settings().colorSwatches }),
        checked: () => st.settings().colorSwatches,
      },
      'view.indentGuides': {
        run: () => st.update({ indentGuides: !st.settings().indentGuides }),
        checked: () => st.settings().indentGuides,
      },
      'app.about': { run: () => this.dialogs.show('about') },
      'app.quit': { run: () => void s.quit() },
      'app.focusMenu': {
        run: () => {
          if (this.mac) return;
          c.menuOnce.set(true);
          setTimeout(() => this.menuBar()?.enter());
        },
        enabled: () => !this.mac,
      },
      'view.toggleMenuBar': {
        run: () => c.toggleMenuBar(),
        checked: () => c.menuBar(),
        enabled: () => !this.mac,
      },
      'view.toggleChrome': { run: () => c.toggleBare(), checked: () => c.bare() },
      'view.fullscreen': {
        run: () =>
          void this.ws
            .toggleFullscreen()
            .then((on) => c.fullscreen.set(on))
            .catch(() => undefined),
        checked: () => c.fullscreen(),
      },
      'view.tabs.auto': tabs('auto'),
      'view.tabs.show': tabs('show'),
      'view.tabs.hide': tabs('hide'),
    };
    for (let n = 1; n <= MAX_RECENT_ACTIONS; n++) {
      handlers[`file.recent.${n}`] = {
        run: () => {
          const p = s.recent()[n - 1];
          if (p) after(s.openPaths([p]));
        },
        enabled: () => s.recent().length >= n,
      };
    }
    for (let n = 1; n <= MAX_SWITCH_ACTIONS; n++) {
      handlers[`file.switch.${n}`] = {
        run: () => {
          const d = s.docs()[n - 1];
          if (d) s.activate(d.id);
        },
        enabled: () => s.docs().length >= n,
        checked: () => s.docs()[n - 1]?.id === s.activeId(),
      };
    }
    this.actions.registerAll(handlers, this.destroyRef);
  }

  protected onFrame(f: Frame): void {
    this.store.onFrame(f);
    this.lastFrame.set(f);
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
