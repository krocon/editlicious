import assert from 'node:assert/strict';
import { test } from 'node:test';
import { publishedName, wanted } from './element-bundle.mjs';

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
