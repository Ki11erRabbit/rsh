use std::collections::{BTreeMap, HashMap};
use crate::jobs::{Job, Process};
use nix::unistd::Pid;
use std::cell::RefCell;
use std::rc::Rc;
use fragile::Fragile;
use lazy_static::lazy_static;
use nix::unistd::getpid;


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
    physical_directory: String,
    //jobs
    jobctl: bool,
    job_warning: i32,
    background_pid: Pid,
    vforked: bool,
    tty_fd: i32,
    job_table: Vec<Rc<RefCell<Job>>>,
    current_job: Option<usize>,
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
            physical_directory: String::new(),
            jobctl: false,
            job_warning: 0,
            background_pid: Pid::from_raw(-1),
            vforked: false,
            tty_fd: -1,
            job_table: Vec::new(),
            current_job: None,
            root_pid: getpid(),
            path: String::new(),
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

pub fn display_jobs() {
    let shell = SHELL.get().borrow();

    for job in &shell.job_table {
        println!("{}", job.borrow());
    }
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
