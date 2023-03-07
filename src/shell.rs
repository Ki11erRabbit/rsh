use std::collections::{BTreeMap, HashMap};
use crate::jobs::{Job, Process};
use nix::unistd::Pid;
use std::cell::RefCell;
use std::rc::Rc;
use std::path::PathBuf;
use fragile::Fragile;
use lazy_static::lazy_static;
use nix::unistd::{getpid, getcwd};
use nix::sys::signal::Signal;
use std::os::raw::c_int;

lazy_static! {
    pub static ref SHELL: Fragile<RefCell<Shell>> = Fragile::new(RefCell::new(Shell::new()));
}


pub struct Shell {
    // variables
    local_vars: HashMap<String, String>,
    local_var_stack: Vec<HashMap<String, String>>,
    var_table: BTreeMap<String, String>,
    // directory
    curr_directory: String,
    physical_directory: PathBuf,
    //jobs
    jobctl: bool,
    job_warning: i32,
    background_pid: Pid,
    vforked: bool,
    tty_fd: i32,
    job_table: Vec<Rc<RefCell<Job>>>,
    current_job: Option<usize>,
    traps: HashMap<Signal, String>,
    signal_mode: HashMap<Signal, usize>,//values are S_DFL, S_CATCH, S_IGN, S_HARD_IGN, S_RESET which are defined in trap.rs
    got_sig: Vec<bool>,
    pending_signal: Option<Signal>,
    //output
    //output: Output,
    //errout: Output,
    //misc
    root_pid: Pid,
    path: String,
}

impl Shell {
    pub fn new() -> Self {
        Self {
            local_vars: HashMap::new(),
            local_var_stack: Vec::new(),
            var_table: BTreeMap::new(),
            curr_directory: String::new(),
            physical_directory: getcwd().unwrap(),
            jobctl: false,
            job_warning: 0,
            background_pid: Pid::from_raw(-1),
            vforked: false,
            tty_fd: -1,
            job_table: Vec::new(),
            current_job: None,
            root_pid: getpid(),
            path: String::new(),
            traps: HashMap::new(),
            got_sig: vec![false; 32],
            pending_signal: None,
            signal_mode: HashMap::new(),
        }
    }

    pub fn delete_job_pid(&mut self, pid: Pid) {
        let mut index = 0;
        'out: for job in &mut self.job_table {
            for process in &job.borrow().processes {
                if process.pid == pid {
                    break 'out;
                }
            }
            index += 1;
        }
        self.job_table.remove(index);
        if self.current_job == Some(index) {
            self.current_job = None;
        }
    }
}


pub fn create_job(processes: Vec<Process>, background: bool) -> Rc<RefCell<Job>> {
    let job = Job::new(processes, background);

    let mut shell = SHELL.get().borrow_mut();

    if background {
        shell.jobctl = true;
    }

    shell.job_table.push(Rc::new(RefCell::new(job)));

    shell.current_job = Some(shell.job_table.len() - 1);

    shell.job_table.last().unwrap().clone() 
}

pub fn get_job(pid: Pid) -> Option<Rc<RefCell<Job>>> {
    let shell = SHELL.get().borrow();
    for job in &shell.job_table {
        for process in &job.borrow().processes {
            if process.pid == pid {
                return Some(job.clone());
            }
        }
    }
    None
}

pub fn delete_job(job: Rc<RefCell<Job>>) {
    let mut shell = SHELL.get().borrow_mut();
    let mut index = 0;
    for j in &shell.job_table {
        if Rc::ptr_eq(&j, &job) {
            shell.job_table.remove(index);
            if shell.current_job == Some(index) {
                shell.current_job = None;
            }
            return;
        }
        index += 1;
    }
}

pub fn delete_job_pid(pid: Pid) {
    let mut shell = SHELL.get().borrow_mut();
    shell.delete_job_pid(pid);
}

pub fn display_jobs() -> String {
    let shell = SHELL.get().borrow();
    let mut output = String::new();
    for job in &shell.job_table {
        output.push_str(&job.borrow().to_string());
    }
    output
}


pub fn get_current_job() -> Option<Rc<RefCell<Job>>> {
    println!("get current job");
    let shell = SHELL.get().borrow();

    println!("shell current job: {:?}", shell.current_job);
    if let Some(index) = shell.current_job {
        Some(shell.job_table[index].clone())
    } else {
        None
    }
}

pub fn is_trap_set(signal: Signal) -> bool {
    let shell = SHELL.get().borrow();
    shell.traps.contains_key(&signal)
}

pub fn get_trap(signal: Signal) -> Option<String> {
    let shell = SHELL.get().borrow();
    shell.traps.get(&signal).map(|s| s.to_string())
}

pub fn set_signal_mode(signal: Signal, mode: usize) {
    let mut shell = SHELL.get().borrow_mut();
    shell.signal_mode.insert(signal, mode);
}
pub fn get_signal_mode(signal: Signal) -> Option<usize> {
    let shell = SHELL.get().borrow();
    shell.signal_mode.get(&signal).map(|s| *s)
}

pub fn vforked() -> bool {
    let shell = SHELL.get().borrow();
    shell.vforked
}
pub fn flip_vforked() {
    let mut shell = SHELL.get().borrow_mut();
    shell.vforked = !shell.vforked;
}

pub fn set_got_sig(sig_num: c_int) {
    let mut shell = SHELL.get().borrow_mut();
    shell.got_sig[sig_num as usize] = true;
}

pub fn set_pending_signal(sig_num: c_int) {
    let mut shell = SHELL.get().borrow_mut();
    shell.pending_signal = Some(Signal::try_from(sig_num).unwrap());
}
