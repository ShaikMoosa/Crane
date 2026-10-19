pub use crane_domain::{extract_tags, Note, Store, MAX_RENDERABLE_TIMESTAMP, SCHEMA_VERSION};
#[cfg(test)]
pub use crane_domain::{MAX_BODY_BYTES, MAX_TITLE_BYTES};

pub fn validate_note(note: &Note) -> Result<(), String> {
    crane_domain::validate_note(note).map_err(|error| error.to_string())
}

pub fn validate_store(store: &Store) -> Result<(), String> {
    crane_domain::validate_store(store).map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_unique_nested_tags() {
        assert_eq!(
            extract_tags("#Design and #work/Notes, then #design"),
            vec!["design", "work/notes"]
        );
    }

    #[test]
    fn validates_note_size_limits() {
        let mut note = Store::seed(100).notes.remove(0);
        note.title = "x".repeat(MAX_TITLE_BYTES + 1);
        assert!(validate_note(&note).is_err());
    }

    #[test]
    fn validate_store_rejects_wrong_schema() {
        let mut store = Store::seed(100);
        store.schema_version = SCHEMA_VERSION + 1;

        let error = validate_store(&store).expect_err("wrong schemas must fail");
        assert!(error.contains("Unsupported store version"));
    }

    #[test]
    fn validate_store_rejects_duplicate_ids() {
        let mut store = Store::seed(100);
        store.notes[1].id = store.notes[0].id.clone();

        let error = validate_store(&store).expect_err("duplicate IDs must fail");
        assert!(error.contains("duplicate note id"));
    }

    #[test]
    fn validate_store_rejects_timestamps_javascript_cannot_render() {
        let mut store = Store::seed(100);
        store.notes[0].updated_at = 8_640_000_000_000_001;

        let error = validate_store(&store).expect_err("out-of-range dates must fail");
        assert!(error.contains("timestamp"));
    }
}
