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

pub static mut EXIT_STATUS: AtomicI32 = AtomicI32::new(0);

pub fn get_exit_status() -> WaitStatus {
    unsafe {
        WaitStatus::from_raw(Pid::from_raw(-1),EXIT_STATUS.load(Ordering::Relaxed)).unwrap()
    }
}
pub fn set_exit_status(status: i32) {
    unsafe {
        EXIT_STATUS.store(status, Ordering::Relaxed);
    }
}


pub fn eval(ast: &mut CompleteCommand) -> Result<i32,&'static str> {

    let list = match &mut ast.list {
        Some(list) => list,
        None => return Ok(0),
    };

    let result = parse_tree(list)?;

    Ok(result)
}

fn parse_tree(list: &mut List) -> Result<i32,&'static str> {
    let mut status = -1;
    for and_or in list.0.iter_mut() {
        status = eval_pipeline(&mut and_or.pipeline)?;
    } 

    Ok(status)
}

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
                match temp_exec(process) {
                    Ok(_) => {},
                    Err(_) => {
                        eprintln!("{}: Command not found\n", process.cmd);
                        std::process::exit(1);
                    },
                }
                unreachable!();
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
        if !background {
            //eprintln!("waiting for job");
            jobs::wait_for_job(Some(job.clone()));
            let id;
            {
                let job = job.borrow();
                id = job.job_id;
            }
            shell::delete_job(id);
        }
        else {
            println!("[{}] ({}) {}", job.borrow().job_id, job.borrow().processes[0].pid, job.borrow());
        }

    //let job = temp_exec(processes)?;

    
    //unblock interrupts
    trap::interrupts_on();

    Ok(0)
}


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

fn eval_function_definition(function_definition: &mut FunctionDefinition) -> Result<Option<(Process,SimpleCommand)>,&'static str> {
    let name = &function_definition.name;
    let body = function_definition.function_body.clone();
    shell::add_function(&name, body);
    Ok(None)
}

fn eval_simple_command(simple_command: &mut SimpleCommand) -> Result<Option<(Process, SimpleCommand)>,&'static str> {
   
    if check_if_builtin(&simple_command.name) {
        return eval_builtin(simple_command);
    }

    simple_command.alias_lookup();
    simple_command.expand_vars();
    simple_command.remove_quotes();

    //todo deal with redirection and assignment
    let argv: Vec<CString> = simple_command.argv();

    let process = Process::new(argv,simple_command.name.clone(),simple_command.cmd());

    Ok(Some((process,simple_command.clone())))// this clone is bad and should be replaced
}

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

fn eval_assignment(assignment: &Vec<String>) {
    for assign in assignment.iter() {
        let assign = assign.split('=').collect::<Vec<&str>>();
        env::set_var(&assign[0], &assign[1]);
    }
}

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


fn check_if_builtin(cmd_name: &str) -> bool {
    match cmd_name {
        "cd" => true,
        "exit" => true,
        "jobs" => true,
        "fg" | "bg" => true,
        "alias" | "unalias" => true,
        "export" => true,
        "" => true,
        _ => false
    }
}

fn eval_builtin(command: &SimpleCommand) -> Result<Option<(Process,SimpleCommand)>,&'static str> {
    match command.name.as_str() {
        "cd" => {
            builtins::change_directory(command).unwrap();//TODO: properly handle error
            Ok(None)
        },
        "exit" => {
            builtins::quit().unwrap();
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
        "" => {
            builtins::assignment(command).unwrap();
            Ok(None)
        }
        _ => {
            Err("Not implemented") 
        }
    }
}

fn check_if_function(cmd_name: &str) -> bool {
    shell::is_function(cmd_name)
}

fn eval_function(command: &SimpleCommand) -> Result<Option<(Process,SimpleCommand)>,&'static str> {
    let function = shell::get_function(&command.name);
    if function.is_none() {
        return Err("Function not found");
    }
    let mut background = false;
    shell::push_var_stack();
    shell::add_var_context(&format!("0={}", command.name));
    if command.suffix.is_some() {
        let suffix = command.suffix.as_ref().unwrap();
        for (i, arg) in suffix.word.iter().enumerate() {
            if arg == "&" {
                background = true;
                break;
            }
            shell::add_var_context(&format!("{}={}", i+1, arg));
        }
    }

    if background {
        //TODO fork and run in background
    }

    eval_compound_command(&function.unwrap().compound_command);


    Ok(None)
}

fn eval_compound_command(command: &CompoundCommand) {

}

fn temp_fork(command: &mut Process) -> Result<Pid,Errno> {
    match unsafe {fork()}?{
        ForkResult::Parent { child } => {
            //need to add in logic for setting up jobs
            command.set_pid(child);
            return Ok(child);
        },
        ForkResult::Child => {
            return Ok(Pid::from_raw(0));
        }
    }
}


fn temp_exec(process: &mut Process) -> Result<i32,String> {

    //set env
    //
    //set assignments
   
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




