import { InjectionToken } from '@angular/core';
import { isTauri } from '@tauri-apps/api/core';

/** `true` inside the desktop app; `false` in a plain browser (`pnpm start`). Tests set it. */
export const DESKTOP = new InjectionToken<boolean>('DESKTOP', {
  providedIn: 'root',
  factory: () => isTauri(),
});
