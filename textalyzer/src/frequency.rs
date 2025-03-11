use std::collections::HashMap;
use pad::{Alignment, PadStr};
use unicode_width::UnicodeWidthStr;

const MAX_LINE_LENGTH: u16 = 80;

/// Generate a frequency map from a given text.
///
/// # Examples
///
/// ```rust
/// use textalyzer::frequency::generate_frequency_map;
///
/// let freq_map = generate_frequency_map(
///    "This test is a test to test the frequency map."
/// );
///
/// let expected_map: std::collections::HashMap<_, _> = vec![
///   ("this", 1),
///   ("test", 3),
///   ("is", 1),
///   ("a", 1),
///   ("to", 1),
///   ("the", 1),
///   ("frequency", 1),
///   ("map", 1),
/// ]
/// .into_iter()
/// .map(|(s, n)| (String::from(s), n))
/// .collect();
///
/// assert_eq!(freq_map, expected_map);
/// ```
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

/// Format a frequency map into a string.
pub fn format_freq_map(freq_map: HashMap<String, i32>) -> String {
  let mut freq_vec: Vec<_> = freq_map.iter().collect();
  freq_vec.sort_by(|t1, t2| t2.1.cmp(t1.1));
  let mut longest_word = "";
  let mut highest_number = &0;

  for (word, count) in &freq_vec {
    let word_length = UnicodeWidthStr::width(&word[..]);

    if word_length > UnicodeWidthStr::width(longest_word) {
      longest_word = word;
    }
    if count > &highest_number {
      highest_number = count;
    }
  }

  let max_number_length = highest_number.to_string().len();
  let max_word_length = UnicodeWidthStr::width(longest_word);

  let max_line_length = max_word_length + 2 + max_number_length + 2;
  let remaining_space = MAX_LINE_LENGTH as usize - max_line_length;

  let mut result = String::new();

  for (word, count) in &freq_vec {
    let bar_width =
      (remaining_space as f32 / *highest_number as f32) * **count as f32;

    result += &format!(
      "{}  {}  {}\n",
      word.pad_to_width_with_alignment(
        max_word_length, // this comment fixes rustfmt
        Alignment::Right
      ),
      count
        .to_string()
        .pad_to_width_with_alignment(max_number_length, Alignment::Right),
      "▆".repeat(bar_width.round() as usize),
    );
  }

  result
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