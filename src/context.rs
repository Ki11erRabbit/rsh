use std::collections::{HashMap,BTreeMap};
use std::default::Default;
use std::env;
use std::rc::Rc;
use std::cell::RefCell;

use nix::unistd::{getpid, getppid, getuid,Uid};

use crate::ast::FunctionBody;

const REG_USER_PROMPT: &str = "$ ";
const SUP_USER_PROMPT: &str = "# ";

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

    fn export(&mut self) {
        env::set_var(&self.name, &self.value);
    }
}
impl ToString for Var {
    fn to_string(&self) -> String {
        format!("{}={}", self.name, self.value)
    }
}

#[derive(Debug, Clone)]
pub struct ContextManager {
    context_stack: Vec<Rc<RefCell<Context>>>,
    exported_contexts: HashMap<String,Rc<RefCell<Context>>>,
}

impl ContextManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push_context(&mut self, context: Context) {
        self.context_stack.push(Rc::new(RefCell::new(context)));
    }
    pub fn push_context_new(&mut self) {
        self.context_stack.push(Rc::new(RefCell::new(Context::default())));
    }

    pub fn pop_context(&mut self) -> Option<Rc<RefCell<Context>>> {
        if self.context_stack.len() == 1 {//to prevent deleting the global context
            return None;
        }
        self.context_stack.pop()
    }

    pub fn get_context(&self) -> Rc<RefCell<Context>> {
        self.context_stack.last().unwrap().clone()
    }

    pub fn get_env_context(&self) -> Rc<RefCell<Context>> {
        self.context_stack.first().unwrap().clone()
    }


    pub fn add_context(&mut self, name: &str, context: Rc<RefCell<Context>>) {
        self.exported_contexts.insert(name.to_string(), context);
    }

    pub fn get_context_by_name(&self, name: &str) -> Option<Rc<RefCell<Context>>> {
        self.exported_contexts.get(name).cloned()
    }

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

    pub fn add_function(&mut self, name: &str, func: Rc<RefCell<FunctionBody>>) {
        self.get_context().borrow_mut().add_function(name, func);
    }

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

    pub fn add_var_pos(&mut self, set: &str, pos: usize) {
        self.context_stack[pos].borrow_mut().add_var(set);
    }

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

#[derive(Debug, Clone)]
pub struct Context {
    pub vars: HashMap<String, Rc<RefCell<Var>>>,
    functions: HashMap<String, Rc<RefCell<FunctionBody>>>,
}

impl Context {
    pub fn new(vars: HashMap<String, Rc<RefCell<Var>>>) -> Self {
        Self {
            vars,
            functions: HashMap::new(),
        }
    }

    pub fn get_var(&self, name: &str) -> Option<Rc<RefCell<Var>>> {
        self.vars.get(name).cloned()
    }

    pub fn get_function(&self, name: &str) -> Option<Rc<RefCell<FunctionBody>>> {
        self.functions.get(name).cloned()
    }

    pub fn add_function(&mut self, name: &str, body: Rc<RefCell<FunctionBody>>) {
        self.functions.insert(name.to_string(), body);
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

impl Default for Context {
    fn default() -> Self {
        Self {
            vars: HashMap::new(),
            functions: HashMap::new(),
        }
    }
}


impl ContextUtils<&str> for Context {

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
    fn add_var(&mut self, var: Var) {
        self.vars.insert(var.name.to_string(), Rc::new(RefCell::new(var)));
    }
}
