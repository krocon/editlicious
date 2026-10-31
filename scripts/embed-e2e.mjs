#!/usr/bin/env node
// End-to-end check of the embedded editor (prompt 025): `pnpm build:element`, then
// `node scripts/embed-e2e.mjs` serves site/ and drives site/embed/ in headless Chromium
// (Playwright – not a dependency of the project: present in the dev container and in CI images
// that have it; without it the script says so and stops).
import { createServer } from 'node:http';
import { createRequire } from 'node:module';
import { readFile } from 'node:fs/promises';
import { extname, join, normalize } from 'node:path';
import { fileURLToPath } from 'node:url';

const site = fileURLToPath(new URL('../site/', import.meta.url));
const TYPES = { '.html': 'text/html', '.js': 'text/javascript', '.wasm': 'application/wasm', '.css': 'text/css' };

let chromium;
try {
  ({ chromium } = createRequire(import.meta.url)('playwright'));
} catch {
  console.log('playwright is not installed – skipped');
  process.exit(0);
}

const server = createServer(async (req, res) => {
  const path = normalize(decodeURIComponent(new URL(req.url, 'http://x').pathname)).replace(/^(\.\.[/\\])+/, '');
  const file = join(site, path.endsWith('/') ? path + 'index.html' : path);
  try {
    const body = await readFile(file);
    res.writeHead(200, { 'content-type': TYPES[extname(file)] ?? 'application/octet-stream' });
    res.end(body);
  } catch {
    res.writeHead(404).end();
  }
});
await new Promise((r) => server.listen(0, '127.0.0.1', r));
const url = `http://127.0.0.1:${server.address().port}/embed/`;

const browser = await chromium.launch();
const ctx = await browser.newContext({ permissions: ['clipboard-read', 'clipboard-write'] });
const page = await ctx.newPage();
const errors = [];
page.on('console', (m) => m.type() === 'error' && errors.push(m.text()));
page.on('pageerror', (e) => errors.push(String(e)));
const check = (ok, what) => {
  if (!ok) {
    console.error(`✗ ${what}`);
    process.exitCode = 1;
  } else console.log(`✓ ${what}`);
};
const shot = process.env.SHOT;

try {
  await page.goto(url);
  await page.waitForFunction(() => document.getElementById('log').textContent.includes('b ready'), null, { timeout: 20000 });
  await page.waitForTimeout(500);
  const rows = (id) => page.evaluate((id) => document.getElementById(id).shadowRoot.querySelectorAll('.row').length, id);
  check((await rows('a')) > 0 && (await rows('b')) > 0, 'both editors render');
  check(await page.evaluate(() => !document.documentElement.hasAttribute('data-theme') && !document.documentElement.hasAttribute('data-scheme') && !document.documentElement.style.cssText), '<html> untouched');
  const bg = (id) => page.evaluate((id) => getComputedStyle(document.getElementById(id)).backgroundColor, id);
  check((await bg('a')) !== (await bg('b')), `own colours per instance (${await bg('a')} / ${await bg('b')})`);

  // typing in A leaves B alone
  const textarea = (id) => page.locator(`#${id} textarea`);
  await textarea('a').focus();
  await page.keyboard.type('// hi\n');
  const va = await page.evaluate(() => document.getElementById('a').getValue());
  const vb = await page.evaluate(() => document.getElementById('b').getValue());
  check(va.startsWith('// hi\n') && !vb.includes('// hi'), `typing goes to A only (${JSON.stringify(va.slice(0, 12))})`);
  check((await page.locator('#log').textContent()).includes('a change'), 'ed-change fired');

  // Ctrl+F opens the find bar in A only; keys outside the editors stay with the page
  await page.keyboard.press('Control+f');
  await page.waitForTimeout(200);
  const findOpen = (id) => page.evaluate((id) => !!document.getElementById(id).shadowRoot.querySelector('app-find-bar'), id);
  check((await findOpen('a')) && !(await findOpen('b')), 'Ctrl+F opens the find bar of A only');
  await page.keyboard.type('println');
  await page.keyboard.press('Escape');
  const n = await page.evaluate(() => document.getElementById('a').find('fn'));
  check(n.total >= 1, `find() counts ${n.total}`);

  // copy goes to the system clipboard
  await page.evaluate(() => document.getElementById('a').runCommand({ type: 'selectAll' }));
  await textarea('a').focus();
  await page.keyboard.press('Control+c');
  await page.waitForTimeout(300);
  const clip = await page.evaluate(() => navigator.clipboard.readText());
  check(clip.startsWith('// hi'), 'Ctrl+C puts the selection on the clipboard');

  // read-only
  await page.check('[data-ro="b"]');
  await textarea('b').focus();
  await page.keyboard.type('zzz');
  const vb2 = await page.evaluate(() => document.getElementById('b').getValue());
  check(!vb2.includes('zzz'), 'read-only refuses typing');
  check(await page.evaluate(() => document.getElementById('b').hasAttribute('data-read-only')), 'read-only shows on the host');

  // a million lines
  await page.click('[data-million="a"]');
  await page.waitForFunction(() => document.getElementById('log').textContent.includes('1 000 000 lines'), null, { timeout: 30000 });
  await page.waitForTimeout(500);
  const count = await page.evaluate(() => document.getElementById('a').shadowRoot.querySelector('app-status-bar')?.textContent ?? '');
  check(/1[.,  ]?000[.,  ]?001|1[.,  ]?000[.,  ]?000/.test(count), 'status bar shows a million lines');
  await textarea('a').focus();
  await page.keyboard.press('Control+End');
  const t0 = Date.now();
  await page.keyboard.type('end');
  await page.waitForTimeout(100);
  check(Date.now() - t0 < 2000, `typing at the end of 1 M lines: ${Date.now() - t0} ms`);
  await page.click('[data-long="a"]');
  await page.waitForFunction(() => document.getElementById('log').textContent.includes('5 000 000 characters'), null, { timeout: 30000 });
  check(true, '5 M characters in one line loaded');
  if (shot) await page.screenshot({ path: shot, fullPage: true });
  check(errors.length === 0, `no console errors${errors.length ? ': ' + errors.join(' | ') : ''}`);
} finally {
  await browser.close();
  server.close();
}
