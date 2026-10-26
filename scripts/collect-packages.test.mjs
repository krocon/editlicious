import { test } from 'node:test';
import assert from 'node:assert/strict';
import { mkdirSync, mkdtempSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { bundleDirs, findPackages, size, sumsText, summary } from './collect-packages.mjs';

function fakeTarget(files) {
  const t = mkdtempSync(join(tmpdir(), 'ed-target-'));
  for (const f of files) {
    const p = join(t, f);
    mkdirSync(join(p, '..'), { recursive: true });
    if (f.endsWith('.app')) mkdirSync(p, { recursive: true });
    else writeFileSync(p, 'x');
  }
  return t;
}

test('finds bundles of the host and of a target triple', () => {
  const t = fakeTarget([
    'release/bundle/deb/a.deb',
    'universal-apple-darwin/release/bundle/dmg/a.dmg',
    'release/build/x/bundle/ignored.deb',
  ]);
  assert.deepEqual(
    bundleDirs(t).map((d) => d.slice(t.length + 1)),
    ['release/bundle', 'universal-apple-darwin/release/bundle'],
  );
});

test('reports every kind of an OS, and what is missing', () => {
  const t = fakeTarget([
    'release/bundle/deb/editlicious_0.1.0_amd64.deb',
    'release/bundle/rpm/editlicious-0.1.0-1.x86_64.rpm',
    'release/bundle/appimage/editlicious_0.1.0_amd64.AppImage',
    'release/bundle/msi/editlicious_0.1.0_x64_en-US.msi',
  ]);
  const linux = findPackages('linux', t);
  assert.deepEqual(linux.found.map((f) => f.kind), ['deb', 'rpm', 'appimage']);
  assert.deepEqual(linux.missing, []);
  const win = findPackages('windows', t);
  assert.deepEqual(win.found.map((f) => f.kind), ['msi']);
  assert.deepEqual(win.missing, ['nsis']);
  const mac = findPackages('macos', fakeTarget(['universal-apple-darwin/release/bundle/macos/editlicious.app']));
  assert.deepEqual(mac.missing, ['dmg']);
  assert.throws(() => findPackages('beos', t));
});

test('sums file and summary', () => {
  const e = [{ name: 'a.deb', bytes: 6_720_000, hash: 'ab'.repeat(32) }];
  assert.equal(sumsText(e), `${'ab'.repeat(32)}  a.deb\n`);
  assert.equal(size(6_720_000), '6.7 MB');
  assert.equal(size(1500), '2 KB');
  assert.match(summary('linux', e, ['rpm']), /\| a\.deb \| 6\.7 MB \|[\s\S]*Missing:\*\* rpm/);
});
