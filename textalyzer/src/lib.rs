pub mod types;
pub mod frequency;
pub mod duplication;
pub mod file_utils;
pub mod output;

extern crate colored;
extern crate ignore;
extern crate memmap2;
extern crate pad;
extern crate rayon;
extern crate terminal_size;
extern crate unicode_width;

use colored::Colorize;
use std::error::Error;
use std::fs;
use std::io::Write;
use std::path::Path;

use duplication::find_multi_line_duplications;
use file_utils::{find_all_files, load_files};
use frequency::{format_freq_map, generate_frequency_map};
use output::output_duplications;
use types::{Command, Config};

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
      let mut all_files = Vec::new();
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