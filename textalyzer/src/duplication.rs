use crate::file_utils::merge_file_lines;
use crate::types::{FileEntry, MappedContent};
use rayon::prelude::*;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

/// Find single-line duplications in a given text.
/// Works with both memory mapped files and regular string content.
/// Only includes lines with more than 5 characters after trimming.
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

/// Find duplications across files, utilizing parallel processing.
///
/// This function detects sequences of consecutive lines that are duplicated
/// across files or within the same file, prioritizing longer sequences.
/// Captures all duplications, including single-line ones, but they will be
/// filtered later based on the min_lines parameter.
/// Empty lines are not counted when determining line count for filtering.
/// When duplications overlap, only the longest one is kept.
///
/// Uses memory mapping for improved performance with large files.
pub fn find_multi_line_duplications(
  files: Vec<FileEntry>,
) -> Vec<(String, Vec<(String, u32)>)> {
  // Type definitions to reduce complexity
  type Location = (String, u32);
  type LineIndex = HashMap<String, Vec<Location>>;
  type BlocksMap = HashMap<String, Vec<Location>>;
  type SharedLineIndex = Arc<Mutex<LineIndex>>;
  type SharedBlocksMap = Arc<Mutex<BlocksMap>>;

  // Create a mapping of file lines for each file
  let file_lines_map: HashMap<String, Vec<String>> = files
    .iter()
    .map(|f| {
      // Get the lines from either mapped or string content
      let lines: Vec<String> = match &f.content {
        MappedContent::Mapped(mmap) => {
          if let Ok(content) = std::str::from_utf8(mmap) {
            content.lines().map(String::from).collect()
          } else {
            Vec::new()
          }
        }
        MappedContent::String(content) => {
          content.lines().map(String::from).collect()
        }
      };
      (f.name.clone(), lines)
    })
    .collect();

  // Create initial line index - map from line content to locations
  // Using a shared hash map for concurrent access
  let line_index: SharedLineIndex = Arc::new(Mutex::new(HashMap::new()));

  // Build the initial index of duplicate lines in parallel
  files.par_iter().for_each(|file_entry| {
    if let Some(file_lines) = file_lines_map.get(&file_entry.name) {
      let mut local_entries = Vec::new();

      // Process each line in the file and store entries in a local collection
      for (i, line) in file_lines.iter().enumerate() {
        let trimmed = line.trim();
        if !trimmed.is_empty() {
          local_entries.push((
            trimmed.to_string(), // key used for matching
            (file_entry.name.clone(), (i + 1) as u32),
          ));
        }
      }

      // Update the shared line index less frequently
      let mut index = line_index.lock().unwrap();
      for (line, location) in local_entries {
        index.entry(line).or_default().push(location);
      }
    }
  });

  // Get the inner value from Arc<Mutex<T>>
  let raw_line_index = Arc::try_unwrap(line_index)
    .expect("References to line_index still exist")
    .into_inner()
    .expect("Failed to unwrap Mutex");

  // Only keep lines that appear in multiple locations (duplicates)
  let duplicate_lines: HashMap<String, Vec<(String, u32)>> = raw_line_index
    .into_iter()
    .filter(|(_, locations)| locations.len() > 1)
    .collect();

  // For efficiency, only consider lines that appear as duplicates
  let duplicate_line_set: HashSet<String> =
    duplicate_lines.keys().cloned().collect();

  // Create a thread-safe container for blocks
  let blocks_map: SharedBlocksMap = Arc::new(Mutex::new(HashMap::new()));

  // Process each file in parallel
  files.par_iter().for_each(|file_entry| {
    let file_name = &file_entry.name;
    if let Some(file_lines) = file_lines_map.get(file_name) {
      let file_len = file_lines.len();

      // Local collection to minimize locks
      let mut local_blocks: HashMap<String, Vec<(String, u32)>> =
        HashMap::new();

      // For each potential starting position
      for start_idx in 0..file_len {
        // Skip if the first line isn't a known duplicate or is empty
        if start_idx < file_lines.len() {
          let first_line = &file_lines[start_idx];
          if !duplicate_line_set.contains(first_line)
            || first_line.trim().is_empty()
          {
            continue;
          }

          // Get all locations where this first line appears
          if let Some(locations) = duplicate_lines.get(first_line) {
            // For each other place this line appears
            for (other_file, other_line_num) in locations {
              // Skip if it's the same position we're checking from
              if other_file == file_name
                && *other_line_num == (start_idx as u32 + 1)
              {
                continue;
              }

              // Look up the other file's lines
              if let Some(other_file_lines) = file_lines_map.get(other_file) {
                let other_start_idx = (*other_line_num - 1) as usize;
                let other_file_len = other_file_lines.len();

                // Calculate maximum possible match length
                let max_len = std::cmp::min(
                  file_len - start_idx,
                  other_file_len - other_start_idx,
                );

                // Find how many consecutive lines match
                let mut match_len = 0;
                for offset in 0..max_len {
                  if start_idx + offset < file_lines.len()
                    && other_start_idx + offset < other_file_lines.len()
                    && file_lines[start_idx + offset].trim()
                      == other_file_lines[other_start_idx + offset].trim()
                  {
                    match_len += 1;
                  } else {
                    break;
                  }
                }

                // Only process matches of at least 1 line
                if match_len >= 1 {
                  // Slice with the original (indented) lines that form this block
                  let block_lines =
                    &file_lines[start_idx..start_idx + match_len];

                  // Determine the minimum leading-whitespace width
                  let min_indent = block_lines
                    .iter()
                    .filter_map(|l| {
                      let trimmed = l.trim_start();
                      if trimmed.is_empty() {
                        None
                      } else {
                        Some(l.len() - trimmed.len()) // number of leading white-space bytes
                      }
                    })
                    .min()
                    .unwrap_or(0);

                  // Re-build block with that common indent removed
                  let block = block_lines
                    .iter()
                    .map(|l| {
                      if l.len() >= min_indent {
                        l[min_indent..].to_string()
                      } else {
                        l.clone()
                      }
                    })
                    .collect::<Vec<String>>()
                    .join("\n");

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
    }
  });

  // Get the inner value from Arc<Mutex<T>>
  let raw_blocks_map = Arc::try_unwrap(blocks_map)
    .expect("References to blocks_map still exist")
    .into_inner()
    .expect("Failed to unwrap Mutex");

  // Convert to Vec and filter basic criteria
  let mut all_blocks: Vec<(String, Vec<(String, u32)>)> = raw_blocks_map
    .into_iter()
    .filter(|(content, _)| {
      // Keep any block with at least one duplicate and one non-empty line
      // Final filtering by min_lines will happen in lib.rs
      content
        .split('\n')
        .filter(|line| !line.trim().is_empty())
        .count()
        >= 1
    })
    .collect();

  // Sort by most non-empty lines first, then by length
  all_blocks.sort_by(|a, b| {
    // Count non-empty lines in each block
    let a_lines = a
      .0
      .split('\n')
      .filter(|line| !line.trim().is_empty())
      .count();
    let b_lines = b
      .0
      .split('\n')
      .filter(|line| !line.trim().is_empty())
      .count();

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

#[cfg(test)]
mod tests {
  use super::*;
  use crate::types::{FileEntry, MappedContent};
  use std::fs::File;
  use std::io::Write;
  use std::time::Instant;
  use tempfile::tempdir;

  #[test]
  fn test_find_duplicate_lines() {
    let file1 = FileEntry {
      name: "file1.txt".to_string(),
      content: MappedContent::String(
        "\
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
      ),
    };
    let file2 = FileEntry {
      name: "file2.txt".to_string(),
      content: MappedContent::String("This is a test.\n".to_string()),
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
      content: MappedContent::String(
        "\
              This is a test.\n\
              This is a second line.\n\
              This is a third line.\n\
              Some other content.\n\
              And another line here.\n\
              This is a test.\n\
              This is a second line.\n\
              A different third line.\n"
          .to_string(),
      ),
    };
    let file2 = FileEntry {
      name: "file2.txt".to_string(),
      content: MappedContent::String(
        "\
              Something unrelated.\n\
              This is a test.\n\
              This is a second line.\n\
              This is a third line.\n\
              Final line.\n"
          .to_string(),
      ),
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
    assert_eq!(
      locations.len(),
      2,
      "Expected 2 locations for 3-line duplication"
    );
    assert!(locations.contains(&("file1.txt".to_string(), 1)));
    assert!(locations.contains(&("file2.txt".to_string(), 2)));

    // The 2-line duplication should not be present because it's covered
    // by the 3-line duplication at the same starting positions
  }

  #[test]
  fn test_multi_line_duplications_with_non_overlapping() {
    let file1 = FileEntry {
      name: "file1.txt".to_string(),
      content: MappedContent::String(
        "\
              Block A line 1.\n\
              Block A line 2.\n\
              Block A line 3.\n\
              Some middle content.\n\
              Block B line 1.\n\
              Block B line 2.\n"
          .to_string(),
      ),
    };
    let file2 = FileEntry {
      name: "file2.txt".to_string(),
      content: MappedContent::String(
        "\
              Different stuff.\n\
              Block A line 1.\n\
              Block A line 2.\n\
              Block A line 3.\n\
              Some other content.\n\
              Block B line 1.\n\
              Block B line 2.\n"
          .to_string(),
      ),
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

  #[test]
  #[ignore] // This is a benchmark test, run it explicitly
  fn benchmark_multi_line_duplications() {
    // Create temporary files with duplications
    let temp_dir = tempdir().unwrap();
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
            content.push_str(&format!(
              "This is duplicated block {} line {}\n",
              block_id, k
            ));
          }
        } else {
          content.push_str(&format!("Unique line {} in file {}\n", j, i));
        }
      }

      File::create(&file_path)
        .unwrap()
        .write_all(content.as_bytes())
        .unwrap();
      files.push(file_path);
    }

    // Load files - now using memory mapping
    let file_entries = crate::file_utils::load_files(files).unwrap();

    // Measure performance
    let start = Instant::now();
    let duplications = find_multi_line_duplications(file_entries);
    let duration = start.elapsed();

    println!("Time elapsed: {:?}", duration);
    println!("Found {} duplications", duplications.len());

    // Verify that we found all duplicated blocks
    assert_eq!(duplications.len(), DUPLICATED_BLOCKS);
  }

  #[test]
  fn test_duplication_ignores_indentation() {
    let file1 = FileEntry {
      name: "file1.txt".into(),
      content: MappedContent::String(
        "    fn main() {\n        println!(\"Hello\");\n    }\n".into(),
      ),
    };
    let file2 = FileEntry {
      name: "file2.txt".into(),
      content: MappedContent::String(
        "fn main() {\nprintln!(\"Hello\");\n}\n".into(),
      ),
    };

    // Detect duplicates (multi-line)
    let dups = find_multi_line_duplications(vec![file1, file2]);

    // Expect exactly one 3-line duplication independent of indentation
    assert_eq!(dups.len(), 1);
    let (block, locs) = &dups[0];
    assert_eq!(
      block, "fn main() {\nprintln!(\"Hello\");\n}",
      "Block should be compared without leading spaces"
    );
    assert_eq!(locs.len(), 2, "Both files should be reported");
  }
}
