# Textalyzer

Analyze key metrics like number of words, readability, complexity, etc.
of any kind of text.


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
```

The duplication command analyzes files for duplicated text blocks. It can:
- Analyze multiple files or recursively scan directories
- Filter duplications based on minimum number of non-empty lines with `--min-lines=N` (default: 2)
- Detect single-line duplications when using `--min-lines=1`
- Rank duplications by number of consecutive lines
- Show all occurrences with file and line references
- Utilize multithreaded processing for optimal performance on all available CPU cores
- Use memory mapping for efficient processing of large files with minimal memory overhead
