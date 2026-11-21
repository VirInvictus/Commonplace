//! Commonplace CLI library half: the verb machinery and the comrak
//! renderer adapter. The binary's verb surface lands with the CLI
//! phases; the engine stays dependency-free behind `render::Renderer`
//! while this crate owns the sanctioned comrak dependency (spec row
//! 66).

pub mod render;
