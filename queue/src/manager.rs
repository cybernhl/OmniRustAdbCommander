use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use crate::task::{TransferTask, TaskType};
use crate::events::TransferEvent;
use crate::worker::Worker;
use adb_explorer_backend::traits::DeviceBackend;
use std::collections::VecDeque;

pub struct QueueManager {
    tasks: Arc<Mutex<VecDeque<TransferTask>>>,
    backend: Arc<dyn DeviceBackend>,
    event_tx: mpsc::UnboundedSender<TransferEvent>,
    // The internal receiver will be wrapped or handed out
    internal_rx: Option<mpsc::UnboundedReceiver<TransferEvent>>,
    next_id: u64,
}

impl QueueManager {
    pub fn new(backend: Arc<dyn DeviceBackend>) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        Self {
            tasks: Arc::new(Mutex::new(VecDeque::new())),
            backend,
            event_tx: tx,
            internal_rx: Some(rx),
            next_id: 1,
        }
    }

    pub async fn add_task(&mut self, task_type: TaskType, auto_start: bool) -> u64 {
        let id = self.next_id;
        self.next_id += 1;

        let task = TransferTask::new(id, task_type, auto_start);
        self.tasks.lock().await.push_back(task);

        if auto_start {
            self.start_worker_if_needed();
        }

        id
    }

    pub async fn start_queue(&mut self) {
        self.start_worker_if_needed();
    }

    fn start_worker_if_needed(&self) {
        let tasks = self.tasks.clone();
        let backend = self.backend.clone();
        let event_tx = self.event_tx.clone();

        tokio::spawn(async move {
            loop {
                let task = {
                    let mut lock = tasks.lock().await;
                    lock.pop_front()
                };

                if let Some(task) = task {
                    let worker = Worker::new(backend.clone(), event_tx.clone());
                    let _ = worker.execute(task).await;
                } else {
                    break;
                }
            }
        });
    }

    /// UI calls this once to get the event stream
    pub fn take_event_rx(&mut self) -> Option<mpsc::UnboundedReceiver<TransferEvent>> {
        self.internal_rx.take()
    }
}
