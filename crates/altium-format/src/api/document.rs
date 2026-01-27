//! AltiumDocument - unified entry point for Altium files.

use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Seek};
use std::path::Path;

use crate::api::cfb::{AltiumCfb, AltiumFileType};
use crate::api::generic::{BinaryContainer, ParamsContainer};
use crate::error::Result;

/// Unified entry point for working with Altium files.
///
/// Provides ergonomic access to any Altium file type through a common
/// interface, with methods for accessing different abstraction layers.
pub struct AltiumDocument<R: Read + Seek> {
    /// Low-level CFB wrapper
    cfb: AltiumCfb<R>,
    /// Cached parameter containers by stream path
    params_containers: HashMap<String, ParamsContainer>,
    /// Cached binary containers by stream path
    binary_containers: HashMap<String, BinaryContainer>,
}

impl AltiumDocument<File> {
    /// Opens a file by path.
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let cfb = AltiumCfb::open_file(path)?;
        Ok(AltiumDocument {
            cfb,
            params_containers: HashMap::new(),
            binary_containers: HashMap::new(),
        })
    }
}

impl<R: Read + Seek> AltiumDocument<R> {
    /// Opens from a reader.
    pub fn from_reader(reader: R) -> Result<Self> {
        let cfb = AltiumCfb::open(reader)?;
        Ok(AltiumDocument {
            cfb,
            params_containers: HashMap::new(),
            binary_containers: HashMap::new(),
        })
    }

    // --- Layer 1: CFB Access ---

    /// Returns the CFB wrapper for low-level operations.
    pub fn cfb(&mut self) -> &mut AltiumCfb<R> {
        &mut self.cfb
    }

    /// Returns the detected file type.
    pub fn file_type(&self) -> AltiumFileType {
        self.cfb.file_type()
    }

    // --- Layer 2: Generic Access ---

    /// Gets records from a stream as a parameter container.
    ///
    /// Loads and caches the container on first access.
    pub fn params(&mut self, stream_path: &str) -> Result<&ParamsContainer> {
        if !self.params_containers.contains_key(stream_path) {
            let blocks = self.cfb.read_blocks(stream_path)?;
            let container = ParamsContainer::from_blocks(stream_path, &blocks);
            self.params_containers
                .insert(stream_path.to_string(), container);
        }
        Ok(self.params_containers.get(stream_path).unwrap())
    }

    /// Gets mutable records from a stream.
    pub fn params_mut(&mut self, stream_path: &str) -> Result<&mut ParamsContainer> {
        if !self.params_containers.contains_key(stream_path) {
            let blocks = self.cfb.read_blocks(stream_path)?;
            let container = ParamsContainer::from_blocks(stream_path, &blocks);
            self.params_containers
                .insert(stream_path.to_string(), container);
        }
        Ok(self.params_containers.get_mut(stream_path).unwrap())
    }

    /// Gets binary records from a stream.
    pub fn binary(&mut self, stream_path: &str) -> Result<&BinaryContainer> {
        if !self.binary_containers.contains_key(stream_path) {
            let blocks = self.cfb.read_blocks(stream_path)?;
            let container = BinaryContainer::from_blocks(stream_path, &blocks);
            self.binary_containers
                .insert(stream_path.to_string(), container);
        }
        Ok(self.binary_containers.get(stream_path).unwrap())
    }

    /// Gets mutable binary records from a stream.
    pub fn binary_mut(&mut self, stream_path: &str) -> Result<&mut BinaryContainer> {
        if !self.binary_containers.contains_key(stream_path) {
            let blocks = self.cfb.read_blocks(stream_path)?;
            let container = BinaryContainer::from_blocks(stream_path, &blocks);
            self.binary_containers
                .insert(stream_path.to_string(), container);
        }
        Ok(self.binary_containers.get_mut(stream_path).unwrap())
    }

    // --- File-Type Helpers ---

    /// For libraries: lists component/footprint names.
    pub fn component_names(&mut self) -> Result<Vec<String>> {
        self.cfb.list_components()
    }

    /// Resolves a component name to its storage path.
    pub fn resolve_component(&mut self, name: &str) -> Result<String> {
        self.cfb.resolve_section(name)
    }

    /// Clears all cached containers.
    pub fn clear_cache(&mut self) {
        self.params_containers.clear();
        self.binary_containers.clear();
    }
}

#[cfg(test)]
mod tests {
    // Integration tests would go here with actual test files
}
