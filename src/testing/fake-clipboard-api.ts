import { Injectable } from '@angular/core';
import { ClipboardApi } from '../app/core/clipboard/clipboard-api';

/** Test double of {@link ClipboardApi}. Never imported by app code. */
@Injectable()
export class FakeClipboardApi extends ClipboardApi {
  text: string | null = null;
  readText(): Promise<string | null> {
    return Promise.resolve(this.text);
  }
  writeText(text: string): Promise<void> {
    this.text = text;
    return Promise.resolve();
  }
}
