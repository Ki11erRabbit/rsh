use crate::ast::*;
use crate::jobs::Process;
use std::ffi::CString;
use crate::jobs::Job;
use crate::builtins;

use nix::errno::Errno;

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

    //block interrupts
    for command in pipeline.iter() {
        let process = eval_command(command)?;
        if process.is_none() {
            break;   
        }
        else {
            processes.push(process.unwrap());
        }
    }
    
    //make job

    if processes.len() > 0 {
        
        let mut pgid = None;

        let mut pip: (RawFd,RawFd) = (-1,-1);
        let mut prev_fd: RawFd = -1;
        let mut count: usize = 0;
        let proc_count = processes.len();
        for process in processes.iter_mut() {
            pip.1 = -1;
            if count < proc_count - 1 {
                let pipe_result = pipe();
                if pipe_result.is_err() {
                    return Err("Failed to create pipe");
                }
                pip = pipe_result.unwrap();
            }

            if temp_fork(process) == Ok(Pid::from_raw(0)) {
                if pgid.is_none() {
                    pgid = Some(getpid());
                }
                let pid = getpid();
                setpgid(pid, pgid.unwrap()).unwrap();

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

                temp_exec(process).unwrap();
                unreachable!();
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




        //let job = temp_exec(processes)?;

        for _ in 0..proc_count {
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
            builtins::quit().unwrap();
            Ok(None)
        },
        _ => {
            Err("Not implemented")
        }
    }
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

    match execv(&process.argv[0], &process.argv) {
        Ok(_) => {
            unreachable!();
        },
        Err(e) => {
            return Err(format!("Failed to execute: {}", e));
        }
    }

}




