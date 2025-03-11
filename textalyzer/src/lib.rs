pub mod types;

extern crate colored;
extern crate ignore;
extern crate pad;
extern crate rayon;
extern crate terminal_size;
extern crate unicode_width;

use colored::Colorize;
use ignore::WalkBuilder;
use pad::{Alignment, PadStr};
use rayon::prelude::*;
use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
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

/// Find multi-line duplications across files, utilizing parallel processing.
///
/// This function detects sequences of consecutive lines that are duplicated
/// across files or within the same file, prioritizing longer sequences.
/// For single-line duplications, it only includes lines with more than 3 non-whitespace characters.
/// Multi-line duplications are always included.
/// When duplications overlap, only the longest one is kept.
pub fn find_multi_line_duplications(
  files: Vec<FileEntry>,
) -> Vec<(String, Vec<(String, u32)>)> {
  // Type definitions to reduce complexity
  type FileLines<'a> = Vec<&'a str>;
  type Location = (String, u32);
  type LineIndex<'a> = HashMap<&'a str, Vec<Location>>;
  type BlocksMap = HashMap<String, Vec<Location>>;
  type SharedLineIndex<'a> = Arc<Mutex<LineIndex<'a>>>;
  type SharedBlocksMap = Arc<Mutex<BlocksMap>>;
  
  // Store the parsed lines for each file to avoid repeated parsing
  let file_lines_map: HashMap<String, FileLines> = files
    .iter()
    .map(|f| (f.name.clone(), f.content.lines().collect()))
    .collect();
  
  // Create initial line index - map from line content to locations
  // Using a shared hash map for concurrent access
  let line_index: SharedLineIndex = Arc::new(Mutex::new(HashMap::new()));
  
  // Build the initial index of duplicate lines in parallel
  files.par_iter().for_each(|file_entry| {
    let file_lines = file_lines_map.get(&file_entry.name).unwrap();
    let mut local_entries = Vec::new();
    
    // Process each line in the file and store entries in a local collection
    for (i, line) in file_lines.iter().enumerate() {
      if !line.trim().is_empty() {
        local_entries.push((
          *line, 
          (file_entry.name.clone(), (i + 1) as u32)
        ));
      }
    }
    
    // Update the shared line index less frequently
    let mut index = line_index.lock().unwrap();
    for (line, location) in local_entries {
      index.entry(line).or_default().push(location);
    }
  });
  
  // Get the inner value from Arc<Mutex<T>>
  let raw_line_index = Arc::try_unwrap(line_index)
    .expect("References to line_index still exist")
    .into_inner()
    .expect("Failed to unwrap Mutex");
  
  // Only keep lines that appear in multiple locations (duplicates)
  let duplicate_lines: HashMap<&str, Vec<(String, u32)>> = raw_line_index
    .into_iter()
    .filter(|(_, locations)| locations.len() > 1)
    .collect();
  
  // For efficiency, only consider lines that appear as duplicates
  let duplicate_line_set: std::collections::HashSet<&str> = 
    duplicate_lines.keys().copied().collect();
  
  // Create a thread-safe container for blocks
  let blocks_map: SharedBlocksMap = Arc::new(Mutex::new(HashMap::new()));
  
  // Process each file in parallel
  files.par_iter().for_each(|file_entry| {
    let file_name = &file_entry.name;
    let file_lines = file_lines_map.get(file_name).unwrap();
    let file_len = file_lines.len();
    
    // Local collection to minimize locks
    let mut local_blocks: HashMap<String, Vec<(String, u32)>> = HashMap::new();
    
    // For each potential starting position
    for start_idx in 0..file_len {
      // Skip if the first line isn't a known duplicate or is empty
      let first_line = file_lines[start_idx];
      if !duplicate_line_set.contains(first_line) || first_line.trim().is_empty() {
        continue;
      }
      
      // Get all locations where this first line appears
      if let Some(locations) = duplicate_lines.get(first_line) {
        // For each other place this line appears
        for (other_file, other_line_num) in locations {
          // Skip if it's the same position we're checking from
          if other_file == file_name && *other_line_num == (start_idx as u32 + 1) {
            continue;
          }
          
          // Look up the other file's lines
          let other_file_lines = file_lines_map.get(other_file).unwrap();
          let other_start_idx = (*other_line_num - 1) as usize;
          let other_file_len = other_file_lines.len();
          
          // Calculate maximum possible match length
          let max_len = std::cmp::min(
            file_len - start_idx,
            other_file_len - other_start_idx
          );
          
          // Find how many consecutive lines match
          let mut match_len = 0;
          for offset in 0..max_len {
            if file_lines[start_idx + offset] == other_file_lines[other_start_idx + offset] {
              match_len += 1;
            } else {
              break;
            }
          }
          
          // Only process matches of at least 1 line
          if match_len >= 1 {
            // Efficiently build the block string
            let block = file_lines[start_idx..(start_idx + match_len)].join("\n");
            
            // Use our local hash map for faster lookups
            let locations = local_blocks.entry(block).or_default();
            
            // Add the current file location if not already present
            let current_loc = (file_name.clone(), start_idx as u32 + 1);
            if !locations.contains(&current_loc) {
              locations.push(current_loc);
            }
            
            // Add the other location if not already present
            let other_loc = (other_file.clone(), *other_line_num);
            if !locations.contains(&other_loc) {
              locations.push(other_loc);
            }
          }
        }
      }
    }
    
    // Merge local blocks into the shared map
    if !local_blocks.is_empty() {
      let mut shared_blocks = blocks_map.lock().unwrap();
      for (block, locations) in local_blocks {
        let shared_locations = shared_blocks.entry(block).or_default();
        for loc in locations {
          if !shared_locations.contains(&loc) {
            shared_locations.push(loc);
          }
        }
      }
    }
  });
  
  // Get the inner value from Arc<Mutex<T>>
  let raw_blocks_map = Arc::try_unwrap(blocks_map)
    .expect("References to blocks_map still exist")
    .into_inner()
    .expect("Failed to unwrap Mutex");
  
  // Convert to Vec and filter by our criteria
  let mut all_blocks: Vec<(String, Vec<(String, u32)>)> = raw_blocks_map
    .into_iter()
    .filter(|(content, _)| {
      // Keep multi-line blocks 
      if content.contains('\n') {
        return true;
      }
      // For single lines, require more than 3 non-whitespace chars
      let non_ws_count = content.chars().filter(|c| !c.is_whitespace()).count();
      non_ws_count > 3
    })
    .collect();
  
  // Sort by most lines first, then by length
  all_blocks.sort_by(|a, b| {
    let a_lines = a.0.matches('\n').count() + 1;
    let b_lines = b.0.matches('\n').count() + 1;
    
    let line_cmp = b_lines.cmp(&a_lines);
    if line_cmp == std::cmp::Ordering::Equal {
      b.0.len().cmp(&a.0.len())
    } else {
      line_cmp
    }
  });
  
  // Process overlapping duplications 
  // This part is not parallelized because it processes items sequentially
  // based on their sorted order
  let mut result = Vec::new();
  let mut used_positions: HashMap<(String, u32), usize> = HashMap::new();
  
  for (content, locations) in all_blocks {
    let lines_count = content.matches('\n').count() + 1;
    let mut valid_locations = Vec::new();
    
    // Check each location for overlap
    for (file, line_num) in &locations {
      let end_line = line_num + lines_count as u32 - 1;
      let mut position_free = true;
      
      // Fast overlap check - break early
      for l in *line_num..=end_line {
        if let Some(idx) = used_positions.get(&(file.clone(), l)) {
          if *idx < result.len() {
            position_free = false;
            break;
          }
        }
      }
      
      if position_free {
        valid_locations.push((file.clone(), *line_num));
        // Mark positions as used
        for l in *line_num..=end_line {
          used_positions.insert((file.clone(), l), result.len());
        }
      }
    }
    
    // Only keep duplications with at least 2 valid locations
    if valid_locations.len() >= 2 {
      result.push((content, valid_locations));
    }
  }
  
  result
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

/// Load multiple files as FileEntry structs using parallel processing
fn load_files(paths: Vec<PathBuf>) -> Result<Vec<FileEntry>, Box<dyn Error>> {
  // Use rayon's parallel iterator to process files in parallel
  let file_entries: Vec<Option<FileEntry>> = paths
    .par_iter()
    .map(|path| {
      // Skip binary files and files we can't read
      match fs::read_to_string(path) {
        Ok(content) if !content.contains('\0') => {
          // Simple check for binary files
          Some(FileEntry {
            name: path.to_string_lossy().into_owned(),
            content,
          })
        }
        _ => None,
      }
    })
    .collect();

  // Filter out None values (failed reads or binary files)
  let valid_entries = file_entries.into_iter().flatten().collect();
  
  Ok(valid_entries)
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
    
    // Add separator line of dashes after each duplication
    let separator = "-".repeat(term_width);
    writeln!(&mut output_stream, "{}", separator)?;
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
    Command::Duplication { paths } => {
      // Collect all file entries from all specified paths
      let mut all_files: Vec<PathBuf> = Vec::new();
      let mut scanned_dirs = 0;
      let mut scanned_files = 0;

      // Process each path argument
      for path_str in paths {
        let path = Path::new(&path_str);

        if path.is_file() {
          // Single file
          all_files.push(path.to_path_buf());
          scanned_files += 1;
        } else if path.is_dir() {
          // Directory traversal
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
          
          all_files.extend(files);
          scanned_dirs += 1;
        } else {
          return Err(format!("Path does not exist: {}", path.display()).into());
        }
      }

      if scanned_dirs == 0 && scanned_files > 0 {
        writeln!(
          &mut output_stream,
          "{}",
          format!(
            "🔎 Scanning {} file(s)",
            all_files.len()
          )
          .bold()
        )?;
      }

      if all_files.is_empty() {
        return Err("No valid files found in the specified paths".into());
      }

      // Load all collected files
      let file_entries = load_files(all_files)?;
      let duplications = find_multi_line_duplications(file_entries);

      output_duplications(duplications, output_stream)
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::fs::File;
  use std::io::Write;
  use std::time::Instant;
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
  
  #[test]
  #[ignore] // This is a benchmark test, run it explicitly
  fn benchmark_multi_line_duplications() -> Result<(), Box<dyn Error>> {
    // Create temporary files with duplications
    let temp_dir = tempdir()?;
    let temp_path = temp_dir.path();
    
    const NUM_FILES: usize = 20;
    const LINES_PER_FILE: usize = 2000;
    const DUPLICATED_BLOCKS: usize = 30;
    const BLOCK_SIZE: usize = 5;
    
    let mut files = Vec::new();
    
    // Generate test files with duplicated blocks
    for i in 0..NUM_FILES {
      let file_path = temp_path.join(format!("file{}.txt", i));
      let mut content = String::new();
      
      for j in 0..LINES_PER_FILE {
        // Insert duplicated blocks at regular intervals
        if j % 50 == 0 && j < DUPLICATED_BLOCKS * 50 {
          let block_id = j / 50;
          for k in 0..BLOCK_SIZE {
            content.push_str(&format!("This is duplicated block {} line {}\n", block_id, k));
          }
        } else {
          content.push_str(&format!("Unique line {} in file {}\n", j, i));
        }
      }
      
      File::create(&file_path)?.write_all(content.as_bytes())?;
      files.push(file_path);
    }
    
    // Load files
    let file_entries = load_files(files)?;
    
    // Measure performance
    let start = Instant::now();
    let duplications = find_multi_line_duplications(file_entries);
    let duration = start.elapsed();
    
    println!("Time elapsed: {:?}", duration);
    println!("Found {} duplications", duplications.len());
    
    // Verify that we found all duplicated blocks
    assert_eq!(duplications.len(), DUPLICATED_BLOCKS);
    
    Ok(())
  }
}
