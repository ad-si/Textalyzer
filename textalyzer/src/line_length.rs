use crate::types::{FileEntry, MappedContent};
use pad::{Alignment, PadStr};
use std::collections::HashMap;
use std::error::Error;
use std::io::Write;
use unicode_width::UnicodeWidthStr;

const MAX_LINE_LENGTH_HISTOGRAM_BAR: usize = 60;

/// Calculates the frequency of each line length across all provided files.
fn calculate_line_length_histogram(
  files: &[FileEntry],
) -> HashMap<usize, usize> {
  let mut histogram: HashMap<usize, usize> = HashMap::new();

  for file in files {
    let lines: Vec<&str> = match &file.content {
      MappedContent::Mapped(mmap) => {
        if let Ok(content) = std::str::from_utf8(mmap) {
          content.lines().collect()
        } else {
          Vec::new() // Skip invalid UTF-8
        }
      }
      MappedContent::String(content) => content.lines().collect(),
    };

    for line in lines {
      let length = UnicodeWidthStr::width(line);
      *histogram.entry(length).or_insert(0) += 1;
    }
  }

  histogram
}

/// Formats the line length histogram into a string suitable for printing.
fn format_line_length_histogram(histogram: HashMap<usize, usize>) -> String {
  if histogram.is_empty() {
    return "No lines found to analyze.".to_string();
  }

  let mut sorted_lengths: Vec<_> = histogram.keys().collect();
  sorted_lengths.sort();

  let max_length = **sorted_lengths.last().unwrap_or(&&0);
  let max_count = *histogram.values().max().unwrap_or(&0);

  let max_length_width = max_length.to_string().len();
  let max_count_width = max_count.to_string().len();

  let mut result = String::new();
  result.push_str(&format!(
    "{:>width$}  {:>count_width$}  Histogram\n",
    "Length",
    "Count",
    width = max_length_width,
    count_width = max_count_width
  ));
  result.push_str(&format!(
    "{}  {}  {}\n",
    "-".repeat(max_length_width),
    "-".repeat(max_count_width),
    "-".repeat(9) // Length of "Histogram"
  ));

  for length in sorted_lengths {
    let count = histogram[length];
    let bar_width = if max_count > 0 {
      (MAX_LINE_LENGTH_HISTOGRAM_BAR as f64 * (count as f64 / max_count as f64))
        .round() as usize
    } else {
      0
    };

    result += &format!(
      "{}  {}  {}\n",
      length
        .to_string()
        .pad_to_width_with_alignment(max_length_width, Alignment::Right),
      count
        .to_string()
        .pad_to_width_with_alignment(max_count_width, Alignment::Right),
      "▆".repeat(bar_width),
    );
  }

  result
}

/// Processes files to calculate and print the line length histogram
pub fn process_and_output_line_length<A: Write>(
  files: Vec<FileEntry>,
  mut output_stream: A,
) -> Result<(), Box<dyn Error>> {
  let histogram = calculate_line_length_histogram(&files);
  let formatted_histogram = format_line_length_histogram(histogram);
  writeln!(output_stream, "{}", formatted_histogram)?;
  Ok(())
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::types::MappedContent;

  #[test]
  fn test_calculate_line_length_histogram_empty() {
    let files = vec![];
    let histogram = calculate_line_length_histogram(&files);
    assert!(histogram.is_empty());
  }

  #[test]
  fn test_calculate_line_length_histogram_basic() {
    let file1 = FileEntry {
      name: "file1.txt".to_string(),
      content: MappedContent::String(
        "line1\nline22\n".to_string(), // lengths 5, 6
      ),
    };
    let file2 = FileEntry {
      name: "file2.txt".to_string(),
      content: MappedContent::String(
        "line1\nline333\n".to_string(), // lengths 5, 7
      ),
    };
    let files = vec![file1, file2];
    let histogram = calculate_line_length_histogram(&files);

    let expected: HashMap<usize, usize> =
      [(5, 2), (6, 1), (7, 1)].iter().cloned().collect();
    assert_eq!(histogram, expected);
  }

  #[test]
  fn test_calculate_line_length_histogram_unicode() {
    let file1 = FileEntry {
      name: "file_unicode.txt".to_string(),
      // "你好" is 2 chars, width 4. "🚀" is 1 char, width 2.
      content: MappedContent::String("你好\n🚀\n".to_string()), // widths 4, 2
    };
    let files = vec![file1];
    let histogram = calculate_line_length_histogram(&files);

    let expected: HashMap<usize, usize> =
      [(4, 1), (2, 1)].iter().cloned().collect();
    assert_eq!(histogram, expected);
  }

  #[test]
  fn test_format_line_length_histogram_empty() {
    let histogram = HashMap::new();
    let formatted = format_line_length_histogram(histogram);
    assert_eq!(formatted, "No lines found to analyze.");
  }

  #[test]
  fn test_format_line_length_histogram_basic() {
    let histogram: HashMap<usize, usize> =
      [(5, 2), (10, 1), (15, 3)].iter().cloned().collect();
    let formatted = format_line_length_histogram(histogram);

    // Basic check for structure, not exact bar length
    assert!(formatted.contains("Length  Count  Histogram"));
    // Max length is 15 (width 2), max count is 3 (width 1)
    assert!(formatted.contains("--  -  ---------"));
    assert!(formatted.contains(" 5  2"));
    assert!(formatted.contains("10  1"));
    assert!(formatted.contains("15  3"));
    assert!(formatted.contains("▆")); // Check that a bar is present
  }
}
