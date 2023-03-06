use crate::ast::*;
use crate::jobs::Process;
use std::ffi::CString;
use crate::jobs::Job;
use crate::builtins;


use std::os::unix::io::RawFd;
use nix::unistd::{close, dup2, pipe,execv, fork, getpid, setpgid, ForkResult, Pid};
use nix::sys::wait::wait;






pub fn eval(ast: &CompleteCommand) -> Result<i32,&'static str> {

    let result = parse_tree(&ast.list)?;

    Ok(result)
}

fn parse_tree(list: &List) -> Result<i32,&'static str> {
    let mut status = -1;
    for and_or in list.0.iter() {
        status = eval_pipeline(&and_or.pipeline)?;
    } 

    Ok(status)
}

fn eval_pipeline(pipeline: &Pipeline) -> Result<i32,&'static str> {

    let pipeline: &PipeSequence = &pipeline.pipe_sequence;
    
    let mut processes: Vec<Process> = Vec::new();

    for command in pipeline.iter() {
        let process = eval_command(command)?;
        if process.is_none() {
            break;   
        }
        else {
            processes.push(process.unwrap());
        }
    }
    
    if processes.len() > 0 {
        let job = temp_exec(processes)?;

        for process in job.processes.iter() {
            wait().unwrap();
        }
    }


    Ok(0)
}

fn eval_command(command: &Command) -> Result<Option<Process>,&'static str> {

    match command {
        Command::SimpleCommand(simple_command) => {
            return eval_simple_command(simple_command);
        },
        _ => {
            return Err("Not implemented");
        }
        
    }

}

fn eval_simple_command(simple_command: &SimpleCommand) -> Result<Option<Process>,&'static str> {
   
    if check_if_builtin(&simple_command.name) {
        return eval_builtin(simple_command);
    }


    //todo deal with redirection and assignment
    let argv: Vec<CString> = simple_command.argv();

    let process = Process::new(argv);

    Ok(Some(process))
}

fn check_if_builtin(cmd_name: &str) -> bool {
    match cmd_name {
        "cd" => true,
        "exit" => true,
        _ => false,
    }
}

fn eval_builtin(command: &SimpleCommand) -> Result<Option<Process>,&'static str> {
    match command.name.as_str() {
        "cd" => {
            builtins::change_directory(command).unwrap();//TODO: properly handle error
            Ok(None)
        },
        "exit" => {
            builtins::quit();
            Ok(None)
        },
        _ => {
            Err("Not implemented")
        }
    }
}

fn temp_exec(processes: Vec<Process>) -> Result<Job,&'static str> {
    let mut procs = processes;

    let mut pgid = None;

    for process in procs.iter_mut() {
        

        match unsafe{fork()}.expect("Fork failed") {
            ForkResult::Parent { child } => {
                process.set_pid(child);
            },
            ForkResult::Child => {
                if pgid.is_none() {
                    pgid = Some(getpid());
                }
                let pid = getpid();
                setpgid(pid, pgid.unwrap()).unwrap();


                //set env
                //
                //set assignments

                match execv(&process.argv[0], &process.argv) {
                    Ok(_) => {
                        unreachable!();
                    },
                    Err(e) => {
                        println!("Failed to execute: {}", e);
                    }
                }
            }
            
        }

    }

   Ok(Job::new(procs))
}




