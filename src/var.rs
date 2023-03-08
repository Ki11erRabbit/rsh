use std::collections::{BTreeMap, HashMap};
use std::default::Default;
use std::env;
use nix::unistd::{Pid, getpid, getppid, getuid, Uid};

const REG_USER_PROMPT: &str = "$ ";
const SUP_USER_PROMPT: &str = "# ";

pub trait VarDataUtils<V> {
    fn update_path(&mut self, path: V);
}

pub struct VarData {
    local_vars: HashMap<String, String>,
    local_var_stack: Vec<HashMap<String, String>>,
    var_table: BTreeMap<String, String>,
    important_vars: ImportantVars,
}

impl VarData {
    pub fn new() -> Self {
        Self {
            local_vars: HashMap::new(),
            local_var_stack: Vec::new(),
            var_table: BTreeMap::new(),
            important_vars: ImportantVars::default(),
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

        self.important_vars.path = path.split(":").map(|s| s.to_string()).collect();

    }
}

impl VarDataUtils<(&str, &str)> for VarData {
    fn update_path(&mut self, (key, path): (&str, &str)) {
        if key == "PATH" {
            env::set_var("PATH", path);
            self.important_vars.path = path.split(":").map(|s| s.to_string()).collect();
        }
    }
}

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
            path: vec!["/usr/local/sbin".to_string(),"/usr/local/bin".to_string(),"/usr/sbin:/usr/bin".to_string(),"/sbin:/bin".to_string()],
            history_location: String::new(),
            ps1,
            ps2: "> ".to_string(),
            ps4: "+ ".to_string(),
            ppid: getppid(),
            pid: getpid(),
        }
    }
}
