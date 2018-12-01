use std::collections::HashMap;
use std::error::Error;
use std::fs;

pub static USAGE_STR: &str = "Usage: textalyzer <command> <filepath>";

pub fn generate_frequency_map(text: &str) -> HashMap<String, i32> {
    let words = text.split_whitespace();
    let mut frequency_map = HashMap::new();

    for word in words {
        let count = frequency_map.entry(String::from(word)).or_insert(0);
        *count += 1;
    }
    frequency_map
}

pub enum Command {
    Hist,
}

impl Command {
    pub fn parse(string: &str) -> Option<Command> {
        if string == "hist" {
            Some(Command::Hist)
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
                None => Err(format!("Command {} not available", cmd_str)),
            }
        } else {
            Err(USAGE_STR.to_string())
        }
    }
}

pub fn run(config: Config) -> Result<(), Box<dyn Error>> {
    match config.command {
        Command::Hist => {
            let file_content = fs::read_to_string(config.filepath)?;
            println!("{:?}", generate_frequency_map(&file_content).len());
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore]
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
