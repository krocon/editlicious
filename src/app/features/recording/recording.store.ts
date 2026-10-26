import { DestroyRef, Injectable, computed, inject, signal } from '@angular/core';
import { ActionHandler, ActionService } from '../../core/actions/action.service';
import { Frame } from '../../core/editor/editor.models';
import { describeFsError, FsErrorCode, toFsError } from '../../core/errors/fs-error';
import { RecordingApi } from '../../core/recording/recording-api';
import { PlaybackEvent, PlaybackSource, Tempo } from '../../core/recording/recording.models';
import { WorkspaceApi } from '../../core/workspace/workspace-api';
import { AskService } from '../../shared/ask/ask.service';
import { PickService } from '../../shared/pick/pick.service';
import { DocumentsStore } from '../documents/documents.store';
import { RECORDING_TEXTS } from './recording-texts';

/** A running playback (design 015). */
export interface PlaybackState {
  docId: number;
  done: number;
  total: number;
  paused: boolean;
  /** Started by an AI over MCP (design 016 E7). */
  ai?: boolean;
}

/** Engine commands that only touch the view – allowed while a playback runs (design 015 E10). */
export const VIEW_ONLY_COMMANDS: ReadonlySet<string> = new Set([
  'frame',
  'resize',
  'scroll',
  'scrollTo',
  'configure',
  'copyHtml',
  'copy',
]);

/**
 * Recording and playback for the whole app (design 015 E11): which document records, which
 * plays, the frames a playback pushes – and the `record.*` actions. The engine does the work.
 */
@Injectable({ providedIn: 'root' })
export class RecordingStore {
  private readonly api = inject(RecordingApi);
  private readonly docs = inject(DocumentsStore);
  private readonly ws = inject(WorkspaceApi);
  private readonly ask = inject(AskService);
  private readonly picker = inject(PickService);
  private readonly t = RECORDING_TEXTS;

  /** Documents that record right now. */
  readonly recording = signal<ReadonlySet<number>>(new Set());
  readonly playback = signal<PlaybackState | null>(null);
  /** The last frame a playback sent – the editor view of that document draws it. */
  readonly pushed = signal<Frame | null>(null);
  /** Documents with a finished recording (save, play again). */
  readonly recorded = signal<ReadonlySet<number>>(new Set());

  readonly activeRecording = computed(() => {
    const id = this.docs.activeId();
    return id !== null && this.recording().has(id);
  });

  isPlaying(docId: number): boolean {
    return this.playback()?.docId === docId;
  }

  isRecording(docId: number): boolean {
    return this.recording().has(docId);
  }

  /** Registers the `record.*` actions (once, from the app root). */
  registerActions(actions: ActionService, destroyRef: DestroyRef): void {
    const active = () => this.docs.activeId();
    const hasDoc = () => active() !== null;
    const playing = () => this.playback() !== null;
    const recorded = () => {
      const id = active();
      return id !== null && this.recorded().has(id);
    };
    const handlers: Record<string, ActionHandler> = {
      'record.toggle': {
        run: () => void this.toggle(),
        enabled: () => hasDoc() && !playing(),
        checked: () => this.activeRecording(),
      },
      'record.replay': {
        run: () => void this.play({ kind: 'last' }, { mode: 'instant' }),
        enabled: () => recorded() && !playing() && !this.activeRecording(),
      },
      'record.play': {
        run: () => void this.playFile(),
        enabled: () => hasDoc() && !playing() && !this.activeRecording(),
      },
      'record.pause': {
        run: () => void this.pauseResume(),
        enabled: playing,
        checked: () => !!this.playback()?.paused,
      },
      'record.stop': { run: () => void this.stopPlayback(), enabled: playing },
      'record.save': { run: () => void this.save(false), enabled: recorded },
      'record.saveScript': { run: () => void this.save(true), enabled: recorded },
    };
    actions.registerAll(handlers, destroyRef);
  }

  async toggle(): Promise<void> {
    const id = this.docs.activeId();
    if (id === null) return;
    try {
      if (this.recording().has(id)) {
        const info = await this.api.stop(id);
        this.setIn(this.recording, id, false);
        if (info && info.steps > 0) {
          this.setIn(this.recorded, id, true);
          this.say(this.t.stopped(info.steps, (info.durationMs / 1000).toFixed(1)));
        } else {
          this.say(this.t.nothing);
        }
      } else {
        await this.api.start(id);
        this.setIn(this.recording, id, true);
        this.say(this.t.started);
      }
    } catch (e) {
      this.fail(e);
    }
  }

  /** "Play a recording or script…": choose the file, check it, choose the tempo, play. */
  async playFile(): Promise<void> {
    const doc = this.docs.active();
    if (!doc) return;
    const files = await this.ws.pickOpenFiles(doc.info.path).catch(() => null);
    const path = files?.[0];
    if (!path) return;
    const source: PlaybackSource = { kind: 'path', path };
    try {
      const check = await this.api.check(doc.id, source);
      if (check.otherText) {
        const answer = await this.ask.ask({
          title: this.t.otherTextTitle,
          message: this.t.otherText,
          buttons: [
            { id: 'play', label: this.t.playAnyway, primary: true },
            { id: 'cancel', label: this.t.cancel },
          ],
          cancel: 'cancel',
        });
        if (answer !== 'play') return;
      }
      const tempo = await this.chooseTempo(check.kind);
      if (tempo === undefined) return;
      await this.play(source, tempo, doc.id);
    } catch (e) {
      this.fail(e);
    }
  }

  /** `undefined` = cancelled, `null` = as the source says. */
  async chooseTempo(kind: 'recording' | 'script'): Promise<Tempo | null | undefined> {
    const t = this.t;
    const options: { id: string; label: string; tempo: Tempo | null }[] = [
      { id: 'source', label: kind === 'script' ? t.asScript : t.asRecorded, tempo: null },
      { id: 'human', label: t.human, tempo: { mode: 'human', cpm: 330, seed: 1, typos: 0 } },
      {
        id: 'typos',
        label: t.humanTypos,
        tempo: { mode: 'human', cpm: 330, seed: 1, typos: 0.02 },
      },
      { id: 'faster', label: t.faster, tempo: { mode: 'factor', factor: 2 } },
      { id: 'slower', label: t.slower, tempo: { mode: 'factor', factor: 0.5 } },
      { id: 'even', label: t.even, tempo: { mode: 'even', ms: 60 } },
      { id: 'instant', label: t.instant, tempo: { mode: 'instant' } },
    ];
    const id = await this.picker.pick({
      title: t.tempo,
      placeholder: t.tempoPlaceholder,
      options: options.map(({ id, label }) => ({ id, label })),
      current: 'source',
    });
    if (id === null) return undefined;
    return options.find((o) => o.id === id)?.tempo ?? null;
  }

  async play(
    source: PlaybackSource,
    tempo: Tempo | null,
    docId = this.docs.activeId(),
  ): Promise<void> {
    if (docId === null || this.playback()) return;
    this.playback.set({ docId, done: 0, total: 0, paused: false });
    try {
      await this.api.play(docId, source, tempo, (e) => this.onEvent(docId, e));
      this.say(this.t.playing);
    } catch (e) {
      this.playback.set(null);
      this.fail(e);
    }
  }

  /** A playback an AI runs over MCP (design 016 E7): shown like an own one. */
  external(docId: number, e: PlaybackEvent): void {
    const p = this.playback();
    if (!p && e.kind !== 'done') {
      this.playback.set({ docId, done: 0, total: 0, paused: false, ai: true });
    }
    this.onEvent(docId, e);
  }

  /** An AI started or stopped a recording. */
  setRecording(docId: number, on: boolean): void {
    this.setIn(this.recording, docId, on);
    if (!on) this.setIn(this.recorded, docId, true);
  }

  private onEvent(docId: number, e: PlaybackEvent): void {
    if (e.kind === 'frame') {
      this.pushed.set(e.frame);
    } else if (e.kind === 'progress') {
      const p = this.playback();
      if (p?.docId === docId) {
        this.playback.set({ ...p, done: e.done, total: e.total, paused: e.paused });
      }
    } else {
      if (this.playback()?.docId === docId) this.playback.set(null);
      if (e.reason === 'finished') this.say(this.t.finished);
      else if (e.reason === 'stopped') this.say(this.t.stoppedPlayback);
      else this.say(describeFsError({ code: e.reason as FsErrorCode, message: '' }));
    }
  }

  async pauseResume(): Promise<void> {
    const p = this.playback();
    if (!p) return;
    await this.api.control(p.docId, p.paused ? 'resume' : 'pause').catch((e) => this.fail(e));
    this.playback.set({ ...p, paused: !p.paused });
  }

  async stopPlayback(): Promise<void> {
    const p = this.playback();
    if (p) await this.api.control(p.docId, 'stop').catch((e) => this.fail(e));
  }

  async save(script: boolean): Promise<void> {
    const doc = this.docs.active();
    if (!doc) return;
    const name = script ? 'recording.edscript' : 'recording.edrec.jsonl';
    const path = await this.ws.pickSavePath(name, doc.info.path).catch(() => null);
    if (!path) return;
    try {
      await this.api.save(doc.id, path, script);
      this.say(this.t.saved);
    } catch (e) {
      this.fail(e);
    }
  }

  private setIn(s: ReturnType<typeof signal<ReadonlySet<number>>>, id: number, on: boolean): void {
    s.update((old) => {
      const n = new Set(old);
      if (on) n.add(id);
      else n.delete(id);
      return n;
    });
  }

  private say(text: string): void {
    this.docs.message.set(text);
  }

  private fail(e: unknown): void {
    this.say(describeFsError(toFsError(e)));
  }
}
