use std::ffi::CString;
use nix::unistd::Pid;
use std::fmt::{Display, Error, Formatter};

pub enum JobState {
    Waiting,
    Running,
    Finished,
}

pub struct Process {
    pid: Pid,
    pub argv: Vec<CString>,
    pub cmd: String,
}

impl Process {
    pub fn new(argv: Vec<CString>, cmd: String) -> Self {
        Self { pid: Pid::from_raw(-1), argv, cmd }
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
    pub fn new(processes: Vec<Process>, jobctl: bool) -> Self {
        Self {
            processes,
            stop_stautus: 0,
            state: JobState::Running,
            sigint: false,
            jobctl,
            waited: false,
            used: false,
        }
    }

    pub fn borrow_processes(&self) -> &Vec<Process> {
        &self.processes
    }

    pub fn borrow_processes_mut(&mut self) -> &mut Vec<Process> {
        &mut self.processes
    }
}

impl Display for Job {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        let mut s = String::new();
        for process in &self.processes {
            s.push_str(&process.cmd);
            s.push_str(" | ");
        }
        write!(f, "{}", s)
    }
}
