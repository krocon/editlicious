#!/usr/bin/env node
// Collects the packages tauri built into one folder, checks that every expected kind is there,
// writes SHA-256 sums and a Markdown summary (design 014, release pipeline).
//
//   node scripts/collect-packages.mjs <os: macos|windows|linux> <out-dir> [<target-dir>]
//
// <target-dir> defaults to src-tauri/target. A macOS .app (a folder) is packed as .app.tar.gz,
// like tauri-action does for releases. Exit 1 when a kind is missing.
import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import {
  appendFileSync,
  copyFileSync,
  existsSync,
  mkdirSync,
  readdirSync,
  readFileSync,
  statSync,
  writeFileSync,
} from 'node:fs';
import { basename, dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const ROOT = join(dirname(fileURLToPath(import.meta.url)), '..');

/** Package kinds per OS: bundle sub-folder and file pattern. */
export const EXPECTED = {
  macos: [
    { kind: 'dmg', dir: 'dmg', match: /\.dmg$/ },
    { kind: 'app', dir: 'macos', match: /\.app$/ },
  ],
  windows: [
    { kind: 'msi', dir: 'msi', match: /\.msi$/ },
    { kind: 'nsis', dir: 'nsis', match: /-setup\.exe$/ },
  ],
  linux: [
    { kind: 'deb', dir: 'deb', match: /\.deb$/ },
    { kind: 'rpm', dir: 'rpm', match: /\.rpm$/ },
    { kind: 'appimage', dir: 'appimage', match: /\.AppImage$/ },
  ],
};

/** Every `bundle` folder below `target` (…/release/bundle, …/<triple>/release/bundle). */
export function bundleDirs(target) {
  const out = [];
  const walk = (dir, depth) => {
    if (depth > 3 || !existsSync(dir)) return;
    for (const e of readdirSync(dir, { withFileTypes: true })) {
      if (!e.isDirectory()) continue;
      const p = join(dir, e.name);
      if (e.name === 'bundle' && basename(dir) === 'release') out.push(p);
      else if (e.name !== 'bundle' && e.name !== 'deps' && e.name !== 'build') walk(p, depth + 1);
    }
  };
  walk(target, 0);
  return out.sort();
}

/** The packages of one OS: `{ kind, path }`, and the kinds that are missing. */
export function findPackages(os, target) {
  const kinds = EXPECTED[os];
  if (!kinds) throw new Error(`unknown os "${os}"`);
  const found = [];
  for (const b of bundleDirs(target)) {
    for (const k of kinds) {
      const dir = join(b, k.dir);
      if (!existsSync(dir)) continue;
      for (const name of readdirSync(dir).sort()) {
        if (k.match.test(name)) found.push({ kind: k.kind, path: join(dir, name) });
      }
    }
  }
  const missing = kinds.map((k) => k.kind).filter((k) => !found.some((f) => f.kind === k));
  return { found, missing };
}

export function sha256(file) {
  return createHash('sha256').update(readFileSync(file)).digest('hex');
}

/** "12.3 MB" */
export function size(bytes) {
  return bytes >= 1e6 ? `${(bytes / 1e6).toFixed(1)} MB` : `${Math.ceil(bytes / 1e3)} KB`;
}

/** Lines of SHA256SUMS.txt (`<hash>  <name>`, the format `sha256sum -c` reads). */
export function sumsText(entries) {
  return entries.map((e) => `${e.hash}  ${e.name}\n`).join('');
}

export function summary(os, entries, missing) {
  const rows = entries.map((e) => `| ${e.name} | ${size(e.bytes)} | \`${e.hash.slice(0, 16)}…\` |`);
  const lines = [`### Packages (${os})`, '', '| File | Size | SHA-256 |', '|---|---|---|', ...rows, ''];
  if (missing.length) lines.push(`**Missing:** ${missing.join(', ')}`, '');
  return lines.join('\n');
}

function main([os, out, target = join(ROOT, 'src-tauri', 'target')]) {
  if (!os || !out) {
    console.error('usage: node scripts/collect-packages.mjs <macos|windows|linux> <out-dir> [<target-dir>]');
    return 2;
  }
  const { found, missing } = findPackages(os, target);
  mkdirSync(out, { recursive: true });
  const entries = [];
  for (const f of found) {
    let file = join(out, basename(f.path));
    if (statSync(f.path).isDirectory()) {
      // the .app folder: one archive, as on the release page
      file = `${file}.tar.gz`;
      execFileSync('tar', ['-czf', file, '-C', dirname(f.path), basename(f.path)]);
    } else {
      copyFileSync(f.path, file);
    }
    entries.push({ name: basename(file), bytes: statSync(file).size, hash: sha256(file) });
  }
  writeFileSync(join(out, `SHA256SUMS-${os}.txt`), sumsText(entries));
  const md = summary(os, entries, missing);
  console.log(md);
  if (process.env.GITHUB_STEP_SUMMARY) appendFileSync(process.env.GITHUB_STEP_SUMMARY, md + '\n');
  if (missing.length) {
    console.error(`missing package kinds: ${missing.join(', ')}`);
    return 1;
  }
  return 0;
}

if (process.argv[1] && fileURLToPath(import.meta.url) === process.argv[1]) {
  process.exitCode = main(process.argv.slice(2));
}
