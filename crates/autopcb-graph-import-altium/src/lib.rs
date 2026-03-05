use std::path::Path;

use altium_format::{PcbLib, SchLib};
use autopcb_graph::DesignWorkspace;

#[derive(Debug)]
pub enum ImportError {
    Altium(altium_format::AltiumFormatError),
}

impl std::fmt::Display for ImportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Altium(err) => write!(f, "{err}"),
        }
    }
}

impl std::error::Error for ImportError {}

impl From<altium_format::AltiumFormatError> for ImportError {
    fn from(value: altium_format::AltiumFormatError) -> Self {
        Self::Altium(value)
    }
}

pub fn import_schlib(path: &Path) -> Result<DesignWorkspace, ImportError> {
    let lib = SchLib::open(path)?;
    let components = lib.components()?;
    let design_name = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("imported-schlib");
    let mut workspace = DesignWorkspace::new(design_name, path.display().to_string());
    let collection = workspace.add_definition_collection("Imported Symbols");
    let _ = workspace.add_import(design_name, "altium.schlib", path.display().to_string());
    for component in components {
        let part = workspace
            .add_part_definition(
                &collection,
                component.lib_reference.clone(),
                component.description.clone(),
                component.designator.clone(),
                component
                    .pins
                    .into_iter()
                    .map(|pin| pin.designator)
                    .collect(),
            )
            .expect("collection exists");
        for footprint in component.footprints {
            let package_name = footprint.model_name;
            let package_def = if let Some(existing) = workspace
                .packages
                .iter()
                .find(|(_, pkg)| pkg.binding == package_name)
                .map(|(id, _)| id.clone())
            {
                existing
            } else {
                workspace
                    .add_package_definition(&collection, package_name.clone(), None, Vec::new())
                    .expect("collection exists")
            };
            let _ = workspace.link_part_to_package(&part, &package_def);
        }
    }
    Ok(workspace)
}

pub fn import_pcblib(path: &Path) -> Result<DesignWorkspace, ImportError> {
    let lib = PcbLib::open(path)?;
    let footprints = lib.footprints();
    let design_name = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("imported-pcblib");
    let mut workspace = DesignWorkspace::new(design_name, path.display().to_string());
    let collection = workspace.add_definition_collection("Imported Packages");
    let _ = workspace.add_import(design_name, "altium.pcblib", path.display().to_string());
    for footprint in footprints {
        let _ = workspace.add_package_definition(
            &collection,
            footprint.display_name,
            Some(footprint.description),
            footprint.pads.into_iter().map(|pad| pad.pad_name).collect(),
        );
    }
    Ok(workspace)
}
