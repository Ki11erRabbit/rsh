

pub enum JobState {
    Waiting,
    Running,
    Finished,
}

pub struct ProcessStatus {
    pid: Pid,
    status: WaitStatus,
    pub cmd: String,
}

pub struct Job {
    pub self_status: ProcessStatus,
    pub children: Vec<ProcessStatus>,
    stop_stautus: i32,
    pub state: JobState,
    sigint: bool,
    jobctl: bool,
    waited: bool,
    used: bool,
}
