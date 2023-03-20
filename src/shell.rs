use std::collections::{BTreeMap, HashMap};
use crate::jobs::{Job, Process, JobControl, JobUtils, JobId};
use crate::trap;
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
use rustyline::history::FileHistory;
use rustyline::config;
use crate::var::{VarData, VarDataUtils};
use crate::ast::FunctionBody;
use crate::context::{ContextManager, Context};
use crate::completion::CompletionHelper;

use std::sync::atomic::AtomicBool;

pub static mut FORKED: AtomicBool = AtomicBool::new(false);

pub fn set_forked(state: bool) {
    unsafe {
        FORKED.store(state, std::sync::atomic::Ordering::Relaxed);
    }
}
pub fn get_forked() -> bool {
    unsafe {
        FORKED.load(std::sync::atomic::Ordering::Relaxed)
    }
}

lazy_static! {
    pub static ref SHELL: Fragile<RefCell<Shell>> = Fragile::new(RefCell::new(Shell::new()));
}


pub trait ShellJobUtils<I> {
    fn delete_job(&mut self, job: I);
    fn get_job(&self, id: I) -> Option<Rc<RefCell<Job>>>;
}

pub trait ShellAliasUtils<S> {
    fn add_alias(&mut self, input: S);
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
    tty_fd: i32,
    pub job_control: JobControl,
    //output
    //output: Output,
    //errout: Output,
    //misc
    root_pid: Pid,
    path: String,
    readline: Rc<RefCell<Editor<CompletionHelper,FileHistory>>>,
    history_location: String,
    aliases: HashMap<String, String>,
    functions: HashMap<String, FunctionBody>,
    context_manager: ContextManager,
}

/*static DEFAULT_KEYS: Vec<KeyEvent> = vec![
    KeyEvent::Ctrl('z'),

];*/


impl Shell {
    pub fn new() -> Self {
     /*   let readline = Editor::<()>::new();
        readline.bind_sequence(KeyEvent::Ctrl('z'), Cmd::new(|_, _| {
            println!("Ctrl-Z");
            Ok(())
        }));*/
        let config = config::Builder::new()
            .behavior(config::Behavior::PreferTerm)
            .auto_add_history(true)
            .bell_style(config::BellStyle::Audible)
            .completion_type(config::CompletionType::Fuzzy)
            .build();
        let readline = Rc::new(RefCell::new(Editor::with_config(config).unwrap()));
       
        let helper = CompletionHelper::default();

        readline.borrow_mut().set_helper(Some(helper));


        Self {
            script_name: "rsh".to_string(),
            interactive: true,
            var_data: VarData::new(),
            curr_directory: String::new(),
            physical_directory: getcwd().unwrap(),
            jobctl: false,
            job_warning: 0,
            background_pid: Pid::from_raw(-1),
            tty_fd: -1,
            job_control: JobControl::new(),
            root_pid: getpid(),
            path: String::new(),
            readline,
            history_location: String::new(),
            aliases: HashMap::new(),
            functions: HashMap::new(),
            context_manager: ContextManager::new(),
        }
    } 

    pub fn get_readline(&self) -> Rc<RefCell<Editor<CompletionHelper,rustyline::history::DefaultHistory>>> {
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

    pub fn lookup_alias(&self, command: &str) -> Option<(String, Option<Vec<String>>)> {

        match self.aliases.get(command) {
            None => None,
            Some(alias) => {
                let mut args = alias.split_whitespace();
                let command = args.next().unwrap();
                let args: Vec<String> = args.map(|s| s.to_string()).collect();
                if args.is_empty() {
                    Some((command.to_string(), None))
                } else {
                    Some((command.to_string(), Some(args)))
                }
            }
        }
    }
    pub fn display_aliases(&self) {
        for (key, value) in &self.aliases {
            println!("alias {}='{}'", key, value);
        }
    }

    pub fn background_jobs(&self) -> bool {
        self.job_control.background_jobs()
    }
    pub fn is_background_job(&self, job_id: JobId) -> bool {
        self.job_control.is_background_job(job_id)
    }
    pub fn update_pid_table(&mut self, job_id: JobId, pid: Pid) {
        self.job_control.update_pid_table(job_id, pid);
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


    pub fn push_context(&mut self, context: Context) {
        self.context_manager.push_context(context);
    }
    pub fn push_context_new(&mut self) {
        self.context_manager.push_context_new();
    }
    pub fn pop_context(&mut self) -> Option<Context> {
        self.context_manager.pop_context()
    } 

    pub fn add_context(&mut self, namespace:&str, context: Context) {
        self.context_manager.add_context(namespace,context);
    }
    pub fn get_current_context(&self) -> Context {//todo: get rid of clone
        self.context_manager.get_context().clone()
    }
    /*pub fn push_var_stack(&mut self) {
        self.var_data.push_var_stack();
    }
    pub fn pop_var_stack(&mut self) {
        self.var_data.pop_var_stack();
    }*/

    pub fn add_var(&mut self, set: &str, position: usize) {
        self.context_manager.add_var_pos(set, position);
        //self.var_data.add_var(set, position);
    }
    pub fn add_var_context(&mut self, set: &str) {
        self.context_manager.add_var(set);
        //let pos = self.var_data.get_current_context_pos();
        //self.var_data.add_var(set, pos);
    }
    pub fn expand_variable(&mut self, var: &str) -> Option<String> {
        let var = self.context_manager.get_var(var);
        if var.is_none() {
            return None;
        }
        let var = var.unwrap();
        Some(var.value.clone())
        //self.var_data.lookup_var(var)
    }

    pub fn add_function(&mut self, name: &str, body: FunctionBody) {
        self.context_manager.add_function(name, body);
        //self.functions.insert(name.to_string(), body);
    }
    pub fn is_function(&self, name: &str) -> bool {
        self.context_manager.is_function(name)
        //self.functions.contains_key(name)
    }
    pub fn get_function(&self, name: &str) -> Option<&FunctionBody> {
        self.context_manager.get_function(name)
        //self.functions.get(name)
    }


    fn trim(word: &str) -> String {
        if (word.starts_with("\"") && word.ends_with("\"")) || (word.starts_with("'") && word.ends_with("'")){
            let mut chars = word.chars();
            chars.next();
            chars.next_back();
            chars.collect::<String>()
        }
        else {
            word.to_string()
        }
    }
}

impl ShellJobUtils<Pid> for Shell {
    fn delete_job(&mut self, pid: Pid) {
        self.job_control.delete_job(pid);
    }

    fn get_job(&self, pid: Pid) -> Option<Rc<RefCell<Job>>> {
        self.job_control.get_job(pid)
    }
}

impl ShellJobUtils<JobId> for Shell {
    fn delete_job(&mut self, id: JobId) {
        self.job_control.delete_job(id);
    }

    fn get_job(&self, id: JobId) -> Option<Rc<RefCell<Job>>> {
        self.job_control.get_job(id)
    }
}

impl ShellAliasUtils<&str> for Shell {
    fn add_alias(&mut self, input: &str) {
        let split = input.split('=').collect::<Vec<&str>>();
        if split.len() != 2 {
            eprintln!("Invalid alias");
            return;
        }
        self.aliases.insert(split[0].to_string(), Self::trim(split[1]));
    }
}
impl ShellAliasUtils<(&str, &str)> for Shell {
    fn add_alias(&mut self, (alias, value): (&str, &str)) {
        self.aliases.insert(alias.to_string(), Self::trim(value));
    }
}

// can take a single &str or a tuple of &str
pub fn add_alias<S>(alias: S)
where Shell: ShellAliasUtils<S>{
    let mut shell = SHELL.get().borrow_mut();
    shell.add_alias(alias);
}

pub fn lookup_alias(command: &str) -> Option<(String, Option<Vec<String>>)> {
    let shell = SHELL.get().borrow();
    shell.lookup_alias(command)
}

pub fn clear_aliases() {
    let mut shell = SHELL.get().borrow_mut();
    shell.aliases.clear();
}

pub fn remove_alias(alias: &str) {
    let mut shell = SHELL.get().borrow_mut();
    shell.aliases.remove(alias);
}

pub fn display_aliases() {
    let shell = SHELL.get().borrow();
    shell.display_aliases();
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

pub fn get_readline() -> Rc<RefCell<Editor<CompletionHelper,rustyline::history::DefaultHistory>>> {
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
    Shell: ShellJobUtils<T>,
{
    let shell = SHELL.get().borrow();
    shell.get_job(id)
}

// takes a job id or a pid
pub fn delete_job<T>(id: T)
where
    Shell: ShellJobUtils<T>,
{
    trap::interrupts_off();
    let mut shell = SHELL.get().borrow_mut();
    shell.delete_job(id);
    trap::interrupts_on();
}



pub fn display_jobs() -> String {
    let shell = SHELL.get().borrow();
    shell.display_jobs()
}


pub fn get_current_job() -> Option<Rc<RefCell<Job>>> {
    let shell = SHELL.get().borrow();
    shell.get_current_job()
}
pub fn get_job_table() -> Rc<RefCell<BTreeMap<usize,Rc<RefCell<Job>>>>> {
    SHELL.get().borrow_mut().job_control.get_job_table()
}
pub fn set_current_job(job_id: usize) {
    SHELL.get().borrow_mut().job_control.set_current_job(job_id);
}
pub fn background_jobs() -> bool {
    let shell = SHELL.get().borrow();
    shell.background_jobs()
}
pub fn is_background_job(job_id: JobId) -> bool {
    let shell = SHELL.get().borrow();
    shell.is_background_job(job_id)
}
pub fn update_pid_table(job_id: JobId, pid: Pid) {
    let mut shell = SHELL.get().borrow_mut();
    shell.update_pid_table(job_id, pid);
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

/*pub fn push_var_stack() {
    let mut shell = SHELL.get().borrow_mut();
    shell.push_var_stack();
}
pub fn pop_var_stack() {
    let mut shell = SHELL.get().borrow_mut();
    shell.pop_var_stack();
}*/

pub fn push_context(context: Context) {
    let mut shell = SHELL.get().borrow_mut();
    shell.push_context(context);
}
pub fn push_context_new() {
    let mut shell = SHELL.get().borrow_mut();
    shell.push_context_new();
}
pub fn pop_context() {
    let mut shell = SHELL.get().borrow_mut();
    shell.pop_context();
}

pub fn add_context(namespace: &str, context: Context) {
    let mut shell = SHELL.get().borrow_mut();
    shell.add_context(namespace, context);
}
pub fn get_current_context() -> Context {
    let shell = SHELL.get().borrow();
    shell.get_current_context()
}

pub fn add_var(set: &str, index: usize) {
    let mut shell = SHELL.get().borrow_mut();
    shell.add_var(set, index);
}
pub fn add_var_context(set: &str) {
    let mut shell = SHELL.get().borrow_mut();
    shell.add_var_context(set);
}
pub fn expand_var(var: &str) -> Option<String> {
    let mut shell = SHELL.get().borrow_mut();
    shell.expand_variable(var)
}


pub fn set_arg_0() {
    let mut shell = SHELL.get().borrow_mut();
    let set = format!("0={}", shell.script_name);

    shell.add_var(&set,1);
}

pub fn add_function(name: &str, body: FunctionBody) {
    let mut shell = SHELL.get().borrow_mut();
    shell.add_function(name, body);
}
pub fn is_function(name: &str) -> bool {
    let shell = SHELL.get().borrow();
    shell.is_function(name)
}
pub fn get_function(name: &str) -> Option<FunctionBody> {
    let shell = SHELL.get().borrow();
    shell.get_function(name).cloned()
}
