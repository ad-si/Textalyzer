# Changelog

# 2025-03-11 - v0.3.0

- Add support for compiling to WASM and include a webapp build
    to analyze text in the browser.
- Subcommand `duplication`:
  - Implement finding multi-line duplications.
  - Check all nested files when a directory is specified.
  - Allow specifying multiple paths in duplicates check.
  - Improve formatting and color scheme of output.
  - Ignore files specified in common ignore files.
  - Add `--files-only` flag to only show files with duplicates
      and not the duplicated code itself.
  - Add `--min-lines` flag to filter out duplications with less
      than a specified number of lines.
