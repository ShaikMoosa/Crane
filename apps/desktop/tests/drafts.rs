use crane_application::{CommandData, CommandResult, ErrorCode};
use crane_desktop::draft::{Draft, DraftCoordinator, SAVE_DELAY_MILLIS};

fn draft(id: &str, body: &str, revision: u64, acknowledged_revision: u64) -> Draft {
    Draft {
        note_id: id.into(),
        title: format!("Title {id}"),
        body: body.into(),
        revision,
        acknowledged_revision,
    }
}

#[test]
fn older_ack_never_replaces_newer_typing() {
    let mut draft = Draft {
        note_id: "a".into(),
        title: "Title".into(),
        body: "older".into(),
        revision: 7,
        acknowledged_revision: 6,
    };

    draft.edit("Title".into(), "newer".into());
    draft.acknowledge(7);

    assert_eq!(draft.revision, 8);
    assert_eq!(draft.body, "newer");
    assert_eq!(draft.acknowledged_revision, 7);
    assert!(draft.is_dirty());
}

#[test]
fn switching_notes_waits_for_the_latest_draft() {
    let mut editor = DraftCoordinator::new(draft("a", "saved", 6, 6));
    editor.edit("Title a".into(), "revision seven".into(), 10);
    assert!(!editor.request_switch(draft("b", "other", 1, 1)));

    let request = editor.take_ready_request(10).expect("switch queues a save");
    let request_id = request.request_id;
    let saved_revision = request.draft_revision().expect("save request");
    assert_eq!(saved_revision, 7);
    editor.complete_success(request_id, saved_revision);
    assert_eq!(editor.current().note_id, "b");
}

#[test]
fn edit_during_a_pending_switch_requires_a_second_save() {
    let mut editor = DraftCoordinator::new(draft("a", "saved", 6, 6));
    editor.edit("Title a".into(), "revision seven".into(), 10);
    assert!(!editor.request_switch(draft("b", "other", 1, 1)));

    let first = editor.take_ready_request(10).expect("first save");
    editor.edit("Title a".into(), "revision eight".into(), 11);
    editor.complete_success(first.request_id, 7);

    assert_eq!(editor.current().note_id, "a");
    assert_eq!(editor.current().body, "revision eight");
    let second = editor.take_ready_request(11).expect("latest save");
    assert_eq!(second.draft_revision(), Some(8));
    editor.complete_success(second.request_id, 8);
    assert_eq!(editor.current().note_id, "b");
}

#[test]
fn retry_retains_the_latest_content() {
    let mut editor = DraftCoordinator::new(draft("a", "saved", 2, 2));
    editor.edit("Title a".into(), "keep this".into(), 100);
    let request = editor
        .take_ready_request(100 + SAVE_DELAY_MILLIS)
        .expect("debounced save");
    editor.complete_failure(
        request.request_id,
        crane_application::AppError::new(ErrorCode::Io, "disk full"),
    );

    assert_eq!(editor.current().body, "keep this");
    assert!(editor.current().is_dirty());
    assert_eq!(
        editor.last_error().map(|error| error.message.as_str()),
        Some("disk full")
    );

    assert!(editor.retry());
    let retry = editor.take_ready_request(700).expect("retry request");
    assert_eq!(retry.draft_revision(), Some(3));
    assert_eq!(retry.save_body(), Some("keep this"));
}

#[test]
fn shutdown_waits_and_refuses_after_a_failed_save() {
    let mut editor = DraftCoordinator::new(draft("a", "saved", 2, 2));
    editor.edit("Title a".into(), "must survive".into(), 1);
    assert!(!editor.request_close());
    assert!(!editor.can_close());

    let request = editor.take_ready_request(1).expect("close flush");
    editor.complete_failure(
        request.request_id,
        crane_application::AppError::new(ErrorCode::Io, "write failed"),
    );
    assert!(!editor.can_close());
    assert_eq!(editor.current().body, "must survive");

    assert!(editor.retry());
    let retry = editor.take_ready_request(2).expect("retry save");
    editor.complete(WorkerResultForTest::saved(retry.request_id, 3));
    assert!(editor.can_close());
}

struct WorkerResultForTest;

impl WorkerResultForTest {
    fn saved(request_id: u64, draft_revision: u64) -> crane_desktop::worker::WorkerResult {
        crane_desktop::worker::WorkerResult {
            request_id,
            result: Ok(CommandResult {
                revision: 1,
                data: CommandData::Saved {
                    note: crane_domain::Note {
                        id: "a".into(),
                        title: "stale committed title".into(),
                        body: "stale committed body".into(),
                        tags: Vec::new(),
                        created_at: 0,
                        updated_at: 1,
                        pinned: false,
                        archived: false,
                        trashed: false,
                        extensions: Default::default(),
                    },
                    draft_revision,
                },
            }),
        }
    }
}

#[test]
fn autosave_waits_for_550_milliseconds_and_coalesces_typing() {
    let mut editor = DraftCoordinator::new(draft("a", "saved", 0, 0));
    editor.edit("Title a".into(), "one".into(), 100);
    editor.edit("Title a".into(), "two".into(), 200);

    assert!(editor.take_ready_request(749).is_none());
    let request = editor.take_ready_request(750).expect("one coalesced save");
    assert_eq!(request.draft_revision(), Some(2));
    assert_eq!(request.save_body(), Some("two"));
}

#[test]
fn queue_saturation_coalesces_the_unsent_save_to_latest_typing() {
    let mut editor = DraftCoordinator::new(draft("a", "saved", 0, 0));
    editor.edit("Title a".into(), "first".into(), 10);
    assert!(!editor.request_close());
    let saturated = editor.take_ready_request(10).expect("queued save");
    let stable_request_id = saturated.request_id;
    editor.return_unsent(saturated);

    editor.edit("Title a".into(), "latest".into(), 11);
    let coalesced = editor.take_ready_request(11).expect("retained save");
    assert_eq!(coalesced.request_id, stable_request_id);
    assert_eq!(coalesced.draft_revision(), Some(2));
    assert_eq!(coalesced.save_body(), Some("latest"));
}
