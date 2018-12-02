extern crate textalyzer;

use std::io;
use textalyzer::*;

#[test]
fn it_can_be_called_with_args() {
    let args = [
        String::from("textalyzer"),
        String::from("histogram"),
        String::from("examples/1984.txt"),
    ];
    let config = Config::new(&args).unwrap_or_else(|error| panic!(error));

    if let Err(error) = run(config, io::sink()) {
        panic!("{}", error);
    }
}
