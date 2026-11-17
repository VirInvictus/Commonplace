//! The note record. Its real semantics (first-line title rules, filename
//! mapping, encodings) land with the storage phases; the skeleton pins
//! the shape so the app and CLI crates have something to hold.

/// A single note, backed by one plain-text file on disk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Note {
    pub title: String,
    pub body: String,
}

#[cfg(test)]
mod tests {
    use super::Note;

    #[test]
    fn note_holds_fields() {
        let n = Note {
            title: "Ideas".to_string(),
            body: "search becomes creation".to_string(),
        };
        assert_eq!(n.title, "Ideas");
        assert_eq!(n.body, "search becomes creation");
    }
}
