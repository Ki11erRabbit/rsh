use crate::ast::*;
use crate::jobs::Process;
use std::ffi::CString;
use crate::jobs;
use crate::builtins;
use crate::shell;
use crate::trap;
use nix::errno::Errno;
use nix::sys::wait::WaitStatus;
use std::fs::OpenOptions;
use std::os::unix::io::AsRawFd;
use std::mem;
use std::env;
use std::sync::atomic::{AtomicI32, Ordering};

use std::os::unix::io::RawFd;
use nix::unistd::{close, dup2, pipe,execv, fork, getpid, setpgid, ForkResult, Pid};

/// Stores the exit status of the last command
pub static mut EXIT_STATUS: AtomicI32 = AtomicI32::new(0);

/// gets the exit status of the last command
pub fn get_exit_status() -> WaitStatus {
    unsafe {
        WaitStatus::from_raw(Pid::from_raw(-1),EXIT_STATUS.load(Ordering::Relaxed)).unwrap()
    }
}
/// sets the exit status of the last command
pub fn set_exit_status(status: i32) {
    unsafe {
        EXIT_STATUS.store(status, Ordering::Relaxed);
    }
}
/// gets the exit code of the last command
pub fn get_exit_code() -> i32 {
    unsafe {
        EXIT_STATUS.load(Ordering::Relaxed)
    }
}

/// This is where we start evaluating an AST
/// We extract the list out of the CompleteCommand then parse the list in
/// the function parse_tree
pub fn eval(ast: &mut CompleteCommand) -> Result<i32,&'static str> {

    let list = match &mut ast.list {
        Some(list) => list,
        None => return Ok(0),
    };

    let result = parse_tree(&mut list.0)?;

    Ok(result)
}

/// This is where we evaluate a CompletCommand's list which is a Vec<AndOr>
/// We iterate through the list and evaluate each AndOr in the list
/// We return the status of the last AndOr in the list
fn parse_tree(list: &mut Vec<AndOr>) -> Result<i32,&'static str> {
    let mut status = -1;

    for and_or in list.iter_mut() {

        status = eval_and_or(and_or)?;
    } 

    Ok(status)
}

/// This is where we evaluate an AndOr
/// We check if the AndOr has a conditional AndOr and if id does
/// we evaluate the conditional AndOr and check the status and evaluate the correct branch
/// After we evaluate the conditonal AndOr (if it exists) and the correct branch
/// we evaluate the pipeline and return the status of the pipeline.
fn eval_and_or(and_or: &mut AndOr) -> Result<i32,&'static str> {

    if and_or.and_or.is_none() {
        return eval_pipeline(&mut and_or.pipeline);
    }
    else {
        let status = eval_and_or(and_or.and_or.as_mut().unwrap())?;
        match and_or.conditional_exec {
            Some(ConditionalExec::And) => {
                if status != 0 {
                    return Ok(status);
                }
            },
            Some(ConditionalExec::Or) => {
                if status == 0 {
                    return Ok(status);
                }
            },
            None => {
                return Err("Error: Conditional Execution requires an operator");
            }
        }
    }

    let status = eval_pipeline(&mut and_or.pipeline)?;
    Ok(status)
}

/// This is where we evaluate a pipeline
/// We iterate through the pipeline and evaluate each command in the pipeline
/// If the evaluation creates Processes we then create a Job and execute the job.
/// If the job is not a background job we wait for the job to finish and return the status
/// Functions are handled here in a special way. If the function is not in the background, we
/// execute the function before going into the the fork and exec loop, removing it from the pipeline.
/// We also have to remove the job from the shell to prevent a panic when trying to wait for the job.
/// If the function is in the background we just add it to the pipeline and let the fork and exec loop
/// handle it.
/// Interupts are blocked during the fork and exec part to prevent being interupted by a signal.
fn eval_pipeline(pipeline: &mut Pipeline) -> Result<i32,&'static str> {

    let background = pipeline.background;
    let mut pipeline: &mut PipeSequence = &mut pipeline.pipe_sequence;
    
    let mut processes = Vec::new();
    let mut commands = Vec::new();

    //block interrupts
    trap::interrupts_off();
    for command in pipeline.iter_mut() {
        let process = eval_command(command)?;
        if process.is_none() {
            break;   
        }
        else {
            let (process, smc) = process.unwrap();
            processes.push(process);
            commands.push(smc);
        }
    }
    
    if processes.len() == 0 {
        return Ok(0);
    }
   
    let mut remove_index: Vec<usize> = Vec::new();
    
    if !background {
        for (index,command) in commands.iter_mut().enumerate() {
            if shell::is_function(&command.name) {
                remove_index.push(index);
            
                let result = eval_function(command);
                
                match result {
                    Ok(_) => {},
                    Err(_) => {
                        eprintln!("{}: Command not found\n", command.name);
                        std::process::exit(1);
                    }
                }
                if shell::get_forked() {
                    std::process::exit(0);
                }
                else {
                    set_exit_status(0);
                }
            }

        }
        for index in remove_index.iter().rev() {
            processes.remove(*index);
            commands.remove(*index);
        }
    }

    

    let proc_count;
    // this code block is to ensure that the mutable borrow is dropped before sigchld is handled
        let job = shell::create_job(processes, background);
    {
        let mut job = job.borrow_mut();
        let job_id = job.job_id;
        let procs = job.borrow_processes_mut();

            
        let mut pgid = None;

        let mut pip: (RawFd,RawFd) = (-1,-1);
        let mut prev_fd: RawFd = -1;
        let mut count: usize = 0;
        proc_count = procs.len();
        for process in procs.iter_mut() {
            pip.1 = -1;
            if count < proc_count - 1 {
                let pipe_result = pipe();
                if pipe_result.is_err() {
                    return Err("Failed to create pipe");
                }
                pip = pipe_result.unwrap();
            }
            
            

            let temp_fork_result = temp_fork(process);
            if temp_fork_result == Ok(Pid::from_raw(0)) {
                if pgid.is_none() {
                    pgid = Some(getpid());
                }
                let pid = getpid();
                if background {
                    setpgid(pid, pgid.unwrap()).unwrap();
                }

                //unblock interrupts
                if pip.1 >= 0 {
                    close(pip.0).unwrap();
                }
                if prev_fd > 0 {
                    dup2(prev_fd, 0).unwrap();
                    close(prev_fd).unwrap();
                }
                if pip.1 > 1 {
                    dup2(pip.1, 1).unwrap();
                    close(pip.1).unwrap();
                }

                eval_prefix_suffix(commands[count].prefix_suffix());
                
                //println!("executing: {:?}", process.argv);
                
                if shell::is_function(&commands[count].name) {
                    let result = eval_function(&mut commands[count]);
                    
                    match result {
                        Ok(_) => {},
                        Err(_) => {
                            eprintln!("{}: Command not found\n", commands[count].name);
                            std::process::exit(1);
                        }
                    }
                    std::process::exit(0);
                }
                else {
                    match temp_exec(process) {
                        Ok(_) => {},
                        Err(_) => { 
                            eprintln!("{}: Command not found\n", process.cmd);
                            std::process::exit(1);
                        },
                    }
                    unreachable!();
                }
            }
            else if matches!(temp_fork_result, Ok(_)) {
                shell::update_pid_table(job_id, temp_fork_result.unwrap());
            }
            if prev_fd >= 0 {
                close(prev_fd).unwrap();
            }
            prev_fd = pip.0;
            match close(pip.1) {
                Ok(_) => {},
                Err(_) => {}
            }

            count += 1;
        }
        
    }
        if !background && proc_count > 0 && job.borrow().processes.len() > 0{
            //eprintln!("waiting for job");
            let status = jobs::wait_for_job(Some(job.clone()));
            let id;
            {
                let job = job.borrow();
                id = job.job_id;
            }
            shell::delete_job(id);

            match status {
                WaitStatus::Exited(_, status) => {
                    set_exit_status(status);
                },
                _ => (),
            }
        }
        else if background && proc_count > 0  && job.borrow().processes.len() > 0 {
            println!("[{}] ({}) {}", job.borrow().job_id, job.borrow().processes[0].pid, job.borrow());
        }
        else if proc_count == 0 {
            let id = {
                let job = job.borrow();
                job.job_id
            };
            shell::delete_job(id);
        }

    //let job = temp_exec(processes)?;

    
    //unblock interrupts
    trap::interrupts_on();

    Ok(get_exit_code())
}

/// This command evaluates a Command and returns a None if the command is a function definition or a shell builtin.
/// if the command is not one of the above two then it returns a tuple with a Process and the SimpleCommand that
/// made the Process.
fn eval_command(command: &mut Command) -> Result<Option<(Process,SimpleCommand)>,&'static str> {

    match command {
        Command::SimpleCommand(simple_command) => {
            return eval_simple_command(simple_command);
        },
        Command::FunctionDefinition(function_definition) => {
            return eval_function_definition(function_definition);
        },
        _ => {
            return Err("Not implemented");
        }
        
    }

}

/// This function evaluates a function definition and adds it to the current context's function table.
fn eval_function_definition(function_definition: &mut FunctionDefinition) -> Result<Option<(Process,SimpleCommand)>,&'static str> {
    let name = &function_definition.name;
    let body = function_definition.function_body.clone();
    shell::add_function(&name, body);
    Ok(None)
}

/// This function evaluates a simple command and returns a tuple with a Process and the SimpleCommand that
/// made the Process.
fn eval_simple_command(simple_command: &mut SimpleCommand) -> Result<Option<(Process, SimpleCommand)>,&'static str> {
   
    if check_if_builtin(&simple_command.name) {
        return eval_builtin(simple_command);
    }

    simple_command.remove_double_quotes();
    simple_command.expand_subshells();
    simple_command.alias_lookup();
    simple_command.expand_vars();
    simple_command.remove_whitespace();
    simple_command.remove_single_quotes();

    //todo deal with redirection and assignment
    let argv: Vec<CString> = simple_command.argv();

    let process = Process::new(argv,simple_command.name.clone(),simple_command.cmd());

    Ok(Some((process,simple_command.clone())))// this clone is bad and should be replaced
}

/// This function evaluates a SimpleCommand's Prefix and Suffix.
fn eval_prefix_suffix(prefix_suffix: (Option<&Prefix>, Option<&Suffix>)) {
    let (prefix, suffix) = prefix_suffix;
    if prefix.is_some() {
        eval_redirect(&prefix.unwrap().io_redirect);
        eval_assignment(&prefix.unwrap().assignment);
    }
    if suffix.is_some() {
        eval_redirect(&suffix.unwrap().io_redirect);
    }
}

/// This function evaluates an assignment in a SimpleCommand's Prefix.
fn eval_assignment(assignment: &Vec<String>) {
    for assign in assignment.iter() {
        let assign = assign.split('=').collect::<Vec<&str>>();
        env::set_var(&assign[0], &assign[1]);
    }
}

/// This function evaluates an IoRedirect in a SimpleCommand's Prefix or Suffix.
fn eval_redirect(redirect: &Vec<IoRedirect>) {
    for redir in redirect.iter() {
        if redir.io_file.is_some() {
            let io_file = redir.io_file.as_ref().unwrap();
            match &io_file.redirect_type {
                RedirectType::Input => {
                    let file = OpenOptions::new()
                        .read(true)
                        .open(&io_file.filename)
                        .unwrap();
                    
                    dup2(file.as_raw_fd(), 0).unwrap();
                    mem::forget(file);
                },
                RedirectType::Output => {
                    let file = OpenOptions::new()
                        .write(true)
                        .create(true)
                        .open(&io_file.filename)
                        .unwrap();
                    dup2(file.as_raw_fd(), 1).unwrap();
                    mem::forget(file);
                },
                RedirectType::Append => {
                    let file = OpenOptions::new()
                        .write(true)
                        .append(true)
                        .create(true)
                        .open(&io_file.filename)
                        .unwrap();
                    dup2(file.as_raw_fd(), 1).unwrap();
                    mem::forget(file);
                },
                _ => (),
            }
        }
    }
}

/// This function checks if a command is a shell builtin.
fn check_if_builtin(cmd_name: &str) -> bool {
    match cmd_name {
        "cd" => true,
        "exit" => true,
        "jobs" => true,
        "fg" | "bg" => true,
        "alias" | "unalias" => true,
        "export" => true,
        "return" => true,
        "" => true,
        _ => false
    }
}

/// This function evaluates a shell builtin. We should handle the error properly here.
fn eval_builtin(command: &SimpleCommand) -> Result<Option<(Process,SimpleCommand)>,&'static str> {
    match command.name.as_str() {
        "cd" => {
            builtins::change_directory(command).unwrap();//TODO: properly handle error
            Ok(None)
        },
        "exit" => {
            builtins::quit(command).unwrap();
            Ok(None)
        },
        "jobs" => {
            builtins::jobs().unwrap();
            Ok(None)
        },
        "fg" | "bg"=> {
            builtins::fgbg(command).unwrap();
            Ok(None)
        },
        "alias" => {
            builtins::alias(command).unwrap();
            Ok(None)
        },
        "unalias" => {
            builtins::unalias(command).unwrap();
            Ok(None)
        },
        "export" => {
            builtins::export(command).unwrap();
            Ok(None)
        },
        "return" => {
            builtins::return_cmd(command).unwrap();
            Ok(None)
        },
        "" => {
            builtins::assignment(command).unwrap();
            Ok(None)
        }
        _ => {
            Err("Not implemented") 
        }
    }
}

/// This function checks if a command is a shell function.
fn check_if_function(cmd_name: &str) -> bool {
    shell::is_function(cmd_name)
}

/// This function evaluates a shell function.
fn eval_function(command: &mut SimpleCommand) -> Result<i32,&'static str> {
    let function = shell::get_function(&command.name);
    if function.is_none() {
        return Err("Function not found");
    }
    //eprintln!("{:?}", function.clone().unwrap());
    //eprintln!("{:?}", command);
    shell::push_context_new();
    shell::add_var_context(&format!("0={}", command.name));
    //eprintln!("0={}", command.name);
    if command.suffix.is_some() {
        let suffix = command.suffix.as_ref().unwrap();
        for (i, arg) in suffix.word.iter().enumerate() {
            if arg == "&" {
                break;
            }
            //eprintln!("{}={}", i+1, arg);
            shell::add_var_context(&format!("{}={}", i+1, arg));
        }
    }


    eval_compound_command(&mut function.unwrap().borrow_mut().compound_command);
    shell::pop_context();

    Ok(0)//todo: change this to return the right exit code
}

/// This function evaluates a CompoundCommand.
fn eval_compound_command(command: &mut CompoundCommand) {
    match command {
        CompoundCommand::BraceGroup(bg) => {
            eval_compound_list(&mut bg.0);
        }
        _ => unimplemented!(),
    }
}

/// This function evaluates a CompoundList.
fn eval_compound_list(compound_list: &mut CompoundList) {
    parse_tree(&mut compound_list.0.0);
}

/// This function is a wraper for fork().
/// It also sets up the command's pid in the parent and sets the shell's forked flag in the child.
fn temp_fork(command: &mut Process) -> Result<Pid,Errno> {
    match unsafe {fork()}?{
        ForkResult::Parent { child } => {
            //need to add in logic for setting up jobs
            command.set_pid(child);
            return Ok(child);
        },
        ForkResult::Child => {
            shell::set_forked(true);

            return Ok(Pid::from_raw(0));
        }
    }
}

/// This function is a wrapper for execv().
fn temp_exec(process: &mut Process) -> Result<i32,String> {

    //set env
    //
    //set assignments
  
    jobs::fork_reset();

    let argv0 = match shell::lookup_command(&process.argv0) {
        Some(cmd) => cmd,
        None => {
            return Err(format!("{}: Command not found", process.argv0));
        }
    };

    let command = CString::new(argv0).unwrap();


    match execv(&command, &process.argv) {
        Ok(_) => {
            unreachable!();
        },
        Err(e) => {
            return Err(format!("Failed to execute: {}", e));
        }
    }

}




