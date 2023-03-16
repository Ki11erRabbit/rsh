use std::ffi::CString;
use nix::unistd::Pid;
use nix::sys::wait::{waitpid, WaitStatus, WaitPidFlag};
use nix::sys::signal;
use nix::errno::Errno;
use std::fmt::{Display, Error, Formatter};
use std::rc::Rc;
use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};
use crate::trap;
use crate::eval::{get_exit_status, set_exit_status};
use crate::shell;

pub type JobId = usize;

const DOWAIT_NONBLOCK: usize = 0;
const DOWAIT_BLOCK: usize = 1;
const DOWAIT_WAITCMD: usize = 2;
const DOWAIT_WAITCMD_ALL: usize = 4;

pub trait JobUtils<I> {
    fn delete_job(&mut self, job: I);
    fn get_job(&self, id: I) -> Option<Rc<RefCell<Job>>>;
}

pub struct JobControl {
    pub job_table: Rc<RefCell<BTreeMap<usize, Rc<RefCell<Job>>>>>,
    pub background_jobs: HashMap<usize, usize>,
    pub pid_to_job: HashMap<Pid, usize>,
    pub current_job: Option<usize>,
    pub next_job_id: usize,
    pub jobctl: bool,
}

impl JobControl {
    pub fn new() -> Self {
        Self {
            job_table: Rc::new(RefCell::new(BTreeMap::new())),
            background_jobs: HashMap::new(),
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
        
        self.job_table.borrow_mut().insert(self.next_job_id, Rc::new(RefCell::new(job)));
        if background {
            self.background_jobs.insert(self.next_job_id, self.next_job_id);
        }

        self.current_job = Some(self.next_job_id);

        self.next_job_id = self.next_job_id + 1;

        self.job_table.borrow().get(&self.current_job.unwrap()).unwrap().clone()
    }

    pub fn update_pid_table(&mut self, job_id: JobId, pid: Pid) {
        self.pid_to_job.insert(pid, job_id);
    }

    pub fn background_jobs(&self) -> bool {
        self.background_jobs.is_empty()
    }
    pub fn is_background_job(&self, job_id: usize) -> bool {
        self.background_jobs.contains_key(&job_id)
    }

    pub fn get_current_job(&self) -> Option<Rc<RefCell<Job>>> {
        if let Some(index) = self.current_job {
            Some(self.job_table.borrow().get(&index).unwrap().clone())
        } else {
            None
        }
    }

    pub fn set_current_job(&mut self, job_id: usize) {
        self.current_job = Some(job_id);
    }

    
    pub fn clear_completed_jobs(&mut self) {
        let mut job_ids = Vec::new();
        job_ids.reserve(self.job_table.borrow().len());
        for (id, job) in self.job_table.borrow().iter() {
            if job.borrow().state == JobState::Finished {
                job_ids.push(*id);
            }
        }

        for id in job_ids {
            self.delete_job(id);
        }
    }

    fn update_next_job_id(&mut self) {
        if self.job_table.borrow().is_empty() {
            self.next_job_id = 1;
        } else {
            self.next_job_id = *self.job_table.borrow().keys().last().unwrap() + 1;
        }
    }

    pub fn get_job_table(&mut self) -> Rc<RefCell<BTreeMap<usize, Rc<RefCell<Job>>>>> {
        self.job_table.clone()
    }

}

impl JobUtils<Pid> for JobControl {
    fn delete_job(&mut self, pid: Pid) {
        let job_id = self.pid_to_job.get(&pid);

        let job_id = match job_id {
            Some(id) => *id,
            None => return,
        };
        
        
        let job = match self.job_table.borrow_mut().remove(&job_id) {
            Some(job) => job,
            None => return,
        };
        
         {
             let mut job = job.borrow_mut();
             if matches!(job.processes[0].status, Some(WaitStatus::Stopped(_,_))) {
                 job.state = JobState::Stopped;
                 job.stop_status = job.processes[0].status.unwrap();
             }
         } 

        //println!("{:?}",job.borrow());
        if matches!(job.borrow().stop_status,WaitStatus::Stopped(_,_)) {
            job.borrow_mut().state = JobState::Stopped;
            self.job_table.borrow_mut().insert(job_id, job);
            return;
        }
        
        if job.borrow().background {
            self.background_jobs.remove(&job_id);
        }

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

        self.job_table.borrow().get(&job_id).cloned()
    }
}

impl JobUtils<JobId> for JobControl {
    fn delete_job(&mut self, job_id: JobId) {
        let job = self.job_table.borrow_mut().remove(&job_id);

        let job = match job {
            Some(job) => job,
            None => return,
        };
         {
             let mut job = job.borrow_mut();
             if matches!(job.processes[0].status, Some(WaitStatus::Stopped(_,_))) {
                 job.state = JobState::Stopped;
                 job.stop_status = job.processes[0].status.unwrap();
             }
         } 
        

        //println!("{:?}",job.borrow());
        if matches!(job.borrow().stop_status,WaitStatus::Stopped(_,_)) {
            job.borrow_mut().state = JobState::Stopped;
            self.job_table.borrow_mut().insert(job_id, job);
            return;
        }

        for process in job.borrow().borrow_processes() {
            self.pid_to_job.remove(&process.pid);
        }

        if job.borrow().background {
            self.background_jobs.remove(&job_id);
        }
        
        self.update_next_job_id();
    }

    fn get_job(&self, job_id: JobId) -> Option<Rc<RefCell<Job>>> {
        self.job_table.borrow().get(&job_id).cloned()
    }
}

impl Display for JobControl {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        let mut s = String::new();
        for (id, job) in self.job_table.borrow().iter() {
            s.push_str(&format!("[{}]+ {}\n",id, job.borrow()));
        }
        write!(f, "{}", s)
    }
}


#[derive(Debug, Clone, Copy, PartialEq)]
pub enum JobState {
    Waiting,
    Running,
    Finished,
    Stopped,
}

impl Display for JobState {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        match self {
            JobState::Waiting => write!(f, "Waiting"),
            JobState::Running => write!(f, "Running"),
            JobState::Finished => write!(f, "Done"),
            JobState::Stopped => write!(f, "Stopped"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Process {
    pub pid: Pid,
    pub argv: Vec<CString>,
    pub argv0: String,
    pub cmd: String,
    pub status: Option<WaitStatus>,
}

impl PartialEq for Process {
    fn eq(&self, other: &Self) -> bool {
        self.pid == other.pid
    }
}

impl Process {
    pub fn new(argv: Vec<CString>, argv0: String ,cmd: String) -> Self {
        Self { pid: Pid::from_raw(-1),status: None, argv, argv0 ,cmd }
    }

    pub fn set_pid(&mut self, pid: Pid) {
        self.pid = pid;
    }
}

#[derive(Debug, Clone)]
pub struct Job {
    pub processes: Vec<Process>,
    pub job_id: usize,
    pub stop_status: WaitStatus,
    pub state: JobState,
    pub background: bool,
    pub sigint: bool,
    pub waited: bool,
    pub used: bool,
    pub changed: bool,
}

impl PartialEq for Job {
    fn eq(&self, other: &Self) -> bool {
        self.job_id == other.job_id
    }
}

impl Job {
    pub fn new(processes: Vec<Process>, job_id: usize, background: bool) -> Self {
        Self {
            processes,
            stop_status: WaitStatus::StillAlive,
            state: JobState::Running,
            sigint: false,
            waited: false,
            used: false,
            job_id,
            changed: false,
            background,
        }
    }

    pub fn borrow_processes(&self) -> &Vec<Process> {
        &self.processes
    }

    pub fn borrow_processes_mut(&mut self) -> &mut Vec<Process> {
        &mut self.processes
    }

    pub fn set_process_status(&mut self, pid: Pid, status: WaitStatus) {
        for process in &mut self.processes {
            if process.pid == pid {
                process.status = Some(status);
            }
        }
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
        write!(f, "{} {}",self.state, s)
    }
}



//pub fn forkshell()


pub fn wait_for_job_sigchld(job: Option<Rc<RefCell<Job>>>, status: WaitStatus) -> WaitStatus {

    if job.is_none() {
        return status;
    }
    let job = job.unwrap();

    if job.borrow().processes.len() == 1 {
        job.borrow_mut().stop_status = status;
        job.borrow_mut().state = JobState::Finished;
        shell::delete_job(job.borrow().job_id);
        return status;
    }

    wait_for_job(Some(job))
}

pub fn wait_for_job(job: Option<Rc<RefCell<Job>>>) -> WaitStatus {
    //eprintln!("wait_for_job");
    let status;
    let block = if job.is_some() {DOWAIT_BLOCK} else {DOWAIT_NONBLOCK};

    do_wait(block, &job);

    if job.is_none() {
        return get_exit_status();
    }

    let job = job.unwrap();

    status = job.borrow().stop_status;

    if job.borrow().state == JobState::Finished || matches!(job.borrow().stop_status,WaitStatus::Exited(_, _)) {
        let id = {
            job.borrow().job_id
        };
        shell::delete_job(id);
    }
    status
}


fn wait_one(block: usize, job: &Option<Rc<RefCell<Job>>>) -> Result<Option<Pid>,Errno> {
    //eprintln!("wait_one");

    let mut this_job: Option<Rc<RefCell<Job>>> = None;

    // block interapt

    let result = wait_process(block)?;


    if result.pid().is_none() {
        // unblock interupt
        if this_job.is_some() && *this_job.as_ref().unwrap().borrow() == *job.as_ref().unwrap().borrow() {

            let output = format_status(result, true);

            if output.is_some() {
                println!("{}", output.unwrap());
            }

        }
        return Ok(None);
    }
   
    let mut status = result;
    let mut pid = Pid::from_raw(-1);

    match result {
        WaitStatus::Exited(id, _) => {
            pid = id;
        },
        WaitStatus::Signaled(id, _, _) => {
            pid = id;
        },
        WaitStatus::Stopped(id, _) => {
            pid = id;
        },
        WaitStatus::StillAlive => {
            return Ok(None);
        },
        _ => (),
    }
   
    let mut state;
    for (_, jb) in shell::get_job_table().borrow_mut().iter_mut() {
        if jb.borrow().state == JobState::Finished {
            continue;
        }
        state = JobState::Finished;
        let mut stopped = false;
        for process in jb.borrow_mut().borrow_processes_mut() {
            if process.pid == pid {
                process.status = Some(status);
                this_job = Some(jb.clone());
            }
            if process.status == None {
                state = JobState::Running;
            }
            if state == JobState::Running {
                continue;
            }
            if matches!(process.status, Some(WaitStatus::Stopped(_, _))) {
                //println!("stopped");
                state = JobState::Stopped;
                stopped = true;
            }
        }
        let stop_status = jb.borrow().processes[0].status.unwrap();
        jb.borrow_mut().stop_status = stop_status;
        if this_job.is_some() {
            if state != JobState::Running {
                this_job.as_ref().unwrap().borrow_mut().changed = true;

                if this_job.as_ref().unwrap().borrow().state != state {
                    // log job state change
                    this_job.as_ref().unwrap().borrow_mut().state = state;
                }
                if state == JobState::Stopped {
                    shell::set_current_job(this_job.as_ref().unwrap().borrow().job_id);
                }
            }
            break;
        }
    }

    // unblock interupts

    if this_job.is_some() && *this_job.as_ref().unwrap().borrow() == *job.as_ref().unwrap().borrow() {

        let output = format_status(result, true);

        if output.is_some() {
            println!("{}", output.unwrap());
        }

    }
    return Ok(Some(pid));
}


fn do_wait(mut block: usize, job: &Option<Rc<RefCell<Job>>>) -> i32 {
    //eprintln!("do_wait");
    let got_sigchld = trap::got_sigchld();

    if job.is_some() && job.as_ref().unwrap().borrow().state != JobState::Running {
        block = DOWAIT_NONBLOCK;
    }

    if block == DOWAIT_NONBLOCK && !got_sigchld {
        //eprintln!("return 1");
        return 1;
    }

    let mut return_pid = 1;
    
    loop {
        let pid = wait_one(block, &job);
        
        if pid.is_ok() && pid.as_ref().unwrap().is_none() {
            break;
        }

        return_pid &= {
            if pid.is_ok() && pid.as_ref().unwrap().is_some() {
                0
            } else {
                1
            }
        };

        block &= !DOWAIT_WAITCMD_ALL;

        if pid.is_ok() || (job.is_some() && job.as_ref().unwrap().borrow().state != JobState::Running) {
            block = DOWAIT_NONBLOCK;
        }
       
        if pid.is_err() {
            break;
        }

    }

    return_pid
}


pub fn wait_process(block: usize) -> Result<WaitStatus, Errno> {
    //eprintln!("wait_process");
    let mut old_mask = signal::SigSet::empty();
    let flags = if block == DOWAIT_BLOCK {WaitPidFlag::WUNTRACED} else {WaitPidFlag::WNOHANG | WaitPidFlag::WUNTRACED};


    let mut result;
    loop {
        trap::set_got_sigchld(false);
        loop {
            result = waitpid(Pid::from_raw(-1), Some(flags));
            //eprintln!("waitpid result: {:?}", result);

            if result.is_ok() && result.unwrap() == WaitStatus::StillAlive {
                return result;
            }
            if result.is_err() && result.err().unwrap() == Errno::EINTR {
                continue;
            } 
            else {
                break;
            } 
        }
        if (result.is_err() || (result.is_ok() && result.unwrap().pid().is_some())) || block == 0 {
            break;
        }

        trap::sig_block_all(&mut old_mask);

        while !trap::got_sigchld() && trap::get_pending_signal().is_none() {
            trap::sig_suspend(&old_mask);
        }

        trap::sig_clear_mask();

        if trap::got_sigchld() {
            continue;
        }
        else {
            break;
        }
    }

    result
}



fn format_status(result: WaitStatus, sig_only: bool) -> Option<String> {
    None
}
