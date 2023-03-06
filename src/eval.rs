use crate::ast::*;
use crate::jobs::Process;
use std::ffi::CString;
use crate::jobs::Job;


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
        processes.push(process);
    }

    let job = temp_exec(processes)?;

    for process in job.processes.iter() {
        wait().unwrap();
    }


    Ok(0)
}

fn eval_command(command: &Command) -> Result<Process,&'static str> {

    match command {
        Command::SimpleCommand(simple_command) => {
            return eval_simple_command(simple_command);
        },
        _ => {
            return Err("Not implemented");
        }
        
    }

}

fn eval_simple_command(simple_command: &SimpleCommand) -> Result<Process,&'static str> {
    
    //todo deal with redirection and assignment
    let mut argv: Vec<CString> = simple_command.argv();

    let process = Process::new(argv);

    Ok(process)
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




