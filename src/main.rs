extern crate textalyzer;

use std::env;
use std::process;

use textalyzer::*;

fn main() {
    let args: Vec<String> = env::args().collect();

    let config = Config::new(&args).unwrap_or_else(|error| {
        eprintln!("{}\n", error);
        eprintln!("{}", USAGE_STR);
        process::exit(1);
    });

    if let Err(error) = run(config) {
        eprintln!("An error occurred during execution:\n{}", error);
        process::exit(1);
    }
}
