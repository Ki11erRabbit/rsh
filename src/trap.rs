use std::sync::atomic::{AtomicBool, Ordering};
use std::io::{self,Write};
use nix::sys::signal;
use nix::sys::signal::Signal;
use std::os::raw::c_int;
use nix::sys::wait::{WaitStatus,WaitPidFlag,waitpid};
use nix::unistd::Pid;

use crate::jobs::{Job, Process, JobState};
use crate::shell;

const S_DFL: usize = 1;
const S_CATCH: usize = 2;
const S_IGN: usize = 3;
const S_HARD_IGN: usize = 4;
const S_RESET: usize = 5;

static mut GOT_SIGCHLD: AtomicBool = AtomicBool::new(false);
static mut SIGINT_PENDING: AtomicBool = AtomicBool::new(false);
static mut SUPRESS_SIGINT: AtomicBool = AtomicBool::new(false);
static mut PENDING_SIGNAL: Option<Signal> = None;


pub fn set_got_sigchld(val: bool) {
    unsafe {
        GOT_SIGCHLD.store(val, Ordering::Relaxed);
    }
}

pub fn got_sigchld() -> bool {
    unsafe {
        GOT_SIGCHLD.load(Ordering::Relaxed)
    }
}

pub fn get_pending_signal() -> Option<Signal> {
    unsafe {
        PENDING_SIGNAL
    }
}


extern "C" fn on_sig(sig_num: c_int) {
    if shell::vforked() {
        return;
    }
    unsafe {
        if sig_num == signal::SIGCHLD as c_int {
            GOT_SIGCHLD.store(true, Ordering::Relaxed);
            //sig_chld();//this is different from how dash does it but it should allow for for traps to be set
            if !shell::is_trap_set(signal::SIGCHLD) {
                return;
            }
        }
    }

    //set which signal is got
    //set pending signal
    shell::set_got_sig(sig_num);
    shell::set_pending_signal(sig_num);


    if sig_num == signal::SIGINT as c_int && !shell::is_trap_set(signal::SIGINT) {
        nix::unistd::write(0,&[0xA as u8]).unwrap();
        //io::stdin().read_line(&mut String::new()).unwrap();
        unsafe {
            if !SUPRESS_SIGINT.load(Ordering::Relaxed) {
                on_sigint();
            }
            SIGINT_PENDING.store(true, Ordering::Relaxed);
        }
        return;
    }
}

// signal handler for SIGCHLD
fn sig_chld() {
    let child_pid: Pid = Pid::from_raw(-1);

    let flag: WaitPidFlag = WaitPidFlag::WNOHANG | WaitPidFlag::WUNTRACED;
    
    loop {
        match waitpid(child_pid, Some(flag)) {
            Ok(WaitStatus::Exited(pid, status)) => {
                shell::delete_job(pid);
            },
            Ok(WaitStatus::Signaled(pid, signal, _)) => {
                let job = shell::get_job(pid);
                let job = match job {
                    Some(job) => job,
                    None => {
                        continue;
                    },
                };
                
                {
                    let mut job = job.borrow_mut();

                    let msg = format!("Job [{}] ({}) terminated by signal {}",job.job_id, pid, signal);
                    std::io::stdout().write_all(&msg.as_bytes()).unwrap();
                    std::io::stdout().flush().unwrap();
                    //job.state = JobState::Finished;
                }
                
                shell::delete_job(pid);
                
                //remove from job list
            },
            Ok(WaitStatus::Stopped(pid, signal)) => {
                let job = shell::get_job(pid);
                let job = match job {
                    Some(job) => job,
                    None => {
                        continue;
                    },
                };
                
                {
                    let mut job = job.borrow_mut();

                    let msg = format!("Job [{}] ({}) stopped by signal {}",job.job_id, pid, signal);
                    std::io::stdout().write_all(&msg.as_bytes()).unwrap();
                    std::io::stdout().flush().unwrap();
                    job.state = JobState::Stopped;
                }
                //update job list
            },
            Ok(WaitStatus::StillAlive) => {
                break;
            },
            Err(_) => {
                break;
            },
            _ => (),
        }
    }





}

pub fn on_sigint() {
    unsafe {
        SIGINT_PENDING.store(false, Ordering::Relaxed);
    }
    sigclearmask();
    // if !rootshell && iflag
    // signal sigint sig_dfl
    // raise sigint
    //exitstatus = 128 + sigint
}

fn sigclearmask() {
    //sigsetmask(0);

    let sigset = signal::SigSet::empty();
    signal::sigprocmask(signal::SigmaskHow::SIG_SETMASK, Some(&sigset), None).unwrap();
}


pub fn set_signal(sig_num: c_int) {
    let sig_handler;
    let mut action;

    let signal = signal::Signal::try_from(sig_num).unwrap();

    let rootshell = true; //todo change this

    let lvforked = shell::vforked();

    let trap = shell::get_trap(signal);

    if trap.is_none() {
        action = S_DFL;
    }
    else if trap.is_some() {
        action = S_CATCH;
    }
    else {
        action = S_IGN;
    }

    if rootshell && action == S_DFL && !lvforked {
        match signal {
            Signal::SIGINT => {
                //if iflag || minusc || sflag
                action = S_CATCH;
            },
            Signal::SIGQUIT | Signal::SIGTERM => {
                //if iflag
                action = S_IGN;
            },
            Signal::SIGTSTP | Signal::SIGTTOU => {
                //if mflag
                action = S_IGN;
            },
            _ => (),
        }
    }

    if signal == Signal::SIGCHLD {
        action = S_CATCH;
    }

    let sig_mode = shell::get_signal_mode(signal);


    /*if sig_mode.is_none() || sig_mode.unwrap() != 0 {
        println!("sig mode is none");
        //current setting unknown

        unsafe {
            if signal::sigaction(signal, &signal::SigAction::new(
                signal::SigHandler::SigDfl,
                signal::SaFlags::empty(),
                signal::SigSet::empty(),
                    )).is_err() {
                /*
                 * Pretend it worked; maybe we should give a warning
                 * here, but other shells don't. We don't alter
                 * sigmode, so that we retry every time.
                 */
                return;
            }
        }
        //ignoring check from dash because it requires sigaction to be set to SIG_IGN
        //but unless that is the default for sigaction then it is impossible to check
        //especially since SIG_IGN is a macro

    }*/

    if sig_mode.is_some() && (sig_mode.unwrap() == S_HARD_IGN || sig_mode.unwrap() == action) {
        return;
    }

    match action {
        S_CATCH => {
            sig_handler = signal::SigHandler::Handler(on_sig);
        },
        S_IGN => {
            sig_handler = signal::SigHandler::SigIgn;
        },
        _ => {
            sig_handler = signal::SigHandler::SigDfl;
        },
    }

    if !lvforked {
        shell::set_signal_mode(signal, action);
    }

    let sig_action = signal::SigAction::new(
        sig_handler,
        signal::SaFlags::empty(),
        signal::SigSet::all(),
    );

    unsafe {
        signal::sigaction(signal, &sig_action).unwrap();
    }
}

pub fn sig_block_all(old_mask: &mut signal::SigSet) {
    let sigset = signal::SigSet::all();
    signal::sigprocmask(signal::SigmaskHow::SIG_SETMASK, Some(&sigset), Some(old_mask)).unwrap();
}


pub fn sig_clear_mask() {
    let sigset = signal::SigSet::empty();
    signal::sigprocmask(signal::SigmaskHow::SIG_SETMASK, Some(&sigset), None).unwrap();
}

pub fn sig_suspend(mask: &signal::SigSet) {
    return;
    while get_pending_signal().is_none() {
        //do nothing
    }
    return;
    unsafe {
        libc::sigsuspend(mask.as_ref() as *const libc::sigset_t);
    }
    //nix::unistd::pause();
    /*match mask.wait() {
        Ok(_) => (),
        Err(_) => (),
    }*/
}
