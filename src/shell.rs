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


/// This is a global variable that is set to true in the child process after a fork.
/// This is to prevent any erroneous signals from being sent to the child process that may cause issues.
pub static mut FORKED: AtomicBool = AtomicBool::new(false);

/// This function is a safe wrapper around setting the FORKED variable.
pub fn set_forked(state: bool) {
    unsafe {
        FORKED.store(state, std::sync::atomic::Ordering::Relaxed);
    }
}
/// This function is a safe wrapper around getting the FORKED variable.
pub fn get_forked() -> bool {
    unsafe {
        FORKED.load(std::sync::atomic::Ordering::Relaxed)
    }
}

lazy_static! {
    /// This is the global singleton that represents the shell as a whole.
    /// It holds all data that the shell might need to have access to.
    /// The Fragile wrapper is a way for the singleton to be global yet thread safe.
    /// The reason why a Mutex isn't being used is to prevent a potiential deadlock if the shell
    /// is inturrupted by a signal while in a signal handler.
    pub static ref SHELL: Fragile<RefCell<Shell>> = Fragile::new(RefCell::new(Shell::new()));
}

/// This is a trait that is basically used for overloading delete_job and get_job.
/// The reason is to allow getting and deleting jobs by either a JobId or a Pid.
pub trait ShellJobUtils<I> {
    fn delete_job(&mut self, job: I);
    fn get_job(&self, id: I) -> Option<Rc<RefCell<Job>>>;
}

/// This is a trait for overloading the add_alias function.
pub trait ShellAliasUtils<S> {
    fn add_alias(&mut self, input: S);
}

/// This is the struct that represents the shell as a whole.
pub struct Shell {
    interactive:bool,
    script_name: String,
    // variables
    //local_vars: HashMap<String, String>,
    //local_var_stack: Vec<HashMap<String, String>>,
    //var_table: BTreeMap<String, String>,
    //var_data: VarData,
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
    /// Creates a new shell.
    /// In the future the readline editor should be initialized outside of here so that we can
    /// read the config files and set the editor up accordingly.
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
            //var_data: VarData::new(),
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

    /// This allows us to access the readline editor within the shell. The Rc<RefCell<>> is used
    /// to allow us to have a mutable reference to the shell.
    pub fn get_readline(&self) -> Rc<RefCell<Editor<CompletionHelper,rustyline::history::DefaultHistory>>> {
        self.readline.clone()
    }

    /// This function loads the history from the history file set by the shell.
    /// In the future this should load it from the config file.
    pub fn load_history(&self) {
        if self.readline.borrow_mut().load_history(&self.history_location).is_err() {
            eprintln!("No previous history.");
        }
    }

    /// This is a wrapper around the readline editor's save_history function.
    /// It should get called by any function that causes the shell to exit.
    pub fn save_history(&self) {
        if self.readline.borrow_mut().save_history(&self.history_location).is_err() {
            eprintln!("Could not save history.");
        }
    }

    /// This function is used to set the history location.
    pub fn set_history_location(&mut self, location: &str) {
        self.history_location = location.to_string();
    }

    /// This function returns None if no alias is found for the given input.
    /// Otherwise it returns a tuple that holds the new command name to use and any arguments
    /// if they exist.
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

    /// This function prints out all the aliases that are currently stored in the shell.
    pub fn display_aliases(&self) {
        for (key, value) in &self.aliases {
            println!("alias {}='{}'", key, value);
        }
    }

    /// This function checks to see if there are any jobs in the background.
    pub fn background_jobs(&self) -> bool {
        self.job_control.background_jobs()
    }
    /// Given a JobId (usize) this function returns true if the job is a background job.
    pub fn is_background_job(&self, job_id: JobId) -> bool {
        self.job_control.is_background_job(job_id)
    }
    /// This function causes the shell to update the the pid table.
    pub fn update_pid_table(&mut self, job_id: JobId, pid: Pid) {
        self.job_control.update_pid_table(job_id, pid);
    }

    /// This function creates a new job and returns a reference to it.
    /// It takes in a Vec of Processes and a boolean that indicates if the job is a background job.
    pub fn create_job(&mut self, processes: Vec<Process>, background: bool) -> Rc<RefCell<Job>> {
        self.job_control.create_job(processes, background)
    }

    /// This function returns a formatted String with all of the jobs to be printed out.
    /// This is so that it can be used in a way to prevent deadlocks if needed.
    pub fn display_jobs(&self) -> String {
        format!("{}",self.job_control)
    }

    /// This function gets the last job that was created.
    pub fn get_current_job(&self) -> Option<Rc<RefCell<Job>>> {
        self.job_control.get_current_job()
    }

    /// This function runs the context manager's lookup_command function.
    pub fn lookup_command(&self, command: &str) -> Option<String> {
        self.context_manager.lookup_command(command)
    }

    /// This function takes a Context and pushes it onto the Context stack.
    pub fn push_context(&mut self, context: Context) {
        self.context_manager.push_context(context);
    }

    /// This function creates a new Context and pushes it onto the Context stack.
    pub fn push_context_new(&mut self) {
        self.context_manager.push_context_new();
    }
    /// This function pops the current Context off of the Context stack and returns it.
    pub fn pop_context(&mut self) -> Option<Rc<RefCell<Context>>> {
        self.context_manager.pop_context()
    } 

    /// This function is used to add a new context with a given namespace.
    pub fn add_context(&mut self, namespace:&str, context: Rc<RefCell<Context>>) {
        self.context_manager.add_context(namespace,context);
    }
    /// This grabs the current context from the Context stack.
    pub fn get_current_context(&self) -> Rc<RefCell<Context>> {//todo: get rid of clone
        self.context_manager.get_context().clone()
    }

    /// This function returns the current environment context.
    /// This Context is stored at the bottom of the Context stack.
    pub fn get_env_context(&self) -> Rc<RefCell<Context>> {
        self.context_manager.get_env_context()
    }

    /*pub fn push_var_stack(&mut self) {
        self.var_data.push_var_stack();
    }
    pub fn pop_var_stack(&mut self) {
        self.var_data.pop_var_stack();
    }*/

    /// This function adds a variable to the context given by the position.
    pub fn add_var(&mut self, set: &str, position: usize) {
        self.context_manager.add_var_pos(set, position);
        //self.var_data.add_var(set, position);
    }
    /// This function adds a variable to the current context.
    pub fn add_var_context(&mut self, set: &str) {
        self.context_manager.add_var(set);
        //let pos = self.var_data.get_current_context_pos();
        //self.var_data.add_var(set, pos);
    }
    /// This function takes in a &str and returns the value of the variable if it exists.
    pub fn expand_variable(&mut self, var: &str) -> Option<String> {
        let var = self.context_manager.get_var(var);
        if var.is_none() {
            return None;
        }
        let var = var.unwrap();
        let x = Some(var.borrow().value.clone());
        x
        //self.var_data.lookup_var(var)
    }

    /// This function takes in a function name and FunctionBody and adds it to the current Context.
    pub fn add_function(&mut self, name: &str, body: FunctionBody) {
        self.context_manager.add_function(name, Rc::new(RefCell::new(body)));
        //self.functions.insert(name.to_string(), body);
    }
    /// This function takes in a function name and returns true if the function exists.
    pub fn is_function(&self, name: &str) -> bool {
        self.context_manager.is_function(name)
        //self.functions.contains_key(name)
    }
    /// This function takes in a function name and returns the FunctionBody if it exists.
    pub fn get_function(&self, name: &str) -> Option<Rc<RefCell<FunctionBody>>> {
        self.context_manager.get_function(name)
        //self.functions.get(name)
    }

    /// This is an internal function that trims off the quotes from a string.
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

/// This is an implementation of the ShellJobUtils trait for the Shell struct.
/// This allows the for overloading of the delete_job and get_job functions.
impl ShellJobUtils<Pid> for Shell {
    /// This variant takes in a Pid and deletes the a job that matches that Pid.
    fn delete_job(&mut self, pid: Pid) {
        self.job_control.delete_job(pid);
    }

    /// This variant takes in a Pid and returns the Job that matches that Pid.
    fn get_job(&self, pid: Pid) -> Option<Rc<RefCell<Job>>> {
        self.job_control.get_job(pid)
    }
}

/// This is an implementation of the ShellJobUtils trait for the Shell struct.
/// This allows the for overloading of the delete_job and get_job functions.
impl ShellJobUtils<JobId> for Shell {
    /// This variant takes in a JobId (usize) and deletes the a job that matches that JobId.
    fn delete_job(&mut self, id: JobId) {
        self.job_control.delete_job(id);
    }
    /// This variant takes in a JobId (usize) and returns the Job that matches that JobId.
    fn get_job(&self, id: JobId) -> Option<Rc<RefCell<Job>>> {
        self.job_control.get_job(id)
    }
}

/// This is an implementation of the ShellJobUtils trait for the Shell struct.
/// This allows the for overloading of the delete_job and get_job functions.
/// This variant takes in a string that contains and equal sign ('=').
impl ShellAliasUtils<&str> for Shell {

    /// This variant takes in a string that contains and equal sign ('=').
    /// The string is split into two parts and the first part is used as the alias name
    /// and the second part is used as the alias value.
    /// We cut off the quotes in the value if they exist.
    fn add_alias(&mut self, input: &str) {
        let split = input.split('=').collect::<Vec<&str>>();
        if split.len() != 2 {
            eprintln!("Invalid alias");
            return;
        }
        self.aliases.insert(split[0].to_string(), Self::trim(split[1]));
    }
}

/// This is an implementation of the ShellJobUtils trait for the Shell struct.
/// This allows the for overloading of the delete_job and get_job functions.
/// This variant takes in a tuple of two &str, so the equal sign ('=') has to be
/// removed before calling this variant.
impl ShellAliasUtils<(&str, &str)> for Shell {
    /// This variant takes in a tuple of two &str, so the equal sign ('=') has to be
    /// removed before calling this variant.
    /// The first part of the tuple is used as the alias name
    /// and the second part is used as the alias value.
    /// We cut off the quotes in the value if they exist.
    fn add_alias(&mut self, (alias, value): (&str, &str)) {
        self.aliases.insert(alias.to_string(), Self::trim(value));
    }
}

/// This takes in either a &str or a tuple of &str and calls the add_alias function
/// with the correct variant. This is basically an overloaded function.
pub fn add_alias<S>(alias: S)
where Shell: ShellAliasUtils<S>{
    let mut shell = SHELL.get().borrow_mut();
    shell.add_alias(alias);
}

/// This function takes in a command and returns the alias if it exists.
/// If the alias exists, then it returns an Option with a Tuple that has the command the alias
/// points to and the arguments that the alias has if any.
pub fn lookup_alias(command: &str) -> Option<(String, Option<Vec<String>>)> {
    let shell = SHELL.get().borrow();
    shell.lookup_alias(command)
}
/// This clears all the aliases in the shell.
pub fn clear_aliases() {
    let mut shell = SHELL.get().borrow_mut();
    shell.aliases.clear();
}

/// This takes an alias name and removes it from the shell.
pub fn remove_alias(alias: &str) {
    let mut shell = SHELL.get().borrow_mut();
    shell.aliases.remove(alias);
}

/// This function prints out all the aliases in the shell.
pub fn display_aliases() {
    let shell = SHELL.get().borrow();
    shell.display_aliases();
}

/// This is a wrapper function that saves the readline history.
/// This function should be called by anything that causes the shell to exit on purpose.
/// It likely should be called frequently to prevent history loss.
pub fn save_history() {
    let shell = SHELL.get().borrow();
    shell.save_history();
}

/// This function loads the history from the history file stored in the shell.
pub fn load_history() {
    let shell = SHELL.get().borrow();
    shell.load_history();
}

/// This function sets the history location.
pub fn set_history_location(location: &str) {
    let mut shell = SHELL.get().borrow_mut();
    shell.set_history_location(location);
}

/// This function gets the readline object from the shell.
pub fn get_readline() -> Rc<RefCell<Editor<CompletionHelper,rustyline::history::DefaultHistory>>> {
    let shell = SHELL.get().borrow();
    shell.get_readline()
}

/// This function creates a job from a vector of processes and a boolean that indicates if the job
/// should be run in the background.
pub fn create_job(processes: Vec<Process>, background: bool) -> Rc<RefCell<Job>> {
    let mut shell = SHELL.get().borrow_mut();
    shell.create_job(processes, background)
}

/// This function gets a job from the shell by either a job id or a pid.
/// This function is effectively an overloaded function.
pub fn get_job<T>(id: T) -> Option<Rc<RefCell<Job>>>
where
    Shell: ShellJobUtils<T>,
{
    let shell = SHELL.get().borrow();
    shell.get_job(id)
}

/// This function deletes a job from the shell by either a job id or a pid.
/// This function is effectively an overloaded function.
pub fn delete_job<T>(id: T)
where
    Shell: ShellJobUtils<T>,
{
    trap::interrupts_off();
    let mut shell = SHELL.get().borrow_mut();
    shell.delete_job(id);
    trap::interrupts_on();
}

/// This function returns a string that contains all the jobs in the shell.
pub fn display_jobs() -> String {
    let shell = SHELL.get().borrow();
    shell.display_jobs()
}

/// This function gets the last job in the shell.
pub fn get_current_job() -> Option<Rc<RefCell<Job>>> {
    let shell = SHELL.get().borrow();
    shell.get_current_job()
}

/// This function gets all the jobs in the shell as a BTreeMap.
pub fn get_job_table() -> Rc<RefCell<BTreeMap<usize,Rc<RefCell<Job>>>>> {
    SHELL.get().borrow_mut().job_control.get_job_table()
}

/// This function sets the current job in the shell via a job id.
pub fn set_current_job(job_id: usize) {
    SHELL.get().borrow_mut().job_control.set_current_job(job_id);
}
/// This function checks if there are any background jobs in the shell.
pub fn background_jobs() -> bool {
    let shell = SHELL.get().borrow();
    shell.background_jobs()
}
/// This function checks if a job is a background job via a job id.
pub fn is_background_job(job_id: JobId) -> bool {
    let shell = SHELL.get().borrow();
    shell.is_background_job(job_id)
}

/// This function adds a pid to the pid table in the shell.
pub fn update_pid_table(job_id: JobId, pid: Pid) {
    let mut shell = SHELL.get().borrow_mut();
    shell.update_pid_table(job_id, pid);
}

/// This function checks the command on the PATH and returns the full path to the command.
pub fn lookup_command(command: &str) -> Option<String> {
    let shell = SHELL.get().borrow();
    shell.lookup_command(command)
}

/// This function checks if the shell is interactive.
pub fn is_interactive() -> bool {
    let shell = SHELL.get().borrow();
    shell.interactive
}
/// This function sets the shell to be interactive or not.
pub fn set_interactive(interactive: bool) {
    let mut shell = SHELL.get().borrow_mut();
    shell.interactive = interactive;
}
/// This function sets the script name.
pub fn set_script_name(script_name: &str) {
    let mut shell = SHELL.get().borrow_mut();
    shell.script_name = script_name.to_string();
    shell.interactive = false;
}
/// This function gets the script name. This is usually the name of the script or the name of the shell.
pub fn get_script_name() -> String {
    let shell = SHELL.get().borrow();
    shell.script_name.clone()
}

/// This function takes in a &str and an index and sets the input arguments.
/// Input arguments are usually $0, $1, $2, etc.
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

/// This function takes in a Context and adds it to the context stack.
pub fn push_context(context: Context) {
    let mut shell = SHELL.get().borrow_mut();
    shell.push_context(context);
}
/// This function creates a new context and adds it to the context stack.
pub fn push_context_new() {
    let mut shell = SHELL.get().borrow_mut();
    shell.push_context_new();
}
/// This function pops a context from the context stack and returns it.
pub fn pop_context() -> Option<Rc<RefCell<Context>>> {
    let mut shell = SHELL.get().borrow_mut();
    shell.pop_context()
}

/// This function adds a context to the shell with a namespace.
pub fn add_context(namespace: &str, context: Rc<RefCell<Context>>) {
    let mut shell = SHELL.get().borrow_mut();
    shell.add_context(namespace, context);
}
/// This function gets the current context from the context stack.
pub fn get_current_context() -> Rc<RefCell<Context>> {
    let shell = SHELL.get().borrow();
    shell.get_current_context()
}
/// This function gets the context at the bottom of the stack which contains the environment variables,
/// exported variables, and exported functions.
pub fn get_env_context() -> Rc<RefCell<Context>> {
    let shell = SHELL.get().borrow();
    shell.get_env_context()
}
/// This function adds a variable to the shell at a given context via an index.
pub fn add_var(set: &str, index: usize) {
    let mut shell = SHELL.get().borrow_mut();
    shell.add_var(set, index);
}
/// This function adds a variable to the shell at the current context.
pub fn add_var_context(set: &str) {
    let mut shell = SHELL.get().borrow_mut();
    shell.add_var_context(set);
}
/// This function takes in a variable name and returns the value of that variable if it exists.
pub fn expand_var(var: &str) -> Option<String> {
    let mut shell = SHELL.get().borrow_mut();
    shell.expand_variable(var)
}

/// This function sets $0 to the script name.
/// This should be called after we have figured out if we are running a script or not.
pub fn set_arg_0() {
    let mut shell = SHELL.get().borrow_mut();
    let set = format!("0={}", shell.script_name);

    shell.add_var(&set,1);
}

/// This function adds a FunctionBody to the shell with a given name.
pub fn add_function(name: &str, body: FunctionBody) {
    let mut shell = SHELL.get().borrow_mut();
    shell.add_function(name, body);
}
/// This function takes a &str and checks if it represents a function in the shell or not.
pub fn is_function(name: &str) -> bool {
    let shell = SHELL.get().borrow();
    shell.is_function(name)
}
/// This function takes a &str and returns the FunctionBody if it exists.
pub fn get_function(name: &str) -> Option<Rc<RefCell<FunctionBody>>> {
    let shell = SHELL.get().borrow();
    shell.get_function(name)
}
