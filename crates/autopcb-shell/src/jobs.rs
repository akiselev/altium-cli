#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JobId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobStatus {
    Queued,
    Running,
    Completed,
    Failed,
}

#[derive(Debug, Clone)]
pub enum JobEvent {
    Started(JobId),
    Progress(JobId, f32),
    Completed(JobId),
    Failed(JobId, String),
}
