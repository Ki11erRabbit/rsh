


pub struct Shell {
    // variables
    local_vars: HashMap<String, LocalVar>,
    local_var_stack: Vec<HashMap<String, LocalVar>>,
    var_table: BTreeMap<String, Var>,
    // directory
    curr_directory: String,
    physical_directory: String,
    //jobs
    jobctl: bool,
    job_warning: i32,
    background_pid: Pid,
    vforked: bool,
    tty_fd: i32,
    job_table: Vec<Job>,
    current_job: Option<Job>,
    //output
    output: Output,
    errout: Output,
    //misc
    root_pid: Pid,
    path: String,
}
