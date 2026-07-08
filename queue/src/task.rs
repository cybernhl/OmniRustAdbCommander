#[derive(Debug, Clone, PartialEq)]
pub enum TaskStatus {
    Queued,
    Preparing,
    Running { progress: f32, speed: f32 },
    Verify,
    MediaScan,
    Finished,
    Failed(String),
    Cancelled,
    Paused,
}

#[derive(Debug, Clone)]
pub enum TaskType {
    Push { local: String, remote: String },
    Pull { remote: String, local: String },
    Delete { path: String },
}

#[derive(Debug, Clone)]
pub struct TransferTask {
    pub id: u64,
    pub task_type: TaskType,
    pub status: TaskStatus,
    pub auto_start: bool,
}

impl TransferTask {
    pub fn new(id: u64, task_type: TaskType, auto_start: bool) -> Self {
        Self {
            id,
            task_type,
            status: TaskStatus::Queued,
            auto_start,
        }
    }
}
