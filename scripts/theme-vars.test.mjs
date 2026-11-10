// The public colour contract of `<editlicious-editor>` (design 026 E2): the README's table, the
// tokens of `src/styles.css`, their use in the component styles and the `scheme="none"` block
// must agree – a colour nobody can reach from outside, or a documented one that does nothing,
// fails here.
import assert from 'node:assert/strict';
import { readdirSync, readFileSync, statSync } from 'node:fs';
import { test } from 'node:test';
import { fileURLToPath } from 'node:url';
import { blocks } from './host-theme.mjs';

const root = fileURLToPath(new URL('..', import.meta.url));
const read = (p) => readFileSync(`${root}${p}`, 'utf8');
const COLOUR = /^--(ed|syn|color)-|^--shadow-popup$/;

/** Custom properties declared in a rule body. */
function declared(body) {
  return [...body.matchAll(/(--[a-z0-9-]+)\s*:/g)].map((m) => m[1]);
}

function cssFiles(dir) {
  return readdirSync(`${root}${dir}`).flatMap((f) => {
    const p = `${dir}/${f}`;
    if (statSync(`${root}${p}`).isDirectory()) return cssFiles(p);
    return f.endsWith('.css') && f !== 'host-theme.css' ? [p] : [];
  });
}

const styles = blocks(read('src/styles.css'));
const base = new Set(declared(styles[0].body).filter((v) => COLOUR.test(v)));
/** The tokens a scheme changes: everything the dark block sets. */
const schemeTokens = new Set(
  declared(styles.find((b) => b.prelude.includes("[data-theme='dark']")).body).filter((v) => COLOUR.test(v)),
);

/** Variables of the README's table; `--ed-marker-0` … `--ed-marker-9` is a range. */
function documented() {
  const out = new Set();
  const text = read('packages/editlicious-editor/README.md');
  const table = text.slice(text.indexOf('### Colour variables'), text.indexOf('The font is set'));
  for (const line of table.split('\n')) {
    const cell = line.split('|')[1] ?? '';
    const names = [...cell.matchAll(/`(--[a-z0-9-]+)`/g)].map((m) => m[1]);
    if (names.length === 2 && cell.includes('…')) {
      const [, stem, a] = names[0].match(/^(.*-)(\d+)$/);
      const b = Number(names[1].match(/(\d+)$/)[1]);
      for (let i = Number(a); i <= b; i++) out.add(`${stem}${i}`);
    } else names.forEach((n) => out.add(n));
  }
  return out;
}

test('the README lists exactly the colour tokens of styles.css', () => {
  const doc = documented();
  assert.deepEqual([...base].filter((v) => !doc.has(v)), [], 'tokens missing in the README');
  assert.deepEqual([...doc].filter((v) => !base.has(v)), [], 'documented but not a token');
  assert.ok(base.size >= 60);
});

test('component styles use only declared tokens and no colour of their own', () => {
  for (const file of cssFiles('src/app')) {
    const css = read(file).replace(/\/\*[\s\S]*?\*\//g, '');
    for (const m of css.matchAll(/var\((--[a-z0-9-]+)/g)) {
      if (COLOUR.test(m[1])) assert.ok(base.has(m[1]), `${file}: ${m[1]} is not a token of styles.css`);
    }
    // the `scheme="none"` block is the one place that derives colours
    const own = file.endsWith('embed/editlicious-editor.css') ? css.replace(/:host\(\[data-scheme='none'\][\s\S]*$/, '') : css;
    const literal = own.match(/#[0-9a-fA-F]{3,8}\b|\brgba?\(|\bhsla?\(/);
    assert.equal(literal, null, `${file}: a colour outside styles.css (${literal?.[0]})`);
  }
});

test('every colour token is used by a style (each variable has its part)', () => {
  const all = cssFiles('src/app').map(read).join('\n') + read('src/styles.css').replace(/^[\s\S]*?\n\}\n/, '');
  const ts = read('src/app/features/editor/editor-view.ts');
  for (const v of base) {
    assert.ok(all.includes(`var(${v}`) || ts.includes(v), `${v} is declared but colours nothing`);
  }
});

test('scheme="none" replaces every token a scheme sets', () => {
  const css = read('src/app/embed/editlicious-editor.css');
  const none = blocks(css).find((b) => b.prelude.startsWith(":host([data-scheme='none'][data-scheme]"));
  const have = new Set(declared(none.body));
  assert.deepEqual([...schemeTokens].filter((v) => !have.has(v)), []);
});
