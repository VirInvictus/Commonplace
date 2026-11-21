//! The app-tier stylesheet (vir-gtk's `install_app_stylesheet` rung,
//! USER + 2): rules the app owns on top of the crate's base sheet,
//! spliced with the live palette and re-installed on every portal
//! flip, exactly like the crate tier re-splices.

use gtk4::glib;
use gtk4::prelude::*;
use vir_gtk::portal;
use vir_gtk::theme::{Palette, install_app_stylesheet};

/// The app's own rules. The quiet-toggle class carries binary state
/// (the sort-direction toggle): an active state reads as selected,
/// not as a suggested action, so it stays on the surface color with
/// normal text and a plain grid border. The crate tier's default
/// paints every checked button in the raw red accent, which reads
/// pink at button scale; the accent belongs to action semantics, not
/// to state display. Filed upstream as the default-change ask; this
/// sheet is the local fix while the default stands.
const APP_SHEET: &str = "\
button.cp-quiet-toggle:checked, button.cp-quiet-toggle:active {
  background-color: %BG_CARD%;
  color: %FG%;
  border-color: %GRID%;
}
";

/// Install the app sheet once now and re-splice it on every
/// dark/light flip; `owner` keeps the flip listener alive.
pub fn install(owner: &impl IsA<glib::Object>) {
    resplice();
    portal::connect_dark_changed(owner, |_is_dark| resplice());
}

fn resplice() {
    let palette = if portal::is_dark() {
        Palette::dragon()
    } else {
        Palette::lotus()
    };
    install_app_stylesheet(&palette.replace_tokens(APP_SHEET));
}
