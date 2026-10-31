import {
  afterNextRender,
  booleanAttribute,
  ChangeDetectionStrategy,
  Component,
  computed,
  DestroyRef,
  effect,
  ElementRef,
  inject,
  InjectionToken,
  input,
  numberAttribute,
  output,
  signal,
  untracked,
  viewChild,
  ViewEncapsulation,
} from '@angular/core';
import { ActionService } from '../core/actions/action.service';
import { KeyboardDispatcher } from '../core/actions/keyboard-dispatcher';
import { EditorApi } from '../core/editor/editor-api';
import { Command, Frame, MatchCount, Query } from '../core/editor/editor.models';
import { systemLanguage, useLanguage } from '../core/i18n/i18n';
import { Scheme, SCHEMES } from '../core/settings/settings.models';
import {
  EDITOR_CLIPBOARD,
  EDITOR_EMBEDDED,
  EDITOR_MCP,
  EDITOR_PLAYBACK,
} from '../features/editor/editor-host';
import { EditorView } from '../features/editor/editor-view';
import { StatusBar } from '../features/editor/status-bar';
import { FindStore } from '../features/find/find.store';
import { schemeAttributes } from '../features/menus/chrome.store';
import { ViewSettings } from '../features/settings/view-settings';
import { DialogService } from '../shared/dialog/dialog.service';
import { PickDialog } from '../shared/pick/pick-dialog';
import { PickService } from '../shared/pick/pick.service';
import { BrowserClipboard } from './browser-clipboard';
import { EmbeddedSettings } from './embedded-settings';
import { EmbedEngineApi } from './wasm-editor-api';

/** `textChange` / `ed-change`: what changed – never the text itself (design 025 E9). */
export interface EdChange {
  revision: number;
  dirty: boolean;
  lineCount: number;
  charCount: number;
}

/** `caretChange` / `ed-caret`: 1-based line and column of the primary caret. */
export interface EdCaret {
  line: number;
  ch: number;
  selChars: number;
  selLines: number;
  carets: number;
}

/** Set by `element.ts`: the outputs also go out as DOM events `ed-change`, `ed-caret`, `ed-ready`. */
export const EMIT_DOM_EVENTS = new InjectionToken<boolean>('EMIT_DOM_EVENTS', {
  providedIn: 'root',
  factory: () => false,
});

/** Where the facade of an element is kept (the custom element's methods use it). */
export const FACADE = Symbol.for('editlicious.facade');

/** `"auto" | true | false` from an attribute or a property. */
export function insertSpacesAttribute(v: unknown): 'auto' | boolean {
  if (v === true || v === false) return v;
  if (v === '' || v === 'true') return true;
  if (v === 'false') return false;
  return 'auto';
}

function schemeAttribute(v: unknown): Scheme {
  return SCHEMES.includes(v as Scheme) ? (v as Scheme) : 'auto';
}

function lineNumbersAttribute(v: unknown): 'absolute' | 'relative' | 'off' {
  return v === 'relative' || v === 'off' ? v : 'absolute';
}

/**
 * The embeddable editor (design 025 E9): the app's editor view on its own engine session, in a
 * shadow root – as Angular component `<ed-editor>` and, via `element.ts`, as custom element
 * `<editlicious-editor>`. Every instance has its own actions, keyboard, find state and settings.
 */
@Component({
  // the public tag of the embeddable component (prompt 025), not an app-internal one
  // eslint-disable-next-line @angular-eslint/component-selector
  selector: 'ed-editor',
  imports: [EditorView, StatusBar, PickDialog],
  encapsulation: ViewEncapsulation.ShadowDom,
  changeDetection: ChangeDetectionStrategy.OnPush,
  styleUrls: ['./host-theme.css', './editlicious-editor.css'],
  providers: [
    ActionService,
    KeyboardDispatcher,
    FindStore,
    DialogService,
    PickService,
    EmbeddedSettings,
    { provide: ViewSettings, useExisting: EmbeddedSettings },
    { provide: EditorApi, useExisting: EmbedEngineApi },
    { provide: EDITOR_PLAYBACK, useValue: null },
    { provide: EDITOR_MCP, useValue: null },
    { provide: EDITOR_EMBEDDED, useValue: true },
    { provide: EDITOR_CLIPBOARD, useClass: BrowserClipboard },
  ],
  host: {
    '[attr.data-theme]': 'colors().theme',
    '[attr.data-scheme]': 'colors().scheme',
    '[attr.data-read-only]': 'isReadOnly() ? "" : null',
    '[style.--editor-font-size.px]': 'fontSize()',
    '[style.--font-editor]': 'settings.fontFamily()',
  },
  template: `
    @if (docId(); as id) {
      <app-editor-view
        class="editor"
        [docId]="id"
        [refresh]="refresh()"
        (frameChange)="onFrame($event)"
      />
      @if (statusBar()) {
        <app-status-bar [frame]="frame()" />
      }
    }
    @if (dialogs.open() === 'pick') {
      <app-pick-dialog />
    }
  `,
})
export class EditliciousEditor {
  // ------------------------------------------------------------------ inputs
  /** The text; set again only when it differs from what was set last (no echo). */
  readonly value = input<string | null>(null);
  /** Grammar name (`Rust`), alias (`TypeScript`) or file name (`app.ts`, `.kt`); empty = plain. */
  readonly language = input<string>('');
  readonly scheme = input<Scheme, unknown>('auto', { transform: schemeAttribute });
  readonly readOnly = input(false, { transform: booleanAttribute });
  /** HTML attribute `readonly` of the custom element (same as `readOnly`). */
  readonly readonly = input(false, { transform: booleanAttribute });
  readonly wordWrap = input(false, { transform: booleanAttribute });
  readonly lineNumbers = input<'absolute' | 'relative' | 'off', unknown>('absolute', {
    transform: lineNumbersAttribute,
  });
  readonly statusBar = input(false, { transform: booleanAttribute });
  readonly tabWidth = input(4, { transform: numberAttribute });
  readonly insertSpaces = input<'auto' | boolean, unknown>('auto', {
    transform: insertSpacesAttribute,
  });
  readonly fontSize = input(14, { transform: numberAttribute });
  readonly fontFamily = input('');
  readonly indentGuides = input(true, { transform: booleanAttribute });
  readonly colorSwatches = input(true, { transform: booleanAttribute });
  readonly overview = input(true, { transform: booleanAttribute });
  readonly mathHints = input(true, { transform: booleanAttribute });
  readonly wordSuggest = input(false, { transform: booleanAttribute });
  readonly uiLanguage = input<'auto' | 'en' | 'de'>('auto');

  // ------------------------------------------------------------------ outputs
  readonly textChange = output<EdChange>();
  readonly caretChange = output<EdCaret>();
  readonly ready = output<void>();

  protected readonly settings = inject(EmbeddedSettings);
  protected readonly dialogs = inject(DialogService);
  protected readonly docId = signal<number | null>(null);
  protected readonly frame = signal<Frame | null>(null);
  protected readonly refresh = signal(0);
  protected readonly isReadOnly = computed(() => this.readOnly() || this.readonly());
  protected readonly colors = computed(() => schemeAttributes(this.scheme()));

  private readonly api = inject(EmbedEngineApi);
  private readonly actions = inject(ActionService);
  private readonly find_ = inject(FindStore);
  private readonly host = inject<ElementRef<HTMLElement>>(ElementRef).nativeElement;
  private readonly destroyRef = inject(DestroyRef);
  private readonly domEvents = inject(EMIT_DOM_EVENTS);
  private readonly view = viewChild(EditorView);
  private destroyed = false;
  /** The text set last – `value` only replaces it when it differs. */
  private lastValue: string | null = null;
  private lastChange = '';
  private lastCaret = '';
  private started: Promise<number | null>;
  private startResolve!: (id: number | null) => void;

  constructor() {
    this.started = new Promise((r) => (this.startResolve = r));
    (this.host as unknown as Record<symbol, unknown>)[FACADE] = this;
    const dispatcher = inject(KeyboardDispatcher);
    dispatcher.setContextProvider((target) => {
      if (this.dialogs.open() !== null) return null;
      const view = target?.closest('app-editor-view');
      if (!view) return ['global'];
      return view.hasAttribute('data-completion')
        ? ['completion', 'editor', 'global']
        : ['editor', 'global'];
    });
    this.registerActions();
    // the settings of this instance follow the inputs (the editor view configures the engine)
    effect(() => {
      const spaces = this.insertSpaces();
      this.settings.update({
        wordWrap: this.wordWrap(),
        lineNumbers: this.lineNumbers(),
        tabWidth: Math.min(16, Math.max(1, Math.round(this.tabWidth() || 4))),
        indentWith: spaces === 'auto' ? 'auto' : spaces ? 'spaces' : 'tabs',
        fontSize: Math.min(72, Math.max(6, this.fontSize() || 14)),
        fontFamily: this.fontFamily(),
        indentGuides: this.indentGuides(),
        colorSwatches: this.colorSwatches(),
        overview: this.overview(),
        mathHints: this.mathHints(),
        mathSuggest: this.mathHints(),
        wordSuggest: this.wordSuggest(),
        statusBar: this.statusBar(),
      });
    });
    effect(() => {
      const l = this.uiLanguage();
      if (l !== 'auto') useLanguage(l);
      else untracked(() => useLanguage(systemLanguage()));
    });
    afterNextRender(() => {
      const target = this.host.shadowRoot ?? this.host;
      const detach = dispatcher.attach(target, { contextMenu: false });
      this.destroyRef.onDestroy(detach);
      void this.start();
    });
    // later changes of the inputs
    effect(() => {
      const v = this.value();
      untracked(() => {
        if (v !== null && v !== this.lastValue && this.docId() !== null) void this.setValue(v);
      });
    });
    effect(() => {
      const spec = this.language();
      untracked(() => {
        const id = this.docId();
        if (id !== null) void this.inOrder(() => this.api.setLanguage(id, spec));
      });
    });
    effect(() => {
      const ro = this.isReadOnly();
      untracked(() => {
        if (this.docId() !== null) void this.view()?.run({ type: 'setReadOnly', readOnly: ro });
      });
    });
    this.destroyRef.onDestroy(() => {
      this.destroyed = true;
      const id = this.docId();
      if (id !== null) void this.api.close(id).catch(() => undefined);
      void this.started.then((late) => {
        // created after the end: free it as well
        if (late !== null && late !== id) void this.api.close(late).catch(() => undefined);
      });
    });
  }

  private async start(): Promise<void> {
    try {
      const v = this.value() ?? '';
      this.lastValue = v;
      const info = await this.api.createText(v, this.language() || null);
      this.startResolve(info.id);
      if (this.destroyed) return;
      if (this.isReadOnly())
        await this.api.command(info.id, { type: 'setReadOnly', readOnly: true });
      this.docId.set(info.id);
      this.ready.emit();
      this.dispatch('ed-ready', null);
    } catch (e) {
      this.startResolve(null);
      console.error('editlicious: the engine did not start', e);
    }
  }

  private registerActions(): void {
    const st = this.settings;
    this.actions.registerAll(
      {
        'view.zoomIn': { run: () => st.zoom(1) },
        'view.zoomOut': { run: () => st.zoom(-1) },
        'view.zoomReset': { run: () => st.zoom(0) },
        'view.lineNumbers': { run: () => st.cycleLineNumbers() },
        'view.wordWrap': {
          run: () => st.update({ wordWrap: !st.settings().wordWrap }),
          checked: () => st.settings().wordWrap,
        },
      },
      this.destroyRef,
    );
  }

  private redraw(): void {
    this.refresh.update((n) => n + 1);
  }

  private dispatch(name: string, detail: unknown): void {
    if (!this.domEvents) return;
    this.host.dispatchEvent(new CustomEvent(name, { detail, bubbles: true, composed: true }));
  }

  protected onFrame(f: Frame): void {
    this.frame.set(f);
    const i = f.info;
    const change = `${i.revision}/${i.dirty}/${i.lineCount}/${i.charCount}`;
    if (change !== this.lastChange) {
      const first = this.lastChange === '';
      this.lastChange = change;
      if (!first) {
        const c: EdChange = {
          revision: i.revision,
          dirty: i.dirty,
          lineCount: i.lineCount,
          charCount: i.charCount,
        };
        this.textChange.emit(c);
        this.dispatch('ed-change', c);
      }
    }
    const caret: EdCaret = {
      line: f.caretLine + 1,
      ch: f.caretCh + 1,
      selChars: f.selChars,
      selLines: f.selLines,
      carets: f.carets.length,
    };
    const key = JSON.stringify(caret);
    if (key !== this.lastCaret) {
      this.lastCaret = key;
      this.caretChange.emit(caret);
      this.dispatch('ed-caret', caret);
    }
  }

  // ------------------------------------------------------------------ methods

  /** The whole text (fetched on request only). */
  async getValue(): Promise<string> {
    const id = await this.started;
    await this.view()?.idle();
    return id === null ? (this.value() ?? '') : this.api.text(id);
  }

  /** Lines `from..to` (1-based, inclusive). */
  async getSlice(from: number, to: number): Promise<string> {
    const id = await this.started;
    if (id === null) return '';
    await this.view()?.idle();
    return this.api.slice(id, Math.max(0, from - 1), Math.max(0, to));
  }

  /** Replaces the whole text (clean, no undo back into the old one). */
  async setValue(text: string): Promise<void> {
    this.lastValue = text;
    const id = await this.started;
    if (id === null) return;
    await this.inOrder(() => this.api.setText(id, text));
  }

  /** Runs `fn` after the commands already sent (typing just before `setValue` lands first). */
  private async inOrder(fn: () => Promise<unknown>): Promise<void> {
    const view = this.view();
    if (view) await view.enqueue(fn);
    else {
      await fn();
      this.redraw();
    }
  }

  /** A new text, optionally in another language. */
  async loadText(text: string, options: { language?: string } = {}): Promise<void> {
    const id = await this.started;
    const spec = options.language;
    if (id !== null && spec !== undefined) await this.inOrder(() => this.api.setLanguage(id, spec));
    await this.setValue(text);
  }

  focus(): void {
    this.view()?.focus();
  }

  /** Runs an engine command (`{ type: 'selectAll' }` …). */
  async runCommand(cmd: Command): Promise<void> {
    await this.started;
    await this.view()?.run(cmd);
  }

  /** Runs an action of the catalog (`find.open`, `edit.sortLines` …); `false` = not available. */
  runAction(id: string): boolean {
    return this.actions.run(id);
  }

  /** Selects the next match and returns how many there are. */
  async find(pattern: string, options: Partial<Omit<Query, 'pattern'>> = {}): Promise<MatchCount> {
    const query: Query = {
      pattern,
      regex: options.regex ?? false,
      caseSensitive: options.caseSensitive ?? false,
      wholeWord: options.wholeWord ?? false,
    };
    this.find_.pattern.set(pattern);
    this.find_.regex.set(query.regex);
    this.find_.caseSensitive.set(query.caseSensitive);
    this.find_.wholeWord.set(query.wholeWord);
    await this.runCommand({ type: 'find', query, backwards: false, incremental: false });
    const id = await this.started;
    return id === null ? { total: 0, current: 0 } : this.api.countMatches(id, query);
  }

  /** Puts the caret at line `line`, column `ch` (both 1-based). */
  goTo(line: number, ch = 1): Promise<void> {
    return this.runCommand({
      type: 'goTo',
      line: Math.max(0, line - 1),
      ch: Math.max(0, ch - 1),
      select: false,
    });
  }
}
