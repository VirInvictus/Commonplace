//! Commonplace: search-becomes-creation notes for Linux, ported from
//! the Notational Velocity / nvALT line. The two-pane NV shell: the
//! search field above the note list, the editor right, the folder
//! watched live (spec rows 71-72). Styling is the house vir-gtk layer
//! with the Kanagawa palettes; the engine stays GUI-free and
//! dependency-free in `commonplace-core`.

use std::path::PathBuf;

use gtk4::Application;
use gtk4::prelude::*;

use commonplace_app::window::{self, app_id};

fn main() {
    // GTK initializes before the styling layer touches settings APIs
    // (Application::run would init it, but only after this point).
    gtk4::init().expect("gtk init");
    // One-call styling: the portal listens, the base sheet resplices
    // with the palette (dragon for dark, lotus for light) on every
    // flip; `true` falls back dark until the portal answers.
    vir_gtk::theme::install_default(true);

    let app = Application::builder().application_id(app_id()).build();
    app.connect_activate(|app| {
        let folder = notes_folder();
        window::build(app, folder);
    });
    app.run();
}

/// The notes folder from the shared config (spec §6): the same file
/// the CLI reads, so `commonplace-cli config --set notes_dir` aims
/// both halves. A missing config is defaults; a missing folder is the
/// window's degraded empty state, never a crash.
fn notes_folder() -> PathBuf {
    let config =
        commonplace_core::config::Config::load(&commonplace_core::config::Config::xdg_path())
            .unwrap_or_default();
    config.notes_dir()
}
