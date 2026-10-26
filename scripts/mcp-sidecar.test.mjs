import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';
import { sidecarFiles, sidecarName } from './mcp-sidecar.mjs';

test('sidecar names follow Tauri externalBin (name-<triple>[.exe])', () => {
  assert.equal(sidecarName('x86_64-unknown-linux-gnu'), 'editlicious-mcp-x86_64-unknown-linux-gnu');
  assert.equal(sidecarName('x86_64-pc-windows-msvc'), 'editlicious-mcp-x86_64-pc-windows-msvc.exe');
  assert.equal(sidecarName('universal-apple-darwin'), 'editlicious-mcp-universal-apple-darwin');
});

test('the release workflow builds the sidecar and bundles it', () => {
  const wf = readFileSync(new URL('../.github/workflows/release.yml', import.meta.url), 'utf8');
  assert.match(wf, /node scripts\/mcp-sidecar\.mjs universal-apple-darwin/);
  assert.equal((wf.match(/--config src-tauri\/tauri\.sidecar\.conf\.json/g) ?? []).length, 2);
  const conf = JSON.parse(
    readFileSync(new URL('../src-tauri/tauri.sidecar.conf.json', import.meta.url), 'utf8'),
  );
  assert.deepEqual(conf.bundle.externalBin, ['binaries/editlicious-mcp']);
});

test('a universal macOS build needs the sidecar of each architecture too', () => {
  assert.deepEqual(sidecarFiles('universal-apple-darwin'), [
    'editlicious-mcp-aarch64-apple-darwin',
    'editlicious-mcp-x86_64-apple-darwin',
    'editlicious-mcp-universal-apple-darwin',
  ]);
  assert.deepEqual(sidecarFiles('x86_64-pc-windows-msvc'), ['editlicious-mcp-x86_64-pc-windows-msvc.exe']);
});
