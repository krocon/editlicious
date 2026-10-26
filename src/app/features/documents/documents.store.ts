import { computed, effect, inject, Injectable, signal, untracked } from '@angular/core';
import { EditorApi } from '../../core/editor/editor-api';
import { DocInfo, Frame } from '../../core/editor/editor.models';
import { describeFsError, FsError, toFsError } from '../../core/errors/fs-error';
import { WorkspaceApi } from '../../core/workspace/workspace-api';
import { SessionFile, TabsMode } from '../../core/workspace/workspace.models';
import { AskService } from '../../shared/ask/ask.service';
import { DOC_TEXTS as T } from './documents-texts';

export interface OpenDoc {
  id: number;
  info: DocInfo;
  line: number;
  ch: number;
  top: number;
}

/** Files shown in the "recent" list (and kept in the session file). */
const MAX_RECENT = 30;
/** Session writes are bundled (caret moves often). */
const PERSIST_DELAY = 1000;

/**
 * The open files (design 004): tabs, new/open/save/close with questions about unsaved
 * changes, session restore, files changed on disk, quitting. The text itself lives in the
 * engine; this store only knows ids, infos and where the caret was.
 */
@Injectable({ providedIn: 'root' })
export class DocumentsStore {
  private readonly api = inject(EditorApi);
  private readonly ws = inject(WorkspaceApi);
  private readonly ask = inject(AskService);

  readonly docs = signal<OpenDoc[]>([]);
  readonly activeId = signal<number | null>(null);
  readonly tabsMode = signal<TabsMode>('auto');
  readonly recent = signal<string[]>([]);
  /** A file being loaded: name and 0…1. */
  readonly loading = signal<{ name: string; progress: number } | null>(null);
  /** Last status message (live region). */
  readonly message = signal('');

  readonly active = computed(() => this.docs().find((d) => d.id === this.activeId()) ?? null);
  readonly tabsVisible = computed(() => {
    const m = this.tabsMode();
    return m === 'show' || (m === 'auto' && this.docs().length > 1);
  });

  private started = false;
  private persistTimer: ReturnType<typeof setTimeout> | null = null;
  private quitting = false;

  constructor() {
    // window title: "• name – editlicious"
    effect(() => {
      const d = this.active();
      const title = d ? `${d.info.dirty ? '• ' : ''}${d.info.name} – ${T.appName}` : T.appName;
      untracked(() => void this.ws.setTitle(title).catch(() => undefined));
    });
    effect(() => {
      this.docs();
      this.activeId();
      this.tabsMode();
      this.recent();
      if (this.started) untracked(() => this.persistSoon());
    });
  }

  // ---------------------------------------------------------------- start & session

  /** Files from the command line, else the last session, else an empty document. */
  async start(): Promise<void> {
    const session: SessionFile | null = await this.ws.loadSession().catch(() => null);
    if (session) {
      this.tabsMode.set(session.tabs);
      this.recent.set(session.recent);
    }
    const paths = await this.api.startupPaths().catch((): string[] => []);
    if (paths.length > 0) {
      await this.openPaths(paths);
    } else if (session && session.files.length > 0) {
      for (const f of session.files) {
        const doc = await this.load(f.path, false);
        if (doc) await this.restore(doc.id, f.line, f.ch, f.top);
      }
      const active = this.docs()[session.active] ?? this.docs()[0];
      if (active) this.activeId.set(active.id);
    }
    if (this.docs().length === 0) await this.newDoc();
    this.started = true;
    void this.ws.takeOverClose().catch(() => undefined);
  }

  private async restore(id: number, line: number, ch: number, top: number): Promise<void> {
    try {
      await this.api.command(id, { type: 'goTo', line, ch });
      const f = await this.api.command(id, { type: 'scrollTo', top });
      this.onFrame(f);
    } catch {
      // the file got shorter – the engine clamped, nothing to do
    }
  }

  private persistSoon(): void {
    if (this.persistTimer) clearTimeout(this.persistTimer);
    this.persistTimer = setTimeout(() => void this.persist(), PERSIST_DELAY);
  }

  async persist(): Promise<void> {
    if (this.persistTimer) clearTimeout(this.persistTimer);
    this.persistTimer = null;
    const docs = this.docs().filter((d) => d.info.path);
    const active = Math.max(
      0,
      docs.findIndex((d) => d.id === this.activeId()),
    );
    try {
      await this.ws.saveSession({
        version: 1,
        files: docs.map((d) => ({ path: d.info.path!, line: d.line, ch: d.ch, top: d.top })),
        active,
        recent: this.recent(),
        tabs: this.tabsMode(),
      });
    } catch {
      // no backend or not writable: the session is a convenience
    }
  }

  // ---------------------------------------------------------------- frames from the view

  /** Every frame tells us dirty state, caret and scroll of its document. */
  onFrame(f: Frame): void {
    this.docs.update((docs) =>
      docs.map((d) =>
        d.id === f.info.id
          ? { ...d, info: f.info, line: f.caretLine, ch: f.caretCh, top: f.top }
          : d,
      ),
    );
  }

  private updateInfo(info: DocInfo): void {
    this.docs.update((docs) => docs.map((d) => (d.id === info.id ? { ...d, info } : d)));
  }

  // ---------------------------------------------------------------- open & new

  async newDoc(): Promise<void> {
    try {
      const info = await this.api.newDocument();
      this.add(info);
    } catch (e) {
      this.fail(e);
    }
  }

  async openDialog(): Promise<void> {
    try {
      const paths = await this.ws.pickOpenFiles(this.active()?.info.path ?? null);
      if (paths) await this.openPaths(paths);
    } catch (e) {
      this.fail(e);
    }
  }

  async openPaths(paths: string[]): Promise<void> {
    let last: OpenDoc | null = null;
    for (const p of paths) last = (await this.load(p, true)) ?? last;
    if (last) this.activeId.set(last.id);
    // an untouched empty document gives way to the opened file
    const blank = this.docs().find(
      (d) => d.id !== last?.id && !d.info.path && !d.info.dirty && d.info.charCount === 0,
    );
    if (last && blank && this.docs().length > 1) await this.dropDoc(blank.id);
  }

  private async load(path: string, activate: boolean): Promise<OpenDoc | null> {
    const existing = this.docs().find((d) => d.info.path === path);
    if (existing) {
      this.remember(path);
      if (activate) this.activeId.set(existing.id);
      return existing;
    }
    const name = baseName(path);
    this.loading.set({ name, progress: 0 });
    try {
      const l = this.api.open(path, (e) =>
        this.loading.set({ name, progress: e.total ? e.loaded / e.total : 1 }),
      );
      const info = await l.done;
      this.remember(path);
      return this.add(info, activate);
    } catch (e) {
      this.message.set(T.openFailed(name, describeFsError(toFsError(e))));
      return null;
    } finally {
      this.loading.set(null);
    }
  }

  private add(info: DocInfo, activate = true): OpenDoc {
    const doc: OpenDoc = { id: info.id, info, line: 0, ch: 0, top: 0 };
    const i = this.docs().findIndex((d) => d.id === this.activeId());
    this.docs.update((docs) => {
      const next = [...docs];
      next.splice(i < 0 ? next.length : i + 1, 0, doc);
      return next;
    });
    if (activate || this.activeId() === null) this.activeId.set(info.id);
    return doc;
  }

  private remember(path: string): void {
    this.recent.update((r) => [path, ...r.filter((p) => p !== path)].slice(0, MAX_RECENT));
  }

  // ---------------------------------------------------------------- save

  /** `true` = saved. New documents go through "Save as". */
  async save(id = this.activeId()): Promise<boolean> {
    const d = this.docs().find((x) => x.id === id);
    if (!d) return false;
    if (!d.info.path) return this.saveAs(d.id);
    return this.write(d, {});
  }

  async saveAs(id = this.activeId()): Promise<boolean> {
    const d = this.docs().find((x) => x.id === id);
    if (!d) return false;
    let path: string | null;
    try {
      path = await this.ws.pickSavePath(
        d.info.path ? d.info.name : `${T.untitled}.txt`,
        d.info.path,
      );
    } catch (e) {
      this.fail(e);
      return false;
    }
    if (!path) return false;
    const ok = await this.write(d, { path });
    if (ok) this.remember(path);
    return ok;
  }

  async saveAll(): Promise<boolean> {
    for (const d of this.docs()) {
      if (d.info.dirty && !(await this.save(d.id))) return false;
    }
    return true;
  }

  private async write(d: OpenDoc, req: { path?: string; encoding?: string }): Promise<boolean> {
    try {
      const info = await this.api.save(d.id, req);
      this.updateInfo(info);
      this.message.set(T.saved(info.name));
      return true;
    } catch (e) {
      const err = toFsError(e);
      if (err.code === 'unmappable_character') return this.offerUtf8(d, req, err);
      this.message.set(T.saveFailed(d.info.name, describeFsError(err)));
      return false;
    }
  }

  private async offerUtf8(d: OpenDoc, req: { path?: string }, err: FsError): Promise<boolean> {
    const [line, ch] = err.message.split(':').map((n) => Number(n) + 1);
    const answer = await this.ask.ask({
      title: T.unmappableTitle,
      message: T.unmappable(d.info.name, d.info.encoding, `${line}:${ch}`),
      buttons: [
        { id: 'cancel', label: T.cancel },
        { id: 'utf8', label: T.saveUtf8, primary: true },
      ],
      cancel: 'cancel',
    });
    return answer === 'utf8' ? this.write(d, { ...req, encoding: 'UTF-8' }) : false;
  }

  // ---------------------------------------------------------------- close & quit

  /** Asks about unsaved changes; `true` = closed. The last file gives way to an empty one. */
  async close(id = this.activeId()): Promise<boolean> {
    const d = this.docs().find((x) => x.id === id);
    if (!d) return false;
    if (!(await this.okToDrop(d))) return false;
    await this.dropDoc(d.id);
    if (this.docs().length === 0 && !this.quitting) await this.newDoc();
    return true;
  }

  async closeAll(): Promise<boolean> {
    for (const d of [...this.docs()]) {
      if (!(await this.okToDrop(d))) return false;
      await this.dropDoc(d.id);
    }
    if (!this.quitting) await this.newDoc();
    return true;
  }

  /** The close button of the window or Quit: ask, keep the session, leave. */
  async quit(): Promise<void> {
    if (this.quitting) return;
    this.quitting = true;
    await this.persist();
    for (const d of this.docs()) {
      if (!(await this.okToDrop(d))) {
        this.quitting = false;
        return;
      }
    }
    await this.ws.quit();
  }

  private async okToDrop(d: OpenDoc): Promise<boolean> {
    if (!d.info.dirty) return true;
    this.activeId.set(d.id);
    const answer = await this.ask.ask({
      title: T.unsavedTitle,
      message: T.unsaved(d.info.name),
      buttons: [
        { id: 'discard', label: T.dontSave },
        { id: 'cancel', label: T.cancel },
        { id: 'save', label: T.save, primary: true },
      ],
      cancel: 'cancel',
    });
    if (answer === 'save') return this.save(d.id);
    return answer === 'discard';
  }

  private async dropDoc(id: number): Promise<void> {
    const docs = this.docs();
    const i = docs.findIndex((d) => d.id === id);
    if (i < 0) return;
    const rest = docs.filter((d) => d.id !== id);
    this.docs.set(rest);
    if (this.activeId() === id) {
      this.activeId.set(rest[Math.min(i, rest.length - 1)]?.id ?? null);
    }
    await this.api.close(id).catch(() => undefined);
  }

  // ---------------------------------------------------------------- tabs

  activate(id: number): void {
    if (this.docs().some((d) => d.id === id)) this.activeId.set(id);
  }

  next(step = 1): void {
    const docs = this.docs();
    if (docs.length < 2) return;
    const i = docs.findIndex((d) => d.id === this.activeId());
    this.activeId.set(docs[(i + step + docs.length) % docs.length].id);
  }

  /** Moves a tab to another place (drag & drop, keyboard). */
  move(id: number, to: number): void {
    this.docs.update((docs) => {
      const i = docs.findIndex((d) => d.id === id);
      if (i < 0) return docs;
      const next = [...docs];
      const [d] = next.splice(i, 1);
      next.splice(Math.max(0, Math.min(to, next.length)), 0, d);
      return next;
    });
  }

  // ---------------------------------------------------------------- changes on disk

  private checking = false;

  /** On window focus: reload unchanged documents, ask for changed ones (design 004 E6). */
  async checkDisk(): Promise<void> {
    if (this.checking) return;
    this.checking = true;
    try {
      for (const d of this.docs()) {
        if (!d.info.path) continue;
        const s = await this.api.diskState(d.id).catch(() => null);
        if (!s) continue;
        if (s.missing) {
          this.message.set(T.missing(d.info.name));
          continue;
        }
        if (!s.changed) continue;
        let reload = !d.info.dirty;
        if (!reload) {
          const answer = await this.ask.ask({
            title: T.changedTitle,
            message: T.changed(d.info.name),
            buttons: [
              { id: 'keep', label: T.keepMine },
              { id: 'reload', label: T.reload, primary: true },
            ],
            cancel: 'keep',
          });
          reload = answer === 'reload';
        }
        if (reload) {
          try {
            const info = await this.api.reload(d.id);
            this.updateInfo(info);
            this.message.set(T.reloaded(info.name));
            this.reloaded.update((n) => n + 1);
          } catch (e) {
            this.fail(e);
          }
        }
      }
    } finally {
      this.checking = false;
    }
  }

  /** Bumped after a reload – the view asks for a fresh frame. */
  readonly reloaded = signal(0);

  private fail(e: unknown): void {
    this.message.set(describeFsError(toFsError(e)));
  }
}

/** Display name of a path (only for messages – the backend names open documents). */
export function baseName(path: string): string {
  const parts = path.split(/[\\/]/);
  return parts[parts.length - 1] || path;
}
