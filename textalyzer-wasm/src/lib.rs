use wasm_bindgen::prelude::*;

use textalyzer::{
  find_duplicate_lines,
  format_freq_map,
  generate_frequency_map,
  types::FileEntry,
  //
};

#[wasm_bindgen]
pub fn get_freq_map(text: String) -> String {
  let freq_map = generate_frequency_map(&text);
  format_freq_map(freq_map)
}

#[wasm_bindgen]
pub fn get_dup_lines(text: String) -> String {
  let temp_file = FileEntry {
    name: "textarea".to_string(),
    content: text,
  };
  let duplications = find_duplicate_lines(vec![temp_file]);
  serde_json::to_string(&duplications).unwrap()
}
