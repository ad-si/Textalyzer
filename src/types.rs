extern crate clap;

use self::clap::Subcommand;

#[derive(Subcommand)]
pub enum Command {
    /// Prints a histogram of word frequency in a file
    Histogram { filepath: String },
    /// Prints sections of a file that might be duplicated
    Duplication { filepath: String },
}

pub struct Config {
    pub command: Command,
}
