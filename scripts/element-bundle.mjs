#!/usr/bin/env node
// After `ng run editlicious:element` (design 025 E11): the custom element as one folder –
// `editlicious-editor.js` (+ its worker) next to the engine (`ed_wasm.js`, `ed_wasm_bg.wasm`) –
// in `dist/element/editlicious-editor/` and for the demo page in `site/embed/lib/` (not
// committed). Prints the sizes (raw and gzip).
import { copyFileSync, mkdirSync, readdirSync, readFileSync, rmSync, statSync } from 'node:fs';
import { gzipSync } from 'node:zlib';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('..', import.meta.url));
const from = `${root}dist/element/browser`;

/** Published name of a built file. */
export function publishedName(file) {
  return file === 'main.js' ? 'editlicious-editor.js' : file;
}

export function wanted(file) {
  return /^(main|worker-[A-Z0-9]+)\.js$|^ed_wasm(\.js|_bg\.wasm)$/.test(file);
}

function kb(n) {
  return `${(n / 1024).toFixed(1)} KB`;
}

if (process.argv[1] && fileURLToPath(import.meta.url) === process.argv[1]) {
  const files = readdirSync(from).filter(wanted);
  if (!files.includes('main.js')) throw new Error('run `ng run editlicious:element` first');
  for (const to of [`${root}dist/element/editlicious-editor`, `${root}site/embed/lib`]) {
    rmSync(to, { recursive: true, force: true });
    mkdirSync(to, { recursive: true });
    for (const f of files) copyFileSync(`${from}/${f}`, `${to}/${publishedName(f)}`);
  }
  let raw = 0;
  let gz = 0;
  for (const f of files) {
    const bytes = readFileSync(`${from}/${f}`);
    const z = gzipSync(bytes, { level: 9 }).length;
    raw += statSync(`${from}/${f}`).size;
    gz += z;
    console.log(`${publishedName(f).padEnd(26)} ${kb(bytes.length).padStart(10)} ${kb(z).padStart(10)} gzip`);
  }
  console.log(`${'total'.padEnd(26)} ${kb(raw).padStart(10)} ${kb(gz).padStart(10)} gzip`);
}
