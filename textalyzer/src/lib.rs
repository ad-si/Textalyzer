pub mod types;

extern crate pad;
extern crate unicode_width;

use pad::{Alignment, PadStr};
use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
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

/// Run Textalyzer with the given configuration.
/// Recursively find all files in a directory
fn find_all_files(dir: &Path) -> Result<Vec<PathBuf>, Box<dyn Error>> {
  let mut files = Vec::new();

  if dir.is_dir() {
    for entry in fs::read_dir(dir)? {
      let entry = entry?;
      let path = entry.path();

      if path.is_file() {
        files.push(path);
      } else if path.is_dir() {
        let mut subdir_files = find_all_files(&path)?;
        files.append(&mut subdir_files);
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

/// Output duplication information to the specified stream
fn output_duplications<A: Write>(
  duplications: Vec<(String, Vec<(String, u32)>)>,
  mut output_stream: A,
) -> Result<(), Box<dyn Error>> {
  writeln!(&mut output_stream, "Duplicate lines:\n")?;
  for (line, line_locs) in duplications {
    write!(&mut output_stream, "{:80} ▐ ", line)?;

    let line_locs_formatted = line_locs
      .iter()
      .map(|loc| format!("{}:{}", loc.0, loc.1))
      .collect::<Vec<String>>()
      .join(", ");
    writeln!(&mut output_stream, "{}", line_locs_formatted)?;
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
        let duplications = find_duplicate_lines(vec![file_entry]);
        output_duplications(duplications, output_stream)
      } else if path.is_dir() {
        // Handle directory traversal
        let files = find_all_files(path)?;

        writeln!(
          &mut output_stream,
          "Scanning {} files in directory: {}",
          files.len(),
          path.display()
        )?;

        let file_entries = load_files(files)?;
        let duplications = find_duplicate_lines(file_entries);

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
