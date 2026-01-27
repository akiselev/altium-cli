//! Output formatting for CLI.

use serde::Serialize;
use serde_json::Value;

/// Format text output with labeled sections.
///
/// Implement this trait for types that need human-readable text representation
/// distinct from JSON serialization (e.g., multi-line formatted output with labels).
pub trait TextFormat {
    fn format_text(&self) -> String;
}

/// Print result in specified format (text, json, json-pretty).
///
/// Routes to TextFormat::format_text() for "text", serde_json for json formats.
/// Returns error if format string is unrecognized.
pub fn print<T: Serialize + TextFormat>(
    data: &T,
    format: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    match format {
        "text" => {
            println!("{}", data.format_text());
        }
        "json" => {
            let json = serde_json::to_string(data)?;
            println!("{}", json);
        }
        "json-pretty" => {
            let json = serde_json::to_string_pretty(data)?;
            println!("{}", json);
        }
        _ => {
            return Err(format!("Unknown format: {}", format).into());
        }
    }
    Ok(())
}

/// Format JSON value as text (fallback for types lacking TextFormat impl).
///
/// Recursively traverses JSON structure, printing key-value pairs with indentation.
/// Used when --json is not specified but type has no TextFormat implementation.
#[allow(dead_code)]
pub fn print_json_as_text(value: &Value) {
    match value {
        Value::Object(map) => {
            for (key, val) in map {
                match val {
                    Value::String(s) => println!("{}: {}", key, s),
                    Value::Number(n) => println!("{}: {}", key, n),
                    Value::Bool(b) => println!("{}: {}", key, b),
                    Value::Array(arr) => {
                        println!("{}:", key);
                        for item in arr {
                            print!("  ");
                            print_json_as_text(item);
                        }
                    }
                    Value::Object(_) => {
                        println!("{}:", key);
                        print!("  ");
                        print_json_as_text(val);
                    }
                    Value::Null => println!("{}: null", key),
                }
            }
        }
        _ => println!("{}", value),
    }
}
