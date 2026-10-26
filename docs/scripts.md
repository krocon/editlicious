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
| `goto 12:5`, `select 12:5 12:9` | place the caret, select |
| `move wordRight [select] [n]` | `left right up down wordLeft wordRight lineStart lineEnd pageUp pageDown docStart docEnd` |
| `newline`, `backspace 3`, `delete`, `undo`, `redo`, `indent`, `outdent`, `selectAll`, `copy`, `cut`, `paste`, `duplicateLines`, `deleteLines`, `joinLines`, `toggleMarker`, `clearMarkers`, `addNextOccurrence`, `singleCursor` | editor commands, optionally repeated |
| `find "text" [regex] [case] [word] [back]` | find and select the next match |
| `do {json}` | any engine command, e.g. `do {"type":"sortLines","options":{"unique":true}}` |

"Save the last recording as a script" turns a recording into such a script – a good start for editing by hand.
