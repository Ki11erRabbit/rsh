use crate::eval::{Context, Result};
use crate::parser::{self, Assignment, Command,Ast, BinaryExpression, ConditionalExpression, Expression, Initializer, LocalDeclaration, RunIf, Word, RedirectionType};
use crate::process::{self, CommandExitStatus};
use std::fs::OpenOptions;
use nix::unistd::{close, dup2, execv, fork, getpid, setpgid, ForkResult, Pid};
use std::ffi::CString;

use std::os::unix::io::FromRawFd;
use std::os::unix::io::RawFd;



pub fn exec(argv: Vec<String>, redirects: &[parser::Redirection], assignments: &[Assignment], context: &Context) -> Result<CommandExitStatus> {

    //let mut fds = Vec::new();

    /*for r in redirects {
        match r.target {
            parser::RedirectionType::Fd(ref fd) => {
                fds.push((*fd, r.fd as RawFd));
            },
            parser::RedirectionType::File(ref wfilepath) => {
                let mut options = OpenOptions::new();
                match &r.direction {
                    parser::RedirectionDirection::Input => {
                        options.read(true);
                    },
                    parser::RedirectionDirection::Output => {
                        options.write(true);
                        options.create(true);
                    },
                    parser::RedirectionDirection::Append => {
                        options.write(true);
                        options.create(true);
                        options.append(true);
                    },
                };

                let filepath = expand_word_into_string(wfilepath)?;
                if let Ok(file) = option.open(&filepath) {
                    fds.push((file.into_raw_fd(), rr.fd as RawFd));
                } else {
                    eprintln!("Failed to open file {}", filepath);
                    return Ok(CommandExitStatus::ExitedWith(1));
                }
            },
            _ => unimplemented!(),
        }
    }*/

    let argv0 = CString::new(argv[0].as_str()).unwrap();

    let mut args = Vec::new();
    for arg in argv {
        args.push(CString::new(arg.as_str()).unwrap());
    }

    match unsafe {fork()}.expect("failed to fork") {
        ForkResult::Parent { child } => {
            return Ok(CommandExitStatus::Running(child));
        },
        ForkResult::Child => {
            let pid = getpid();
            setpgid(pid, pid).expect("failed to setpgid");

            //set env

            //set assignments

            let args: Vec<&std::ffi::CStr> = args.iter().map(|s| s.as_c_str()).collect();
            match execv(&argv0, &args) {
                    Ok(_) => {
                        unreachable!();
                    }
                    Err(nix::errno::Errno::EACCES) => {
                        eprintln!("Failed to exec {:?} (EACCESS). chmod(1) may help.", argv0);
                        std::process::exit(1);
                    }
                    Err(err) => {
                        eprintln!("Failed to exec {:?} ({})", argv0, err);
                        std::process::exit(1);
                    }
                }

            }
    }
}
