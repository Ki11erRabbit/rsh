use std::ffi::CString;
use nix::unistd::Pid;
use std::fmt::{Display, Error, Formatter};
use std::rc::Rc;
use std::cell::RefCell;

pub enum JobState {
    Waiting,
    Running,
    Finished,
    Stopped,
}

pub struct Process {
    pub pid: Pid,
    pub argv: Vec<CString>,
    pub cmd: String,
}

impl PartialEq for Process {
    fn eq(&self, other: &Self) -> bool {
        self.pid == other.pid
    }
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
    pub job_id: usize,
    pub stop_status: i32,
    pub state: JobState,
    pub sigint: bool,
    pub jobctl: bool,
    pub waited: bool,
    pub used: bool,
}

impl Job {
    pub fn new(processes: Vec<Process>, jobctl: bool) -> Self {
        Self {
            processes,
            stop_status: 0,
            state: JobState::Running,
            sigint: false,
            jobctl,
            waited: false,
            used: false,
            job_id: 0,
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
            if process != self.processes.last().unwrap() {
                s.push_str(" | ");
            }
        }
        write!(f, "{}", s)
    }
}



//pub fn forkshell()



pub fn wait_for_job(job: Option<Rc<RefCell<Job>>>) -> i32 {
    
    //let block = if job.is_some() {DOWAIT_BLOCK} else {DOWAIT_NONBLOCK};
    //do_wait(block, job);
    0
}
