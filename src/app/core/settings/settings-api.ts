import { Injectable } from '@angular/core';
import { call } from '../backend/call';
import { LoadedSettings, Settings } from './settings.models';

/** Facade of `load_settings` / `save_settings` (design 009). */
export abstract class SettingsApi {
  abstract load(): Promise<LoadedSettings>;
  /** Returns the settings as stored (checked and clamped by Rust). */
  abstract save(s: Settings): Promise<Settings>;
}

@Injectable()
export class TauriSettingsApi extends SettingsApi {
  load(): Promise<LoadedSettings> {
    return call<LoadedSettings>('load_settings');
  }

  save(settings: Settings): Promise<Settings> {
    return call<Settings>('save_settings', { settings });
  }
}
