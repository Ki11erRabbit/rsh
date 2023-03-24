use std::sync::atomic::{AtomicBool, Ordering};
use std::io::{self,Write};
use nix::sys::signal;
use nix::sys::signal::Signal;
use std::os::raw::c_int;
use nix::sys::wait::{WaitStatus,WaitPidFlag,waitpid};
use nix::unistd::Pid;
use std::collections::HashMap;
use lazy_static::lazy_static;
use fragile::Fragile;
use std::cell::RefCell;
use std::cell::{Ref,RefMut};

use crate::jobs::{self,JobState};
use crate::shell;
use crate::shell::ShellJobUtils;

/// This constant represents that a signal should use its default behavior
const S_DFL: usize = 1;
/// This constant represents that a signal should be caught
const S_CATCH: usize = 2;
/// This constant represents that a signal should be ignored
const S_IGN: usize = 3;
/// This constant represents that a signal should be ignored no matter what
const S_HARD_IGN: usize = 4;
/// This constant represents that a signal should be reset to its default behavior
const S_RESET: usize = 5;

/// This global represents if the shell has received a SIGCHLD signal
static mut GOT_SIGCHLD: AtomicBool = AtomicBool::new(false);
/// This global represents if the shell has received a SIGINT signal that has not been handled
static mut SIGINT_PENDING: AtomicBool = AtomicBool::new(false);
/// This global represents if the shell should supress SIGINT signals
static mut SUPRESS_SIGINT: AtomicBool = AtomicBool::new(false);
/// This global holds the a pending signal that has not been handled
static mut PENDING_SIGNAL: Option<Signal> = None;
/// This global represents if the shell should block signals
static mut BLOCK_SIGNALS: AtomicBool = AtomicBool::new(false);

lazy_static! {
    /// While this should be in the shell struct, because of the way that rustyline works, having it in the shell causes a thread panic.
    static ref TRAP_DATA: TrapWrapper = TrapWrapper::new(RefCell::new(TrapData::new()));
}


/**
 * This isn't pretty but it gets the job done. If only the Unix developers realized that signals
 * should be done similar to how Windows does it.
 */
pub struct TrapWrapper {
    pub data: RefCell<TrapData>,
}

impl TrapWrapper {
    pub fn new(data: RefCell<TrapData>) -> Self {
        Self {
            data,
        }
    }

    /// A wrapper function that acesses the refcell data
    pub fn get(&self) -> Ref<'_,TrapData> {
        self.data.borrow()
    }
    /// A wrapper function that acesses the refcell data mutably
    pub fn get_mut(&self) -> RefMut<'_,TrapData> {
        self.data.borrow_mut()
    }
}

/// This is so that we can use the TrapWrapper as a global without having to use a mutex which could cause a deadlock
unsafe impl std::marker::Sync for TrapWrapper {}
unsafe impl std::marker::Send for TrapWrapper {}

/// This struct holds all the information on about the traps that are set as well as modes for each of the signals
pub struct TrapData {
    /// This hashmap holds the script strings to be executed when a signal is received
    traps: HashMap<Signal, String>,
    /// This hashmap holds the modes for each signal
    signal_mode: HashMap<Signal, usize>,
    /// This vec holds a bool to mark if a signal has been received
    got_sig: Vec<bool>,
    /// This holds a pending signal that has not been handled
    pending_signal: Option<Signal>,
}

impl TrapData {
    /// On Linux there are 32 signals that are defined.
    pub fn new() -> Self {
        Self {
            traps: HashMap::new(),
            signal_mode: HashMap::new(),
            got_sig: vec![false; 32],
            pending_signal: None,
        }
    }
}


/// This function checks to see if there is an action for a given signal
pub fn is_trap_set(signal: Signal) -> bool {
    let data = TRAP_DATA.get();
    data.traps.contains_key(&signal)
}

/// This function gets a script string for a given signal
/// Returns None if there is no script string for the given signal
pub fn get_trap(signal: Signal) -> Option<String> {
    let data = TRAP_DATA.get();
    data.traps.get(&signal).map(|s| s.to_string())
}

/// This function sets the mode for a given signal
pub fn set_signal_mode(signal: Signal, mode: usize) {
    let mut data = TRAP_DATA.get_mut();
    data.signal_mode.insert(signal, mode);
}

/// This function gets the mode for a given signal
pub fn get_signal_mode(signal: Signal) -> Option<usize> {
    let data = TRAP_DATA.get();
    data.signal_mode.get(&signal).map(|s| *s)
}
/// This function sets the got_sig vec for a given signal
pub fn set_got_sig(sig_num: c_int) {
    let mut data = TRAP_DATA.get_mut();
    data.got_sig[sig_num as usize] = true;
}

/// This function sets the current pending signal
pub fn set_pending_signal(sig_num: c_int) {
    let mut data = TRAP_DATA.get_mut();
    data.pending_signal = Some(Signal::try_from(sig_num).unwrap());
}

/// This function blocks all signals except for SIGINT and SIGTSTP
pub fn interrupts_off() {
    let mut sigset = signal::SigSet::all();
    sigset.remove(signal::SIGINT);
    sigset.remove(signal::SIGTSTP);
    //sigset.remove(signal::SIGCHLD);

    signal::sigprocmask(signal::SigmaskHow::SIG_SETMASK, Some(&sigset), None).unwrap();
    
}
/// This function unblocks all signals
pub fn interrupts_on() {
    sig_clear_mask();
}

/// This function checks for if we should be blocking signals
fn is_blocked() -> bool {
    unsafe {
        BLOCK_SIGNALS.load(Ordering::Relaxed)
    }
}

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

/// This function is the signal handler for all signals
///
/// This function is very similar to how the Dash shell does signals. The only difference being that we handle the SIGCHLD signal.
/// This function returns early if we are in the child.
extern "C" fn on_sig(sig_num: c_int) {
    if is_blocked() {
        return;
    }
    if shell::get_forked() {
        return;
    }
    unsafe {
        if sig_num == signal::SIGCHLD as c_int {
            GOT_SIGCHLD.store(true, Ordering::Relaxed);

            sig_chld();
            
            

            //sig_chld();//this is different from how dash does it but it should allow for for traps to be set
            if !is_trap_set(signal::SIGCHLD) {
                return;
            }
        }
    }
    //set which signal is got
    //set pending signal
    set_got_sig(sig_num);
    set_pending_signal(sig_num);


    if sig_num == signal::SIGINT as c_int && !is_trap_set(signal::SIGINT) {
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

/// This is the signal handler for SIGCHLD.
///
/// We have to do some interesting logic here since if we accidentally wait on a non-background process then we will have to return early.
/// If the process is in the background then we can wait on it.
fn sig_chld() {
    let pid = Pid::from_raw(-1);

    let flag: WaitPidFlag = WaitPidFlag::WNOHANG | WaitPidFlag::WUNTRACED;
    let result = waitpid(pid, Some(flag));
    if result.is_err() {
        return;
    }
    let status = result.unwrap();
    let pid;
    let status = match &status {
        WaitStatus::Exited(id, _) => {
            pid = id;
            status
        },
        WaitStatus::Signaled(id, _, _) => {
            pid = id;
            status
        },
        WaitStatus::Stopped(id, _) => {
            pid = id;
            status
        },
        WaitStatus::StillAlive => {
            return;
        },
        _ => {
            return;
        }, 
    };

    let job = shell::get_job(*pid);
    if job.is_none() {
        return;
    }
    job.as_ref().unwrap().borrow_mut().set_process_status(*pid, status);
   
    if job.as_ref().unwrap().borrow().background {
        match status {
            WaitStatus::Signaled(_, _, _) => {
                let msg = format!("Job [{}] ({}) terminated by signal",job.as_ref().unwrap().borrow().job_id, pid);
                std::io::stdout().write_all(&msg.as_bytes()).unwrap();
                std::io::stdout().flush().unwrap();
            },
            WaitStatus::Stopped(_, _) => {
                let msg = format!("Job [{}] ({}) stopped by signal",job.as_ref().unwrap().borrow().job_id, pid);
                std::io::stdout().write_all(&msg.as_bytes()).unwrap();
                std::io::stdout().flush().unwrap();
            },
            _ => {},
        }
        
        jobs::wait_for_job_sigchld(job, status);
    }
}

/// This functions is called if the signal was a SIGINT.
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

/// This function clears the signal mask
/// # Panics
/// This function panics if sigprocmask fails
fn sigclearmask() {
    //sigsetmask(0);

    let sigset = signal::SigSet::empty();
    signal::sigprocmask(signal::SigmaskHow::SIG_SETMASK, Some(&sigset), None).unwrap();
}


/// This function sets up the signal handler for signals
/// It takes in a c_int which is the signal number
pub fn set_signal(sig_num: c_int) {
    let sig_handler;
    let mut action;

    let signal = signal::Signal::try_from(sig_num).unwrap();

    let rootshell = true; //todo change this

    let lvforked = shell::get_forked();

    let trap = get_trap(signal);

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
            Signal::SIGINT | Signal::SIGTSTP => {
                //if iflag || minusc || sflag
                action = S_CATCH;
            },
            Signal::SIGQUIT | Signal::SIGTERM => {
                //if iflag
                action = S_IGN;
            },
            Signal::SIGTTOU => {
                //if mflag
                action = S_IGN;
            },
            _ => (),
        }
    }

    if signal == Signal::SIGCHLD {
        action = S_CATCH;
    }

    let sig_mode = get_signal_mode(signal);


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
        set_signal_mode(signal, action);
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

/// This function blocks all signals and uses the old_mask to store the old mask
pub fn sig_block_all(old_mask: &mut signal::SigSet) {
    let sigset = signal::SigSet::all();
    signal::sigprocmask(signal::SigmaskHow::SIG_SETMASK, Some(&sigset), Some(old_mask)).unwrap();
}

/// This function clears the signal mask
pub fn sig_clear_mask() {
    let sigset = signal::SigSet::empty();
    signal::sigprocmask(signal::SigmaskHow::SIG_SETMASK, Some(&sigset), None).unwrap();
}

/// This function should be the same as sigsuspend but there are problems so far
/// It is hard to test since I am not aware of the logic needed to test it
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

/// This function removes all signal handlers
/// however it is not complete
/// # Todo
/// make this more inclusive
pub fn remove_handlers() {
    let sigset = signal::SigSet::empty();
    signal::sigprocmask(signal::SigmaskHow::SIG_SETMASK, Some(&sigset), None).unwrap();

    let sig_handler = signal::SigHandler::SigDfl;

    let sig_action = signal::SigAction::new(
        sig_handler,
        signal::SaFlags::empty(),
        signal::SigSet::all(),
    );

    unsafe {
        signal::sigaction(Signal::SIGINT, &sig_action).unwrap();
        signal::sigaction(Signal::SIGQUIT, &sig_action).unwrap();
        signal::sigaction(Signal::SIGTERM, &sig_action).unwrap();
        signal::sigaction(Signal::SIGTSTP, &sig_action).unwrap();
        signal::sigaction(Signal::SIGTTIN, &sig_action).unwrap();
        signal::sigaction(Signal::SIGTTOU, &sig_action).unwrap();
        signal::sigaction(Signal::SIGCHLD, &sig_action).unwrap();
    }
}

