import { Injectable } from '@angular/core';
import { SettingsApi } from '../app/core/settings/settings-api';
import { DEFAULT_SETTINGS, LoadedSettings, Settings } from '../app/core/settings/settings.models';

/** Test double of {@link SettingsApi}. Never imported by app code. */
@Injectable()
export class FakeSettingsApi extends SettingsApi {
  stored: Settings = { ...DEFAULT_SETTINGS };
  saves: Settings[] = [];

  load(): Promise<LoadedSettings> {
    return Promise.resolve({ value: { ...this.stored }, warning: null });
  }

  save(s: Settings): Promise<Settings> {
    this.saves.push(s);
    this.stored = { ...s };
    return Promise.resolve({ ...s });
  }
}
