import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { mkdirSync, mkdtempSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { compareSums, parseSums, releaseSums } from './release-sums.mjs';

const h = (c) => c.repeat(64);

test('reads the lines sha256sum writes', () => {
  assert.deepEqual(parseSums(`${h('a')}  x.dmg\n${h('b')} *y.msi\n\n`), [
    { hash: h('a'), name: 'x.dmg' },
    { hash: h('b'), name: 'y.msi' },
  ]);
});

test('the release archive of the .app has its own name and bytes – no problem', () => {
  // v1.2.0: CI packed editlicious.app.tar.gz, tauri-action uploaded editlicious_universal.app.tar.gz
  const built = [
    { hash: h('a'), name: 'editlicious_1.2.0_universal.dmg' },
    { hash: h('c'), name: 'editlicious.app.tar.gz' },
  ];
  const release = [
    { hash: h('a'), name: 'editlicious_1.2.0_universal.dmg' },
    { hash: h('d'), name: 'editlicious_universal.app.tar.gz' },
  ];
  assert.deepEqual(compareSums(built, release), []);
});

test('a package that is missing on the release, or differs, is a problem', () => {
  const built = [
    { hash: h('a'), name: 'a.dmg' },
    { hash: h('b'), name: 'b.msi' },
    { hash: h('c'), name: 'x.app.tar.gz' },
  ];
  assert.deepEqual(compareSums(built, [{ hash: h('9'), name: 'a.dmg' }]), [
    'a.dmg: release differs from the build',
    'b.msi: built, but not on the release',
    'x.app.tar.gz: built, but the release has no .app.tar.gz',
  ]);
});

test('SHA256SUMS.txt lists exactly the files of the release', () => {
  const dir = mkdtempSync(join(tmpdir(), 'ed-release-'));
  const built = join(dir, 'built');
  const rel = join(dir, 'release');
  mkdirSync(built);
  mkdirSync(rel);
  writeFileSync(join(rel, 'b.dmg'), 'dmg');
  writeFileSync(join(rel, 'a_universal.app.tar.gz'), 'tar from tauri-action');
  writeFileSync(join(rel, 'SHA256SUMS.txt'), 'old');
  const dmg = createHash('sha256').update('dmg').digest('hex');
  writeFileSync(join(built, 'SHA256SUMS-macos.txt'), `${dmg}  b.dmg\n${h('e')}  a.app.tar.gz\n`);
  const { text, problems } = releaseSums(rel, built);
  assert.deepEqual(problems, []);
  assert.deepEqual(parseSums(text), [
    { hash: createHash('sha256').update('tar from tauri-action').digest('hex'), name: 'a_universal.app.tar.gz' },
    { hash: dmg, name: 'b.dmg' },
  ]);
});

test('a build that differs from the release is reported', () => {
  const dir = mkdtempSync(join(tmpdir(), 'ed-release-'));
  writeFileSync(join(dir, 'b.dmg'), 'dmg');
  writeFileSync(join(dir, 'SHA256SUMS-macos.txt'), `${h('0')}  b.dmg\n`);
  assert.deepEqual(releaseSums(dir, dir).problems, ['b.dmg: release differs from the build']);
});
