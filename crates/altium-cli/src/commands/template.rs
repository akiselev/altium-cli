//! Template commands for creating Altium objects from JSON templates.

use clap::Subcommand;
use std::path::PathBuf;

#[derive(Subcommand)]
pub enum TemplateCommands {
    /// List available template types
    List,

    /// Export JSON Schema for a template type (for LLM tool-calling)
    Schema {
        /// Template name: "schlib-component", "pcblib-footprint", "schdoc-placement"
        name: String,
    },

    /// Apply a template to create objects in an Altium file
    Apply {
        /// Template name: "schlib-component", "pcblib-footprint", "schdoc-placement"
        name: String,

        /// Target file path (created if it doesn't exist for library types)
        path: PathBuf,

        /// JSON template input from file (use "-" for stdin)
        #[arg(short, long)]
        file: Option<String>,

        /// JSON template input as string
        #[arg(long = "input")]
        input: Option<String>,
    },
}

pub fn run(cmd: &TemplateCommands, format: &str) -> Result<(), Box<dyn std::error::Error>> {
    match cmd {
        TemplateCommands::List => {
            let templates = altium_format::templates::list_templates();
            if format == "json" {
                let json = serde_json::to_string(&templates)?;
                println!("{}", json);
            } else if format == "json-pretty" {
                let json = serde_json::to_string_pretty(&templates)?;
                println!("{}", json);
            } else {
                println!("Available templates:");
                for name in templates {
                    println!("  {}", name);
                }
                println!();
                println!("Use 'template schema <name>' to get the JSON Schema for a template.");
                println!("Use 'template apply <name> <file> --input <json>' to apply a template.");
            }
        }

        TemplateCommands::Schema { name } => {
            let schema = altium_format::templates::get_template_schema(name)
                .ok_or_else(|| format!("Unknown template '{}'. Use 'template list' to see available templates.", name))?;

            let json = if format == "json" {
                serde_json::to_string(&schema)?
            } else {
                serde_json::to_string_pretty(&schema)?
            };
            println!("{}", json);
        }

        TemplateCommands::Apply {
            name,
            path,
            file,
            input,
        } => {
            let json_content = read_json_input(file.clone(), input.clone())?;

            match name.as_str() {
                "schlib-component" => {
                    apply_schlib_component(path, &json_content)?;
                }
                "pcblib-footprint" => {
                    apply_pcblib_footprint(path, &json_content)?;
                }
                "schdoc-placement" => {
                    apply_schdoc_placement(path, &json_content)?;
                }
                _ => {
                    return Err(format!(
                        "Unknown template '{}'. Use 'template list' to see available templates.",
                        name
                    )
                    .into());
                }
            }
        }
    }

    Ok(())
}

fn read_json_input(
    file: Option<String>,
    json_str: Option<String>,
) -> Result<String, Box<dyn std::error::Error>> {
    use std::io::Read;

    match (file, json_str) {
        (_, Some(s)) => Ok(s),
        (Some(ref path), None) if path == "-" => {
            let mut buffer = String::new();
            std::io::stdin().read_to_string(&mut buffer)?;
            Ok(buffer)
        }
        (Some(ref file_path), None) => Ok(std::fs::read_to_string(file_path)?),
        (None, None) => Err("Must provide either --file <file> or --input <string>".into()),
    }
}

/// Validate that a file path has an expected extension for the template type.
fn validate_file_extension(
    path: &std::path::Path,
    expected_ext: &str,
    template_name: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");
    if !ext.eq_ignore_ascii_case(expected_ext) {
        return Err(format!(
            "template '{}' expects a .{} file, got '{}'",
            template_name,
            expected_ext,
            path.display()
        )
        .into());
    }
    Ok(())
}

fn apply_schlib_component(
    path: &std::path::Path,
    json_content: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    validate_file_extension(path, "SchLib", "schlib-component")?;

    use altium_format::templates::schlib::SchComponentTemplate;

    let template: SchComponentTemplate = serde_json::from_str(json_content)?;

    // Open or create the library using the ops module (handles blank template embedding)
    let mut lib = altium_format::ops::schlib::open_or_create(path)?;

    // Check for duplicate
    if lib
        .components
        .iter()
        .any(|c| c.component.lib_reference == template.name)
    {
        return Err(format!("Component '{}' already exists in library", template.name).into());
    }

    let component = template.apply()?;
    let name = component.component.lib_reference.clone();
    lib.components.push(component);

    lib.save_to_file(path)?;
    println!("Applied schlib-component template: added '{}' to {}", name, path.display());

    Ok(())
}

fn apply_pcblib_footprint(
    path: &std::path::Path,
    json_content: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    validate_file_extension(path, "PcbLib", "pcblib-footprint")?;

    use altium_format::templates::pcblib::PcbFootprintTemplate;

    let template: PcbFootprintTemplate = serde_json::from_str(json_content)?;

    // Open or create the library using the ops module (handles blank template embedding)
    let mut lib = altium_format::ops::pcblib::open_or_create(path)?;

    // Check for duplicate
    if lib.components.iter().any(|c| c.pattern == template.name) {
        return Err(format!("Footprint '{}' already exists in library", template.name).into());
    }

    let component = template.apply()?;
    let name = component.pattern.clone();
    lib.components.push(component);

    lib.save_to_file(path)?;
    println!(
        "Applied pcblib-footprint template: added '{}' to {}",
        name,
        path.display()
    );

    Ok(())
}

fn apply_schdoc_placement(
    _path: &std::path::Path,
    _json_content: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    Err("schdoc-placement template apply is not yet implemented. \
         Use 'altium-cli edit <file> -c \"add-wire ...\"' for individual operations, \
         or use 'template schema schdoc-placement' to get the JSON Schema for LLM structured output."
        .into())
}
