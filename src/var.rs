use std::collections::{BTreeMap, HashMap};
use std::default::Default;
use std::env;
use nix::unistd::{Pid, getpid, getppid, getuid, Uid};
use std::fs::metadata;

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
}

impl ToString for Var {
    fn to_string(&self) -> String {
        format!("{}={}", self.name, self.value)
    }
}




pub trait VarDataUtils<V> {
    fn update_path(&mut self, path: V);
    fn add_var(&mut self, var: V, position: isize);
}

#[derive(Debug, Clone)]
pub struct VarData {
    local_vars: HashMap<String, Var>,
    local_var_stack: Vec<HashMap<String, Var>>,
    var_table: BTreeMap<String, Var>,
    //important_vars: ImportantVars,
}



impl VarData {
    pub fn new() -> Self {
        let mut val = Self {
            local_vars: HashMap::new(),
            local_var_stack: Vec::new(),
            var_table: BTreeMap::new(),
            //important_vars: ImportantVars::default(),
        };

        val.convert_env();
       
        let ps1 = if Uid::is_root(getuid()) {
            SUP_USER_PROMPT.to_string()
        } else {
            REG_USER_PROMPT.to_string()
        };
        val.add_var("PS1", Var::new("PS1", &ps1), -1);
        val.add_var("PS2", Var::new("PS2", "> "), -1);
        val.add_var("PS4", Var::new("PS4", "+ "), -1);
        val.add_var("PPID", Var::new("PPID", &getppid().to_string()), -1);
        val.add_var("PID", Var::new("PID", &getpid().to_string()), -1);
        val.add_var("PATH", Var::new("PATH", "/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin"), -1);
        
        val
    }

    pub fn get_current_context_pos(&self) -> isize {
        self.local_var_stack.len() as isize - 1
    }

    fn convert_env(&mut self) {
        for (key, value) in env::vars() {
            let var = Var::new(&key, &value);
            self.add_var(&key, var, -1);
        }
    }

    fn path(&self) -> Vec<&str> {
        self.lookup_internal("PATH").unwrap().split(':').collect()
    }

    fn add_var(&mut self, key: &str, value: Var, pos: isize) {
        match pos {
            -1 => {
                self.local_vars.insert(key.to_string(), value.clone());
            },
            _ => {
                if pos < self.local_var_stack.len() as isize {
                    self.local_var_stack[pos as usize].insert(key.to_string(), value.clone());
                } else {
                    self.push_var_stack();
                    self.local_var_stack[pos as usize].insert(key.to_string(), value.clone());
                }
            }
        }

        self.var_table.insert(key.to_string(), value.clone());
    

    }

    pub fn push_var_stack(&mut self) {
        self.local_var_stack.push(HashMap::new());
    }
    pub fn pop_var_stack(&mut self) {
        match self.local_var_stack.pop() {
            None => (),
            Some(table) => {
                for (key, _) in table.iter() {
                    self.var_table.remove(key);
                }
            }
        }
    }
    
    fn lookup_internal(&self, key: &str) -> Option<&str> {
        let mut ret = self.var_table.get(key);
        match ret {
            Some(var) => Some(&var.value),
            None => {
                ret = self.local_vars.get(key);
                match ret {
                    Some(var) => Some(&var.value),
                    None => {
                        for table in self.local_var_stack.iter().rev() {
                            ret = table.get(key);
                            match ret {
                                Some(var) => return Some(&var.value),
                                None => (),
                            }
                        }
                        None
                    }
                }
            }
        }
    }

    pub fn lookup_var(&self, key: &str) -> Option<String> {
        self.lookup_internal(key).map(|s| s.to_string())
    }

    pub fn lookup_command(&self, cmd: &str) -> Option<String> {
        for path in self.path().iter() {
            let path = format!("{}/{}", path, cmd);
            
            let metadata = metadata(&path);
            if metadata.is_err() {
                continue;
            }
            else {
                let metadata = metadata.unwrap();
                if metadata.is_file() {
                    return Some(path);
                }
            }
        }

        None
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

impl VarDataUtils<&str> for VarData {
    fn update_path(&mut self, path: &str) {
        let path = if path.contains("=") {
            let mut split = path.split("=");
            split.next();
            split.next().unwrap()
        } else {
            path
        };

        env::set_var("PATH", path);

        //self.important_vars.path = path.split(":").map(|s| s.to_string()).collect();

    }

    fn add_var(&mut self, var: &str,pos: isize) {
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


        self.add_var(&key, var_struct, pos);
    } 

    
}

impl VarDataUtils<(&str, &str)> for VarData {
    fn update_path(&mut self, (key, path): (&str, &str)) {
        if key == "PATH" {
            env::set_var("PATH", path);
            //self.important_vars.path = path.split(":").map(|s| s.to_string()).collect();
        }
    }

    fn add_var(&mut self, (name, value): (&str, &str), pos: isize) {
        let var_struct = Var::new(name, &Self::trim(value));

        let key = if name.chars().nth(0).unwrap().to_digit(10).is_some() {
            name.chars().filter(|c| {
                (*c == '0') | (*c == '1') | (*c == '2') | (*c == '3') | (*c == '4') | (*c == '5') | (*c == '6') | (*c == '7') | (*c == '8') | (*c == '9')
            }).collect::<String>().parse::<usize>().unwrap().to_string()
        }
        else {
            name.to_string()
        };


        self.add_var(&key, var_struct, pos);
    }
}

#[derive(Debug, Clone)]
pub struct ImportantVars {
    pub home: String,
    pub pwd: String,
    pub path: Vec<String>,
    pub history_location: String,
    pub ps1: String,
    pub ps2: String,
    pub ps4: String,
    pub ppid: Pid,
    pub pid: Pid,
}

impl ImportantVars {
    pub fn make_into_vars(self) -> Vec<String> {
        let mut vars = Vec::new();
        vars.push(format!("HISTFILE={}", self.history_location));
        vars
    }

    pub fn export_env_vars(&self) {
        env::set_var("HOME", &self.home);
        env::set_var("PWD", &self.pwd);
        env::set_var("PATH", &self.path.join(":"));
        env::set_var("PS1", &self.ps1);
        env::set_var("PS2", &self.ps2);
        env::set_var("PS4", &self.ps4);
        env::set_var("PPID", self.ppid.to_string());
        env::set_var("PID", self.pid.to_string());
    }

    pub fn set_path(&mut self, path: &str) {
        self.path = path.split(":").map(|s| s.to_string()).collect();
        
    }

    pub fn set_var(&mut self, var: Var) {
        match var.name.as_str() {
            "HISTFILE" => self.history_location = var.value,
            _ => (),
        }
    }
}

impl Default for ImportantVars {
    fn default() -> Self {

        let ps1 = if Uid::is_root(getuid()) {
            SUP_USER_PROMPT.to_string()
        } else {
            REG_USER_PROMPT.to_string()
        };

        Self {
            home: env::var("HOME").unwrap_or_else(|_| "/root".to_string()),
            pwd: env::current_dir().unwrap().to_str().unwrap().to_string(),
            path: vec!["/bin".to_string(),"/usr/local/sbin".to_string(),"/usr/local/bin".to_string(),"/usr/sbin".to_string(),"/usr/bin".to_string(),"/sbin:/bin".to_string()],
            history_location: String::new(),
            ps1,
            ps2: "> ".to_string(),
            ps4: "+ ".to_string(),
            ppid: getppid(),
            pid: getpid(),
        }
    }
}
