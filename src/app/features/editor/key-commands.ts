import { Command, Motion } from '../../core/editor/editor.models';

/** The keys of a keydown that matter for the editor (testable without DOM events). */
export interface KeyInput {
  key: string;
  ctrlKey: boolean;
  metaKey: boolean;
  altKey: boolean;
  shiftKey: boolean;
}

export type Os = 'mac' | 'other';

export function detectOs(nav: Pick<Navigator, 'userAgent'> | undefined = globalThis.navigator): Os {
  return nav && /Mac|iPhone|iPad/.test(nav.userAgent) ? 'mac' : 'other';
}

/**
 * Editing keys of the text area → an engine command (design 003 E6). `null` = not ours (typing
 * goes through the textarea's `input` event). Shortcuts of the app (open, save …) follow with
 * the action system in prompt 005.
 */
export function keyToCommand(e: KeyInput, os: Os): Command | null {
  const mac = os === 'mac';
  const mod = mac ? e.metaKey : e.ctrlKey;
  /** word-wise: Alt on macOS, Ctrl elsewhere */
  const word = mac ? e.altKey && !e.metaKey && !e.ctrlKey : e.ctrlKey && !e.altKey;
  const select = e.shiftKey;
  const move = (to: Motion): Command => ({ type: 'move', to, select });

  switch (e.key) {
    case 'ArrowLeft':
      if (mac && e.metaKey) return move('lineStart');
      return move(word ? 'wordLeft' : 'left');
    case 'ArrowRight':
      if (mac && e.metaKey) return move('lineEnd');
      return move(word ? 'wordRight' : 'right');
    case 'ArrowUp':
      if (mac && e.metaKey) return move('docStart');
      if (!mac && e.ctrlKey) return { type: 'scroll', lines: -1, cells: 0 };
      return move('up');
    case 'ArrowDown':
      if (mac && e.metaKey) return move('docEnd');
      if (!mac && e.ctrlKey) return { type: 'scroll', lines: 1, cells: 0 };
      return move('down');
    case 'Home':
      return move(e.ctrlKey || e.metaKey ? 'docStart' : 'lineStart');
    case 'End':
      return move(e.ctrlKey || e.metaKey ? 'docEnd' : 'lineEnd');
    case 'PageUp':
      return move('pageUp');
    case 'PageDown':
      return move('pageDown');
    case 'Backspace':
      return { type: word ? 'deleteWordLeft' : 'backspace' };
    case 'Delete':
      return { type: word ? 'deleteWordRight' : 'delete' };
    case 'Enter':
      return mod || e.altKey ? null : { type: 'newline' };
    case 'Tab':
      return mod || e.altKey ? null : { type: e.shiftKey ? 'outdent' : 'indent' };
    case 'Insert':
      return e.shiftKey || mod ? null : { type: 'toggleOverwrite' };
  }
  if (!mod || e.altKey) return null;
  switch (e.key.toLowerCase()) {
    case 'a':
      return { type: 'selectAll' };
    case 'z':
      return { type: e.shiftKey ? 'redo' : 'undo' };
    case 'y':
      return mac ? null : { type: 'redo' };
    case 'c':
      return { type: 'copy' };
    case 'x':
      return { type: 'cut' };
    case 'v':
      return { type: 'paste' };
  }
  return null;
}
