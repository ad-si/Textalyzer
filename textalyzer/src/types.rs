extern crate clap;
extern crate memmap2;

use self::clap::Subcommand;

#[derive(Subcommand)]
pub enum Command {
  /// Prints a histogram of word frequency in a file
  Histogram { filepath: String },
  /// Prints sections that might be duplicated in files or directories (recursive)
  Duplication {
    /// Paths to files or directories to scan for duplicates
    paths: Vec<String>,
    /// Minimum number of non-empty lines required to consider a block as a duplication
    #[clap(long, default_value = "3")]
    min_lines: usize,
    /// Only show the file paths with duplications, not the duplicated content
    #[clap(long)]
    files_only: bool,
  },
}

pub struct Config {
  pub command: Command,
}

#[derive(Debug)]
pub struct FileEntry {
  pub name: String,
  pub content: MappedContent,
}

#[derive(Debug)]
pub enum MappedContent {
  Mapped(memmap2::Mmap),
  String(String),
}

// Implement methods for MappedContent for easier use
impl MappedContent {
  // Get content as a string slice
  pub fn as_str(&self) -> Option<&str> {
    match self {
      MappedContent::Mapped(mmap) => std::str::from_utf8(mmap).ok(),
      MappedContent::String(s) => Some(s),
    }
  }

  // Get content as a string
  pub fn to_string(&self) -> Option<String> {
    self.as_str().map(String::from)
  }
}

// Implement PartialEq to compare with strings
impl PartialEq<str> for MappedContent {
  fn eq(&self, other: &str) -> bool {
    match self.as_str() {
      Some(s) => s == other,
      None => false,
    }
  }
}

impl PartialEq<&str> for MappedContent {
  fn eq(&self, other: &&str) -> bool {
    match self.as_str() {
      Some(s) => s == *other,
      None => false,
    }
  }
}

impl PartialEq<String> for MappedContent {
  fn eq(&self, other: &String) -> bool {
    match self.as_str() {
      Some(s) => s == other,
      None => false,
    }
  }
}

#[derive(PartialEq, Debug)]
pub struct LineEntry {
  pub file_name: String,
  pub line_number: u32,
  pub content: String,
}
