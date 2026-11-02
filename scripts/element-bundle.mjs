#!/usr/bin/env node
// After `ng run editlicious:element` (design 025 E11): the custom element as one folder –
// `editlicious-editor.js` (+ its worker) next to the engine (`ed_wasm.js`, `ed_wasm_bg.wasm`) –
// in `dist/element/editlicious-editor/` and for the demo page in `site/embed/lib/` (not
// committed). The dist folder is the npm package `editlicious-editor`: package.json (version of
// the app), types and README from `packages/editlicious-editor/`, LICENSE and notices. Prints the
// sizes (raw and gzip); `npm pack dist/element/editlicious-editor` makes the tarball.
import { copyFileSync, mkdirSync, readdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { gzipSync } from 'node:zlib';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('..', import.meta.url));
const from = `${root}dist/element/browser`;
const pkgSrc = `${root}packages/editlicious-editor`;

/** Published name of a built file. */
export function publishedName(file) {
  return file === 'main.js' ? 'editlicious-editor.js' : file;
}

export function wanted(file) {
  return /^(main|worker-[A-Z0-9]+)\.js$|^ed_wasm(\.js|_bg\.wasm)$/.test(file);
}

/** The package's package.json: the template with the app's version. */
export function packageJson(template, version) {
  return `${JSON.stringify({ ...JSON.parse(template), version }, null, 2)}\n`;
}

function kb(n) {
  return `${(n / 1024).toFixed(1)} KB`;
}

/** The README's size table (`rows`: `{ name, raw, gzip }` in bytes). */
export function sizeTable(rows) {
  const total = rows.reduce((t, r) => ({ raw: t.raw + r.raw, gzip: t.gzip + r.gzip }), { raw: 0, gzip: 0 });
  const line = (name, r) => `| ${name} | ${kb(r.raw)} | ${kb(r.gzip)} |`;
  const named = rows.map((r) => line(`\`${r.name.replace(/^worker-[A-Z0-9]+/, 'worker-*')}\``, r));
  return ['| File | Size | gzip |', '|---|---:|---:|', ...named, line('**total**', total)].join('\n');
}

if (process.argv[1] && fileURLToPath(import.meta.url) === process.argv[1]) {
  const files = readdirSync(from).filter(wanted);
  if (!files.includes('main.js')) throw new Error('run `ng run editlicious:element` first');
  const rows = files.map((f) => {
    const bytes = readFileSync(`${from}/${f}`);
    return { name: publishedName(f), raw: bytes.length, gzip: gzipSync(bytes, { level: 9 }).length };
  });
  const pkg = `${root}dist/element/editlicious-editor`;
  for (const to of [pkg, `${root}site/embed/lib`]) {
    rmSync(to, { recursive: true, force: true });
    mkdirSync(to, { recursive: true });
    for (const f of files) copyFileSync(`${from}/${f}`, `${to}/${publishedName(f)}`);
  }
  const { version } = JSON.parse(readFileSync(`${root}package.json`, 'utf8'));
  writeFileSync(`${pkg}/package.json`, packageJson(readFileSync(`${pkgSrc}/package.json`, 'utf8'), version));
  writeFileSync(
    `${pkg}/README.md`,
    readFileSync(`${pkgSrc}/README.md`, 'utf8').replace('SIZE_TABLE', sizeTable(rows)),
  );
  copyFileSync(`${pkgSrc}/editlicious-editor.d.ts`, `${pkg}/editlicious-editor.d.ts`);
  copyFileSync(`${root}LICENSE`, `${pkg}/LICENSE`);
  copyFileSync(`${root}THIRD-PARTY-NOTICES.txt`, `${pkg}/THIRD-PARTY-NOTICES.txt`);
  for (const r of rows) {
    console.log(`${r.name.padEnd(26)} ${kb(r.raw).padStart(10)} ${kb(r.gzip).padStart(10)} gzip`);
  }
  console.log(`npm package ${version}: ${pkg}`);
}
