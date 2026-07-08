#[derive(Debug, Clone)]
pub enum TransferEvent {
    Started { task_id: u64 },
    Progress { task_id: u64, progress: f32, speed: f32 },
    Finished { task_id: u64 },
    Error { task_id: u64, message: String },
}
