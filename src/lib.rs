extern crate pad;
extern crate unicode_width;

use pad::{Alignment, PadStr};
use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::io;
use std::io::Write;
use unicode_width::UnicodeWidthStr;

pub static USAGE_STR: &str = "Usage: textalyzer histogram <filepath>";
pub static MAX_LINE_LENGTH: i32 = 80;

pub fn generate_frequency_map(text: &str) -> HashMap<String, i32> {
    let words = text
        .split(|character| !char::is_alphabetic(character))
        .filter(|word| word != &"");
    let mut frequency_map = HashMap::new();

    for word in words {
        let count = frequency_map.entry(word.to_lowercase()).or_insert(0);
        *count += 1;
    }
    frequency_map
}

pub fn format_freq_map(frequency_map: Vec<(&String, &i32)>) -> String {
    let mut longest_word = "";
    let mut highest_number = 0;

    for (word, count) in &frequency_map {
        let word_length = UnicodeWidthStr::width(&word[..]);

        if word_length > UnicodeWidthStr::width(longest_word) {
            longest_word = word;
        }
        if count > &&highest_number {
            highest_number = **count;
        }
    }

    let max_number_length = highest_number.to_string().len();
    let max_word_length = UnicodeWidthStr::width(longest_word);

    let max_line_length = max_word_length + 2 + max_number_length + 2;
    let remaining_space = MAX_LINE_LENGTH as usize - max_line_length;

    let mut result = String::new();

    for (word, count) in &frequency_map {
        let bar_width =
            (remaining_space as f32 / highest_number as f32) * **count as f32;

        result += &format!(
            "{}  {}  {}\n",
            word.pad_to_width_with_alignment(
                max_word_length, // this comment fixes rustfmt
                Alignment::Right
            ),
            count.to_string().pad_to_width_with_alignment(
                max_number_length,
                Alignment::Right
            ),
            "▆".repeat(bar_width.round() as usize),
        );
    }

    result
}

pub enum Command {
    Histogram,
}

impl Command {
    pub fn parse(string: &str) -> Option<Command> {
        if string == "histogram" {
            Some(Command::Histogram)
        } else {
            None
        }
    }
}

pub struct Config {
    command: Command,
    filepath: String,
}

impl Config {
    pub fn new(args: &[String]) -> Result<Config, String> {
        if let [_name, cmd_str, filepath] = args {
            match Command::parse(cmd_str) {
                Some(command) => Ok(Config {
                    command,
                    filepath: filepath.to_string(),
                }),
                None => Err(format!("Command \"{}\" not available", cmd_str)),
            }
        } else {
            Err(String::from("Error: Not enough arguments were provided!"))
        }
    }
}

pub fn run(config: Config) -> Result<(), Box<dyn Error>> {
    match config.command {
        Command::Histogram => {
            let file_content = fs::read_to_string(config.filepath)?;
            let freq_map = generate_frequency_map(&file_content);
            let mut freq_vec: Vec<_> = freq_map.iter().collect();
            freq_vec.sort_by(|t1, t2| t2.1.cmp(t1.1));

            let formatted = format_freq_map(freq_vec);
            // Use instead writeln! of println! to avoid "broken pipe" errors
            writeln!(io::stdout(), "{}", formatted);
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_frequency_map_from_text() {
        let text = "Hello World! A warm welcome to the world.";
        let frequency_map = generate_frequency_map(&text);
        let expected_map = [
            (String::from("a"), 1),
            (String::from("hello"), 1),
            (String::from("the"), 1),
            (String::from("to"), 1),
            (String::from("warm"), 1),
            (String::from("welcome"), 1),
            (String::from("world"), 2),
        ]
            .iter()
            .cloned()
            .collect();

        assert_eq!(frequency_map, expected_map);
    }
}
