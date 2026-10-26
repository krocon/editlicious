// editlicious live demo (prompt 011): the real engine (`ed-core`, compiled to WebAssembly)
// runs in the page; this script only turns keys and mouse into commands and draws the
// frames – exactly the split of the app (design 002 E13). No other code, no requests.
import init, { Editor, languages } from './pkg/ed_wasm.js';

const SAMPLE = `// Welcome to editlicious – this is the real engine, running in your browser.
// Type, select with Shift, add carets with Ctrl/⌘+D, move lines with Alt+↑/↓,
// undo with Ctrl/⌘+Z. Try the buttons above for a really big file.

use std::collections::HashMap;

/// Counts the words of a text, case-insensitively.
fn word_count(text: &str) -> HashMap<String, usize> {
    let mut counts = HashMap::new();
    for word in text.split_whitespace() {
        let w = word.trim_matches(|c: char| !c.is_alphanumeric()).to_lowercase();
        if !w.is_empty() {
            *counts.entry(w).or_insert(0) += 1;
        }
    }
    counts
}

fn main() {
    let text = "Is tastier than chocolate. Tastier than chocolate? Yes!";
    let mut words: Vec<_> = word_count(text).into_iter().collect();
    words.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    for (word, n) in words.iter().take(3) {
        println!("{word:>12} {n}"); // 'tastier' and 'than' lead
    }
}
`;

const MAC = /Mac|iPhone|iPad/.test(navigator.platform);
const $ = (s) => document.querySelector(s);
const host = $('#editor');
const gutter = $('#gutter');
const textEl = $('#text');
const input = $('#input');
const vbar = $('#vbar');
const thumb = $('#thumb');
const statusEl = $('#status');
const measure = $('#measure');

let ed = null;
let frame = null;
let cw = 8;
let lh = 22;
let wrap = false;
let pending = null;
let raf = 0;

const esc = (s) => s.replace(/[&<>]/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;' })[c]);

function send(cmd) {
  if (!ed) return;
  try {
    frame = JSON.parse(ed.command(JSON.stringify(cmd)));
    draw();
  } catch (e) {
    statusEl.textContent = String(e);
  }
}

function measureFont() {
  const r = measure.getBoundingClientRect();
  cw = r.width / 64 || 8;
  lh = Math.round(parseFloat(getComputedStyle(measure).fontSize) * 1.55) || 22;
  host.style.setProperty('--lh', `${lh}px`);
}

const gutterCells = () => Math.max(3, String(frame?.info.lineCount ?? 1).length) + 2;
const textLeft = () => (gutterCells() + 1) * cw;
const viewLines = () => host.clientHeight / lh;
const viewCols = () => Math.max(1, host.clientWidth - textLeft() - 10) / cw;

function resize() {
  send({ type: 'resize', lines: viewLines(), cols: viewCols() });
}

// ------------------------------------------------------------------ drawing

function rowOf(line, cell) {
  let r = 0;
  while (r < line.wraps.length && line.wraps[r] <= cell) r++;
  return r;
}

function offset(line) {
  return frame.wrap && line && !line.long ? 0 : frame.left;
}

function px(line, cell) {
  const r = rowOf(line, cell);
  const start = r ? line.wraps[r - 1] : 0;
  return { x: (cell - start - offset(line)) * cw, y: r * lh };
}

function boxes(line, a, b, cls) {
  let out = '';
  let from = a;
  while (from < b) {
    const r = rowOf(line, from);
    const end = Math.min(b, r < line.wraps.length ? line.wraps[r] : b);
    const p = px(line, from);
    out += `<div class="${cls}" style="left:${p.x}px;top:${p.y}px;width:${(end - from) * cw}px"></div>`;
    from = end;
  }
  return out;
}

function draw() {
  const f = frame;
  const tl = textLeft();
  gutter.style.width = `${gutterCells() * cw}px`;
  textEl.style.left = `${tl}px`;
  let g = '';
  let t = '';
  const caretLines = new Set(f.carets.map((c) => c.line));
  for (const line of f.lines) {
    const y = line.row * lh;
    const rows = line.wraps.length + 1;
    const active = caretLines.has(line.line);
    g += `<div class="ln${active ? ' active' : ''}" style="top:${y}px">${line.line + 1}</div>`;
    let inner = '';
    for (const s of line.sel) inner += boxes(line, s[0], s[1], 'sel');
    for (const m of line.marks) inner += boxes(line, m[0], m[1], m[2] ? 'mark current' : 'mark');
    for (const m of line.markers) inner += boxes(line, m[0], m[1], `marker c${m[2]}`);
    if (line.selEol) {
      const p = px(line, line.width);
      inner += `<div class="sel" style="left:${p.x}px;top:${p.y}px;width:${cw * 0.6}px"></div>`;
    }
    for (const run of line.runs) {
      const p = px(line, run.cell);
      const pos = `left:${p.x}px;top:${p.y}px`;
      if (run.kind === 'tab') {
        inner += `<span class="run tab" style="${pos};width:${run.width * cw}px"></span>`;
      } else if (run.kind === 'wide') {
        inner += `<span class="run" style="${pos}">${Array.from(run.text, (c) => `<span class="wide" style="width:${2 * cw}px">${esc(c)}</span>`).join('')}</span>`;
      } else {
        const s = run.style ? ` data-s="${run.style}"` : '';
        const k = run.kind === 'ctrl' ? ' ctrl' : '';
        inner += `<span class="run${k}"${s} style="${pos}">${esc(run.text)}</span>`;
      }
    }
    const cur = active && f.selChars === 0 ? ' current' : '';
    t += `<div class="row${cur}" style="top:${y}px;height:${rows * lh}px">${inner}</div>`;
  }
  f.caretXy.forEach((xy, i) => {
    const line = f.lines.find((l) => l.line === f.carets[i].line);
    const x = (xy.x - offset(line)) * cw;
    t += `<div class="caret" style="transform:translate(${x}px,${xy.row * lh}px)"></div>`;
  });
  gutter.innerHTML = g;
  textEl.innerHTML = t;
  const p = f.caretXy[f.primary] ?? { x: 0, row: 0 };
  input.style.left = `${tl + p.x * cw}px`;
  input.style.top = `${p.row * lh}px`;
  // the scroll bar (lines – approximate when wrapped)
  const h = host.clientHeight;
  const n = f.info.lineCount;
  if (n > Math.floor(viewLines())) {
    const th = Math.max(28, (h * viewLines()) / (n + viewLines()));
    vbar.hidden = false;
    thumb.style.height = `${th}px`;
    thumb.style.transform = `translateY(${((h - th) * f.top) / Math.max(1, n - 1)}px)`;
  } else {
    vbar.hidden = true;
  }
  const c = f.carets[f.primary];
  const sel = f.selChars ? ` · ${f.selChars.toLocaleString()} selected` : '';
  const many = f.carets.length > 1 ? ` · ${f.carets.length} carets` : '';
  statusEl.textContent =
    `Ln ${(f.caretLine + 1).toLocaleString()}, Col ${(c?.cell ?? 0) + 1}${sel}${many} · ` +
    `${n.toLocaleString()} lines · ${f.language ?? 'Plain text'}${f.wrap ? ' · wrapped' : ''}`;
}

// ------------------------------------------------------------------ keyboard

const MOTION = {
  ArrowLeft: 'left',
  ArrowRight: 'right',
  ArrowUp: 'up',
  ArrowDown: 'down',
  Home: 'lineStart',
  End: 'lineEnd',
  PageUp: 'pageUp',
  PageDown: 'pageDown',
};

function keyCommand(e) {
  const mod = MAC ? e.metaKey : e.ctrlKey;
  const k = e.key.length === 1 ? e.key.toLowerCase() : e.key;
  if (e.altKey && !mod && (k === 'ArrowUp' || k === 'ArrowDown')) {
    return { type: 'moveLines', up: k === 'ArrowUp' };
  }
  if (e.altKey && k === 'z') return 'wrap';
  if (MOTION[k]) {
    let to = MOTION[k];
    if (mod && (k === 'Home' || k === 'End')) to = k === 'Home' ? 'docStart' : 'docEnd';
    if (mod && !MAC && (k === 'ArrowLeft' || k === 'ArrowRight')) to = k === 'ArrowLeft' ? 'wordLeft' : 'wordRight';
    if (MAC && e.altKey && (k === 'ArrowLeft' || k === 'ArrowRight')) to = k === 'ArrowLeft' ? 'wordLeft' : 'wordRight';
    if (MAC && mod && (k === 'ArrowLeft' || k === 'ArrowRight')) to = k === 'ArrowLeft' ? 'lineStart' : 'lineEnd';
    if (MAC && mod && (k === 'ArrowUp' || k === 'ArrowDown')) to = k === 'ArrowUp' ? 'docStart' : 'docEnd';
    return { type: 'move', to, select: e.shiftKey };
  }
  if (mod) {
    switch (k) {
      case 'z':
        return { type: e.shiftKey ? 'redo' : 'undo' };
      case 'y':
        return { type: 'redo' };
      case 'a':
        return { type: 'selectAll' };
      case 'd':
        return e.shiftKey ? { type: 'duplicateLines' } : { type: 'addNextOccurrence' };
      case 'u':
        return { type: 'changeCase', to: 'toggle' };
      case 'j':
        return { type: 'joinLines' };
      case 'Backspace':
        return e.shiftKey ? { type: 'deleteLines' } : { type: 'deleteWordLeft' };
      default:
        return null;
    }
  }
  switch (k) {
    case 'Enter':
      return { type: 'newline' };
    case 'Backspace':
      return { type: e.altKey ? 'deleteWordLeft' : 'backspace' };
    case 'Delete':
      return { type: 'delete' };
    case 'Tab':
      return { type: e.shiftKey ? 'outdent' : 'indent' };
    case 'Insert':
      return { type: 'toggleOverwrite' };
    case 'Escape':
      return { type: 'singleCursor' };
    default:
      return null;
  }
}

input.addEventListener('keydown', (e) => {
  if (e.isComposing) return;
  const cmd = keyCommand(e);
  if (!cmd) return;
  e.preventDefault();
  if (cmd === 'wrap') {
    setWrap(!wrap);
    return;
  }
  send(cmd);
});

input.addEventListener('input', () => {
  if (composing) return;
  const text = input.value;
  input.value = '';
  if (text) send({ type: 'type', text });
});
let composing = false;
input.addEventListener('compositionstart', () => (composing = true));
input.addEventListener('compositionend', (e) => {
  composing = false;
  input.value = '';
  if (e.data) send({ type: 'type', text: e.data });
});
input.addEventListener('copy', (e) => clip(e, false));
input.addEventListener('cut', (e) => clip(e, true));
input.addEventListener('paste', (e) => {
  e.preventDefault();
  const text = e.clipboardData?.getData('text/plain');
  if (text) send({ type: 'insert', text });
});

function clip(e, cut) {
  e.preventDefault();
  send({ type: cut ? 'cut' : 'copy' });
  const text = ed.clipboard();
  if (text != null) e.clipboardData?.setData('text/plain', text);
}

// ------------------------------------------------------------------ mouse

function pointAt(ev) {
  const r = host.getBoundingClientRect();
  const rows = (ev.clientY - r.top) / lh;
  const line = frame?.lines.find((l) => rows >= l.row && rows < l.row + l.wraps.length + 1);
  return {
    line: (frame?.top ?? 0) + rows,
    cell: offset(line ?? frame?.lines.at(-1)) + (ev.clientX - r.left - textLeft()) / cw,
  };
}

host.addEventListener('mousedown', (e) => {
  if (e.button !== 0 || e.target === vbar || e.target === thumb) return;
  e.preventDefault();
  input.focus({ preventScroll: true });
  const p = pointAt(e);
  send({ type: 'click', ...p, count: Math.min(3, e.detail || 1), extend: e.shiftKey, add: e.altKey });
  const move = (ev) => {
    pending = { drag: { ...pointAt(ev), column: ev.altKey } };
    schedule();
  };
  const up = () => {
    document.removeEventListener('mousemove', move);
    document.removeEventListener('mouseup', up);
  };
  document.addEventListener('mousemove', move);
  document.addEventListener('mouseup', up);
});

host.addEventListener(
  'wheel',
  (e) => {
    e.preventDefault();
    const unit = e.deltaMode === 1 ? lh : e.deltaMode === 2 ? host.clientHeight : 1;
    let dx = e.deltaX * unit;
    let dy = e.deltaY * unit;
    if (e.shiftKey && !dx) [dx, dy] = [dy, 0];
    const s = pending?.scroll ?? { lines: 0, cells: 0 };
    s.lines += dy / lh;
    s.cells += dx / cw;
    pending = { ...pending, scroll: s };
    schedule();
  },
  { passive: false },
);

thumb.addEventListener('mousedown', (e) => {
  e.preventDefault();
  e.stopPropagation();
  const start = e.clientY;
  const top0 = frame.top;
  const per = (frame.info.lineCount - 1) / Math.max(1, host.clientHeight - thumb.offsetHeight);
  const move = (ev) => send({ type: 'scrollTo', top: top0 + (ev.clientY - start) * per });
  const up = () => {
    document.removeEventListener('mousemove', move);
    document.removeEventListener('mouseup', up);
    input.focus({ preventScroll: true });
  };
  document.addEventListener('mousemove', move);
  document.addEventListener('mouseup', up);
});

function schedule() {
  if (raf) return;
  raf = requestAnimationFrame(() => {
    raf = 0;
    const p = pending;
    pending = null;
    if (p?.scroll) send({ type: 'scroll', ...p.scroll });
    if (p?.drag) send({ type: 'drag', ...p.drag });
  });
}

// ------------------------------------------------------------------ toolbar

function setWrap(on) {
  wrap = on;
  $('#wrap').checked = on;
  send({ type: 'configure', wrap: on });
}

function load(text, lang, label) {
  const t0 = performance.now();
  ed?.free();
  ed = new Editor(text, lang);
  send({ type: 'configure', tabWidth: 4, detect: true, wrap });
  resize();
  const ms = Math.round(performance.now() - t0);
  $('#lang').value = lang ?? '';
  $('#loaded').textContent = `${label}: ${ms.toLocaleString()} ms`;
  input.focus({ preventScroll: true });
}

function bigFile(lines) {
  const words = 'lorem ipsum dolor sit amet tastier than chocolate editlicious rope frame'.split(' ');
  const out = new Array(lines);
  let seed = 7;
  for (let i = 0; i < lines; i++) {
    let s = `${i + 1}\t`;
    const n = 4 + (i % 13);
    for (let k = 0; k < n; k++) {
      seed = (seed * 1103515245 + 12345) & 0x7fffffff;
      s += `${words[seed % words.length]} `;
    }
    out[i] = s;
  }
  return out.join('\n');
}

$('#sample').addEventListener('click', () => load(SAMPLE, 'Rust', 'Example'));
$('#big').addEventListener('click', () => load(bigFile(1_000_000), null, '1,000,000 lines'));
$('#long').addEventListener('click', () =>
  load(`A single line of 5,000,000 characters follows – scroll sideways with Shift+wheel.\n${'0123456789abcdef'.repeat(312_500)}\nThe end.\n`, null, 'One 5,000,000-character line'),
);
$('#wrap').addEventListener('change', (e) => setWrap(e.target.checked));
$('#lang').addEventListener('change', (e) => send({ type: 'setLanguage', name: e.target.value || null }));

$('#find').addEventListener('keydown', (e) => {
  if (e.key === 'Enter' || e.key === 'Escape') {
    e.preventDefault();
    if (e.key === 'Escape') {
      e.target.value = '';
      send({ type: 'clearSearch' });
      input.focus({ preventScroll: true });
      return;
    }
    find(e.shiftKey, false);
  }
});
$('#find').addEventListener('input', () => find(false, true));

function find(backwards, incremental) {
  const pattern = $('#find').value;
  if (!pattern) {
    send({ type: 'clearSearch' });
    return;
  }
  send({
    type: 'find',
    query: { pattern, regex: false, caseSensitive: false, wholeWord: false },
    backwards,
    incremental,
  });
}

new ResizeObserver(() => frame && resize()).observe(host);

// ------------------------------------------------------------------ start

try {
  await init();
  for (const name of JSON.parse(languages())) {
    const o = document.createElement('option');
    o.value = o.textContent = name;
    $('#lang').append(o);
  }
  measureFont();
  load(SAMPLE, 'Rust', 'Example');
  document.body.classList.add('ready');
} catch (e) {
  statusEl.textContent = `The engine could not start in this browser: ${e}`;
}
