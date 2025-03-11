use colored::Colorize;
use std::error::Error;
use std::io::Write;
use terminal_size::{terminal_size, Width};

/// Attempt to detect if terminal is using a light theme
pub fn is_light_theme() -> bool {
  // Try to detect light theme by checking environment variables
  // This is an approximate detection, as there's no standard way to detect themes

  // Check for common environment variables that might indicate theme
  if let Ok(color_theme) = std::env::var("COLORFGBG") {
    // COLORFGBG is set by some terminals with foreground/background colors
    // If background color (last value) is high, it's likely a light theme
    if let Some(bg) = color_theme.split(';').last() {
      if let Ok(bg_val) = bg.parse::<u8>() {
        return bg_val > 10; // Higher values usually indicate lighter backgrounds
      }
    }
  }

  // Check for specific terminal settings
  if let Ok(term_program) = std::env::var("TERM_PROGRAM") {
    if let Ok(theme) = std::env::var(format!("{}_THEME", term_program)) {
      return theme.to_lowercase().contains("light");
    }
  }

  // Default to dark theme as it's more common in terminals
  false
}

/// Output duplication information to the specified stream
pub fn output_duplications<A: Write>(
  duplications: Vec<(String, Vec<(String, u32)>)>,
  mut output_stream: A,
) -> Result<(), Box<dyn Error>> {
  let is_light = is_light_theme();

  // Show the number of duplications found
  if duplications.is_empty() {
    writeln!(&mut output_stream, "No duplications found.")?;
    return Ok(());
  }

  let count_msg = format!("📚 Found {} duplicate entries", duplications.len());
  writeln!(&mut output_stream, "{}\n", count_msg.bold())?;

  for (line, line_locs) in duplications {
    // Format the duplicate line with borders but no background color
    let formatted_line = if is_light {
      // In light themes, make the line darker for better readability
      line.bold()
    } else {
      // In dark themes, use normal color for better readability
      line.normal()
    };

    write!(&mut output_stream, "{:76}", formatted_line)?;

    // Get terminal width or default to 80 if it can't be determined
    let term_width = terminal_size()
      .map(|(Width(w), _)| w as usize)
      .unwrap_or(80);

    // Line displayed on the left side is fixed at 80 chars (76 for content + 4 for borders and spaces)
    let left_width = 80;

    // Remaining width for file paths
    let avail_width = if term_width > left_width {
      term_width - left_width
    } else {
      40
    };

    // Format file locations with dynamic width to prevent overflow
    let mut current_line = String::new();
    let total_locations = line_locs.len();

    for (i, loc) in line_locs.iter().enumerate() {
      // Format each location as a colored item
      let file_path = loc.0.clone();
      let line_num = loc.1;
      let _is_last_location = i == total_locations - 1; // For potential future use

      // Adjust colors based on detected theme
      let loc_str = if is_light {
        // In light themes, use darker colors for better visibility
        format!("{}:{}", file_path.blue(), line_num.to_string().dimmed())
      } else {
        // In dark themes, use brighter colors for better visibility
        format!("{}:{}", file_path.dimmed(), line_num.to_string().yellow())
      };

      let list_marker = if is_light {
        "\n└─ ".blue().bold()
      } else {
        "\n└─ ".bright_blue().bold()
      };

      // Check if adding this location would exceed the available width
      if !current_line.is_empty()
        && current_line.len() + list_marker.len() + loc_str.len() > avail_width
      {
        write!(&mut output_stream, "{}", current_line)?;

        // Start a new line
        write!(&mut output_stream, "{}", list_marker)?;
        current_line = loc_str.to_string();
      } else {
        // Add to current line
        current_line = format!("{}{}{}", current_line, list_marker, loc_str);
      }
    }

    // Then write the file paths with the end marker
    writeln!(&mut output_stream, "{}", current_line)?;
    
    // Add separator line of dashes after each duplication
    let separator = "-".repeat(term_width);
    writeln!(&mut output_stream, "{}", separator)?;
  }

  Ok(())
}