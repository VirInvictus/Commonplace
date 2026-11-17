//! Commonplace: search-becomes-creation notes for Linux, ported from
//! the Notational Velocity / nvALT line. The skeleton shows an empty
//! window; the two-pane NV layout (search field over the note list)
//! lands with the app phases, styled by vir-gtk then.

use gtk4::prelude::*;
use gtk4::{Application, ApplicationWindow};

const APP_ID: &str = "io.github.virinvictus.Commonplace";

fn main() {
    let app = Application::builder().application_id(APP_ID).build();
    app.connect_activate(|app| {
        let window = ApplicationWindow::builder()
            .application(app)
            .title(format!("Commonplace {}", commonplace_core::VERSION))
            .default_width(900)
            .default_height(600)
            .build();
        window.present();
    });
    app.run();
}
