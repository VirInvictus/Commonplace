//! The two-pane NV shell (spec row 71): the search field above the
//! results list, the editor right, the sort picker between them. The
//! list runs on the house shape (house §Q3); the editor is a
//! non-editing display until Phase 12 wires autosave and undo.

use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use gtk4::gio;
use gtk4::glib;
use gtk4::prelude::*;
use gtk4::*;

use commonplace_core::search::{SortColumn, SortOptions, sort_visible};
use commonplace_core::store::{Store, load_folder};

use crate::lists::{NoteNode, body_preview, note_factory};

const APP_ID: &str = "io.github.virinvictus.Commonplace";

pub fn app_id() -> &'static str {
    APP_ID
}

/// The window state, main-thread only (everything lives behind the
/// Rc the GTK callbacks share).
struct State {
    store: Store,
    folder: PathBuf,
    sort: SortOptions,
    query: String,
    list_store: gio::ListStore,
    selection: SingleSelection,
    stack: Stack,
    editor: TextView,
}

type Shared = Rc<RefCell<State>>;

/// Build the window and every behavior inside it. The watch channel
/// wires after the widgets exist.
pub fn build(app: &Application, folder: PathBuf) -> gtk4::ApplicationWindow {
    let (store, warnings) = match load_folder(&folder) {
        Ok(outcome) => (outcome.store, outcome.warnings),
        Err(e) => {
            // A missing or unreadable folder pages the empty state; the
            // app stays alive so the user can configure notes_dir.
            return build_empty(app, &folder, &format!("{e}"));
        }
    };
    let _ = warnings.len(); // scan warnings surface with the editor phase's status row

    let window = ApplicationWindow::builder()
        .application(app)
        .title(format!("Commonplace {}", commonplace_core::VERSION))
        .default_width(980)
        .default_height(680)
        .build();

    let root = Box::new(Orientation::Vertical, 6);
    root.set_margin_top(6);
    root.set_margin_bottom(6);
    root.set_margin_start(6);
    root.set_margin_end(6);
    window.set_child(Some(&root));

    // The search field: the one field, on top.
    let search = SearchEntry::new();
    search.set_placeholder_text(Some("Search or create"));
    root.append(&search);

    // The controls row: the column sort picker and its direction.
    let controls = Box::new(Orientation::Horizontal, 6);
    let sort_model = gtk4::StringList::new(&["Date Modified", "Date Created", "Title"]);
    let picker = DropDown::builder().model(&sort_model).build();
    picker.set_selected(0); // upstream's default: date modified
    let reverse = ToggleButton::with_label("Reverse");
    reverse.set_active(true); // upstream's default: date modified, reversed
    controls.append(&picker);
    controls.append(&reverse);
    root.append(&controls);

    // The panes: list left, editor right.
    let paned = Paned::new(Orientation::Horizontal);
    paned.set_position(380);
    paned.set_vexpand(true);
    root.append(&paned);

    let list_store = gio::ListStore::new::<NoteNode>();
    let selection = SingleSelection::new(Some(list_store.clone()));
    selection.set_autoselect(false);
    selection.set_can_unselect(true);
    let list = ListView::new(Some(selection.clone()), Some(note_factory()));
    let scrolled = ScrolledWindow::new();
    scrolled.set_child(Some(&list));
    scrolled.set_vexpand(true);

    let stack = Stack::new();
    stack.add_named(&scrolled, Some("list"));
    let empty_label = Label::new(None);
    empty_label.set_wrap(true);
    empty_label.add_css_class("dimmed");
    empty_label.set_valign(Align::Center);
    empty_label.set_halign(Align::Center);
    stack.add_named(&empty_label, Some("empty"));
    paned.set_start_child(Some(&stack));

    let editor = TextView::new();
    editor.set_editable(false);
    editor.set_cursor_visible(false);
    editor.set_wrap_mode(WrapMode::Word);
    editor.add_css_class("monospace");
    let editor_scroll = ScrolledWindow::new();
    editor_scroll.set_child(Some(&editor));
    editor_scroll.set_vexpand(true);
    paned.set_end_child(Some(&editor_scroll));

    let state: Shared = Rc::new(RefCell::new(State {
        store,
        folder: folder.clone(),
        sort: SortOptions::default(),
        query: String::new(),
        list_store,
        selection: selection.clone(),
        stack: stack.clone(),
        editor: editor.clone(),
    }));

    // Sort picker + reverse flip -> repopulate.
    {
        let state = state.clone();
        picker.connect_selected_notify(move |picker| {
            state.borrow_mut().sort.column = match picker.selected() {
                1 => SortColumn::DateCreated,
                2 => SortColumn::Title,
                _ => SortColumn::DateModified,
            };
            populate(&state.borrow());
        });
    }
    {
        let state = state.clone();
        reverse.connect_toggled(move |button| {
            state.borrow_mut().sort.reverse = button.is_active();
            populate(&state.borrow());
        });
    }

    // The search field filters incrementally (the engine's
    // FilterState; Phase 11 layers the full field machine over this).
    {
        let state = state.clone();
        search.connect_search_changed(move |entry| {
            state.borrow_mut().query = entry.text().to_string();
            populate(&state.borrow());
        });
    }

    // Selection -> the editor pane shows the note body.
    {
        let state = state.clone();
        selection.connect_selection_changed(move |selection, _, _| {
            // A shared borrow: this handler fires synchronously from
            // populate's set_selected, which holds the shared borrow.
            let state = state.borrow();
            let position = selection.selected();
            if position == LIST_INVALID_POSITION {
                state.editor.buffer().set_text("");
                return;
            }
            let Some(item) = selection.selected_item() else {
                return;
            };
            let node = item.downcast::<NoteNode>().expect("the model is NoteNode");
            let body = state
                .store
                .find_by_path(Path::new(&node.path()))
                .map(|note| note.body.as_str().to_owned())
                .unwrap_or_default();
            state.editor.buffer().set_text(&body);
        });
    }

    // Populate once, then watch the folder: every coalesced event
    // batch is one reconcile-then-repopulate (spec row 72: the events
    // decide WHEN, the reconcile decides WHAT).
    populate(&state.borrow());
    start_watching(&state);

    window.present();
    window
}

/// The degraded window: the folder could not be opened; the empty
/// state names the path and the reason. No watch (nothing to watch).
fn build_empty(app: &Application, folder: &Path, reason: &str) -> gtk4::ApplicationWindow {
    let window = ApplicationWindow::builder()
        .application(app)
        .title(format!("Commonplace {}", commonplace_core::VERSION))
        .default_width(680)
        .default_height(320)
        .build();
    let label = Label::new(Some(&format!(
        "The notes folder could not be opened.\n\n{}\n\nConfigure it with:\n  commonplace-cli config --set notes_dir {}",
        reason,
        folder.display()
    )));
    label.set_wrap(true);
    label.set_valign(Align::Center);
    label.set_halign(Align::Center);
    label.add_css_class("dimmed");
    window.set_child(Some(&label));
    window.present();
    window
}

/// One reconcile-then-repopulate pass. External events map to this;
/// the reconcile is the truth (row 72).
fn reconcile_and_populate(state: &Shared) {
    {
        let mut borrowed = state.borrow_mut();
        let folder = borrowed.folder.clone();
        if let Err(e) = borrowed.store.reconcile(&folder) {
            eprintln!("commonplace: reconcile failed: {e}");
            return;
        }
    }
    populate(&state.borrow());
}

/// Start the process-lifetime watcher: the worker thread owns the
/// watch handle and forwards every coalesced batch to the main loop,
/// where one reconcile-then-repopulate runs (row 72).
fn start_watching(state: &Shared) {
    let (tx, rx) = async_channel::unbounded::<crate::watch::FolderMoved>();
    let folder = state.borrow().folder.clone();
    std::thread::spawn(move || {
        let result =
            crate::watch::spawn(&folder, std::time::Duration::from_millis(300), move |_| {
                let _ = tx.try_send(crate::watch::FolderMoved);
            });
        match result {
            Ok(handle) => {
                // The debouncer must outlive this thread for the watch
                // to keep running; the process owns it forever.
                std::mem::forget(handle);
            }
            Err(e) => eprintln!("commonplace: watch failed: {e}"),
        }
    });
    let state = state.clone();
    glib::spawn_future_local(async move {
        while let Ok(_moved) = rx.recv().await {
            reconcile_and_populate(&state);
        }
    });
}

// -- population -----------------------------------------------------------------

/// The LIST_INVALID_POSITION sentinel, re-exported for callers.
const LIST_INVALID_POSITION: u32 = gtk4::INVALID_LIST_POSITION;

/// Rebuild the list from the store under the current query + sort,
/// preserving the selection by path, and page the empty state.
fn populate(state: &State) {
    let notes = state.store.notes();
    let mut indexes: Vec<usize> = (0..notes.len()).collect();
    sort_visible(&mut indexes, notes, state.sort);

    // The query filters through the engine's incremental machinery,
    // fresh per populate (the resume path is the UI's Phase 11 job).
    let visible: Vec<usize> = if state.query.is_empty() {
        indexes
    } else {
        let mut filter = commonplace_core::search::FilterState::new(state.sort);
        match filter.update(&state.query, notes) {
            Ok(view) => view,
            Err(e) => {
                // Grammar failures keep the previous results showing
                // (row 59's error values, row 17's selection contract).
                eprintln!("commonplace: query error: {e}");
                return;
            }
        }
    };

    let selected_path = selected_path(&state.selection);
    state.list_store.remove_all();
    for index in visible {
        let note = &notes[index];
        let node = NoteNode::new(
            note.display_title(),
            &body_preview(note.body.as_str()),
            &note.path.display().to_string(),
        );
        state.list_store.append(&node);
    }
    restore_selection(
        &state.selection,
        &state.list_store,
        selected_path.as_deref(),
    );
    let page = if state.list_store.n_items() == 0 {
        "empty"
    } else {
        "list"
    };
    state.stack.set_visible_child_name(page);
}

fn selected_path(selection: &SingleSelection) -> Option<String> {
    if selection.selected() == LIST_INVALID_POSITION {
        return None;
    }
    selection
        .selected_item()
        .and_downcast::<NoteNode>()
        .map(|node| node.path())
}

fn restore_selection(selection: &SingleSelection, list_store: &gio::ListStore, path: Option<&str>) {
    let Some(wanted) = path else {
        return;
    };
    for position in 0..list_store.n_items() {
        let node = list_store
            .item(position)
            .and_downcast::<NoteNode>()
            .expect("the model is NoteNode");
        if node.path() == wanted {
            selection.set_selected(position);
            return;
        }
    }
    // The selected note vanished: deselect (can_unselect allows it).
    selection.set_selected(LIST_INVALID_POSITION);
}
