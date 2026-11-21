//! The two-pane NV shell (spec rows 71-73): the search field above the
//! results list, the editor right, the sort picker between them, the
//! folder watched live. The field runs the headless state machine
//! ([`crate::field::FieldModel`], contract §1J) and this shell applies
//! its actions; the list runs on the house shape (house §Q3); the
//! editor is a non-editing display until Phase 12 wires autosave and
//! undo (the create-on-empty-body behavior becomes reachable there).

use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use gtk4::gdk;
use gtk4::gdk::Key;
use gtk4::gio;
use gtk4::glib;
use gtk4::prelude::*;
use gtk4::*;

use commonplace_core::config::Config;
use commonplace_core::search::{FilterState, SortColumn, SortOptions, sort_visible};
use commonplace_core::store::{Store, load_folder};

use crate::field::{Action, Dir, FieldModel, Focus, Row};
use crate::lists::{NoteNode, body_preview, note_factory};

const APP_ID: &str = "io.github.virinvictus.Commonplace";

pub fn app_id() -> &'static str {
    APP_ID
}

/// The restore-save debounce (the config file is written at most this
/// often from launch-restore updates).
const RESTORE_SAVE_DEBOUNCE_MS: u64 = 500;

/// The window state, main-thread only (everything lives behind the
/// Rc the GTK callbacks share).
struct State {
    store: Store,
    folder: PathBuf,
    config: Config,
    config_path: PathBuf,
    sort: SortOptions,
    field: FieldModel,
    /// Guard: the shell is applying a model-driven SetText, so the
    /// entry's changed signal must not feed back into the model.
    applying_text: bool,
    /// Guard: a model-driven SelectRow is in flight, so the selection
    /// handler must not treat it as an external click.
    applying_selection: bool,
    pending_restore: Option<(String, Option<String>)>,
    restore_save_source: Option<glib::SourceId>,
    list_store: gio::ListStore,
    selection: SingleSelection,
    stack: Stack,
    editor: TextView,
}

type Shared = Rc<RefCell<State>>;

/// Build the window and every behavior inside it. Launch restore
/// (spec row 73): the saved search seeds the field and the saved note
/// keeps its selection through the first populate.
pub fn build(app: &Application, folder: PathBuf) -> gtk4::ApplicationWindow {
    let (store, config, config_path) = match load_folder(&folder) {
        Ok(outcome) => {
            let config_path = Config::xdg_path();
            let config = Config::load(&config_path).unwrap_or_default();
            (outcome.store, config, config_path)
        }
        Err(e) => {
            // A missing or unreadable folder pages the empty state; the
            // app stays alive so the user can configure notes_dir.
            return build_empty(app, &folder, &format!("{e}"));
        }
    };

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
    capture_search_entry(&search);
    root.append(&search);

    // The controls row: the column sort picker and its direction.
    let controls = Box::new(Orientation::Horizontal, 6);
    let sort_model = gtk4::StringList::new(&["Date Modified", "Date Created", "Title"]);
    let picker = DropDown::builder().model(&sort_model).build();
    picker.set_selected(0); // upstream's default: date modified
    let reverse = ToggleButton::with_label("Reverse");
    reverse.set_active(true); // upstream's default: date modified, reversed
    // Binary state, not a suggested action: vir-gtk 1.5.0's quiet checked
    // default renders it so, and the app-tier shim filed for it is gone.
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

    // Launch restore: the saved search seeds the field; the saved
    // note resolves on the first populate.
    let restore = Some((
        config.last_query.clone().unwrap_or_default(),
        config.last_note.clone(),
    ));
    let field = FieldModel::new(restore);
    if !field.text().is_empty() {
        search.set_text(field.text());
    }

    let state: Shared = Rc::new(RefCell::new(State {
        store,
        folder: folder.clone(),
        config,
        config_path,
        sort: SortOptions::default(),
        field,
        applying_text: false,
        applying_selection: false,
        pending_restore: None,
        restore_save_source: None,
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
            populate(&state);
        });
    }
    {
        let state = state.clone();
        reverse.connect_toggled(move |button| {
            state.borrow_mut().sort.reverse = button.is_active();
            populate(&state);
        });
    }

    // The search field feeds the model; the model's actions come back
    // through apply_actions (the completion SetText arrives inside the
    // applying guard, so the changed signal never re-enters).
    {
        let state = state.clone();
        search.connect_search_changed(move |entry| {
            let skip = state.borrow().applying_text;
            if skip {
                return;
            }
            let text = entry.text().to_string();
            let actions = state.borrow_mut().field.on_text_changed(&text);
            apply_actions(&state, actions);
        });
    }

    // The field's keys (contract §1J): Escape, arrows, Cmd-arrows
    // (mac Cmd -> Ctrl), Tab, Ctrl-U, Ctrl-[ (the Escape alias).
    {
        let state = state.clone();
        let controller = EventControllerKey::new();
        controller.set_propagation_phase(PropagationPhase::Capture);
        controller.connect_key_pressed(move |_, key, _code, modifier| {
            let shift = modifier.contains(gdk::ModifierType::SHIFT_MASK);
            let ctrl = modifier.contains(gdk::ModifierType::CONTROL_MASK);
            let handled = match (key, ctrl) {
                (Key::Escape, _) => {
                    let actions = state.borrow_mut().field.on_escape();
                    apply_actions(&state, actions);
                    true
                }
                (Key::bracketleft, true) => {
                    let actions = state.borrow_mut().field.on_escape();
                    apply_actions(&state, actions);
                    true
                }
                (Key::Up, false) => {
                    let actions = state.borrow_mut().field.on_arrow(Dir::Up);
                    apply_actions(&state, actions);
                    true
                }
                (Key::Down, false) => {
                    let actions = state.borrow_mut().field.on_arrow(Dir::Down);
                    apply_actions(&state, actions);
                    true
                }
                (Key::Up, true) => {
                    let actions = state.borrow_mut().field.on_arrow_end(Dir::Up);
                    apply_actions(&state, actions);
                    true
                }
                (Key::Down, true) => {
                    let actions = state.borrow_mut().field.on_arrow_end(Dir::Down);
                    apply_actions(&state, actions);
                    true
                }
                (Key::Tab, _) if !shift => {
                    let actions = state.borrow_mut().field.on_tab();
                    apply_actions(&state, actions);
                    true
                }
                (Key::u, true) => {
                    let actions = state.borrow_mut().field.on_delete_to_start();
                    apply_actions(&state, actions);
                    true
                }
                _ => false,
            };
            if handled {
                glib::signal::Propagation::Stop
            } else {
                glib::signal::Propagation::Proceed
            }
        });
        search.add_controller(controller);
    }

    // Selection -> the editor pane shows the note body; external
    // clicks reach the model (kept-typed-text + snapback slot);
    // model-driven selections pass through under the guard.
    {
        let state = state.clone();
        selection.connect_selection_changed(move |selection, _, _| {
            let position = selection.selected();
            let was_applying = state.borrow().applying_selection;
            if position == LIST_INVALID_POSITION {
                state.borrow().editor.buffer().set_text("");
                if !was_applying {
                    let actions = state.borrow_mut().field.on_external_selection(None);
                    apply_actions(&state, actions);
                }
                return;
            }
            let Some(item) = selection.selected_item() else {
                return;
            };
            let node = item.downcast::<NoteNode>().expect("the model is NoteNode");
            let path = node.path();
            {
                let borrowed = state.borrow();
                let body = borrowed
                    .store
                    .find_by_path(Path::new(&path))
                    .map(|note| note.body.as_str().to_owned())
                    .unwrap_or_default();
                borrowed.editor.buffer().set_text(&body);
            }
            if !was_applying {
                let model_position = visible_position_of(&list_store_from(&state), &path);
                let actions = state
                    .borrow_mut()
                    .field
                    .on_external_selection(model_position);
                apply_actions(&state, actions);
            }
        });
    }

    // Typing while the list is focused forwards to the field
    // (NTV:976-984; crosswalk §2: GTK has no event-forwarding
    // primitive, so the capture controller synthesizes the append).
    {
        let state = state.clone();
        let controller = EventControllerKey::new();
        controller.set_propagation_phase(PropagationPhase::Capture);
        controller.connect_key_pressed(move |_, key, _, modifier| {
            let ctrl = modifier.contains(gdk::ModifierType::CONTROL_MASK);
            if ctrl {
                return glib::signal::Propagation::Proceed;
            }
            let Some(ch) = key.to_unicode() else {
                return glib::signal::Propagation::Proceed;
            };
            if ch.is_control() {
                return glib::signal::Propagation::Proceed;
            }
            let mut borrowed = state.borrow_mut();
            let mut text = borrowed.field.text().to_owned();
            text.push(ch);
            let actions = borrowed.field.on_text_changed(&text);
            drop(borrowed);
            // The entry widget shows the appended char (the keystroke
            // originated in the list); the applying guard keeps the
            // changed signal quiet.
            if let Some(entry) = SEARCH_ENTRY.with(|slot| slot.borrow().clone()) {
                state.borrow_mut().applying_text = true;
                entry.set_text(&text);
                entry.select_region(text.len() as i32, text.len() as i32);
                state.borrow_mut().applying_text = false;
            }
            apply_actions(&state, actions);
            true.into()
        });
        list.add_controller(controller);
    }

    // Populate once (this resolves the launch-restore selection), then
    // watch the folder: every coalesced event batch is one
    // reconcile-then-repopulate (spec row 72: the events decide WHEN,
    // the reconcile decides WHAT).
    populate(&state);
    start_watching(&state);

    // Flush the debounced restore save when the window closes.
    {
        let state = state.clone();
        window.connect_close_request(move |_| {
            flush_restore(&state);
            glib::Propagation::Proceed
        });
    }

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
    populate(state);
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

// -- the model boundary ---------------------------------------------------------

/// Apply the model's actions. Refilter loops back through populate
/// (which feeds set_view; the model never answers with a Refilter
/// from there, so this terminates).
fn apply_actions(state: &Shared, actions: Vec<Action>) {
    for action in actions {
        match action {
            Action::Refilter => populate(state),
            Action::SetText { text, select } => {
                let entry = SEARCH_ENTRY.with(|slot| slot.borrow().clone());
                let Some(entry) = entry else {
                    continue;
                };
                state.borrow_mut().applying_text = true;
                entry.set_text(&text);
                match select {
                    Some((start, end)) => entry.select_region(start as i32, end as i32),
                    None => entry.select_region(text.len() as i32, text.len() as i32),
                }
                state.borrow_mut().applying_text = false;
            }
            Action::KeepFieldSelected => {
                let entry = SEARCH_ENTRY.with(|slot| slot.borrow().clone());
                if let Some(entry) = entry {
                    // The field's text stays selected after list
                    // navigation (AC:1465-1481), so typing replaces it.
                    entry.select_region(0, -1);
                }
            }
            Action::SelectRow(position) => {
                state.borrow_mut().applying_selection = true;
                let selection = state.borrow().selection.clone();
                selection.set_selected(match position {
                    Some(p) => p as u32,
                    None => gtk4::INVALID_LIST_POSITION,
                });
                state.borrow_mut().applying_selection = false;
            }
            Action::Focus(target) => {
                let borrowed = state.borrow();
                if target == Focus::Field {
                    let entry = SEARCH_ENTRY.with(|slot| slot.borrow().clone());
                    if let Some(entry) = entry {
                        entry.grab_focus();
                    }
                } else {
                    borrowed.editor.grab_focus();
                }
            }
            Action::CreateNote { title } => {
                let path = {
                    let mut borrowed = state.borrow_mut();
                    match borrowed.store.create_note(&title, "") {
                        Ok(note) => note.path.clone(),
                        Err(e) => {
                            eprintln!("commonplace: create failed: {e}");
                            continue;
                        }
                    }
                };
                populate(state);
                // The new note becomes the selection; focus its body.
                select_by_path(state, &path.display().to_string());
                state.borrow().editor.grab_focus();
            }
            Action::EditNote { path } => {
                select_by_path(state, &path);
                let editor = { state.borrow().editor.clone() };
                editor.grab_focus();
            }
            Action::SaveRestore { query, path } => {
                schedule_restore_save(state, query, path);
            }
        }
    }
}

thread_local! {
    /// The search entry handle, captured at build time so actions can
    /// drive it without threading it through every closure.
    static SEARCH_ENTRY: RefCell<Option<SearchEntry>> = const { RefCell::new(None) };
}

/// Capture the entry handle the actions drive.
fn capture_search_entry(entry: &SearchEntry) {
    SEARCH_ENTRY.with(|slot| *slot.borrow_mut() = Some(entry.clone()));
}

fn select_by_path(state: &Shared, wanted: &str) {
    let borrowed = state.borrow();
    let list_store = borrowed.list_store.clone();
    let selection = borrowed.selection.clone();
    drop(borrowed);
    for position in 0..list_store.n_items() {
        let node = list_store
            .item(position)
            .and_downcast::<NoteNode>()
            .expect("the model is NoteNode");
        if node.path() == wanted {
            state.borrow_mut().applying_selection = true;
            selection.set_selected(position);
            state.borrow_mut().applying_selection = false;
            return;
        }
    }
}

fn list_store_from(state: &Shared) -> gio::ListStore {
    state.borrow().list_store.clone()
}

fn visible_position_of(list_store: &gio::ListStore, path: &str) -> Option<usize> {
    (0..list_store.n_items())
        .find(|position| {
            list_store
                .item(*position)
                .and_downcast::<NoteNode>()
                .is_some_and(|node| node.path() == path)
        })
        .map(|p| p as usize)
}

/// Debounced launch-restore persistence (spec row 73): the config
/// file is rewritten at most once per debounce window, and once more
/// on window close.
fn schedule_restore_save(state: &Shared, query: String, path: Option<String>) {
    let mut borrowed = state.borrow_mut();
    borrowed.pending_restore = Some((query, path));
    if borrowed.restore_save_source.is_some() {
        return;
    }
    let state = state.clone();
    borrowed.restore_save_source = Some(glib::timeout_add_local(
        std::time::Duration::from_millis(RESTORE_SAVE_DEBOUNCE_MS),
        move || {
            flush_restore(&state);
            state.borrow_mut().restore_save_source = None;
            glib::ControlFlow::Break
        },
    ));
}

fn flush_restore(state: &Shared) {
    let Some((query, path)) = state.borrow_mut().pending_restore.take() else {
        return;
    };
    let mut borrowed = state.borrow_mut();
    borrowed.config.last_query = Some(query);
    borrowed.config.last_note = path;
    let path = borrowed.config_path.clone();
    let config = borrowed.config.clone();
    drop(borrowed);
    if let Err(e) = config.save(&path) {
        eprintln!("commonplace: restore save failed: {e}");
    }
}

// -- population -----------------------------------------------------------------

/// The LIST_INVALID_POSITION sentinel, re-exported for callers.
const LIST_INVALID_POSITION: u32 = gtk4::INVALID_LIST_POSITION;

/// Rebuild the list from the store under the field's current query +
/// sort, preserving the selection by path, page the empty state, and
/// feed the fresh view back to the field model (whose completion
/// actions apply before this returns).
fn populate(state: &Shared) {
    let (view_rows, selected_path) = {
        let borrowed = state.borrow();
        let notes = borrowed.store.notes();
        let mut indexes: Vec<usize> = (0..notes.len()).collect();
        sort_visible(&mut indexes, notes, borrowed.sort);

        let query = borrowed.field.text().to_owned();
        let visible: Vec<usize> = if query.is_empty() {
            indexes
        } else {
            let mut filter = FilterState::new(borrowed.sort);
            match filter.update(&query, notes) {
                Ok(view) => view,
                Err(e) => {
                    // Grammar failures keep the previous results
                    // showing (row 59's error values, row 17's
                    // selection contract).
                    eprintln!("commonplace: query error: {e}");
                    return;
                }
            }
        };

        let selected_path = selected_path(&borrowed.selection);
        let rows: Vec<Row> = visible
            .iter()
            .map(|&index| {
                let note = &notes[index];
                Row {
                    path: note.path.display().to_string(),
                    title: note.display_title().to_owned(),
                }
            })
            .collect();
        (rows, selected_path)
    };

    let selected_position = {
        let borrowed = state.borrow_mut();
        borrowed.list_store.remove_all();
        let mut selected_position = None;
        for row in &view_rows {
            let node = NoteNode::new(
                &row.title,
                &body_preview(
                    &borrowed
                        .store
                        .find_by_path(Path::new(&row.path))
                        .map(|note| note.body.as_str().to_owned())
                        .unwrap_or_default(),
                ),
                &row.path,
            );
            if Some(row.path.as_str()) == selected_path.as_deref() {
                selected_position = Some(borrowed.list_store.n_items());
            }
            borrowed.list_store.append(&node);
        }
        let page = if borrowed.list_store.n_items() == 0 {
            "empty"
        } else {
            "list"
        };
        borrowed.stack.set_visible_child_name(page);
        selected_position
    };

    // Preserve the selection across the rebuild by position. The
    // borrow drops BEFORE set_selected: the signal fires synchronously
    // and its handler borrows mutably.
    let selection = state.borrow().selection.clone();
    selection.set_selected(match selected_position {
        Some(position) => position,
        None => LIST_INVALID_POSITION,
    });

    // The model sees the fresh view (and answers with completion
    // actions); the applying guard keeps its SetText out of the
    // changed signal.
    let actions = state.borrow_mut().field.set_view(view_rows);
    apply_actions(state, actions);
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
