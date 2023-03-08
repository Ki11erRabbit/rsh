use std::ffi::CString;
use nix::unistd::Pid;
use std::fmt::{Display, Error, Formatter};
use std::rc::Rc;
use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};

pub type JobId = usize;


pub trait JobUtils<I> {
    fn delete_job(&mut self, job: I);
    fn get_job(&self, id: I) -> Option<Rc<RefCell<Job>>>;
}

pub struct JobControl {
    pub job_table: BTreeMap<usize, Rc<RefCell<Job>>>,
    pub pid_to_job: HashMap<Pid, usize>,
    pub current_job: Option<usize>,
    pub next_job_id: usize,
    pub jobctl: bool,
}

impl JobControl {
    pub fn new() -> Self {
        Self {
            job_table: BTreeMap::new(),
            pid_to_job: HashMap::new(),
            current_job: None,
            jobctl: false,
            next_job_id: 1,
        }
    }

    pub fn create_job(&mut self, processes: Vec<Process>, background: bool) -> Rc<RefCell<Job>> {
        
        for process in &processes {
            self.pid_to_job.insert(process.pid, self.next_job_id);
        }

        let job = Job::new(processes, self.next_job_id, background);
        
        self.job_table.insert(self.next_job_id, Rc::new(RefCell::new(job)));

        self.current_job = Some(self.next_job_id);

        self.next_job_id = self.next_job_id + 1;

        self.job_table.get(&self.current_job.unwrap()).unwrap().clone()
    }

    pub fn get_current_job(&self) -> Option<Rc<RefCell<Job>>> {
        if let Some(index) = self.current_job {
            Some(self.job_table.get(&index).unwrap().clone())
        } else {
            None
        }
    }

    fn update_next_job_id(&mut self) {
        if self.job_table.is_empty() {
            self.next_job_id = 1;
        } else {
            self.next_job_id = *self.job_table.keys().last().unwrap() + 1;
        }
    }

}

impl JobUtils<Pid> for JobControl {
    fn delete_job(&mut self, pid: Pid) {
        
        let job_id = self.pid_to_job.get(&pid);

        let job_id = match job_id {
            Some(id) => *id,
            None => return,
        };
        
        
        let job = self.job_table.remove(&job_id).unwrap();

        for process in job.borrow().borrow_processes() {
            self.pid_to_job.remove(&process.pid);
        }

        self.update_next_job_id();
    }

    fn get_job(&self, pid: Pid) -> Option<Rc<RefCell<Job>>> {
        let job_id = self.pid_to_job.get(&pid);

        let job_id = match job_id {
            Some(id) => *id,
            None => return None,
        };

        self.job_table.get(&job_id).cloned()
    }
}

impl JobUtils<JobId> for JobControl {
    fn delete_job(&mut self, job_id: JobId) {
        let job = self.job_table.remove(&job_id);

        let job = match job {
            Some(job) => job,
            None => return,
        };

        for process in job.borrow().borrow_processes() {
            self.pid_to_job.remove(&process.pid);
        }
        
        self.update_next_job_id();
    }

    fn get_job(&self, job_id: JobId) -> Option<Rc<RefCell<Job>>> {
        self.job_table.get(&job_id).cloned()
    }
}

impl Display for JobControl {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        let mut s = String::new();
        for job in self.job_table.values() {
            s.push_str(&format!("{}\n", job.borrow()));
        }
        write!(f, "{}", s)
    }
}



pub enum JobState {
    Waiting,
    Running,
    Finished,
    Stopped,
}

pub struct Process {
    pub pid: Pid,
    pub argv: Vec<CString>,
    pub argv0: String,
    pub cmd: String,
}

impl PartialEq for Process {
    fn eq(&self, other: &Self) -> bool {
        self.pid == other.pid
    }
}

impl Process {
    pub fn new(argv: Vec<CString>, argv0: String ,cmd: String) -> Self {
        Self { pid: Pid::from_raw(-1), argv, argv0 ,cmd }
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
    pub fn new(processes: Vec<Process>, job_id: usize, jobctl: bool) -> Self {
        Self {
            processes,
            stop_status: 0,
            state: JobState::Running,
            sigint: false,
            jobctl,
            waited: false,
            used: false,
            job_id,
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
