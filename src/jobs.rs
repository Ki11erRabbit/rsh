use std::ffi::CString;
use nix::unistd::Pid;

pub enum JobState {
    Waiting,
    Running,
    Finished,
}

pub struct Process {
    pid: Pid,
    pub argv: Vec<CString>,
}

impl Process {
    pub fn new(argv: Vec<CString>) -> Self {
        Self { pid: Pid::from_raw(-1), argv }
    }

    pub fn set_pid(&mut self, pid: Pid) {
        self.pid = pid;
    }
}

pub struct Job {
    pub processes: Vec<Process>,
    stop_stautus: i32,
    pub state: JobState,
    sigint: bool,
    jobctl: bool,
    waited: bool,
    used: bool,
}

impl Job {
    pub fn new(processes: Vec<Process>) -> Self {
        Self {
            processes,
            stop_stautus: 0,
            state: JobState::Running,
            sigint: false,
            jobctl: false,
            waited: false,
            used: false,
        }
    }
}
