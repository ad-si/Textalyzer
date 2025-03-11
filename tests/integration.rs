extern crate textalyzer;

use std::process::Command;

#[test]
fn it_can_be_called_with_histogram_args() {
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

#[test]
fn it_can_be_called_with_duplication_multiple_paths() {
    let output = Command::new("./target/debug/textalyzer")
        .args(&["duplication", "./examples/duplicates.py", "./examples/herr_von_ribbeck.txt"])
        .output()
        .expect("failed to execute process");

    // Verify we get output that contains duplication information
    let output_str = String::from_utf8_lossy(&output.stdout);
    assert!(
        output_str.contains("Scanning") && !output_str.contains("Error"),
        "\n\nERROR or unexpected output:\n{}",
        String::from_utf8_lossy(&output.stderr),
    );
}
