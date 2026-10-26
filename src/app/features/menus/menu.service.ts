import { computed, effect, inject, Injectable, Injector, signal, untracked } from '@angular/core';
import { ActionService } from '../../core/actions/action.service';
import { MenuApi } from '../../core/menu/menu-api';
import { DialogService } from '../../shared/dialog/dialog.service';
import { baseName, DocumentsStore } from '../documents/documents.store';
import { buildMenuModel, menuChord, resolveMenu, ResolvedMenu, toSpec } from './menu-model';

/** Pause before the native menu is rebuilt after a change. */
const NATIVE_DEBOUNCE_MS = 150;

/**
 * The menus of the app (taken over from comic-commander design 008 E8): one resolved model from
 * the catalog, the registered actions, recent and open files. On macOS it becomes the native
 * menu bar (rebuilt when a state changes) and its clicks come back as actions; elsewhere the
 * HTML menu bar draws it.
 */
@Injectable({ providedIn: 'root' })
export class MenuService {
  private readonly actions = inject(ActionService);
  private readonly api = inject(MenuApi);
  private readonly docs = inject(DocumentsStore);
  private readonly dialogs = inject(DialogService);
  private readonly injector = inject(Injector);
  /** Bumped after every native click: muda toggles a check item itself. */
  private readonly clicks = signal(0);
  private started = false;
  private timer: ReturnType<typeof setTimeout> | undefined;

  readonly menus = computed<ResolvedMenu[]>(() => {
    const os = this.actions.os;
    const model = buildMenuModel(os, {
      recent: this.docs.recent().map(baseName),
      open: this.docs.docs().map((d) => d.info.name),
    });
    return resolveMenu(
      model,
      {
        label: (id) => this.actions.def(id)?.label ?? id,
        enabled: (id) => this.actions.enabled(id),
        checked: (id) => this.actions.checked(id),
        chord: (id) => menuChord(this.actions.chords(id)),
      },
      os,
    );
  });

  /** Native menu (macOS) and its clicks; call once at start. */
  start(): void {
    if (this.started) return;
    this.started = true;
    void this.api.onAction((id) => {
      this.clicks.update((n) => n + 1);
      // a dialog owns the app – native shortcuts and clicks wait, quitting does not
      if (this.dialogs.open() !== null && id !== 'app.quit') return;
      this.actions.run(id);
    });
    if (this.actions.os !== 'mac') return;
    effect(
      () => {
        this.clicks();
        const spec = toSpec(this.menus());
        untracked(() => {
          clearTimeout(this.timer);
          this.timer = setTimeout(
            () => void this.api.setAppMenu(spec).catch((e) => console.warn('menu', e)),
            NATIVE_DEBOUNCE_MS,
          );
        });
      },
      { injector: this.injector },
    );
  }
}
