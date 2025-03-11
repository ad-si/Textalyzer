extern crate textalyzer;

use std::process::Command;
use std::path::Path;

#[test]
fn it_can_be_called_with_histogram_args() {
    // Build paths correctly
    let root_dir = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let exe_path = root_dir.join("target/debug/textalyzer");
    let example_path = root_dir.join("examples/1984.txt");
    
    let output = Command::new(exe_path)
        .args(&["histogram", example_path.to_str().unwrap()])
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
    // Build paths correctly
    let root_dir = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let exe_path = root_dir.join("target/debug/textalyzer");
    let example_path1 = root_dir.join("examples/duplicates.py");
    let example_path2 = root_dir.join("examples/herr_von_ribbeck.txt");
    
    let output = Command::new(exe_path)
        .args(&["duplication", example_path1.to_str().unwrap(), example_path2.to_str().unwrap()])
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