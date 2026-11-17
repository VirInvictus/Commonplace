//! Commonplace engine: notes-as-files storage, the search-first query
//! language, and the indexing that lets one field drive find-and-create.
//! GUI-free and dependency-free by contract; behavior contract in
//! `spec.md` at the workspace root.

pub mod fold;
pub mod note;
pub mod store;

/// Crate version, mirrored from the workspace `VERSION`.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
