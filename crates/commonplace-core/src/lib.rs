//! Commonplace engine: notes-as-files storage, the search-first query
//! language, and the indexing that lets one field drive find-and-create.
//! GUI-free and dependency-free by contract; behavior contract in
//! `spec.md` at the workspace root.

pub mod fold;
pub mod links;
pub mod metadata;
pub mod note;
pub mod query;
pub mod render;
pub mod search;
pub mod store;
pub mod tags;

/// Crate version, mirrored from the workspace `VERSION`.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
