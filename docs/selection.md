# Growing selections by double click

Double-click a word to select it. **Double-click again inside the selection** and it grows by one step; keep going
and it grows to the line and then the paragraph – one more double click starts with the word again
(prompt 018a). A click elsewhere, a cursor move, typing or any edit starts a new sequence. A triple click still
selects the line.

## The rule

Separators have weights:

| Boundary | Weight |
|---|---:|
| no separator (letters, digits, umlauts, combining marks, emoji) | 0 |
| `_` | 1 |
| `-` | 2 |
| `.` | 3 |
| `/` `\` `:` `@` `?` `#` `&` `=` (paths, URLs, keys) | 3.5 |
| other punctuation `, ; + * …` | 3.8 |
| space | 4 |
| tab | 5 |

Each step crosses the **lightest** boundary next to the selection and takes the piece behind it – everything up to
the next boundary that is at least as heavy. Equal weights left and right: the smaller piece first, then the left
one. Bracket and quote pairs around the selection are steps of their own (first the content, then with the
delimiters); a bracket group beside the selection is taken as a whole. Of all candidates the smallest one that is
larger than the selection comes next, so a selection never shrinks or jumps.

| Double click on | Steps |
|---|---|
| `user_pro`**`file`**`_name` | `profile` → `user_profile` → `user_profile_name` |
| `com.example.ser`**`vice`** | `service` → `example.service` → `com.example.service` |
| `src/components/`**`user`**`-profile.ts` | `user` → `user-profile` → `user-profile.ts` → `components/user-profile.ts` → `src/components/user-profile.ts` |
| `call(`**`user`**`.name)` | `user` → `user.name` → `(user.name)` → `call(user.name)` |
| `[`**`link`**` text](http://x.y)` | `link` → `link text` → `[link text]` → `[link text](http://x.y)` |

…then the whole line, the paragraph (lines between blank lines), and the word again.

## Other weights

In `settings.json` (next to the app's other settings) – a character or `space`, `tab`, `punctuation`, values 0.1–9:

```json
{ "expandWeights": { "_": 3.5, ".": 1 } }
```
