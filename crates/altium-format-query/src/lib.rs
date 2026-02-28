//! Altium Query Language (AQL) — a query language for selecting and filtering
//! entities in Altium Designer files.
//!
//! Operates exclusively through high-level API types ([`Component`], [`Pin`],
//! [`Parameter`], etc.) — never raw records or primitives.
//!
//! # Usage
//!
//! ```no_run
//! use altium_format::SchLib;
//! use altium_format_query::{parse_query, eval_query};
//!
//! let lib = SchLib::open("library.SchLib").unwrap();
//! let query = parse_query("component > pin:power").unwrap();
//! let results = eval_query(&query, &lib).unwrap();
//! for m in &results {
//!     println!("{}", m.node.display_name());
//! }
//! ```

mod adapter;
pub mod ast;
pub mod diagnostic;
pub mod error;
mod eval;
pub mod lexer;
mod parser;
#[allow(dead_code)]
mod schema;
mod value;

// ── Public API ───────────────────────────────────────────────────────────────

pub use adapter::{Queryable, QueryMatch, QueryNode, QueryResultSet};
pub use error::{QueryError, QueryErrorCode};
pub use eval::eval_query;
pub use parser::parse_query;
pub use ast::Query;
