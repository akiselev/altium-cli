//! Inspect command implementation.

use serde::Serialize;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use crate::output::{self, TextFormat};
use altium_format::io::{PcbLib, SchLib};

#[derive(Serialize)]
struct InspectResult {
    file_type: String,
    component_count: usize,
    components: Vec<ComponentInfo>,
}

impl TextFormat for InspectResult {
    fn format_text(&self) -> String {
        let mut output = String::new();
        output.push_str(&format!("File Type: {}\n", self.file_type));
        output.push_str(&format!("Components: {}\n\n", self.component_count));

        for (i, comp) in self.components.iter().enumerate() {
            output.push_str(&format!("{}. {}\n", i + 1, comp.name));
            if !comp.description.is_empty() {
                output.push_str(&format!("   Description: {}\n", comp.description));
            }
            if let Some(pins) = comp.pin_count {
                output.push_str(&format!("   Pins: {}\n", pins));
            }
            if let Some(pads) = comp.pad_count {
                output.push_str(&format!("   Pads: {}\n", pads));
            }
        }

        output
    }
}

#[derive(Serialize)]
struct ComponentInfo {
    name: String,
    description: String,
    pin_count: Option<usize>,
    pad_count: Option<usize>,
}

pub fn run(path: &Path, format: &str, _verbose: bool) -> Result<(), Box<dyn std::error::Error>> {
    let extension = path.extension().and_then(|s| s.to_str()).unwrap_or("");

    match extension.to_lowercase().as_str() {
        "schlib" => inspect_schlib(path, format),
        "pcblib" => inspect_pcblib(path, format),
        _ => Err(format!("Unsupported file type: {}", extension).into()),
    }
}

fn inspect_schlib(path: &Path, format: &str) -> Result<(), Box<dyn std::error::Error>> {
    let file = File::open(path)?;
    let lib = SchLib::open(BufReader::new(file))?;

    let components = map_schlib_components(&lib);

    let result = InspectResult {
        file_type: "SchLib".to_string(),
        component_count: lib.components.len(),
        components,
    };

    output::print(&result, format)?;
    Ok(())
}

fn inspect_pcblib(path: &Path, format: &str) -> Result<(), Box<dyn std::error::Error>> {
    let file = File::open(path)?;
    let lib = PcbLib::open(BufReader::new(file))?;

    let components = map_pcblib_components(&lib);

    let result = InspectResult {
        file_type: "PcbLib".to_string(),
        component_count: lib.components.len(),
        components,
    };

    output::print(&result, format)?;
    Ok(())
}

fn map_schlib_components(lib: &SchLib) -> Vec<ComponentInfo> {
    lib.components
        .iter()
        .map(|comp| ComponentInfo {
            name: comp.name().to_string(),
            description: comp.description().to_string(),
            pin_count: Some(comp.pin_count()),
            pad_count: None,
        })
        .collect()
}

fn map_pcblib_components(lib: &PcbLib) -> Vec<ComponentInfo> {
    lib.components
        .iter()
        .map(|comp| ComponentInfo {
            name: comp.pattern.clone(),
            description: comp.description.clone(),
            pin_count: None,
            pad_count: Some(comp.pad_count()),
        })
        .collect()
}
