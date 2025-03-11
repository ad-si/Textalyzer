extern crate clap;

use self::clap::Subcommand;

#[derive(Subcommand)]
pub enum Command {
  /// Prints a histogram of word frequency in a file
  Histogram { filepath: String },
  /// Prints sections that might be duplicated in files or directories (recursive)
  Duplication {
    /// Paths to files or directories to scan for duplicates
    paths: Vec<String>,
  },
}

pub struct Config {
  pub command: Command,
}

#[derive(Debug, Clone)]
pub struct FileEntry {
  pub name: String,
  pub content: String,
}

#[derive(PartialEq, Debug)]
pub struct LineEntry {
  pub file_name: String,
  pub line_number: u32,
  pub content: String,
}
