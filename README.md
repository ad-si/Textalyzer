# Textalyzer

Analyze key metrics like number of words, readability, complexity, etc.
of any kind of text.

CLI | Web
--- | ---
![CLI Screenshot][cli_ss] | ![Web Screenshot][web_ss]

[cli_ss]: ./images/2024-03-08t1219_cli_screenshot.png
[web_ss]: ./images/2024-03-08t1213_web_screenshot.png


## Usage

```sh
# Word frequency histogram
textalyzer histogram <filepath>

# Find duplicated code blocks (default: minimum 3 non-empty lines)
textalyzer duplication <path> [<additional paths...>]

# Find duplications with at least 5 non-empty lines
textalyzer duplication --min-lines=5 <path> [<additional paths...>]

# Include single-line duplications
textalyzer duplication --min-lines=1 <path> [<additional paths...>]

# Output duplications as JSON
textalyzer duplication --json <path> [<additional paths...>]
```

Example JSON output:

```json
[{
  "content": "<duplicated text block>",
  "locations": [
    { "path": "file1.txt", "line": 12 },
    { "path": "file2.txt", "line": 34 }
  ]
}, {
  "content": "<another duplicated block>",
  "locations": [
    { "path": "file1.txt", "line": 56 },
    { "path": "file3.txt", "line": 78 }
  ]
}]
```

The duplication command analyzes files for duplicated text blocks. It can:
- Analyze multiple files or recursively scan directories
- Filter duplications based on minimum number of non-empty lines with `--min-lines=N` (default: 2)
- Detect single-line duplications when using `--min-lines=1`
- Rank duplications by number of consecutive lines
- Show all occurrences with file and line references
- Utilize multithreaded processing for optimal performance on all available CPU cores
- Use memory mapping for efficient processing of large files with minimal memory overhead
- Output duplication data as JSON with `--json`


## Related

- [jscpd] - Copy/paste detector for programming source code.
- [megalinter] - Code quality and linter tool.
- [pmd] - Source code analysis tool.
- [qlty] - Code quality and security analysis tool.
- [superdiff] - Find duplicate code blocks in files.
- [wf] - Command line utility for counting word frequency.

[jscpd]: https://github.com/kucherenko/jscpd
[megalinter]: https://megalinter.io
[pmd]: https://github.com/pmd/pmd
[qlty]: https://github.com/qltysh/qlty
[superdiff]: https://github.com/chuck-sys/superdiff
[wf]: https://github.com/jarcane/wf


## Rewrite in Rust

This CLI tool was originally written in JavaScript and was later
rewritten in Rust to improve the performance.

Before:

```txt
hyperfine --warmup 3 'time ./cli/index.js examples/1984.txt'
Benchmark #1: time ./cli/index.js examples/1984.txt
  Time (mean ± σ):     390.3 ms ±  15.6 ms    [User: 402.6 ms, System: 63.5 ms]
  Range (min … max):   366.7 ms … 425.7 ms
```

After:

```txt
hyperfine --warmup 3 'textalyzer histogram examples/1984.txt'
Benchmark #1: textalyzer histogram examples/1984.txt
  Time (mean ± σ):      40.4 ms ±   2.5 ms    [User: 36.0 ms, System: 2.7 ms]
  Range (min … max):    36.9 ms …  48.7 ms
```

Pretty impressive 10x performance improvement! 😁
