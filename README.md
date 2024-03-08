# Textalyzer

Analyze key metrics like number of words, readability, complexity, etc.
of any kind of text.

CLI | Web
--- | ---
![CLI Screenshot][cli_ss] | ![Web Screenshot][web_ss]

[cli_ss]: ./images/2024-03-08t1219_cli_screenshot.png
[web_ss]: ./images/2024-03-08t1213_web_screenshot.png


## Usage

```
textalyzer histogram <filepath>
```


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


## Related

- [wf] - Command line utility for counting word frequency

[wf]: https://github.com/jarcane/wf
