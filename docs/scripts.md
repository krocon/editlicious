# Recordings and typing scripts

editlicious can record what you do in a document and play it back – at the original speed, faster, slower or like
a person typing (prompt 015). Use it for macros, reproducible demos and screen videos that look the same in every
language.

| Action | Shortcut | |
|---|---|---|
| Start/stop recording | <kbd>Ctrl/⌘ ⇧ R</kbd> | status bar shows **● REC** |
| Play the last recording again | <kbd>Ctrl/⌘ Alt R</kbd> | at once, as one undo step (a macro) |
| Play a recording or script… | palette / menu *Edit → Recording* | choose file and tempo |
| Pause/resume, stop | palette / menu, <kbd>Esc</kbd> stops | status bar shows **▶ 37 %** |
| Save the last recording (as a script)… | palette / menu | `*.edrec.jsonl` / `*.edscript` |

A playback is **one undo step**: <kbd>Ctrl/⌘ Z</kbd> takes all of it back. While it runs, the document only scrolls –
typing waits. Playback never touches the system clipboard.

## Recordings (`*.edrec.jsonl`)

The first line describes the document the recording started on (name, lines, hash) and the selection; every other
line is one engine command with its time in milliseconds. Clicks and page moves are stored as the selection they
produced, so a recording plays the same in any window size. Played on another text, editlicious asks first.

## Scripts (`*.edscript`)

One command per line, `#` starts a comment. Lines and columns start at 1.

```text
# a 60-second video, German and English share the choreography
tempo human cpm=320 seed=42 typos=0.02
type "// Pattern: Strategy\n"
pause 600
type @snippets/strategy.de.ts
pause 1500
goto 3:5
select 3:5 3:13
type "pricing"
move lineEnd
newline
type <<END
  return price * 0.9;
END
find "price" word
do {"type":"toggleMarker"}
```

| Command | Meaning |
|---|---|
| `tempo original` / `instant` / `factor 2` / `even 60` / `human cpm=330 seed=1 typos=0` | how fast what follows is typed (`seed` = same rhythm every time, `typos` = chance of a corrected typo per letter) |
| `type "text"` | type it character by character (`\n`, `\t`, `\"`, `\\`, `\u{1F642}`); indentation after a line break comes in one go |
| `type <<END` … `END` | type a block of lines |
| `type @file` | type a file (relative to the script) |
| `pause 800` | wait (ms) |
| `keys "Ctrl+D"` | show keys or a note in an exported video for a moment (nothing happens in the editor) |
| `goto 12:5`, `select 12:5 12:9` | place the caret, select |
| `move wordRight [select] [n]` | `left right up down wordLeft wordRight lineStart lineEnd pageUp pageDown docStart docEnd` |
| `newline`, `backspace 3`, `delete`, `undo`, `redo`, `indent`, `outdent`, `selectAll`, `copy`, `cut`, `paste`, `duplicateLines`, `deleteLines`, `joinLines`, `toggleMarker`, `clearMarkers`, `addNextOccurrence`, `singleCursor` | editor commands, optionally repeated |
| `find "text" [regex] [case] [word] [back]` | find and select the next match |
| `do {json}` | any engine command, e.g. `do {"type":"sortLines","options":{"unique":true}}` |

## Videos

*Export as video…* (menu *Edit → Recording*) or the command line renders a recording or script **without a screen
recording**: every frame is drawn from the engine on a virtual clock – sharp at any size, the same timing every
time, in any colour scheme of the app. Videos need [ffmpeg](https://ffmpeg.org) (path in the settings if it is not on
the `PATH`); without it, a folder as output gets numbered PNG frames.

```bash
editlicious render demo.edscript -o demo.mp4 --size 1080x1920 --theme night --chrome mac
editlicious render demo.edscript -o frames/ --hold 0.5,1 --tempo human --seed 7
editlicious render demo.edscript -o demo.de.mp4 --fallback-font /usr/share/fonts/NotoSansCJK.ttc
```

`editlicious render --help` lists every option (size, fps, theme, font and fallback fonts, window look, padding,
line numbers, caret blinking, still picture at start/end, start text, language, tempo, `--best`, `--ffmpeg`). The
built-in font is JetBrains Mono; characters it lacks (CJK, emoji) need a `--fallback-font`, otherwise they show as
boxes – never a random system font. `.mp4` = H.264 (`-preset veryfast`, `--best` = slower and better), `.mov` =
ProRes, `.webm` = VP9.

"Save the last recording as a script" turns a recording into such a script – a good start for editing by hand.

## Scripts in a web page (videos made from HTML)

The WebAssembly build of the engine (`site/demo/pkg`) can play scripts on a virtual clock inside a page – for
videos rendered from HTML frame by frame (e.g. HyperFrames), where the editor sits next to other animated content:

```js
import init, { Player, scriptDuration } from './pkg/ed_wasm.js';
await init();
const p = new Player('.card-title {\n}\n', 'CSS');
p.setup(JSON.stringify({ type: 'resize', lines: 14, cols: 40 })); // holds after every jump back
const typing = 'tempo human cpm=300 seed=3\ngoto 2:1\ntype "  white-space: nowrap;\\n"';
p.add(typing, 12_000 - scriptDuration(typing));                   // ends exactly at 12 s
const frame = JSON.parse(p.seek(11_500));                         // the same picture for every seek
```

`seek(ms)` replays the steps up to that moment (backwards from the start text), so frames can be asked for in any
order. `add` never lets a script start before the one before ends. `events()` lists the steps with their time and kind
(`char`, `space`, `enter`, `delete`, `move`, `keys`) – for key sounds; `finalText()` is the text after all scripts.
`type @file` is not available in the page.
