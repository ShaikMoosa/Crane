use crane_application::{AppError, Command, CommandResult, ErrorCode, Library};
use std::sync::mpsc::{self, Receiver, SyncSender, TrySendError};
use std::thread::{self, JoinHandle};

const CHANNEL_CAPACITY: usize = 8;

#[derive(Clone, Debug)]
pub struct WorkerRequest {
    pub request_id: u64,
    pub command: Command,
}

impl WorkerRequest {
    pub fn draft_revision(&self) -> Option<u64> {
        match &self.command {
            Command::SaveNote { draft_revision, .. } => Some(*draft_revision),
            _ => None,
        }
    }

    pub fn save_body(&self) -> Option<&str> {
        match &self.command {
            Command::SaveNote { body, .. } => Some(body),
            _ => None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct WorkerResult {
    pub request_id: u64,
    pub result: Result<CommandResult, AppError>,
}

pub struct WorkerHandle {
    requests: Option<SyncSender<WorkerRequest>>,
    results: Option<Receiver<WorkerResult>>,
    worker: Option<JoinHandle<()>>,
}

impl WorkerHandle {
    pub fn try_send(&self, request: WorkerRequest) -> Result<(), AppError> {
        let Some(requests) = self.requests.as_ref() else {
            return Err(worker_stopped());
        };
        requests.try_send(request).map_err(|error| match error {
            TrySendError::Full(_) => AppError::new(
                ErrorCode::QueueFull,
                "Crane is still saving earlier changes. This change remains queued.",
            ),
            TrySendError::Disconnected(_) => worker_stopped(),
        })
    }

    pub fn try_recv(&self) -> Option<WorkerResult> {
        self.results.as_ref()?.try_recv().ok()
    }
}

impl Drop for WorkerHandle {
    fn drop(&mut self) {
        self.requests.take();
        self.results.take();
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

pub fn spawn_worker(mut library: Library, wake: Box<dyn Fn() + Send>) -> WorkerHandle {
    let (request_sender, request_receiver) = mpsc::sync_channel::<WorkerRequest>(CHANNEL_CAPACITY);
    let (result_sender, result_receiver) = mpsc::sync_channel::<WorkerResult>(CHANNEL_CAPACITY);
    let worker = thread::Builder::new()
        .name("crane-library-worker".into())
        .spawn(move || {
            while let Ok(request) = request_receiver.recv() {
                let result = WorkerResult {
                    request_id: request.request_id,
                    result: library.execute(request.command),
                };
                if result_sender.send(result).is_err() {
                    break;
                }
                wake();
            }
        })
        .expect("Crane must be able to start its library worker");

    WorkerHandle {
        requests: Some(request_sender),
        results: Some(result_receiver),
        worker: Some(worker),
    }
}

fn worker_stopped() -> AppError {
    AppError::new(
        ErrorCode::Busy,
        "The local library worker is not available.",
    )
}
