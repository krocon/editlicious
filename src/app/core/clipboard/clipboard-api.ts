import { forwardRef, Injectable } from '@angular/core';
import { call } from '../backend/call';

/**
 * The system clipboard for the app's small text fields (find, "Open path…", dialogs): on macOS
 * the native menu takes ⌘C/⌘X/⌘V before the webview sees them, so the fields get them through
 * Rust like the editor does (design 003 E5). The editor itself never uses this – its text stays
 * in Rust.
 */
@Injectable({ providedIn: 'root', useClass: forwardRef(() => TauriClipboardApi) })
export abstract class ClipboardApi {
  /** `null` = no text on the clipboard. */
  abstract readText(): Promise<string | null>;
  abstract writeText(text: string): Promise<void>;
}

@Injectable({ providedIn: 'root' })
export class TauriClipboardApi extends ClipboardApi {
  readText(): Promise<string | null> {
    return call<string | null>('read_clipboard_text');
  }

  writeText(text: string): Promise<void> {
    return call<void>('write_clipboard_text', { text });
  }
}
