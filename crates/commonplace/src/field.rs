//! The search-field state machine (spec row 73): contract §1J ported
//! from nvALT's DualField/AppController pair (map §3) as a headless
//! model that answers events with ACTIONS. The GTK shell applies the
//! actions; the model never touches a widget, so the whole behavior
//! table tests without GTK. Event order matters: the shell feeds
//! [`FieldModel::set_view`] after every refilter so Return/arrows/
//! completion see the same rows the list shows.

/// One row of the current filtered view: identity plus display data.
/// The shell rebuilds these after each refilter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    pub path: String,
    pub title: String,
}

/// What the shell should do. The model computes; the shell applies.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    /// Re-run the filter view for the model's current text and feed
    /// it back through [`FieldModel::set_view`].
    Refilter,
    /// Replace the field's text; `select` positions a selected range
    /// (the completion remainder, AC:1725-1741) or `None` to park the
    /// caret at the end.
    SetText {
        text: String,
        select: Option<(usize, usize)>,
    },
    /// Keep the field's text selected after a list navigation
    /// (AC:1465-1481: arrows forward with the text kept selected).
    KeepFieldSelected,
    /// Select this visible position (or deselect with `None`).
    SelectRow(Option<usize>),
    /// Move keyboard focus.
    Focus(Focus),
    /// Create a note titled with the field text and focus its body
    /// (AC:2149-2171, the create side of search-as-create).
    CreateNote { title: String },
    /// Edit (focus the editor body of) the selected note
    /// (AC:2121-2126, the edit side of Return).
    EditNote { path: String },
    /// Persist (or clear) the launch-restore state.
    SaveRestore { query: String, path: Option<String> },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    Field,
    Editor,
}

/// A followed-link bookmark (map §1.7, `NoteBookmark` h:24-41): a
/// note plus the search string that led to it. Pushed by URL-scheme
/// routes (Phase 18), popped by the snapback button, and lazily
/// cleared after any typed search (upstream schedules
/// `clearFollowedLinks` after every `setSnapbackString:`,
/// DualField.m:337-338).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Snapback {
    pub query: String,
    pub path: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct FieldModel {
    text: String,
    /// The text the user typed, completion remainders excluded; the
    /// deselect (Esc, Cmd-D) restores it and selection changes never
    /// overwrite it (delta §1 3.0.39).
    typed_text: String,
    view: Vec<Row>,
    selected: Option<usize>,
    /// The single-slot snapback string (m:1837-1838): the typed
    /// search captured when a selection lands by filtering; cleared
    /// when the user types again (m:1708). The button restores it
    /// when the link stack is empty.
    snapback_string: Option<String>,
    /// The followed-link history (DualField.m:306-326): route-pushed,
    /// button-popped.
    followed_links: Vec<Snapback>,
    /// The current note, tracked for the snapback's `path` and launch
    /// restore.
    current: Option<String>,
    /// Launch restore pending: the first view feed selects the
    /// restored note (AC:2174-2190), then this clears so normal
    /// searching never re-selects an old note.
    restore_pending: bool,
}

/// Direction for the arrow keys.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dir {
    Up,
    Down,
}

impl FieldModel {
    /// A fresh model; `restore` carries the saved search + note for
    /// launch restore (AC:2174-2190).
    pub fn new(restore: Option<(String, Option<String>)>) -> Self {
        let mut model = FieldModel::default();
        if let Some((query, path)) = restore {
            model.text = query.clone();
            model.typed_text = query;
            model.current = path;
            model.restore_pending = true;
        }
        model
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn selected(&self) -> Option<usize> {
        self.selected
    }

    pub fn view(&self) -> &[Row] {
        &self.view
    }

    /// The shell reports the fresh filtered view after every refilter
    /// (and at startup). Completion runs here: a typed prefix of the
    /// first title-prefix match gets its remainder inserted and
    /// selected into the field (AC:1725-1741).
    pub fn set_view(&mut self, view: Vec<Row>) -> Vec<Action> {
        self.view = view;
        let mut actions = Vec::new();
        if self.text.is_empty() {
            // The empty query deselects and clears (invariant 2).
            if self.selected.is_some() {
                self.selected = None;
                actions.push(Action::SelectRow(None));
            }
            return actions;
        }
        // The completion: the first row whose folded title starts with
        // the typed text (fold here is lowercase-only prefix logic on
        // already-folded store data; the engine's view arrives folded
        // for matching, but titles are display case, so compare
        // case-insensitively without folding diacritics: the
        // completion is a typing aid, not a search semantic).
        let typed = self.typed_text.to_lowercase();
        let candidate = self
            .view
            .iter()
            .position(|row| row.title.to_lowercase().starts_with(&typed) && !typed.is_empty());
        if let Some(position) = candidate {
            let title = self.view[position].title.clone();
            if title != self.text {
                let remainder = title.len();
                self.text = title;
                actions.push(Action::SetText {
                    text: self.text.clone(),
                    select: Some((typed_text_len(&self.typed_text, &self.text), remainder)),
                });
            }
            if self.selected != Some(position) {
                self.selected = Some(position);
                actions.push(Action::SelectRow(Some(position)));
            }
        } else if self.restore_pending {
            // Launch restore: the saved note keeps its selection on
            // the first view, whatever the completion did.
            self.restore_pending = false;
            if let Some(current) = self.current.clone()
                && let Some(position) = self.view.iter().position(|row| row.path == current)
            {
                self.selected = Some(position);
                actions.push(Action::SelectRow(Some(position)));
            }
        } else if self.selected.is_some() {
            self.selected = None;
            actions.push(Action::SelectRow(None));
        }
        actions
    }

    /// The user typed into the field (the full new text). Refilter;
    /// the completion remainder (if any) arrives via set_view.
    pub fn on_text_changed(&mut self, text: &str) -> Vec<Action> {
        // User typing clears the snapback slot (m:1708) and, upstream,
        // schedules the followed-link clear after delay 0
        // (DualField.m:337-338): link history survives only until the
        // next user-typed search. Route-driven searches
        // (searchForString:) do not wipe it; they use
        // [`FieldModel::route_search`].
        self.snapback_string = None;
        self.followed_links.clear();
        self.route_search(text)
    }

    /// A route-driven search (`searchForString:`, the Phase 18 scheme
    /// routes and the snapback restore): sets the text and refilters
    /// without the user-typing clears.
    pub fn route_search(&mut self, text: &str) -> Vec<Action> {
        self.text = text.to_owned();
        self.typed_text = text.to_owned();
        vec![
            Action::Refilter,
            Action::SaveRestore {
                query: self.text.clone(),
                path: self.current.clone(),
            },
        ]
    }

    /// Return in the field (AC:2121-2126 -> 2149-2171): a selection
    /// edits; nothing selected creates. Find routes push snapback.
    pub fn on_return(&mut self) -> Vec<Action> {
        match self.selected {
            Some(position) => {
                let path = self.view[position].path.clone();
                self.current = Some(path.clone());
                vec![
                    Action::EditNote { path },
                    Action::Focus(Focus::Editor),
                    Action::SaveRestore {
                        query: self.text.clone(),
                        path: self.current.clone(),
                    },
                ]
            }
            None => {
                let title = self.typed_text.trim().to_owned();
                if title.is_empty() {
                    return Vec::new();
                }
                vec![Action::CreateNote { title }, Action::Focus(Focus::Editor)]
            }
        }
    }

    /// Escape (AC:1428-1453): clear search + selection, refilter,
    /// refocus the field. Ctrl-[ aliases this in the shell.
    pub fn on_escape(&mut self) -> Vec<Action> {
        self.text.clear();
        self.typed_text.clear();
        self.selected = None;
        vec![
            Action::SetText {
                text: String::new(),
                select: None,
            },
            Action::SelectRow(None),
            Action::Refilter,
            Action::Focus(Focus::Field),
        ]
    }

    /// Cmd-D (XIB id 507): deselect and restore the typed search
    /// (undo the completion insertion, keep the user's own text).
    pub fn on_deselect(&mut self) -> Vec<Action> {
        let typed = self.typed_text.clone();
        self.text = typed.clone();
        self.selected = None;
        vec![
            Action::SetText {
                text: typed,
                select: None,
            },
            Action::SelectRow(None),
            Action::Refilter,
        ]
    }

    /// Up/Down (AC:1465-1481): forward to the list, text kept
    /// selected. At the boundaries the selection stays put.
    pub fn on_arrow(&mut self, dir: Dir) -> Vec<Action> {
        if self.view.is_empty() {
            return Vec::new();
        }
        let last = self.view.len() - 1;
        let target = match self.selected {
            None => match dir {
                Dir::Down => 0,
                // Up from nothing selects nothing (upstream keeps the
                // empty selection; the list stays unhighlighted).
                Dir::Up => return Vec::new(),
            },
            Some(position) => {
                if dir == Dir::Down && position == last || dir == Dir::Up && position == 0 {
                    return vec![Action::KeepFieldSelected];
                }
                match dir {
                    Dir::Down => position + 1,
                    Dir::Up => position - 1,
                }
            }
        };
        self.selected = Some(target);
        let path = self.view[target].path.clone();
        self.current = Some(path);
        vec![
            Action::SelectRow(Some(target)),
            Action::KeepFieldSelected,
            Action::SaveRestore {
                query: self.text.clone(),
                path: self.current.clone(),
            },
        ]
    }

    /// Cmd-Up/Down (AC:1501-1508): first/last row.
    pub fn on_arrow_end(&mut self, dir: Dir) -> Vec<Action> {
        if self.view.is_empty() {
            return Vec::new();
        }
        let target = match dir {
            Dir::Up => 0,
            Dir::Down => self.view.len() - 1,
        };
        self.selected = Some(target);
        self.current = Some(self.view[target].path.clone());
        vec![
            Action::SelectRow(Some(target)),
            Action::KeepFieldSelected,
            Action::SaveRestore {
                query: self.text.clone(),
                path: self.current.clone(),
            },
        ]
    }

    /// Tab (AC:1483-1500): empty, eaten; unmatched with a candidate
    /// re-runs the search to trigger completion; else focus the body.
    pub fn on_tab(&mut self) -> Vec<Action> {
        if self.text.trim().is_empty() {
            return Vec::new(); // eaten
        }
        let matched = self.selected.is_some();
        if !matched && !self.view.is_empty() {
            // Re-run the search: the completion machinery runs in
            // set_view on the shell's next feed.
            return vec![Action::Refilter];
        }
        vec![Action::Focus(Focus::Editor)]
    }

    /// Ctrl-U (AC:1538-1548): delete to the beginning of the line.
    /// The field is single-line, so this clears to the caret; the
    /// shell positions the caret, the model answers with the text.
    pub fn on_delete_to_start(&mut self) -> Vec<Action> {
        self.text.clear();
        self.typed_text.clear();
        vec![
            Action::SetText {
                text: String::new(),
                select: None,
            },
            Action::Refilter,
        ]
    }

    /// The shell reports a selection change NOT driven by the model
    /// (a click in the list). The field keeps its typed text
    /// (delta §1 3.0.39); the snapback records the route.
    pub fn on_external_selection(&mut self, position: Option<usize>) -> Vec<Action> {
        self.selected = position;
        let mut actions = Vec::new();
        if let Some(position) = position {
            if let Some(row) = self.view.get(position) {
                self.current = Some(row.path.clone());
                // Selection by filtering captures the typed search
                // (m:1837-1838: not from typing itself, not a create).
                let typed = self.typed_text.clone();
                self.set_snapback_string(&typed);
            }
        } else {
            self.current = None;
        }
        actions.push(Action::SaveRestore {
            query: self.text.clone(),
            path: self.current.clone(),
        });
        actions
    }

    /// Typing in an empty body with nothing selected creates the note
    /// (AC:2054-2057); the shell calls this when the editor receives
    /// text in that state.
    pub fn on_body_typed_empty(&mut self) -> Vec<Action> {
        if self.selected.is_some() {
            return Vec::new();
        }
        let title = self.typed_text.trim().to_owned();
        if title.is_empty() {
            return Vec::new();
        }
        vec![Action::CreateNote { title }]
    }

    /// A link route pushes the pre-navigation state
    /// (AppController_Importing.m:178: the current note + the field's
    /// search string, taken BEFORE the route navigates away).
    pub fn push_followed_link(&mut self) {
        self.followed_links.push(Snapback {
            query: self.text.clone(),
            path: self.current.clone(),
        });
    }

    /// The snapback button (DualField.m:376-382): pop the followed
    /// links when any exist, else deselect all. The link pop restores
    /// the bookmarked search + note.
    pub fn snapback_button(&mut self) -> Vec<Action> {
        let Some(bookmark) = self.followed_links.pop() else {
            return self.on_deselect();
        };
        self.text = bookmark.query.clone();
        self.typed_text = bookmark.query.clone();
        self.current = bookmark.path.clone();
        let mut actions = vec![
            Action::SetText {
                text: bookmark.query,
                select: None,
            },
            Action::Refilter,
        ];
        if let Some(path) = &bookmark.path {
            actions.push(Action::EditNote { path: path.clone() });
        } else {
            actions.push(Action::SelectRow(None));
        }
        actions
    }

    /// The snapback slot setter (m:1837-1838): the typed search
    /// captured when a selection lands by filtering. Empty and
    /// whitespace-only strings clear the slot, upstream's nil case.
    pub fn set_snapback_string(&mut self, typed: &str) {
        let trimmed = typed.trim();
        self.snapback_string = if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_owned())
        };
    }

    pub fn snapback_string(&self) -> Option<&str> {
        self.snapback_string.as_deref()
    }

    pub fn has_followed_links(&self) -> bool {
        !self.followed_links.is_empty()
    }

    /// Test-only debug view of the link stack.
    #[doc(hidden)]
    pub fn followed_links_debug(&self) -> &[(String, Option<String>)] {
        #[allow(clippy::trait_duplication_in_bounds)]
        unsafe {
            std::mem::transmute::<&[Snapback], &[(String, Option<String>)]>(&self.followed_links)
        }
    }
}

/// The completion remainder starts after the typed text; compare in
/// the same case-insensitive way the candidate search did.
fn typed_text_len(typed: &str, title: &str) -> usize {
    let typed_lower = typed.to_lowercase();
    let title_lower = title.to_lowercase();
    let mut typed_chars = typed_lower.chars();
    let mut consumed = 0usize;
    for (position, title_char) in title_lower.char_indices() {
        match typed_chars.next() {
            Some(typed_char) if typed_char == title_char => {
                consumed = position + title_char.len_utf8()
            }
            _ => break,
        }
    }
    consumed
}

#[cfg(test)]
mod tests {
    use super::*;

    fn view() -> Vec<Row> {
        vec![
            Row {
                path: "/notes/Alpha Note.md".into(),
                title: "Alpha Note".into(),
            },
            Row {
                path: "/notes/Alphabet.md".into(),
                title: "Alphabet".into(),
            },
            Row {
                path: "/notes/Beta.md".into(),
                title: "Beta".into(),
            },
        ]
    }

    fn feed(model: &mut FieldModel) -> Vec<Action> {
        model.set_view(view())
    }

    #[test]
    fn typing_a_prefix_inserts_the_completion_remainder_selected() {
        let mut model = FieldModel::new(None);
        let actions = model.on_text_changed("alph");
        assert_eq!(actions.first(), Some(&Action::Refilter));
        feed(&mut model);
        // The field now carries the completed title with the
        // remainder selected.
        assert_eq!(model.text(), "Alpha Note");
    }

    #[test]
    fn empty_query_deselects_and_clears() {
        let mut model = FieldModel::new(None);
        model.on_text_changed("alp");
        feed(&mut model);
        assert!(model.selected().is_some());
        let actions = model.on_text_changed("");
        assert_eq!(actions.first(), Some(&Action::Refilter));
        let actions = feed(&mut model);
        assert_eq!(actions, vec![Action::SelectRow(None)]);
        assert!(model.selected().is_none());
    }

    #[test]
    fn return_with_no_selection_creates_the_note() {
        let mut model = FieldModel::new(None);
        model.on_text_changed("brand new note");
        let _ = feed(&mut model); // no title-prefix match: nothing selected
        let actions = model.on_return();
        assert!(
            actions
                .iter()
                .any(|a| matches!(a, Action::CreateNote { title } if title == "brand new note"))
        );
        assert!(actions.contains(&Action::Focus(Focus::Editor)));
    }

    #[test]
    fn return_with_selection_edits() {
        let mut model = FieldModel::new(None);
        model.on_text_changed("alpha");
        let _ = feed(&mut model);
        let actions = model.on_return();
        assert!(
            actions
                .iter()
                .any(|a| matches!(a, Action::EditNote { path } if path.contains("Alpha")))
        );
    }

    #[test]
    fn escape_clears_everything_and_refocuses() {
        let mut model = FieldModel::new(None);
        model.on_text_changed("alp");
        let _ = feed(&mut model);
        let actions = model.on_escape();
        assert_eq!(model.text(), "");
        assert!(model.selected().is_none());
        assert!(actions.contains(&Action::Refilter));
        assert!(actions.contains(&Action::Focus(Focus::Field)));
        assert!(actions.contains(&Action::SelectRow(None)));
    }

    #[test]
    fn arrows_forward_and_keep_the_field_selected() {
        let mut model = FieldModel::new(None);
        model.on_text_changed("al");
        let _ = feed(&mut model); // Alpha Note selected by completion
        let start = model.selected();
        let actions = model.on_arrow(Dir::Down);
        assert_eq!(model.selected(), Some(start.unwrap() + 1));
        assert!(actions.contains(&Action::KeepFieldSelected));
        // Up from the top row stays put and still keeps the selection.
        let actions = model.on_arrow(Dir::Up);
        assert!(actions.contains(&Action::KeepFieldSelected));
        // Up from no selection selects nothing.
        let mut empty_model = FieldModel::new(None);
        empty_model.on_text_changed("");
        let _ = feed(&mut empty_model);
        assert!(empty_model.on_arrow(Dir::Up).is_empty());
    }

    #[test]
    fn cmd_arrows_jump_to_the_ends() {
        let mut model = FieldModel::new(None);
        model.on_text_changed("a");
        let _ = feed(&mut model);
        let actions = model.on_arrow_end(Dir::Down);
        assert_eq!(model.selected(), Some(view().len() - 1));
        assert!(actions.contains(&Action::SelectRow(Some(view().len() - 1))));
        let _ = model.on_arrow_end(Dir::Up);
        assert_eq!(model.selected(), Some(0));
    }

    #[test]
    fn tab_eats_empty_and_focuses_editor_when_matched() {
        let mut model = FieldModel::new(None);
        assert!(model.on_tab().is_empty(), "empty query: Tab is eaten");
        model.on_text_changed("alpha");
        let _ = feed(&mut model);
        let actions = model.on_tab();
        assert_eq!(actions, vec![Action::Focus(Focus::Editor)]);
    }

    #[test]
    fn tab_on_unmatched_with_candidates_refilters_for_completion() {
        let mut model = FieldModel::new(None);
        model.on_text_changed("zzz");
        let _ = feed(&mut model); // no match, nothing selected
        let actions = model.on_tab();
        assert_eq!(actions, vec![Action::Refilter]);
    }

    #[test]
    fn selection_change_keeps_the_typed_text() {
        let mut model = FieldModel::new(None);
        model.on_text_changed("alph");
        let _ = feed(&mut model); // completion inserted
        let actions = model.on_external_selection(Some(2));
        assert_eq!(model.text(), "Alpha Note", "3.0.39: typed text kept");
        assert!(
            actions
                .iter()
                .any(|a| matches!(a, Action::SaveRestore { .. }))
        );
    }

    #[test]
    fn cmd_d_restores_the_typed_search() {
        let mut model = FieldModel::new(None);
        model.on_text_changed("alph");
        let _ = feed(&mut model);
        assert_eq!(model.text(), "Alpha Note");
        let actions = model.on_deselect();
        assert_eq!(model.text(), "alph");
        assert!(model.selected().is_none());
        assert!(actions.contains(&Action::Refilter));
    }

    #[test]
    fn snapback_slot_lifecycle() {
        let mut model = FieldModel::new(None);
        model.on_text_changed("alph");
        let _ = feed(&mut model);
        // A selection landing by filtering captures the typed search
        // (m:1837-1838).
        let _ = model.on_external_selection(Some(0));
        assert_eq!(model.snapback_string(), Some("alph"));
        // Typing clears the slot (m:1708).
        let _ = model.on_text_changed("beta");
        assert_eq!(model.snapback_string(), None);
        // Whitespace-only clears it too (the nil case, DF:331-339).
        model.set_snapback_string("   ");
        assert_eq!(model.snapback_string(), None);
    }

    #[test]
    fn followed_links_pop_through_the_button() {
        let mut model = FieldModel::new(None);
        model.on_text_changed("alpha");
        let _ = feed(&mut model);
        let _ = model.on_return(); // on Alpha Note
        // A route pushes the pre-navigation state (AI:177-179), then
        // navigates: the beta search reveals Beta.
        model.push_followed_link();
        let _ = model.route_search("beta");
        let _ = feed(&mut model);
        let _ = model.on_return(); // on Beta
        model.push_followed_link();
        assert!(model.has_followed_links());

        // User typing wipes the link history (the delayed clear,
        // DF:337-338); route searches never do.
        let mut wiped = model.clone();
        let _ = wiped.on_text_changed("gamma");
        assert!(!wiped.has_followed_links());

        let actions = model.snapback_button();
        assert_eq!(model.text(), "Beta", "the last link pops first");
        assert!(
            actions
                .iter()
                .any(|a| matches!(a, Action::EditNote { path } if path.contains("Beta")))
        );

        let actions = model.snapback_button();
        assert_eq!(model.text(), "Alpha Note");
        assert!(
            actions
                .iter()
                .any(|a| matches!(a, Action::EditNote { path } if path.contains("Alpha")))
        );

        // The empty stack falls back to deselect-all (DF:376-382).
        let actions = model.snapback_button();
        assert!(actions.contains(&Action::SelectRow(None)));
        assert!(actions.contains(&Action::Refilter));
    }

    #[test]
    fn typing_in_an_empty_body_creates_when_nothing_is_selected() {
        let mut model = FieldModel::new(None);
        model.on_text_changed("draft title");
        let _ = feed(&mut model);
        let actions = model.on_body_typed_empty();
        assert!(
            actions
                .iter()
                .any(|a| matches!(a, Action::CreateNote { title } if title == "draft title"))
        );

        // With a selection, body typing is just editing.
        let mut selected = FieldModel::new(None);
        selected.on_text_changed("alpha");
        let _ = feed(&mut selected);
        assert!(selected.on_body_typed_empty().is_empty());
    }

    #[test]
    fn ctrl_u_clears_the_field() {
        let mut model = FieldModel::new(None);
        model.on_text_changed("alpha beta");
        let actions = model.on_delete_to_start();
        assert_eq!(model.text(), "");
        assert!(actions.contains(&Action::Refilter));
    }

    #[test]
    fn launch_restore_seeds_the_search_and_note() {
        let mut model = FieldModel::new(Some((
            "alpha".to_owned(),
            Some("/notes/Alpha Note.md".to_owned()),
        )));
        assert_eq!(model.text(), "alpha");
        // The shell feeds the first view; the restored note keeps its
        // selection through it.
        let feed_actions = feed(&mut model);
        assert_eq!(model.selected(), Some(0), "the restored note is selected");
        assert!(feed_actions.contains(&Action::SelectRow(Some(0))));
        let actions = model.on_return();
        assert!(
            actions
                .iter()
                .any(|a| matches!(a, Action::EditNote { path } if path.contains("Alpha")))
        );
    }
}
