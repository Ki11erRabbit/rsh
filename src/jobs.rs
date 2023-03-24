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

/// A type alias for a job id
pub type JobId = usize;

/// A constant for do_wait to indicate that the wait should be non-blocking
const DOWAIT_NONBLOCK: usize = 0;
/// A constant for do_wait to indicate that the wait should be blocking
const DOWAIT_BLOCK: usize = 1;
/// A constant for do_wait to indicate that we should wait for a specific job
const DOWAIT_WAITCMD: usize = 2;
/// A constant for do_wait to indicate that we should wait for all jobs
const DOWAIT_WAITCMD_ALL: usize = 4;

/// A trait that allows us to overload the delete_job and get_job methods
pub trait JobUtils<I> {
    /// Delete a job from the job table
    fn delete_job(&mut self, job: I);
    /// Get a job from the job table
    fn get_job(&self, id: I) -> Option<Rc<RefCell<Job>>>;
}

/// A struct that holds all job related information
pub struct JobControl {
    /// A map of job ids to jobs
    pub job_table: Rc<RefCell<BTreeMap<usize, Rc<RefCell<Job>>>>>,
    /// A map of background job ids to job ids
    pub background_jobs: HashMap<usize, usize>,
    /// A map of pids to job ids
    pub pid_to_job: HashMap<Pid, usize>,
    /// The current job
    pub current_job: Option<usize>,
    /// The next job id
    pub next_job_id: usize,
    /// Whether or not job control is enabled
    pub jobctl: bool,
}

/// All of these methods get called by the shell.
impl JobControl {
    /// Create a new JobControl struct
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

    /// This function gets called by the shell.
    /// It creates a new job and adds it to the job table.
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

    /// Thi function takes a JobId (usize) and a Pid and updates the pid_to_job table
    pub fn update_pid_table(&mut self, job_id: JobId, pid: Pid) {
        self.pid_to_job.insert(pid, job_id);
    }

    /// This method checks to see if there are any background jobs
    pub fn background_jobs(&self) -> bool {
        self.background_jobs.is_empty()
    }
    /// This method takes a JobId and sees if it corresponds to a background job
    pub fn is_background_job(&self, job_id: usize) -> bool {
        self.background_jobs.contains_key(&job_id)
    }

    /// This method gets the curent Job if there is one
    pub fn get_current_job(&self) -> Option<Rc<RefCell<Job>>> {
        if let Some(index) = self.current_job {
            Some(self.job_table.borrow().get(&index).unwrap().clone())
        } else {
            None
        }
    }

    /// This method sets the current job
    pub fn set_current_job(&mut self, job_id: usize) {
        self.current_job = Some(job_id);
    }

    /// This method removes all jobs that have been finished from the job table.
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

    /// This method figures out what the next_job_id should be and gets called whenever a job is deleted.
    fn update_next_job_id(&mut self) {
        if self.job_table.borrow().is_empty() {
            self.next_job_id = 1;
        } else {
            self.next_job_id = *self.job_table.borrow().keys().last().unwrap() + 1;
        }
    }

    /// This method gets a reference to the job table
    pub fn get_job_table(&mut self) -> Rc<RefCell<BTreeMap<usize, Rc<RefCell<Job>>>>> {
        self.job_table.clone()
    }

}

/// This variant of JobUtils takes a Pid as an argument.
impl JobUtils<Pid> for JobControl {
    /// This method deletes a job from the job table given a pid.
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

    /// This method gets a job given a pid.
    /// We must update the pid_to_job table in order for this to work.
    fn get_job(&self, pid: Pid) -> Option<Rc<RefCell<Job>>> {
        let job_id = self.pid_to_job.get(&pid);

        let job_id = match job_id {
            Some(id) => *id,
            None => return None,
        };

        self.job_table.borrow().get(&job_id).cloned()
    }
}

/// This variant of JobUtils takes a JobId as an argument.
impl JobUtils<JobId> for JobControl {
    /// This method deletes a job from the job table given a job id.
    fn delete_job(&mut self, job_id: JobId) {
        let job = self.job_table.borrow_mut().remove(&job_id);

        let job = match job {
            Some(job) => job,
            None => return,
        };

        if job.borrow().processes.len() == 0 {
            return;
        }

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

    /// This method gets a job given a job id.
    fn get_job(&self, job_id: JobId) -> Option<Rc<RefCell<Job>>> {
        self.job_table.borrow().get(&job_id).cloned()
    }
}

/// This implementation allows us to print out the job table.
impl Display for JobControl {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        let mut s = String::new();
        for (id, job) in self.job_table.borrow().iter() {
            s.push_str(&format!("[{}] {}{}\n",id, shell::expand_var("PS4").unwrap(), job.borrow()));
        }
        write!(f, "{}", s)
    }
}

/// This enum represents the passible states a job can be in.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum JobState {
    Waiting,
    Running,
    Finished,
    Stopped,
}

/// This is so that we can print out the job state.
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

/// This struct represents a running process for the shell.
/// It contains the pid of the process as well as the arguments for an exec call.
/// It also holds some of that information as a Rust String so that it is easier to print out.
#[derive(Debug, Clone)]
pub struct Process {
    /// The Pid of the running process.
    pub pid: Pid,
    /// The arguments for the exec call.
    pub argv: Vec<CString>,
    /// The name of the commmand/path to executable.
    pub argv0: String,
    /// The whole argument string to be used for printing.
    pub cmd: String,
    /// The status of the process created by waitpid.
    pub status: Option<WaitStatus>,
}

impl PartialEq for Process {
    /// Processes are equal if they have the same pid.
    fn eq(&self, other: &Self) -> bool {
        self.pid == other.pid
    }
}

impl Process {
    /// It takes in a vector of CStrings, a String for argv0, and a String for cmd.
    pub fn new(argv: Vec<CString>, argv0: String ,cmd: String) -> Self {
        Self { pid: Pid::from_raw(-1),status: None, argv, argv0 ,cmd }
    }

    /// This method sets the pid of the process.
    pub fn set_pid(&mut self, pid: Pid) {
        self.pid = pid;
    }
}

/// This struct represents a job for the shell.
/// It contains a vector of processes, a job id, a status, a state, and a boolean for if it is a background job.
/// It also contains a boolean for if it has been waited on, a boolean for if it has been used, and a boolean for if it has changed.
#[derive(Debug, Clone)]
pub struct Job {
    /// The processes in the job.
    pub processes: Vec<Process>,
    /// The job id.
    pub job_id: usize,
    /// The status of the job created by waitpid.
    pub stop_status: WaitStatus,
    /// The state of the job.
    pub state: JobState,
    /// A boolean for if the job is a background job.
    pub background: bool,
    /// A boolean for if the job has received a sigint.
    pub sigint: bool,
    /// A boolean for if the job has been waited on.
    pub waited: bool,
    /// A boolean for if the job has been used.
    pub used: bool,
    /// A boolean for if the job has changed state.
    pub changed: bool,
}

impl PartialEq for Job {
    /// Jobs are equal if they have the same job id.
    fn eq(&self, other: &Self) -> bool {
        self.job_id == other.job_id
    }
}

impl Job {
    /// It takes in a vector of processes, a job id, and a boolean for if it is a background job.
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

    /// This method gives us a non mutable reference to the processes in the job.
    pub fn borrow_processes(&self) -> &Vec<Process> {
        &self.processes
    }

    /// This method gives us a mutable reference to the processes in the job.
    pub fn borrow_processes_mut(&mut self) -> &mut Vec<Process> {
        &mut self.processes
    }

    /// This method sets the status of a process in the job via the pid.
    pub fn set_process_status(&mut self, pid: Pid, status: WaitStatus) {
        for process in &mut self.processes {
            if process.pid == pid {
                process.status = Some(status);
		return;
            }
        }
    }
}

/// This implementation allows us to print out the job.
impl Display for Job {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        let mut s = String::new();
        for process in &self.processes {
            s.push_str(&process.cmd);
            if process != self.processes.last().unwrap() {
                s.push_str(" | ");
            }
        }
        if self.background {
            s.push_str(" &");
        }
        write!(f, "{} {}",self.state, s)
    }
}



//pub fn forkshell()

/// This function waits for a job to finish.
///
/// This variant of the function is similar to to wait_for_job but also takes in a WaitStatus.
/// This is called from a SIGCHLD handler as we need to to know the if the job only has one process or not.
/// If the job has more than one process then we call the normal wait_for_job function.
pub fn wait_for_job_sigchld(job: Option<Rc<RefCell<Job>>>, status: WaitStatus) -> WaitStatus {

    if job.is_none() {
        return status;
    }
    let job = job.unwrap();

    if job.borrow().processes.len() == 1 {
        job.borrow_mut().stop_status = status;
        job.borrow_mut().state = JobState::Finished;
        let id = {
            job.borrow().job_id
        };
        shell::delete_job(id);
        return status;
    }
    trap::interrupts_off();
    let result = wait_for_job(Some(job));
    trap::interrupts_on();
    result
}

/// This function waits for a job to finish.
///
/// It takes in an Option with a reference to a job and returns a WaitStatus.
/// If the job is None then it will just return the exit status of the last job that was waited on in a non blocking way.
/// If the job is Some then it will perform a blocking wait on the job. If the job has finished or exited then the job will be deleted.
///
/// This function shoult be called with interrupts disabled.
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

/// This function waits for a process to terminate.
fn wait_one(block: usize, job: &Option<Rc<RefCell<Job>>>) -> Result<Option<Pid>,Errno> {
    //eprintln!("wait_one");

    let mut this_job: Option<Rc<RefCell<Job>>> = None;

    // block interapt
    trap::interrupts_off();

    let result = wait_process(block)?;


    if result.pid().is_none() {
        // unblock interupt
        trap::interrupts_on();
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
        let result = jb.try_borrow();
        if result.is_err() || result.unwrap().state == JobState::Finished {
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
    trap::interrupts_on();

    if this_job.is_some() && *this_job.as_ref().unwrap().borrow() == *job.as_ref().unwrap().borrow() {

        let output = format_status(result, true);

        if output.is_some() {
            println!("{}", output.unwrap());
        }

    }
    return Ok(Some(pid));
}

/// This function takes in some flags listed at the tob of job.rs.
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

/// This is where we do a wait system call.
///If the result from waitpid is StillAlive we return the result. If the result is an Err but errno is EINTR we try again.
/// Otherwise we break out of the inner loop. Then if the result is an error or the result is ok and we can get a pid from it or if block is not set then we break out of the outer loop.
/// If that isn't the case then we block all signals and wait for a SIGCHLD to be sent.
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


/// This function is used to format a status message for a job.
fn format_status(result: WaitStatus, sig_only: bool) -> Option<String> {
    let mut output = String::new();
    match result {
        WaitStatus::Exited(pid, status) => {
            if sig_only {
                return None;
            }
            let job = match shell::get_job(pid) {
                Some(job) => job,
                None => return None,
            };
            output.push_str(&format!("[{}] {}\t\t{} ", job.borrow().job_id,shell::expand_var("PS4").unwrap(), job.borrow()));
        },
        WaitStatus::Signaled(pid, signal, dump) => {
            let job = match shell::get_job(pid) {
                Some(job) => job,
                None => return None,
            };
            output.push_str(&format!("[{}] {}\t\t{} ", job.borrow().job_id,shell::expand_var("PS4").unwrap(), job.borrow()));
            if dump {
                output.push_str(&format!("({}) (core dumped)", signal));
            }
            else {
                output.push_str(&format!("({})", signal));
            }
        },
        WaitStatus::Stopped(pid, signal) => {
            let job = match shell::get_job(pid) {
                Some(job) => job,
                None => return None,
            };
            output.push_str(&format!("[{}] {}\t\t{} ", job.borrow().job_id,shell::expand_var("PS4").unwrap(), job.borrow()));
            output.push_str(&format!("({})", signal));
        },
        _ => return None,
    }
    return Some(output);
}

/// This is to remove all signal handlers from a forked child.
pub fn fork_reset() {
    trap::remove_handlers(); 
}


