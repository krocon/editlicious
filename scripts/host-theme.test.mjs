import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';
import { hostSelector, hostTheme, SOURCE, TARGET } from './host-theme.mjs';

test('root, scheme and theme selectors become :host forms; previews and html go', () => {
  assert.equal(hostSelector(':root'), ':host');
  assert.equal(hostSelector(":root[data-scheme='glass']"), ":host([data-scheme='glass'])");
  assert.equal(hostSelector(":root:not([data-theme='light'])"), ":host(:not([data-theme='light']))");
  assert.equal(hostSelector('body'), ':host');
  assert.equal(hostSelector("[data-preview][data-preview='sand']"), null);
  assert.equal(hostSelector('.btn'), '.btn');
});

test('media queries and nesting survive, html rules are dropped', () => {
  const css = `/* c */
:root, [data-preview] { --a: 1; }
@media (prefers-color-scheme: dark) { :root:not([data-theme='light']) { --a: 2; } }
html, body { height: 100%; }
.btn { color: red; &:hover { color: blue; } }`;
  const out = hostTheme(css);
  assert.match(out, /^:host \{ --a: 1;/m);
  assert.match(out, /@media \(prefers-color-scheme: dark\) \{\n:host\(:not\(\[data-theme='light'\]\)\)/);
  assert.doesNotMatch(out, /height: 100%/);
  assert.doesNotMatch(out, /data-preview|:root|host-context/);
  assert.match(out, /&:hover \{ color: blue; \}/);
});

test('the generated file is up to date with src/styles.css', () => {
  assert.equal(readFileSync(TARGET, 'utf8'), hostTheme(readFileSync(SOURCE, 'utf8')));
  // all 13 schemes are there
  const schemes = [...readFileSync(TARGET, 'utf8').matchAll(/:host\(\[data-scheme='(\w+)'\]\)/g)];
  assert.ok(new Set(schemes.map((m) => m[1])).size >= 11);
});
