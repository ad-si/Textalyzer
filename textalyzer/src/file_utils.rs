use crate::types::{FileEntry, LineEntry, MappedContent};
use ignore::WalkBuilder;
use memmap2::MmapOptions;
use rayon::prelude::*;
use std::error::Error;
use std::fs::{self, File};
use std::path::{Path, PathBuf};

/// Merge lines from multiple files that pass the given filter
/// into a single list. Works with both memory mapped and string content.
pub fn merge_file_lines(
  filter: &dyn Fn(&&str) -> bool,
  files: Vec<FileEntry>,
) -> Vec<LineEntry> {
  files
    .iter()
    .flat_map(|file| {
      // Process based on content type
      match &file.content {
        MappedContent::Mapped(mmap) => {
          // Convert mmap to string slice
          if let Ok(content) = std::str::from_utf8(mmap) {
            // Process the content by lines
            content
              .lines()
              .enumerate()
              .filter(|(_num, line)| !line.trim().is_empty() && filter(line))
              .map(move |(num, line)| LineEntry {
                file_name: file.name.clone(),
                line_number: (num as u32 + 1),
                content: line.to_string(),
              })
              .collect::<Vec<_>>()
          } else {
            // Skip invalid UTF-8 content
            Vec::new()
          }
        }
        MappedContent::String(content) => {
          // Process string content
          content
            .lines()
            .enumerate()
            .filter(|(_num, line)| !line.trim().is_empty() && filter(line))
            .map(move |(num, line)| LineEntry {
              file_name: file.name.clone(),
              line_number: (num as u32 + 1),
              content: line.to_string(),
            })
            .collect::<Vec<_>>()
        }
      }
    })
    .collect()
}

/// Run Textalyzer with the given configuration.
/// Recursively find all files in a directory using the ignore crate
/// This respects .gitignore, .ignore, and other standard ignore files
pub fn find_all_files(dir: &Path) -> Result<Vec<PathBuf>, Box<dyn Error>> {
  let mut files = Vec::new();

  // Use WalkBuilder from the ignore crate to handle gitignore patterns properly
  let mut builder = WalkBuilder::new(dir);

  // Configure the walker to respect standard ignore files
  builder
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
/// using memory mapping for improved performance
pub fn load_files(
  paths: Vec<PathBuf>,
) -> Result<Vec<FileEntry>, Box<dyn Error>> {
  // Use rayon's parallel iterator to process files in parallel
  let file_entries: Vec<Option<FileEntry>> = paths
    .par_iter()
    .map(|path| {
      // Try to memory map the file first
      let result = (|| -> Result<FileEntry, Box<dyn Error>> {
        // Open the file
        let file = match File::open(path) {
          Ok(f) => f,
          Err(e) => {
            return Err(
              format!("Failed to open {}: {}", path.display(), e).into(),
            )
          }
        };

        // Check if the file is empty
        let metadata = file.metadata()?;
        if metadata.len() == 0 {
          // Empty files can't be memory mapped, use empty string instead
          return Ok(FileEntry {
            name: path.to_string_lossy().into_owned(),
            content: MappedContent::String(String::new()),
          });
        }

        // Try to memory map the file
        match unsafe { MmapOptions::new().map(&file) } {
          Ok(mmap) => {
            // Check if this looks like a binary file (contains null bytes)
            if mmap.contains(&0) {
              return Err("Binary file detected".into());
            }

            // Basic UTF-8 validation
            match std::str::from_utf8(&mmap) {
              Ok(_) => Ok(FileEntry {
                name: path.to_string_lossy().into_owned(),
                content: MappedContent::Mapped(mmap),
              }),
              Err(_) => Err("Invalid UTF-8 file".into()),
            }
          }
          Err(e) => {
            Err(format!("Failed to mmap {}: {}", path.display(), e).into())
          }
        }
      })();

      // If memory mapping fails, use regular string loading for small files
      match result {
        Ok(entry) => Some(entry),
        Err(_) => {
          // Fall back to reading the file as a string for small files
          match fs::metadata(path) {
            Ok(metadata) if metadata.len() < 1024 * 10 => {
              // Only fall back for files < 10KB
              match fs::read_to_string(path) {
                Ok(content) if !content.contains('\0') => Some(FileEntry {
                  name: path.to_string_lossy().into_owned(),
                  content: MappedContent::String(content),
                }),
                _ => None,
              }
            }
            _ => None,
          }
        }
      }
    })
    .collect();

  // Filter out None values (failed reads or binary files)
  let valid_entries = file_entries.into_iter().flatten().collect();

  Ok(valid_entries)
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::types::MappedContent;
  use std::fs::File;
  use std::io::Write;
  use tempfile::tempdir;

  #[test]
  fn test_merge_file_lines() {
    let file1 = FileEntry {
      name: "file1.txt".to_string(),
      content: MappedContent::String("Line one\nLine Two\n".to_string()),
    };
    let file2 = FileEntry {
      name: "file2.txt".to_string(),
      content: MappedContent::String("Another line\n".to_string()),
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

    // Check that we found all the files
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
    // Content checks using our PartialEq implementation
    assert!(file_entries[0].content == "Test content 1");
    assert_eq!(file_entries[1].name, file2.to_string_lossy());
    assert!(file_entries[1].content == "Test content 2");

    // Additional check with as_str()
    match &file_entries[0].content {
      MappedContent::Mapped(mmap) => {
        assert_eq!(std::str::from_utf8(mmap).unwrap(), "Test content 1");
      }
      MappedContent::String(s) => {
        assert_eq!(s, "Test content 1");
      }
    }

    Ok(())
  }
}
