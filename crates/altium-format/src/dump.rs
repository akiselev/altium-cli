//! Pretty-printing utilities for dumping Altium file structures.
//!
//! Provides a tree-structured dump format for visualizing PCB and schematic
//! library contents in a human-readable way.

use std::fmt::Write;

use crate::types::ParameterCollection;

/// Options for controlling dump output.
#[derive(Debug, Clone, Default)]
pub struct DumpOptions {
    /// Show all parameter keys in the output.
    pub show_keys: bool,
}

/// A builder for creating tree-structured output.
#[derive(Default)]
pub struct TreeBuilder {
    output: String,
    indent_stack: Vec<bool>, // true = has more siblings after, false = last in group
    options: DumpOptions,
}

impl TreeBuilder {
    /// Create a new tree builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a new tree builder with options.
    pub fn with_options(options: DumpOptions) -> Self {
        Self {
            options,
            ..Self::default()
        }
    }

    /// Get the dump options.
    pub fn options(&self) -> &DumpOptions {
        &self.options
    }

    /// Check if show_keys is enabled.
    pub fn show_keys(&self) -> bool {
        self.options.show_keys
    }

    /// Get the current indentation prefix.
    fn prefix(&self) -> String {
        let mut s = String::new();
        for (i, &has_more) in self.indent_stack.iter().enumerate() {
            if i == self.indent_stack.len() - 1 {
                if has_more {
                    s.push_str("├── ");
                } else {
                    s.push_str("└── ");
                }
            } else if has_more {
                s.push_str("│   ");
            } else {
                s.push_str("    ");
            }
        }
        s
    }

    /// Get prefix for continuation lines (properties).
    fn continuation_prefix(&self) -> String {
        let mut s = String::new();
        for &has_more in &self.indent_stack {
            if has_more {
                s.push_str("│   ");
            } else {
                s.push_str("    ");
            }
        }
        s
    }

    /// Add a node with a title (creates a branch point).
    pub fn begin_node(&mut self, title: &str) {
        let prefix = self.prefix();
        writeln!(&mut self.output, "{}{}", prefix, title).expect("writing to String cannot fail");
    }

    /// Add a leaf node with properties.
    pub fn add_leaf(&mut self, title: &str, props: &[(&str, String)]) {
        let prefix = self.prefix();
        if props.is_empty() {
            writeln!(&mut self.output, "{}{}", prefix, title)
                .expect("writing to String cannot fail");
        } else {
            writeln!(&mut self.output, "{}{}", prefix, title)
                .expect("writing to String cannot fail");
            let cont = self.continuation_prefix();
            for (name, value) in props {
                writeln!(&mut self.output, "{}    {} = {}", cont, name, value)
                    .expect("writing to String cannot fail");
            }
        }
    }

    /// Add a leaf node with properties and optional params (for --show-keys debug).
    ///
    /// When `show_keys` option is enabled, dumps all parameters from the collection.
    /// Otherwise behaves like `add_leaf`.
    pub fn add_leaf_with_params(
        &mut self,
        title: &str,
        props: &[(&str, String)],
        params: Option<&ParameterCollection>,
    ) {
        let prefix = self.prefix();
        writeln!(&mut self.output, "{}{}", prefix, title).expect("writing to String cannot fail");
        let cont = self.continuation_prefix();

        // Always show the standard props
        for (name, value) in props {
            writeln!(&mut self.output, "{}    {} = {}", cont, name, value)
                .expect("writing to String cannot fail");
        }

        // If show_keys is enabled and we have params, dump all keys
        if self.options.show_keys {
            if let Some(params) = params {
                writeln!(&mut self.output, "{}    [all keys]", cont)
                    .expect("writing to String cannot fail");
                for (key, value) in params.iter() {
                    writeln!(
                        &mut self.output,
                        "{}        {} = {}",
                        cont,
                        key,
                        value.as_str()
                    )
                    .expect("writing to String cannot fail");
                }
            }
        }
    }

    /// Push into a child group (for nesting).
    pub fn push(&mut self, has_more_siblings: bool) {
        self.indent_stack.push(has_more_siblings);
    }

    /// Pop out of a child group.
    pub fn pop(&mut self) {
        self.indent_stack.pop();
    }

    /// Add a root node (no prefix).
    pub fn root(&mut self, title: &str) {
        writeln!(&mut self.output, "{}", title).expect("writing to String cannot fail");
    }

    /// Consume and return the built string.
    pub fn finish(self) -> String {
        self.output
    }
}

/// Trait for types that can dump themselves as a tree structure.
pub trait DumpTree {
    /// Dump this item to the tree builder.
    fn dump(&self, tree: &mut TreeBuilder);

    /// Convenience method to dump to a string.
    fn dump_to_string(&self) -> String {
        let mut tree = TreeBuilder::new();
        self.dump(&mut tree);
        tree.finish()
    }

    /// Convenience method to dump to a string with options.
    fn dump_to_string_with_options(&self, options: DumpOptions) -> String {
        let mut tree = TreeBuilder::with_options(options);
        self.dump(&mut tree);
        tree.finish()
    }
}

/// Format a coordinate value nicely (in mils).
pub fn fmt_coord(raw: i32) -> String {
    let mils = raw as f64 / 10000.0;
    if mils == mils.trunc() {
        format!("{:.0} mil", mils)
    } else {
        format!("{:.2} mil", mils)
    }
}

/// Format a coordinate point.
pub fn fmt_point(x: i32, y: i32) -> String {
    format!("({}, {})", fmt_coord(x), fmt_coord(y))
}

/// Format a coordinate point from Coord types.
pub fn fmt_coord_point(pt: &crate::types::CoordPoint) -> String {
    fmt_point(pt.x.to_raw(), pt.y.to_raw())
}

/// Format a single Coord value.
pub fn fmt_coord_val(c: &crate::types::Coord) -> String {
    fmt_coord(c.to_raw())
}

/// Format an angle in degrees.
pub fn fmt_angle(degrees: f64) -> String {
    format!("{:.1}°", degrees)
}

/// Format a layer.
pub fn fmt_layer(layer: &crate::types::Layer) -> String {
    layer.name().to_string()
}

/// Format a boolean as yes/no.
pub fn fmt_bool(b: bool) -> String {
    if b {
        "yes".to_string()
    } else {
        "no".to_string()
    }
}
