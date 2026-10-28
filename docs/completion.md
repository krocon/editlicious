# Word completion

**Ctrl+Space** (on the Mac also **⌥Esc**) completes the word left of the cursor (prompt 023):

1. An arithmetic expression at the cursor comes first (`255/8=` → `31.875`, prompt 020).
2. Otherwise the list offers words of the document – the nearest lines first – and then typical words of the
   language: keywords, built-in types and common library names.
3. A single candidate goes in right away; with a list, keep typing to narrow it, **Enter**/**Tab** takes the
   entry, **Esc** closes it. Taking a word is one undo step.

Settings → *Completion*: “Suggest words while typing” opens the list by itself from two letters on (off by
default), “Include typical words of the language” switches the built-in and your own lists on or off.

## Built-in lists

| File type | List |
|---|---|
| Java | Java + Spring Boot (annotations after `@`, `ResponseEntity`, `JpaRepository` …) |
| Kotlin (`.kt`, `.kts`) | Kotlin + Spring Boot |
| TypeScript (`.ts`, `.tsx`, `.mts`, `.cts`) | JavaScript/TypeScript + Angular (decorators, signals, lifecycle, router, forms, RxJS) |
| JavaScript | JavaScript/TypeScript |
| Python, SQL, Rust, C, C++, C#, Go, PHP, Ruby, Shell, CSS, HTML | one list each |

SQL ignores case: `sel` becomes `select`, `SEL` becomes `SELECT` – unless the file already writes the word, then
its spelling wins. In CSS, `-` belongs to a word (`background-co` → `background-color`).

The list follows the language of the highlighting; pick another language in the status bar and the words change
with it. Large files cost the same as small ones: only the lines around the cursor are read.

## Your own lists

Put text files into the folder `words` of the configuration folder – one word per line:

| System | Folder |
|---|---|
| macOS | `~/Library/Application Support/io.github.krocon.editlicious/words/` |
| Windows | `%APPDATA%\io.github.krocon.editlicious\words\` |
| Linux | `~/.config/io.github.krocon.editlicious/words/` |

The file name says where the words count: `java.txt`, `kotlin.txt`, `typescript.txt`, `javascript.txt`,
`python.txt`, `sql.txt`, `rust.txt`, `c.txt`, `cpp.txt`, `csharp.txt`, `go.txt`, `php.txt`, `ruby.txt`,
`shell.txt`, `css.txt`, `html.txt` – any other language by its name in lower case without spaces
(`markdown.txt`, `groovy.txt`, `yaml.txt`), plain text as `text.txt`, and `all.txt` for every file.

```text
# lines starting with # are comments
# case-insensitive      ← the spelling follows what you type (like SQL)
# word-chars: -         ← characters that belong to words besides letters, digits and _
myProjectService
CustomerRepository
```

Your words come after the built-in ones; at most 5 000 words per file, 64 files of up to 256 KB. The lists are read
again whenever a file opens.
