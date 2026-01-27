//! Output formatting for CLI.

use serde::Serialize;
use serde_json::Value;

/// Format text output with labeled sections.
pub trait TextFormat {
    fn format_text(&self) -> String;
}

/// Print result in specified format.
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

/// Helper function to format JSON value as text (for cases without TextFormat impl).
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
