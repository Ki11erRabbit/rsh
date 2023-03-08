use std::collections::{BTreeMap, HashMap};
use crate::jobs::{Job, Process, JobControl, JobUtils, JobId};
use nix::unistd::Pid;
use std::cell::RefCell;
use std::rc::Rc;
use std::path::PathBuf;
use fragile::Fragile;
use lazy_static::lazy_static;
use nix::unistd::{getpid, getcwd};
use nix::sys::signal::Signal;
use std::os::raw::c_int;
use rustyline::Editor;
use crate::var::{VarData, VarDataUtils};

lazy_static! {
    pub static ref SHELL: Fragile<RefCell<Shell>> = Fragile::new(RefCell::new(Shell::new()));
}


pub trait ShellUtils<I> {
    fn delete_job(&mut self, job: I);
    fn get_job(&self, id: I) -> Option<Rc<RefCell<Job>>>;
}


pub struct Shell {
    interactive:bool,
    script_name: String,
    // variables
    //local_vars: HashMap<String, String>,
    //local_var_stack: Vec<HashMap<String, String>>,
    //var_table: BTreeMap<String, String>,
    var_data: VarData,
    // directory
    curr_directory: String,
    physical_directory: PathBuf,
    //jobs
    jobctl: bool,
    job_warning: i32,
    background_pid: Pid,
    vforked: bool,
    tty_fd: i32,
    job_control: JobControl,
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
    readline: Rc<RefCell<Editor<()>>>,
    history_location: String,
}

impl Shell {
    pub fn new() -> Self {
        Self {
            script_name: "rsh".to_string(),
            interactive: true,
            var_data: VarData::new(),
            curr_directory: String::new(),
            physical_directory: getcwd().unwrap(),
            jobctl: false,
            job_warning: 0,
            background_pid: Pid::from_raw(-1),
            vforked: false,
            tty_fd: -1,
            job_control: JobControl::new(),
            root_pid: getpid(),
            path: String::new(),
            traps: HashMap::new(),
            got_sig: vec![false; 32],
            pending_signal: None,
            signal_mode: HashMap::new(),
            readline: Rc::new(RefCell::new(Editor::<()>::new())),
            history_location: String::new(),
        }
    } 

    pub fn get_readline(&self) -> Rc<RefCell<Editor<()>>> {
        self.readline.clone()
    }

    pub fn load_history(&self) {
        if self.readline.borrow_mut().load_history(&self.history_location).is_err() {
            eprintln!("No previous history.");
        }
    }

    pub fn save_history(&self) {
        if self.readline.borrow_mut().save_history(&self.history_location).is_err() {
            eprintln!("Could not save history.");
        }
    }

    pub fn set_history_location(&mut self, location: &str) {
        self.history_location = location.to_string();
    }

    pub fn create_job(&mut self, processes: Vec<Process>, background: bool) -> Rc<RefCell<Job>> {
        self.job_control.create_job(processes, background)
    }

    pub fn display_jobs(&self) -> String {
        format!("{}",self.job_control)
    }

    pub fn get_current_job(&self) -> Option<Rc<RefCell<Job>>> {
        self.job_control.get_current_job()
    }

    pub fn lookup_command(&self, command: &str) -> Option<String> {
        self.var_data.lookup_command(command)
    }

    pub fn add_var(&mut self, set: &str, position: isize) {
        self.var_data.add_var(set, position);
    }
}

impl ShellUtils<Pid> for Shell {
    fn delete_job(&mut self, pid: Pid) {
        self.job_control.delete_job(pid);
    }

    fn get_job(&self, pid: Pid) -> Option<Rc<RefCell<Job>>> {
        self.job_control.get_job(pid)
    }
}

impl ShellUtils<JobId> for Shell {
    fn delete_job(&mut self, id: JobId) {
        self.job_control.delete_job(id);
    }

    fn get_job(&self, id: JobId) -> Option<Rc<RefCell<Job>>> {
        self.job_control.get_job(id)
    }
}

pub fn save_history() {
    let shell = SHELL.get().borrow();
    shell.save_history();
}

pub fn load_history() {
    let shell = SHELL.get().borrow();
    shell.load_history();
}

pub fn set_history_location(location: &str) {
    let mut shell = SHELL.get().borrow_mut();
    shell.set_history_location(location);
}

pub fn get_readline() -> Rc<RefCell<Editor<()>>> {
    let shell = SHELL.get().borrow();
    shell.get_readline()
}


pub fn create_job(processes: Vec<Process>, background: bool) -> Rc<RefCell<Job>> {
    let mut shell = SHELL.get().borrow_mut();
    shell.create_job(processes, background)
}

// takes a job id or a pid
pub fn get_job<T>(id: T) -> Option<Rc<RefCell<Job>>>
where
    Shell: ShellUtils<T>,
{
    let shell = SHELL.get().borrow();
    shell.get_job(id)
}

// takes a job id or a pid
pub fn delete_job<T>(id: T)
where
    Shell: ShellUtils<T>,
{
    let mut shell = SHELL.get().borrow_mut();
    shell.delete_job(id);
}



pub fn display_jobs() -> String {
    let shell = SHELL.get().borrow();
    shell.display_jobs()
}


pub fn get_current_job() -> Option<Rc<RefCell<Job>>> {
    let shell = SHELL.get().borrow();
    shell.get_current_job()
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

pub fn lookup_command(command: &str) -> Option<String> {
    let shell = SHELL.get().borrow();
    shell.lookup_command(command)
}

pub fn is_interactive() -> bool {
    let shell = SHELL.get().borrow();
    shell.interactive
}
pub fn set_interactive(interactive: bool) {
    let mut shell = SHELL.get().borrow_mut();
    shell.interactive = interactive;
}
pub fn set_script_name(script_name: &str) {
    let mut shell = SHELL.get().borrow_mut();
    shell.script_name = script_name.to_string();
    shell.interactive = false;
}
pub fn get_script_name() -> String {
    let shell = SHELL.get().borrow();
    shell.script_name.clone()
}

pub fn set_input_args(arg: &str, index: usize) {
    let mut shell = SHELL.get().borrow_mut();

    let set = format!("{}={}", index, arg);

    shell.add_var(&set,0);

} 
    

pub fn set_arg_0() {
    let mut shell = SHELL.get().borrow_mut();
    let set = format!("0={}", shell.script_name);

    shell.add_var(&set,0);
}



