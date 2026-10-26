// Checks of the website (prompt 011): every picture exists and has its size, nothing is loaded
// from other servers, the page has no script, the demo has its engine. `pnpm site:test`
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync, existsSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const site = join(dirname(fileURLToPath(import.meta.url)), '..', 'site');
const html = readFileSync(join(site, 'index.html'), 'utf8');

function pngSize(file) {
  const b = readFileSync(file);
  return { width: b.readUInt32BE(16), height: b.readUInt32BE(20) };
}

test('every picture exists, lives in img/ and has its real size', () => {
  const imgs = [...html.matchAll(/<img\s[^>]*src="([^"]+)"[^>]*>/g)];
  assert.ok(imgs.length >= 5);
  for (const [tag, src] of imgs) {
    assert.ok(src.startsWith('img/') || src === 'favicon.svg', src);
    assert.ok(existsSync(join(site, src)), `missing ${src}`);
    if (src.endsWith('.png')) {
      const w = Number(/width="(\d+)"/.exec(tag)?.[1]);
      const h = Number(/height="(\d+)"/.exec(tag)?.[1]);
      assert.deepEqual(pngSize(join(site, src)), { width: w, height: h }, src);
      assert.match(tag, /alt="[^"]{20,}"/, `alt text for ${src}`);
    }
  }
});

test('no script, no requests to other servers', () => {
  assert.doesNotMatch(html, /<script/i);
  assert.doesNotMatch(html, /\b(src|href)="(https?:)?\/\/(?!github\.com|krocon\.de)/);
  assert.doesNotMatch(html, /<link[^>]+href="https?:/);
});

test('sections the navigation and buttons point at exist', () => {
  for (const [, id] of html.matchAll(/href="#([\w-]+)"/g)) {
    assert.match(html, new RegExp(`id="${id}"`), id);
  }
});

test('the demo page and its engine', () => {
  const demo = readFileSync(join(site, 'demo', 'index.html'), 'utf8');
  assert.match(demo, /<script type="module" src="demo.js">/);
  assert.match(readFileSync(join(site, 'demo', 'demo.js'), 'utf8'), /from '.\/pkg\/ed_wasm.js'/);
  for (const f of ['ed_wasm.js', 'ed_wasm_bg.wasm']) {
    assert.ok(existsSync(join(site, 'demo', 'pkg', f)), `site/demo/pkg/${f} – run pnpm site:demo`);
  }
});
