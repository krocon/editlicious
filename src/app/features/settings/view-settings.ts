import { inject, Injectable, Signal } from '@angular/core';
import { Settings } from '../../core/settings/settings.models';
import { SettingsStore } from './settings.store';

/**
 * What an editor view needs of the settings (design 025 E7): the values, the font and a few
 * changes. In the app it is the {@link SettingsStore} (loaded, saved, applied to the window);
 * the embedded editor provides its own per instance (`EmbeddedSettings`) – nothing global.
 */
@Injectable({ providedIn: 'root', useFactory: () => inject(SettingsStore) })
export abstract class ViewSettings {
  abstract readonly settings: Signal<Settings>;
  /** The editor font as a CSS value. */
  abstract readonly fontFamily: Signal<string>;
  abstract update(patch: Partial<Settings>): void;
  abstract zoom(step: number): void;
  abstract cycleLineNumbers(): void;
}
