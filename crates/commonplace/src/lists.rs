//! The note list (spec row 71): the house list shape (house §Q3) —
//! `GtkListView` + `SingleSelection` (autoselect off, can-unselect on)
//! over a `gio::ListStore` of domain GObjects, rows via
//! `SignalListItemFactory` with bind-time recycling discipline. The
//! row is NV's: title over a body preview, the preview hard-capped at
//! 400 chars before Pango sees it (Viaduct's load-bearing rule:
//! Pango shapes the entire text before ellipsizing).

use gtk4::glib;
use gtk4::prelude::*;

mod imp {
    use std::cell::RefCell;

    use gtk4::glib;
    use gtk4::glib::Properties;
    use gtk4::glib::prelude::*;
    use gtk4::glib::subclass::prelude::*;

    #[derive(Properties, Default)]
    #[properties(wrapper_type = super::NoteNode)]
    pub struct NoteNode {
        /// The display title (metadata override or first line).
        #[property(get, set)]
        pub title: RefCell<String>,
        /// The body preview, 400 chars max, newlines flattened.
        #[property(get, set)]
        pub preview: RefCell<String>,
        /// The note's path, the stable identity a repopulate
        /// preserves selection by.
        #[property(get, set)]
        pub path: RefCell<String>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for NoteNode {
        const NAME: &'static str = "CommonplaceNoteNode";
        type Type = super::NoteNode;
        type ParentType = glib::Object;
    }

    #[glib::derived_properties]
    impl ObjectImpl for NoteNode {}
}

glib::wrapper! {
    pub struct NoteNode(ObjectSubclass<imp::NoteNode>);
}

impl NoteNode {
    pub fn new(title: &str, preview: &str, path: &str) -> Self {
        glib::Object::builder()
            .property("title", title)
            .property("preview", preview)
            .property("path", path)
            .build()
    }
}

/// The 400-char preview cap (house §Q3, timeline.rs PREVIEW_MAX_CHARS):
/// applied BEFORE Pango ever sees the text.
pub const PREVIEW_MAX_CHARS: usize = 400;

/// The row preview: the body with its title line dropped, newlines
/// flattened to spaces, capped at [`PREVIEW_MAX_CHARS`].
pub fn body_preview(body: &str) -> String {
    let rest = body
        .lines()
        .skip(1)
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_owned();
    if rest.chars().count() <= PREVIEW_MAX_CHARS {
        rest
    } else {
        rest.chars().take(PREVIEW_MAX_CHARS).collect()
    }
}

/// The single shared factory: setup builds the row widget tree once,
/// bind copies the node's properties in, unbind clears references.
/// (The recycling discipline: nothing stale survives a bind.)
pub fn note_factory() -> gtk4::SignalListItemFactory {
    let factory = gtk4::SignalListItemFactory::new();
    factory.connect_setup(|_, item| {
        let list_item = item.downcast_ref::<gtk4::ListItem>().expect("a list item");
        let row = gtk4::Box::new(gtk4::Orientation::Vertical, 2);
        let title = gtk4::Label::new(None);
        title.set_xalign(0.0);
        title.add_css_class("heading");
        title.set_ellipsize(gtk4::pango::EllipsizeMode::End);
        let preview = gtk4::Label::new(None);
        preview.set_xalign(0.0);
        preview.add_css_class("dimmed");
        preview.set_wrap(true);
        preview.set_wrap_mode(gtk4::pango::WrapMode::WordChar);
        preview.set_ellipsize(gtk4::pango::EllipsizeMode::End);
        preview.set_lines(2);
        preview.set_max_width_chars(60);
        row.append(&title);
        row.append(&preview);
        list_item.set_child(Some(&row));
    });
    factory.connect_bind(|_, item| {
        let list_item = item.downcast_ref::<gtk4::ListItem>().expect("a list item");
        let Some(item) = list_item.item() else {
            return;
        };
        let node = item.downcast::<NoteNode>().expect("the model is NoteNode");
        let row = list_item.child().expect("setup built the row");
        let title = row.first_child().expect("title label");
        let preview = title.next_sibling().expect("preview label");
        title
            .downcast::<gtk4::Label>()
            .expect("title label")
            .set_text(&node.title());
        preview
            .downcast::<gtk4::Label>()
            .expect("preview label")
            .set_text(&node.preview());
    });
    factory
}
