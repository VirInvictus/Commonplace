//! Commonplace CLI library half: the verb machinery over the engine,
//! the comrak renderer adapter, the hand-rolled arg parser and JSON
//! envelope writer (spec §6). The engine stays dependency-free behind
//! `render::Renderer` while this crate owns the sanctioned comrak
//! dependency (spec row 66).

pub mod args;
pub mod completions;
pub mod json;
pub mod render;
pub mod verbs;
