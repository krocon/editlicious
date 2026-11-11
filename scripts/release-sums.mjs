#!/usr/bin/env node
// SHA256SUMS.txt for the release (docs/release.md): the sums of the files that really hang on the
// draft release, compared with what the build jobs packed (SHA256SUMS-<os>.txt of
// collect-packages.mjs).
//
//   node scripts/release-sums.mjs <release-dir> <built-dir>
//
// Why not just the build's sums: tauri-action packs the macOS .app for the release itself
// (`editlicious_universal.app.tar.gz`), with other bytes than the archive collect-packages.mjs
// makes (`editlicious.app.tar.gz`) – in v1.2.0 that line of SHA256SUMS.txt named a file the release
// did not have. Every other package must be on the release byte for byte as built.
// Writes <release-dir>/SHA256SUMS.txt; exit 1 on a problem.
import { readdirSync, readFileSync, statSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { sha256, sumsText } from './collect-packages.mjs';

const SUMS = 'SHA256SUMS.txt';
const APP_ARCHIVE = /\.app\.tar\.gz$/;

/** Lines of a sha256sum file → `{ hash, name }` (binary marker `*` dropped). */
export function parseSums(text) {
  return text
    .split('\n')
    .map((l) => l.match(/^([0-9a-f]{64}) [ *](.+)$/))
    .filter(Boolean)
    .map((m) => ({ hash: m[1], name: m[2] }));
}

/** What is wrong with the release compared with the build (empty: nothing). */
export function compareSums(built, release) {
  const byName = new Map(release.map((r) => [r.name, r.hash]));
  const problems = [];
  for (const b of built) {
    if (APP_ARCHIVE.test(b.name)) {
      if (!release.some((r) => APP_ARCHIVE.test(r.name))) {
        problems.push(`${b.name}: built, but the release has no .app.tar.gz`);
      }
    } else if (!byName.has(b.name)) {
      problems.push(`${b.name}: built, but not on the release`);
    } else if (byName.get(b.name) !== b.hash) {
      problems.push(`${b.name}: release differs from the build`);
    }
  }
  return problems;
}

/** Sums of every file in `releaseDir` (but an old SHA256SUMS.txt), checked against `builtDir`. */
export function releaseSums(releaseDir, builtDir) {
  const release = readdirSync(releaseDir)
    .filter((n) => n !== SUMS && !/^SHA256SUMS-.*\.txt$/.test(n) && statSync(join(releaseDir, n)).isFile())
    .sort()
    .map((name) => ({ hash: sha256(join(releaseDir, name)), name }));
  const built = readdirSync(builtDir)
    .filter((n) => /^SHA256SUMS-.*\.txt$/.test(n))
    .flatMap((n) => parseSums(readFileSync(join(builtDir, n), 'utf8')));
  return { text: sumsText(release), problems: compareSums(built, release) };
}

function main([releaseDir, builtDir]) {
  if (!releaseDir || !builtDir) {
    console.error('usage: node scripts/release-sums.mjs <release-dir> <built-dir>');
    return 2;
  }
  const { text, problems } = releaseSums(releaseDir, builtDir);
  writeFileSync(join(releaseDir, SUMS), text);
  process.stdout.write(text);
  for (const p of problems) console.error(p);
  return problems.length ? 1 : 0;
}

if (process.argv[1] && fileURLToPath(import.meta.url) === process.argv[1]) {
  process.exitCode = main(process.argv.slice(2));
}
