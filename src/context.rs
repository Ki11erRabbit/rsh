use std::collections::{HashMap,BTreeMap};
use std::default::Default;
use std::env;
use std::rc::Rc;
use std::cell::RefCell;

use nix::unistd::{getpid, getppid, getuid,Uid};

use crate::ast::FunctionBody;

/// Default prompt for a regular user
const REG_USER_PROMPT: &str = "$ ";
/// Default prompt for a super user (root)
const SUP_USER_PROMPT: &str = "# ";

/// A struct to represent a variable
#[derive(Debug, Clone)]
pub struct Var {
    pub name: String,
    pub value: String,
}

impl Var {
    pub fn new(name: &str, value: &str) -> Self {
        Self {
            name: name.to_string(),
            value: value.to_string(),
        }
    }

    /// Adds an variable to the environment
    fn export(&mut self) {
        env::set_var(&self.name, &self.value);
    }
}
impl ToString for Var {
    fn to_string(&self) -> String {
        format!("{}={}", self.name, self.value)
    }
}

/// A struct that manages Contexts of a shell
#[derive(Debug, Clone)]
pub struct ContextManager {
    /// A stack of contexts
    /// The first context is always the environment context where it holds everything from /etc/profile and what
    /// has been exported from other contexts
    context_stack: Vec<Rc<RefCell<Context>>>,
    /// This holds all the contexts that have been exported
    /// The key is a namespace for the context
    exported_contexts: HashMap<String,Rc<RefCell<Context>>>,
}

impl ContextManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// Pushes a new context onto the stack with a given Context
    pub fn push_context(&mut self, context: Context) {
        self.context_stack.push(Rc::new(RefCell::new(context)));
    }
    /// Pushes a new context onto the stack with a default Context
    pub fn push_context_new(&mut self) {
        self.context_stack.push(Rc::new(RefCell::new(Context::default())));
    }

    /// Removes the last context from the stack and returns it
    /// # Panics
    /// Panics if the last context is attempted to be removed
    pub fn pop_context(&mut self) -> Option<Rc<RefCell<Context>>> {
        if self.context_stack.len() == 1 {//to prevent deleting the global context
            panic!("Cannot delete global context");
        }
        self.context_stack.pop()
    }

    /// Returns a reference to the current Context.
    pub fn get_context(&self) -> Rc<RefCell<Context>> {
        self.context_stack.last().unwrap().clone()
    }

    /// Returns a reference to the environment Context or Context 0.
    pub fn get_env_context(&self) -> Rc<RefCell<Context>> {
        self.context_stack.first().unwrap().clone()
    }

    /// Adds a context with a namespace to the exported contexts.
    pub fn add_context(&mut self, name: &str, context: Rc<RefCell<Context>>) {
        self.exported_contexts.insert(name.to_string(), context);
    }

    /// Returns a reference to a context with a given namespace.
    pub fn get_context_by_name(&self, name: &str) -> Option<Rc<RefCell<Context>>> {
        self.exported_contexts.get(name).cloned()
    }

    /// Gets a variable from the contexts.
    /// It will never attempt to get a variable from an exported context without a namespace.
    /// It will search the Context stack in reverse order for the variable.
    pub fn get_var(&self, name: &str) -> Option<Rc<RefCell<Var>>> {
        if name.contains("::") {
            let mut split = name.split("::");
            let context_name = split.next().unwrap();
            let var_name = split.next().unwrap();
            if let Some(context) = self.get_context_by_name(context_name) {
                context.borrow().get_var(var_name)
            } else {
                None
            }
        }
        else {
            for context in self.context_stack.iter().rev() {
                if let Some(var) = context.borrow().get_var(name) {
                    return Some(var);
                }
            }
            None
        }
    } 


    /// Gets a function from the Contexts.
    /// It will never attempt to get a function from an exported context without a namespace.
    /// It will search the Context stack in reverse order for the function.
    pub fn get_function(&self, name: &str) -> Option<Rc<RefCell<FunctionBody>>> {
        if name.contains("::") {
            let mut split = name.split("::");
            let context_name = split.next().unwrap();
            let func_name = split.next().unwrap();
            if let Some(context) = self.get_context_by_name(context_name) {
                context.borrow().get_function(func_name)
            } else {
                None
            }
        }
        else {
            for context in self.context_stack.iter().rev() {
                if let Some(func) = context.borrow().get_function(name) {
                    return Some(func);
                }
            }
            None
        }
    }

    /// Checks to see if a name corresponds to a function name.
    pub fn is_function(&self, name: &str) -> bool {
        if name.contains("::") {
            let mut split = name.split("::");
            let context_name = split.next().unwrap();
            let func_name = split.next().unwrap();
            if let Some(context) = self.get_context_by_name(context_name) {
                context.borrow().get_function(func_name).is_some()
            } else {
                false
            }
        }
        else {
            for context in self.context_stack.iter().rev() {
                if context.borrow().get_function(name).is_some() {
                    return true;
                }
            }
            false
        }
    }

    /// Adds a function to the current context.
    pub fn add_function(&mut self, name: &str, func: Rc<RefCell<FunctionBody>>) {
        self.get_context().borrow_mut().add_function(name, func);
    }

    /// Adds a variable to the current context.
    pub fn add_var(&mut self, set: &str) {
        let mut split = set.split("=");
        let name = split.next().unwrap();

        for context in self.context_stack.iter_mut().rev() {
            if context.borrow().get_var(name).is_some() {
                context.borrow_mut().add_var(set);
                return;
            }
        }
        self.get_context().borrow_mut().add_var(set);
    }

    /// Adds a variable to a particular context on the stack.
    pub fn add_var_pos(&mut self, set: &str, pos: usize) {
        self.context_stack[pos].borrow_mut().add_var(set);
    }

    /// Looks up a command in the PATH variable.
    pub fn lookup_command(&self, cmd: &str) -> Option<String> {
        for path in self.context_stack.first().unwrap().borrow().get_var("PATH").unwrap().borrow().value.split(":") {
            let path = format!("{}/{}", path, cmd);
            let metadata = std::fs::metadata(&path);
            if metadata.is_ok() {
                if metadata.unwrap().is_file() {
                    return Some(path);
                }
            }
        }
        None
    }


    /// Returns a list of all variables in all contexts.
    /// The reason why this is a BTreeMap is because it is used to print out the variables, which must be in alphabetical order.
    pub fn all_vars(&self) -> BTreeMap<String, Rc<RefCell<Var>>> {
        let mut vars = BTreeMap::new();
        for context in &self.context_stack {
            for (name, var) in &context.borrow().vars {
                vars.insert(name.clone(), var.clone());
            }
        }
        for (_,context) in &self.exported_contexts {
            for (name, var) in &context.borrow().vars {
                vars.insert(name.clone(), var.clone());
            }
        }
        vars
    }

    /// Converts the environment variables into a HashMap of variables.
    fn convert_env() -> HashMap<String, Rc<RefCell<Var>>> {
        let mut vars = HashMap::new();
        for (key, value) in std::env::vars() {
            vars.insert(key.clone(), Rc::new(RefCell::new(Var::new(&key, &value))));
        }
        vars
    }
    
}

impl Default for ContextManager {
    fn default() -> Self {
        let mut vars = HashMap::new();


        let ps1 = if Uid::is_root(getuid()) {
            SUP_USER_PROMPT
        } else {
            REG_USER_PROMPT
        };


        if env::var("PATH").is_err() {
            vars.insert("PATH".to_owned(), Rc::new(RefCell::new(Var::new("PATH", "/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin"))));
        }
        if env::var("PWD").is_err() {//TODO: Change this to use a nix function that will be more reliable
            vars.insert("PWD".to_owned(), Rc::new(RefCell::new(Var::new("PWD", nix::unistd::getcwd().unwrap().to_str().unwrap()))));
        }
        vars.insert("PS1".to_owned(), Rc::new(RefCell::new(Var::new("PS1", ps1))));
        vars.insert("PS2".to_owned(), Rc::new(RefCell::new(Var::new("PS2", "> "))));
        vars.insert("PS4".to_owned(), Rc::new(RefCell::new(Var::new("PS4", "+ "))));
        vars.insert("PPID".to_owned(), Rc::new(RefCell::new(Var::new("PPID", getppid().to_string().as_str()))));
        vars.insert("PID".to_owned(), Rc::new(RefCell::new(Var::new("PID", getpid().to_string().as_str()))));


        ContextManager::convert_env().drain().for_each(|(key, value)| {
            vars.insert(key.to_owned(), value);
        });
        
        Self {
            context_stack: vec![Rc::new(RefCell::new(Context::new(vars)))],
            exported_contexts: HashMap::new(),
        }
    }
}


pub trait ContextUtils<V> {
    fn add_var(&mut self, var: V);
}

/// A struct that represents a context.
/// A context is similar to a stack frame in other programming languages.
/// The difference is that functions can also be stored in a context.
#[derive(Debug, Clone)]
pub struct Context {
    /// Stores the variables of the context.
    pub vars: HashMap<String, Rc<RefCell<Var>>>,
    /// Stores the functions of the context.
    functions: HashMap<String, Rc<RefCell<FunctionBody>>>,
}

impl Context {
    pub fn new(vars: HashMap<String, Rc<RefCell<Var>>>) -> Self {
        Self {
            vars,
            functions: HashMap::new(),
        }
    }

    /// Gets a variable from the context with a given name.
    pub fn get_var(&self, name: &str) -> Option<Rc<RefCell<Var>>> {
        self.vars.get(name).cloned()
    }

    /// Gets a function from the context with a given name.
    pub fn get_function(&self, name: &str) -> Option<Rc<RefCell<FunctionBody>>> {
        self.functions.get(name).cloned()
    }

    /// Adds a function to the context with a given name.
    pub fn add_function(&mut self, name: &str, body: Rc<RefCell<FunctionBody>>) {
        self.functions.insert(name.to_string(), body);
    }

    /// Internal function that removes quotes from a string.
    /// # Arguments
    /// * `word` - The &str to remove quotes from.
    /// # Example
    /// ```
    /// use shell::context::Context;
    /// let word = "\"hello\"";
    /// let trimmed = Context::trim(word);
    /// assert_eq!(trimmed, "hello");
    /// ```
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

impl Default for Context {
    fn default() -> Self {
        Self {
            vars: HashMap::new(),
            functions: HashMap::new(),
        }
    }
}


impl ContextUtils<&str> for Context {
    /// Adds a variable to the context in the format of `name=value`.
    /// # Arguments
    /// * `var` - The &str set that is to be added to the context.
    fn add_var(&mut self, var: &str) {
        let (name, value) = if var.contains("=") {
            let mut split = var.split("=");
            (split.next().unwrap(), split.next().unwrap())
        } else {
            (var, "")
        };

        let var_struct = Var::new(name, &Self::trim(value));
    
        let key = if var.chars().nth(0).unwrap().to_digit(10).is_some() {
            var.chars().filter(|c| {
                (*c == '0') | (*c == '1') | (*c == '2') | (*c == '3') | (*c == '4') | (*c == '5') | (*c == '6') | (*c == '7') | (*c == '8') | (*c == '9')
            }).collect::<String>().parse::<usize>().unwrap().to_string()
        }
        else {
            name.to_string()
        };


        self.add_var(var_struct);
    }
}

impl ContextUtils<(&str, &str)> for Context {

    /// Adds a variable to the context in the format of `(name, value)`.
    /// # Arguments
    /// * `var` - A tuple of &str to be add to the context.
    fn add_var(&mut self, (name, value): (&str, &str)) {
        let var_struct = Var::new(name, &Self::trim(value));

        let key = if name.chars().nth(0).unwrap().to_digit(10).is_some() {
            name.chars().filter(|c| {
                (*c == '0') | (*c == '1') | (*c == '2') | (*c == '3') | (*c == '4') | (*c == '5') | (*c == '6') | (*c == '7') | (*c == '8') | (*c == '9')
            }).collect::<String>().parse::<usize>().unwrap().to_string()
        }
        else {
            name.to_string()
        };


        self.add_var(var_struct);
    }
}

impl ContextUtils<Var> for Context {
    /// Adds a variable to the context.
    /// # Arguments
    /// * `var` - The Var struct to be added to the context.
    fn add_var(&mut self, var: Var) {
        self.vars.insert(var.name.to_string(), Rc::new(RefCell::new(var)));
    }
}
