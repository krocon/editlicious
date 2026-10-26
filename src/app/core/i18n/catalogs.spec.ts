import { describe, expect, it } from 'vitest';
import { ACTIONS } from '../actions/action-catalog';
import { ACTION_LABELS_DE } from '../actions/action-catalog.de';
import { CATALOGS } from './i18n';
// every catalog registers itself on import
import '../errors/fs-error';
import '../../features/about/about-texts';
import '../../features/documents/documents-texts';
import '../../features/editor/editor-texts';
import '../../features/find/find-texts';
import '../../features/menus/menus-texts';
import '../../features/recording/recording-texts';
import '../../features/settings/settings-texts';

/** Shape of a catalog: keys and value kinds (functions by arity). */
function shape(v: unknown): unknown {
  if (typeof v === 'function') return `fn/${v.length}`;
  if (typeof v === 'string') return 'string';
  if (Array.isArray(v)) return v.map(shape);
  if (v && typeof v === 'object') {
    return Object.fromEntries(
      Object.keys(v)
        .sort()
        .map((k) => [k, shape((v as Record<string, unknown>)[k])]),
    );
  }
  return typeof v;
}

describe('catalogs (design 009)', () => {
  it('registers every catalog', () => {
    for (const name of ['about', 'documents', 'editor', 'find', 'menus', 'recording', 'settings']) {
      expect([...CATALOGS.keys()].some((k) => k.startsWith(name))).toBe(true);
    }
  });

  it('German has exactly the keys of English', () => {
    for (const [name, c] of CATALOGS) {
      if (name.startsWith('test.')) continue;
      expect(shape(c.tr['de']), name).toEqual(shape(c.en));
    }
  });

  it('German texts are not empty', () => {
    for (const [name, c] of CATALOGS) {
      const walk = (v: unknown, path: string): void => {
        if (typeof v === 'string') expect(v.trim(), `${name}.${path}`).not.toBe('');
        else if (v && typeof v === 'object')
          for (const [k, x] of Object.entries(v)) walk(x, `${path}.${k}`);
      };
      walk(c.tr['de'], '');
    }
  });

  it('every action has a German label', () => {
    const missing = ACTIONS.map((a) => a.id).filter((id) => !ACTION_LABELS_DE[id]);
    expect(missing).toEqual([]);
  });

  it('no German label for an action that does not exist', () => {
    const ids = new Set(ACTIONS.map((a) => a.id));
    expect(Object.keys(ACTION_LABELS_DE).filter((id) => !ids.has(id))).toEqual([]);
  });
});
