//! Commonplace GTK4 app library half: the window shell, the note
//! list, the file watcher, and (Phase 11) the search-field state
//! machine. The engine stays GUI-free and dependency-free in
//! `commonplace-core`; this crate owns the whole GTK stack plus the
//! watching dependencies (spec rows 71-72).

pub mod lists;
pub mod style;
pub mod watch;
pub mod window;
