extern crate textalyzer;

use std::process::Command;

#[test]
fn it_can_be_called_with_args() {
    let output = Command::new("./target/debug/textalyzer")
        .args(&["histogram", "./examples/1984.txt"])
        .output()
        .expect("failed to execute process");

    assert_eq!(
        String::from_utf8_lossy(&output.stdout).len(),
        239902,
        "\n\nERROR:\n{}",
        String::from_utf8_lossy(&output.stderr),
    );
}
