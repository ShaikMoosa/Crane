use crate::worker::WorkerRequest;
use crane_application::{AppError, ErrorCode};
use std::collections::{HashMap, VecDeque};

pub(crate) enum PendingQuery {
    NoteIds(
        (
            u64,
            crane_application::knowledge::CollectionView,
            Option<String>,
            String,
        ),
    ),
    TagTree,
    Counts,
    QuickOpen(u64),
    CompleteWikiLink(u64),
    ResolveWikiLink(u64),
    KnowledgeFor {
        note_id: String,
        revision: u64,
    },
}

fn same_interactive_query_lane(left: &PendingQuery, right: &PendingQuery) -> bool {
    matches!(
        (left, right),
        (PendingQuery::QuickOpen(_), PendingQuery::QuickOpen(_))
            | (
                PendingQuery::CompleteWikiLink(_),
                PendingQuery::CompleteWikiLink(_)
            )
            | (
                PendingQuery::ResolveWikiLink(_),
                PendingQuery::ResolveWikiLink(_)
            )
    )
}

pub(crate) fn enqueue_deferred_query(
    queue: &mut VecDeque<WorkerRequest>,
    pending: &mut HashMap<u64, PendingQuery>,
    request: WorkerRequest,
    pending_query: PendingQuery,
) {
    let superseded: Vec<u64> = queue
        .iter()
        .filter_map(|queued| {
            pending
                .get(&queued.request_id)
                .filter(|queued_pending| {
                    same_interactive_query_lane(queued_pending, &pending_query)
                })
                .map(|_| queued.request_id)
        })
        .collect();
    if !superseded.is_empty() {
        queue.retain(|queued| !superseded.contains(&queued.request_id));
        for request_id in superseded {
            pending.remove(&request_id);
        }
    }
    pending.insert(request.request_id, pending_query);
    queue.push_back(request);
}

pub(crate) fn flush_deferred_queries(
    queue: &mut VecDeque<WorkerRequest>,
    try_send: &mut dyn FnMut(WorkerRequest) -> Result<(), AppError>,
) -> Vec<(u64, AppError)> {
    let mut failures = Vec::new();
    while let Some(request) = queue.front().cloned() {
        match try_send(request.clone()) {
            Ok(()) => {
                queue.pop_front();
            }
            Err(error) if error.code == ErrorCode::QueueFull => break,
            Err(error) => {
                queue.pop_front();
                failures.push((request.request_id, error));
            }
        }
    }
    failures
}

#[cfg(test)]
mod tests {
    use super::*;
    use crane_application::knowledge::Query;

    fn quick_open_request(request_id: u64, query: &str) -> WorkerRequest {
        WorkerRequest::query(
            request_id,
            Query::QuickOpen {
                query: query.to_owned(),
                include_archived: false,
            },
        )
    }

    #[test]
    fn deferred_interactive_queries_keep_only_the_latest_request() {
        let mut queue = VecDeque::new();
        let mut pending = HashMap::new();

        enqueue_deferred_query(
            &mut queue,
            &mut pending,
            quick_open_request(1, "old"),
            PendingQuery::QuickOpen(1),
        );
        enqueue_deferred_query(
            &mut queue,
            &mut pending,
            quick_open_request(2, "new"),
            PendingQuery::QuickOpen(2),
        );

        assert_eq!(queue.len(), 1);
        assert_eq!(queue.front().map(|request| request.request_id), Some(2));
        assert!(!pending.contains_key(&1));
        assert!(pending.contains_key(&2));
    }

    #[test]
    fn queue_full_keeps_query_for_a_later_retry() {
        let mut queue = VecDeque::from([quick_open_request(7, "later")]);
        let mut attempts = 0;

        let failures = flush_deferred_queries(&mut queue, &mut |_request| {
            attempts += 1;
            Err(AppError::new(ErrorCode::QueueFull, "full"))
        });

        assert_eq!(attempts, 1);
        assert!(failures.is_empty());
        assert_eq!(queue.front().map(|request| request.request_id), Some(7));

        let failures = flush_deferred_queries(&mut queue, &mut |_request| Ok(()));
        assert!(failures.is_empty());
        assert!(queue.is_empty());
    }
}
