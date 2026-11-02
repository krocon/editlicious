import assert from 'node:assert/strict';
import { test } from 'node:test';
import { packageJson, publishedName, sizeTable, wanted } from './element-bundle.mjs';

test('the bundle keeps the element, its worker and the engine', () => {
  assert.equal(publishedName('main.js'), 'editlicious-editor.js');
  assert.equal(publishedName('worker-ISWCHNBU.js'), 'worker-ISWCHNBU.js');
  assert.ok(wanted('main.js'));
  assert.ok(wanted('worker-ISWCHNBU.js'));
  assert.ok(wanted('ed_wasm.js'));
  assert.ok(wanted('ed_wasm_bg.wasm'));
  assert.ok(!wanted('3rdpartylicenses.txt'));
  assert.ok(!wanted('chunk-ABC.js'));
});

test('the npm package gets the app version and a size table', () => {
  const pkg = JSON.parse(packageJson('{"name":"editlicious-editor","version":"0.0.0"}', '1.2.3'));
  assert.deepEqual(pkg, { name: 'editlicious-editor', version: '1.2.3' });
  const table = sizeTable([
    { name: 'editlicious-editor.js', raw: 2048, gzip: 1024 },
    { name: 'worker-ISWCHNBU.js', raw: 1024, gzip: 512 },
  ]);
  assert.match(table, /\| `worker-\*\.js` \| 1\.0 KB \| 0\.5 KB \|/);
  assert.match(table, /\| \*\*total\*\* \| 3\.0 KB \| 1\.5 KB \|/);
});
