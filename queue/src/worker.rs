use std::sync::Arc;
use tokio::sync::mpsc;
use crate::task::{TransferTask, TaskType};
use crate::events::TransferEvent;
use adb_explorer_backend::traits::DeviceBackend;
use anyhow::Result;

pub struct Worker {
    backend: Arc<dyn DeviceBackend>,
    event_tx: mpsc::UnboundedSender<TransferEvent>,
}

impl Worker {
    pub fn new(backend: Arc<dyn DeviceBackend>, event_tx: mpsc::UnboundedSender<TransferEvent>) -> Self {
        Self { backend, event_tx }
    }

    pub async fn execute(&self, task: TransferTask) -> Result<()> {
        let _ = self.event_tx.send(TransferEvent::Started { task_id: task.id });

        match &task.task_type {
            TaskType::Push { local, remote } => {
                match self.backend.push(local, remote).await {
                    Ok(_) => {
                        let _ = self.backend.refresh_media(remote).await;
                        let _ = self.event_tx.send(TransferEvent::Finished { task_id: task.id });
                    }
                    Err(e) => {
                        let _ = self.event_tx.send(TransferEvent::Error {
                            task_id: task.id,
                            message: e.to_string()
                        });
                    }
                }
            }
            TaskType::Pull { remote, local } => {
                match self.backend.pull(remote, local).await {
                    Ok(_) => {
                        let _ = self.event_tx.send(TransferEvent::Finished { task_id: task.id });
                    }
                    Err(e) => {
                        let _ = self.event_tx.send(TransferEvent::Error {
                            task_id: task.id,
                            message: e.to_string()
                        });
                    }
                }
            }
            TaskType::Delete { path } => {
                match self.backend.delete(path).await {
                    Ok(_) => {
                        let _ = self.event_tx.send(TransferEvent::Finished { task_id: task.id });
                    }
                    Err(e) => {
                        let _ = self.event_tx.send(TransferEvent::Error {
                            task_id: task.id,
                            message: e.to_string()
                        });
                    }
                }
            }
        }

        Ok(())
    }
}
