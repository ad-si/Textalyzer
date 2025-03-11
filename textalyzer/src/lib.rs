pub mod types;

extern crate colored;
extern crate ignore;
extern crate pad;
extern crate terminal_size;
extern crate unicode_width;

use colored::Colorize;
use ignore::WalkBuilder;
use pad::{Alignment, PadStr};
use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use terminal_size::{terminal_size, Width};
use unicode_width::UnicodeWidthStr;

use types::{Command, Config, FileEntry, LineEntry};

const MAX_LINE_LENGTH: u16 = 80;

/// Generate a frequency map from a given text.
///
/// # Examples
///
/// ```rust
/// use textalyzer::generate_frequency_map;
///
/// let freq_map = generate_frequency_map(
///    "This test is a test to test the frequency map."
/// );
///
/// let expected_map: std::collections::HashMap<_, _> = vec![
///   ("this", 1),
///   ("test", 3),
///   ("is", 1),
///   ("a", 1),
///   ("to", 1),
///   ("the", 1),
///   ("frequency", 1),
///   ("map", 1),
/// ]
/// .into_iter()
/// .map(|(s, n)| (String::from(s), n))
/// .collect();
///
/// assert_eq!(freq_map, expected_map);
/// ```
pub fn generate_frequency_map(text: &str) -> HashMap<String, i32> {
  let words = text
    .split(|character| !char::is_alphabetic(character))
    .filter(|word| word != &"");
  let mut frequency_map = HashMap::new();

  for word in words {
    let count = frequency_map.entry(word.to_lowercase()).or_insert(0);
    *count += 1;
  }
  frequency_map
}

/// Format a frequency map into a string.
pub fn format_freq_map(freq_map: HashMap<String, i32>) -> String {
  let mut freq_vec: Vec<_> = freq_map.iter().collect();
  freq_vec.sort_by(|t1, t2| t2.1.cmp(t1.1));
  let mut longest_word = "";
  let mut highest_number = &0;

  for (word, count) in &freq_vec {
    let word_length = UnicodeWidthStr::width(&word[..]);

    if word_length > UnicodeWidthStr::width(longest_word) {
      longest_word = word;
    }
    if count > &highest_number {
      highest_number = count;
    }
  }

  let max_number_length = highest_number.to_string().len();
  let max_word_length = UnicodeWidthStr::width(longest_word);

  let max_line_length = max_word_length + 2 + max_number_length + 2;
  let remaining_space = MAX_LINE_LENGTH as usize - max_line_length;

  let mut result = String::new();

  for (word, count) in &freq_vec {
    let bar_width =
      (remaining_space as f32 / *highest_number as f32) * **count as f32;

    result += &format!(
      "{}  {}  {}\n",
      word.pad_to_width_with_alignment(
        max_word_length, // this comment fixes rustfmt
        Alignment::Right
      ),
      count
        .to_string()
        .pad_to_width_with_alignment(max_number_length, Alignment::Right),
      "▆".repeat(bar_width.round() as usize),
    );
  }

  result
}

/// Merge lines from multiple files that pass the given filter
/// into a single list.
fn merge_file_lines(
  filter: &dyn Fn(&&str) -> bool,
  files: Vec<FileEntry>,
) -> Vec<LineEntry> {
  files
    .iter()
    .flat_map(|file| {
      file
        .content
        .lines()
        .enumerate()
        .filter(|(_num, line)| !line.trim().is_empty() && filter(line))
        .map(move |(num, line)| LineEntry {
          file_name: file.name.clone(),
          line_number: (num as u32 + 1),
          content: line.to_string(),
        })
    })
    .collect()
}

#[test]
fn test_merge_file_lines() {
  let file1 = FileEntry {
    name: "file1.txt".to_string(),
    content: "Line one\nLine Two\n".to_string(),
  };
  let file2 = FileEntry {
    name: "file2.txt".to_string(),
    content: "Another line\n".to_string(),
  };
  let lines = merge_file_lines(
    &|line: &&str| line.trim().len() > 5,
    vec![file1, file2],
    //
  );
  let expected_lines = vec![
    LineEntry {
      file_name: "file1.txt".to_string(),
      line_number: 1,
      content: "Line one".to_string(),
    },
    LineEntry {
      file_name: "file1.txt".to_string(),
      line_number: 2,
      content: "Line Two".to_string(),
    },
    LineEntry {
      file_name: "file2.txt".to_string(),
      line_number: 1,
      content: "Another line".to_string(),
    },
  ];
  assert_eq!(lines, expected_lines);
}

/// Find duplications in a given text.
pub fn find_duplicate_lines(
  files: Vec<FileEntry>,
) -> Vec<(String, Vec<(String, u32)>)> {
  let lines = merge_file_lines(
    &|line: &&str| line.trim().len() > 5,
    files, //
  );
  let mut line_map = HashMap::new();
  let mut duplications = Vec::new();

  for line_entry in lines.iter() {
    let line_count = line_map //
      .entry(&line_entry.content)
      .or_insert_with(Vec::new);
    line_count.push((line_entry.file_name.clone(), line_entry.line_number));
  }

  for (line, line_locations) in line_map {
    if line_locations.len() > 1 {
      duplications.push((line.clone(), line_locations));
    }
  }

  duplications.sort_by(|a, b| {
    b.0.trim().len().cmp(
      &a.0.trim().len(), //
    )
  });

  duplications
}

/// Find multi-line duplications across files.
///
/// This function detects sequences of consecutive lines that are duplicated
/// across files or within the same file, prioritizing longer sequences.
/// For single-line duplications, it only includes lines with more than 3 non-whitespace characters.
/// Multi-line duplications are always included.
/// When duplications overlap, only the longest one is kept.
pub fn find_multi_line_duplications(
  files: Vec<FileEntry>,
) -> Vec<(String, Vec<(String, u32)>)> {
  let lines = merge_file_lines(
    &|line: &&str| !line.trim().is_empty(), // Include all non-empty lines
    files.clone(),                          //
  );

  // Create a map of file_entries by name for quick lookup
  let file_map: HashMap<String, &FileEntry> =
    files.iter().map(|f| (f.name.clone(), f)).collect();

  // Group lines by content to find duplicated individual lines first
  let mut line_map: HashMap<&String, Vec<(String, u32)>> = HashMap::new();
  for line_entry in lines.iter() {
    let line_count = line_map.entry(&line_entry.content).or_default();
    line_count.push((line_entry.file_name.clone(), line_entry.line_number));
  }

  // Filter to only keep lines that appear more than once
  let dup_lines: HashMap<&String, Vec<(String, u32)>> = line_map
    .into_iter()
    .filter(|(_, locations)| locations.len() > 1)
    .collect();

  // Store multi-line duplications
  let mut multi_line_dups: Vec<(String, Vec<(String, u32)>)> = Vec::new();

  // For each file in the input
  for file_entry in &files {
    let file_lines: Vec<&str> = file_entry.content.lines().collect();

    // For each starting position in this file
    for start_idx in 0..file_lines.len() {
      // Skip line if it's not in our duplicated lines map
      let first_line = file_lines[start_idx];
      let first_line_string = first_line.to_string();
      if first_line.trim().is_empty()
        || !dup_lines.contains_key(&first_line_string)
      {
        continue;
      }

      // Find all places where this line appears (in other files or later in this file)
      let line_locations = dup_lines.get(&first_line_string).unwrap();

      // For each location of this first line
      for (other_file, other_line_num) in line_locations {
        // Skip if it's the same position we're checking from
        if other_file == &file_entry.name
          && *other_line_num == (start_idx as u32 + 1)
        {
          continue;
        }

        // Try to extend the match as far as possible
        let other_file_entry = file_map.get(other_file).unwrap();
        let other_file_lines: Vec<&str> =
          other_file_entry.content.lines().collect();
        let other_start_idx = (*other_line_num - 1) as usize;

        // Determine how many consecutive lines match
        let mut consecutive_match_count = 0;
        let max_possible_matches = std::cmp::min(
          file_lines.len() - start_idx,
          other_file_lines.len() - other_start_idx,
        );

        for offset in 0..max_possible_matches {
          if file_lines[start_idx + offset]
            == other_file_lines[other_start_idx + offset]
          {
            consecutive_match_count += 1;
          } else {
            break;
          }
        }

        // Only consider matches of at least 1 line
        if consecutive_match_count >= 1 {
          // Combine the matching lines into a single text block
          let block = file_lines
            [start_idx..(start_idx + consecutive_match_count)]
            .join("\n");

          // Create an entry for this multi-line match
          let mut found = false;
          for (existing_block, locations) in &mut multi_line_dups {
            if *existing_block == block {
              // This exact block already exists, just add the locations
              if !locations
                .contains(&(file_entry.name.clone(), start_idx as u32 + 1))
              {
                locations.push((file_entry.name.clone(), start_idx as u32 + 1));
              }
              if !locations.contains(&(other_file.clone(), *other_line_num)) {
                locations.push((other_file.clone(), *other_line_num));
              }
              found = true;
              break;
            }
          }

          if !found {
            // New duplication block
            multi_line_dups.push((
              block,
              vec![
                (file_entry.name.clone(), start_idx as u32 + 1),
                (other_file.clone(), *other_line_num),
              ],
            ));
          }
        }
      }
    }
  }

  // Filter duplications based on our criteria:
  // 1. Keep all multi-line blocks (2+ lines)
  // 2. For single lines, only keep those with more than 3 non-whitespace chars
  let filtered_dups: Vec<(String, Vec<(String, u32)>)> = multi_line_dups
    .into_iter()
    .filter(|(content, _)| {
      if content.contains('\n') {
        // Keep all multi-line blocks
        true
      } else {
        // For single-line duplications, count non-whitespace characters
        let non_whitespace_count =
          content.chars().filter(|c| !c.is_whitespace()).count();
        // We want more than 3 non-whitespace characters for single-line duplications
        non_whitespace_count > 3
      }
    })
    .collect();

  // Sort duplications by: 1) number of lines, 2) total character length
  let mut sorted_dups = filtered_dups;
  sorted_dups.sort_by(|a, b| {
    // First compare by line count (number of newlines + 1)
    let a_lines = a.0.matches('\n').count() + 1;
    let b_lines = b.0.matches('\n').count() + 1;

    let line_cmp = b_lines.cmp(&a_lines);

    if line_cmp == std::cmp::Ordering::Equal {
      // If line count is the same, sort by content length
      b.0.len().cmp(&a.0.len())
    } else {
      line_cmp
    }
  });

  // Handle overlapping duplications - only keep the longest ones
  let mut non_overlapping_dups = Vec::new();
  
  // Track used (line, file) pairs to detect overlaps
  let mut used_positions: HashMap<(String, u32), usize> = HashMap::new();
  
  // Process sorted duplications (from longest to shortest)
  for (content, locations) in sorted_dups {
    let lines_count = content.matches('\n').count() + 1;
    let mut new_locations = Vec::new();
    
    // Check each location for overlap
    for (file, line_num) in &locations {
      // Calculate range of lines covered by this duplication
      let end_line = line_num + lines_count as u32 - 1;
      let mut position_is_free = true;
      
      // Check if any line in this range is already used
      for l in *line_num..=end_line {
        if let Some(dup_idx) = used_positions.get(&(file.clone(), l)) {
          // Only consider it an overlap if it belongs to a duplication we've already kept
          if *dup_idx < non_overlapping_dups.len() {
            position_is_free = false;
            break;
          }
        }
      }
      
      // If no overlap, add this location
      if position_is_free {
        new_locations.push((file.clone(), *line_num));
        // Mark all lines used by this duplication
        for l in *line_num..=end_line {
          used_positions.insert((file.clone(), l), non_overlapping_dups.len());
        }
      }
    }
    
    // Only add this duplication if it has at least 2 non-overlapping locations
    if new_locations.len() >= 2 {
      non_overlapping_dups.push((content, new_locations));
    }
  }

  non_overlapping_dups
}

#[test]
fn test_find_duplicate_lines() {
  let file1 = FileEntry {
    name: "file1.txt".to_string(),
    content: "\
            This is a test.\n\
            This is only a test.\n\
            This is a test.\n\
            # Ignore empty lines\n\
            \n\
            \n\
            # Ignore short lines\n\
            abc\n\
            abc\n"
      .to_string(),
  };
  let file2 = FileEntry {
    name: "file2.txt".to_string(),
    content: "This is a test.\n".to_string(),
  };
  let duplications = find_duplicate_lines(vec![file1, file2]);
  let expected_duplications = vec![(
    "This is a test.".to_string(),
    vec![
      ("file1.txt".to_string(), 1),
      ("file1.txt".to_string(), 3),
      ("file2.txt".to_string(), 1),
    ],
  )];

  assert_eq!(duplications, expected_duplications);
}

#[test]
fn test_find_multi_line_duplications() {
  let file1 = FileEntry {
    name: "file1.txt".to_string(),
    content: "\
            This is a test.\n\
            This is a second line.\n\
            This is a third line.\n\
            Some other content.\n\
            And another line here.\n\
            This is a test.\n\
            This is a second line.\n\
            A different third line.\n"
      .to_string(),
  };
  let file2 = FileEntry {
    name: "file2.txt".to_string(),
    content: "\
            Something unrelated.\n\
            This is a test.\n\
            This is a second line.\n\
            This is a third line.\n\
            Final line.\n"
      .to_string(),
  };

  let files = vec![file1, file2];
  let duplications = find_multi_line_duplications(files);

  // With overlap handling, we should only have the 3-line duplication
  // because it's longer than the 2-line duplication and they overlap
  assert_eq!(duplications.len(), 1, "Expected exactly 1 duplication");

  // Look for the 3-line duplication
  let three_line_dup =
    "This is a test.\nThis is a second line.\nThis is a third line.";
  
  // The only duplication should be the 3-line one
  let (block, locations) = &duplications[0];
  assert_eq!(block, three_line_dup, "Expected 3-line duplication");
  assert_eq!(locations.len(), 2, "Expected 2 locations for 3-line duplication");
  assert!(locations.contains(&("file1.txt".to_string(), 1)));
  assert!(locations.contains(&("file2.txt".to_string(), 2)));
  
  // The 2-line duplication should not be present because it's covered
  // by the 3-line duplication at the same starting positions
}

#[test]
fn test_multi_line_duplications_with_non_overlapping() {
  let file1 = FileEntry {
    name: "file1.txt".to_string(),
    content: "\
            Block A line 1.\n\
            Block A line 2.\n\
            Block A line 3.\n\
            Some middle content.\n\
            Block B line 1.\n\
            Block B line 2.\n"
      .to_string(),
  };
  let file2 = FileEntry {
    name: "file2.txt".to_string(),
    content: "\
            Different stuff.\n\
            Block A line 1.\n\
            Block A line 2.\n\
            Block A line 3.\n\
            Some other content.\n\
            Block B line 1.\n\
            Block B line 2.\n"
      .to_string(),
  };

  let files = vec![file1, file2];
  let duplications = find_multi_line_duplications(files);

  // We should have both duplications since they don't overlap
  assert_eq!(duplications.len(), 2, "Expected exactly 2 duplications");

  let block_a = "Block A line 1.\nBlock A line 2.\nBlock A line 3.";
  let block_b = "Block B line 1.\nBlock B line 2.";
  
  let mut found_block_a = false;
  let mut found_block_b = false;
  
  for (block, locations) in &duplications {
    if block == block_a {
      found_block_a = true;
      assert_eq!(locations.len(), 2);
      assert!(locations.contains(&("file1.txt".to_string(), 1)));
      assert!(locations.contains(&("file2.txt".to_string(), 2)));
    } else if block == block_b {
      found_block_b = true;
      assert_eq!(locations.len(), 2);
      assert!(locations.contains(&("file1.txt".to_string(), 5)));
      assert!(locations.contains(&("file2.txt".to_string(), 6)));
    }
  }
  
  assert!(found_block_a, "Did not find Block A duplication");
  assert!(found_block_b, "Did not find Block B duplication");
}

/// Run Textalyzer with the given configuration.
/// Recursively find all files in a directory using the ignore crate
/// This respects .gitignore, .ignore, and other standard ignore files
fn find_all_files(dir: &Path) -> Result<Vec<PathBuf>, Box<dyn Error>> {
  let mut files = Vec::new();

  // Use WalkBuilder from the ignore crate to handle gitignore patterns properly
  let mut builder = WalkBuilder::new(dir);

  // Configure the walker to respect standard ignore files
  builder
    .hidden(false) // Don't skip hidden files (let .gitignore decide)
    .git_global(true) // Use global gitignore
    .git_ignore(true) // Use git ignore
    .ignore(true) // Use .ignore files
    .git_exclude(true) // Use git exclude
    .filter_entry(|e| {
      // Add explicit filter for .git directories
      let path = e.path();
      // Skip .git directories
      !(path.file_name() == Some(".git".as_ref())
        || path.to_string_lossy().contains("/.git/"))
    });

  // Walk the directory and collect all files
  for result in builder.build() {
    match result {
      Ok(entry) => {
        let path = entry.path().to_path_buf();
        if path.is_file() {
          files.push(path);
        }
      }
      Err(err) => {
        // Log error but continue with other files
        eprintln!("Error accessing path: {}", err);
      }
    }
  }

  Ok(files)
}

/// Load multiple files as FileEntry structs
fn load_files(paths: Vec<PathBuf>) -> Result<Vec<FileEntry>, Box<dyn Error>> {
  let mut file_entries = Vec::new();

  for path in paths {
    // Skip binary files and files we can't read
    if let Ok(content) = fs::read_to_string(&path) {
      if !content.contains('\0') {
        // Simple check for binary files
        file_entries.push(FileEntry {
          name: path.to_string_lossy().into_owned(),
          content,
        });
      }
    }
  }

  Ok(file_entries)
}

/// Attempt to detect if terminal is using a light theme
fn is_light_theme() -> bool {
  // Try to detect light theme by checking environment variables
  // This is an approximate detection, as there's no standard way to detect themes

  // Check for common environment variables that might indicate theme
  if let Ok(color_theme) = std::env::var("COLORFGBG") {
    // COLORFGBG is set by some terminals with foreground/background colors
    // If background color (last value) is high, it's likely a light theme
    if let Some(bg) = color_theme.split(';').last() {
      if let Ok(bg_val) = bg.parse::<u8>() {
        return bg_val > 10; // Higher values usually indicate lighter backgrounds
      }
    }
  }

  // Check for specific terminal settings
  if let Ok(term_program) = std::env::var("TERM_PROGRAM") {
    if let Ok(theme) = std::env::var(format!("{}_THEME", term_program)) {
      return theme.to_lowercase().contains("light");
    }
  }

  // Default to dark theme as it's more common in terminals
  false
}

/// Output duplication information to the specified stream
fn output_duplications<A: Write>(
  duplications: Vec<(String, Vec<(String, u32)>)>,
  mut output_stream: A,
) -> Result<(), Box<dyn Error>> {
  let is_light = is_light_theme();

  // Show the number of duplications found
  if duplications.is_empty() {
    writeln!(&mut output_stream, "No duplications found.")?;
    return Ok(());
  }

  let count_msg = format!("📚 Found {} duplicate entries", duplications.len());
  writeln!(&mut output_stream, "{}\n", count_msg.bold())?;

  for (line, line_locs) in duplications {
    // Format the duplicate line with borders but no background color
    let formatted_line = if is_light {
      // In light themes, make the line darker for better readability
      line.bold()
    } else {
      // In dark themes, use normal color for better readability
      line.normal()
    };

    write!(&mut output_stream, "{:76}", formatted_line)?;

    // Get terminal width or default to 80 if it can't be determined
    let term_width = terminal_size()
      .map(|(Width(w), _)| w as usize)
      .unwrap_or(80);

    // Line displayed on the left side is fixed at 80 chars (76 for content + 4 for borders and spaces)
    let left_width = 80;

    // Remaining width for file paths
    let avail_width = if term_width > left_width {
      term_width - left_width
    } else {
      40
    };

    // Format file locations with dynamic width to prevent overflow
    let mut current_line = String::new();
    let total_locations = line_locs.len();

    for (i, loc) in line_locs.iter().enumerate() {
      // Format each location as a colored item
      let file_path = loc.0.clone();
      let line_num = loc.1;
      let _is_last_location = i == total_locations - 1; // For potential future use

      // Adjust colors based on detected theme
      let loc_str = if is_light {
        // In light themes, use darker colors for better visibility
        format!("{}:{}", file_path.blue(), line_num.to_string().dimmed())
      } else {
        // In dark themes, use brighter colors for better visibility
        format!("{}:{}", file_path.dimmed(), line_num.to_string().yellow())
      };

      let list_marker = if is_light {
        "\n└─ ".blue().bold()
      } else {
        "\n└─ ".bright_blue().bold()
      };

      // Check if adding this location would exceed the available width
      if !current_line.is_empty()
        && current_line.len() + list_marker.len() + loc_str.len() > avail_width
      {
        write!(&mut output_stream, "{}", current_line)?;

        // Start a new line
        write!(&mut output_stream, "{}", list_marker)?;
        current_line = loc_str.to_string();
      } else {
        // Add to current line
        current_line = format!("{}{}{}", current_line, list_marker, loc_str);
      }
    }

    // Then write the file paths with the end marker
    writeln!(&mut output_stream, "{}", current_line)?;
  }

  Ok(())
}

pub fn run<A: Write>(
  config: Config,
  mut output_stream: A,
) -> Result<(), Box<dyn Error>> {
  match config.command {
    Command::Histogram { filepath } => {
      let file_content = fs::read_to_string(filepath)?;
      let freq_map = generate_frequency_map(&file_content);
      let formatted = format_freq_map(freq_map);
      // Use instead writeln! of println! to avoid "broken pipe" errors
      writeln!(&mut output_stream, "{}", formatted)?;
      Ok(())
    }
    Command::Duplication { path } => {
      let path = Path::new(&path);

      if path.is_file() {
        // Handle single file duplication
        let file_entry = FileEntry {
          name: path.to_string_lossy().into_owned(),
          content: fs::read_to_string(path)?,
        };
        let duplications = find_multi_line_duplications(vec![file_entry]);
        output_duplications(duplications, output_stream)
      } else if path.is_dir() {
        // Handle directory traversal
        let files = find_all_files(path)?;

        writeln!(
          &mut output_stream,
          "{}",
          format!(
            "🔎 Scanning {} files in directory: {}",
            files.len(),
            path.display()
          )
          .bold()
        )?;

        let file_entries = load_files(files)?;
        let duplications = find_multi_line_duplications(file_entries);

        output_duplications(duplications, output_stream)
      } else {
        Err(format!("Path does not exist: {}", path.display()).into())
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::fs::File;
  use std::io::Write;
  use tempfile::tempdir;

  #[test]
  fn generate_frequency_map_from_text() {
    let text = "Hello World! A warm welcome to the world.";
    let frequency_map = generate_frequency_map(&text);
    let expected_map = [
      (String::from("a"), 1),
      (String::from("hello"), 1),
      (String::from("the"), 1),
      (String::from("to"), 1),
      (String::from("warm"), 1),
      (String::from("welcome"), 1),
      (String::from("world"), 2),
    ]
    .iter()
    .cloned()
    .collect();

    assert_eq!(frequency_map, expected_map);
  }

  #[test]
  fn test_find_all_files() -> Result<(), Box<dyn Error>> {
    // Create a temporary directory structure
    let temp_dir = tempdir()?;
    let temp_path = temp_dir.path();

    // Create some nested directories
    let subdir = temp_path.join("subdir");
    fs::create_dir(&subdir)?;

    // Create some files
    let file1 = temp_path.join("file1.txt");
    let file2 = subdir.join("file2.txt");

    File::create(&file1)?.write_all(b"Test content 1")?;
    File::create(&file2)?.write_all(b"Test content 2")?;

    // Test the function
    let files = find_all_files(temp_path)?;

    assert_eq!(files.len(), 2);

    // Check that we found all the files (using contains instead of equality due to platform differences)
    assert!(files.iter().any(|p| p.ends_with("file1.txt")));
    assert!(files.iter().any(|p| p.ends_with("file2.txt")));

    Ok(())
  }

  #[test]
  fn test_load_files() -> Result<(), Box<dyn Error>> {
    // Create a temporary directory
    let temp_dir = tempdir()?;
    let temp_path = temp_dir.path();

    // Create some files
    let file1 = temp_path.join("file1.txt");
    let file2 = temp_path.join("file2.txt");

    File::create(&file1)?.write_all(b"Test content 1")?;
    File::create(&file2)?.write_all(b"Test content 2")?;

    // Test the function
    let file_entries = load_files(vec![file1.clone(), file2.clone()])?;

    assert_eq!(file_entries.len(), 2);
    assert_eq!(file_entries[0].name, file1.to_string_lossy());
    assert_eq!(file_entries[0].content, "Test content 1");
    assert_eq!(file_entries[1].name, file2.to_string_lossy());
    assert_eq!(file_entries[1].content, "Test content 2");

    Ok(())
  }
}
