extern crate clap;
extern crate textalyzer;

use std::io;
use std::process;

use clap::Parser;

use textalyzer::run;
use textalyzer::types::{Command, Config};

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
  #[command(subcommand)]
  command: Option<Command>,
}

fn main() {
  let cli = Cli::parse();

  if let Some(command) = cli.command {
    if let Err(error) = run(Config { command }, io::stdout()) {
      eprintln!("ERROR:\n{error}");
      process::exit(1);
    }
  }
}
