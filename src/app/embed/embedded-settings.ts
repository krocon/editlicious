import { computed, Injectable, signal } from '@angular/core';
import { DEFAULT_SETTINGS, Settings } from '../core/settings/settings.models';
import { ViewSettings } from '../features/settings/view-settings';
import { MAX_FONT, MIN_FONT } from '../features/settings/settings.store';

/**
 * The settings of one embedded editor (design 025 E7): plain signals – no backend, nothing on
 * `<html>`; the facade feeds them from its inputs.
 */
@Injectable()
export class EmbeddedSettings extends ViewSettings {
  readonly settings = signal<Settings>({ ...DEFAULT_SETTINGS, statusBar: false });

  readonly fontFamily = computed(() => {
    const f = this.settings().fontFamily.trim();
    return f ? `${f}, var(--font-editor-default)` : 'var(--font-editor-default)';
  });

  update(patch: Partial<Settings>): void {
    this.settings.update((s) => ({ ...s, ...patch }));
  }

  zoom(step: number): void {
    const size = step === 0 ? DEFAULT_SETTINGS.fontSize : this.settings().fontSize + step;
    this.update({ fontSize: Math.min(MAX_FONT, Math.max(MIN_FONT, size)) });
  }

  cycleLineNumbers(): void {
    const order: Settings['lineNumbers'][] = ['absolute', 'relative', 'off'];
    const i = order.indexOf(this.settings().lineNumbers);
    this.update({ lineNumbers: order[(i + 1) % order.length] });
  }
}
