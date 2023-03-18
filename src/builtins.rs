use std::env;
use std::io;
use std::process::exit;
use std::io::Write;
use crate::ast::SimpleCommand;
use crate::shell;
use nix::unistd::Pid;
use nix::sys::signal::kill;
use nix::sys::signal::Signal;
use crate::jobs;
use crate::trap;


enum IdType {
    Pid,
    Jid,
}



// this needs to change several variables when changing but for now we won't care
pub fn change_directory(command: &SimpleCommand) -> Result<(), std::io::Error> {
    
    trap::interrupts_off();
    let path;
    if command.suffix.is_none() {
        path = env::var("HOME").unwrap();
    } else {
        path = command.suffix.as_ref().unwrap().word[0].to_string();
    }
    env::set_current_dir(path)?;
    trap::interrupts_on();
    Ok(())
}

pub fn quit(command: &SimpleCommand) -> Result<(), std::io::Error> {
    shell::save_history();
    if command.suffix.is_none() {
        exit(0);
    }
    let mut chars = command.suffix.as_ref().unwrap().word[0].chars();
    chars.next();
    let code = chars.as_str().parse::<i32>().unwrap();

    exit(code);
}

pub fn jobs() -> Result<(), std::io::Error> {
    print!("{}", shell::display_jobs());
    io::stdout().flush().unwrap();
    Ok(())
}

pub fn fgbg(command: &SimpleCommand) -> Result<(), std::io::Error> {
    trap::interrupts_off();
    let id;
    let id_type;
    if command.suffix.is_none() {
        return Err(std::io::Error::new(std::io::ErrorKind::Other, "fgbg needs an argument"));
    }
    if command.suffix.as_ref().unwrap().word[0].chars().nth(0).unwrap() == '%' {
        let mut chars = command.suffix.as_ref().unwrap().word[0].chars();
        chars.next();
        id = chars.as_str().parse().unwrap();
        id_type = IdType::Jid;
    } else {
        id = command.suffix.as_ref().unwrap().word[0].parse::<u32>().unwrap();
        id_type = IdType::Pid;
    }

    if command.name == "fg" {
        fg(id.try_into().unwrap(), id_type)?;
    } else {
        bg(id.try_into().unwrap(), id_type)?;
    }
    trap::interrupts_on();

    Ok(())
}

fn fg(id: usize, id_type: IdType) -> Result<(), std::io::Error> {
    let mut job: Option<std::rc::Rc<std::cell::RefCell<jobs::Job>>> = None;
    match id_type {
        IdType::Pid => {
            let pid = Pid::from_raw(id as i32);
            job = shell::get_job(pid);
            if job.is_none() {
                return Err(std::io::Error::new(std::io::ErrorKind::Other, "No job with that pid"));
            }

        },
        IdType::Jid => {
            job = shell::get_job(id as usize);
            if job.is_none() {
                return Err(std::io::Error::new(std::io::ErrorKind::Other, "No job with that jid"));
            }
        }
    }
    
    
    job.as_ref().unwrap().borrow_mut().state = jobs::JobState::Running;
    job.as_ref().unwrap().borrow_mut().background = false;

    for process in job.as_ref().unwrap().borrow().processes.iter() {
        kill(process.pid, Signal::SIGCONT).unwrap();
    }
    println!("[{}] {}", job.as_ref().unwrap().borrow().job_id, job.as_ref().unwrap().borrow());

    jobs::wait_for_job(job);
    Ok(())
}

fn bg(id: usize, id_type: IdType) -> Result<(), std::io::Error> {
    let mut job: Option<std::rc::Rc<std::cell::RefCell<jobs::Job>>> = None;
    match id_type {
        IdType::Pid => {
            let pid = Pid::from_raw(id as i32);
            job = shell::get_job(pid);
            if job.is_none() {
                return Err(std::io::Error::new(std::io::ErrorKind::Other, "No job with that pid"));
            }

        },
        IdType::Jid => {
            job = shell::get_job(id as usize);
            if job.is_none() {
                return Err(std::io::Error::new(std::io::ErrorKind::Other, "No job with that jid"));
            }
        }
    }
    job.as_ref().unwrap().borrow_mut().state = jobs::JobState::Running;
    job.as_ref().unwrap().borrow_mut().background = true;
    
    for process in job.as_ref().unwrap().borrow().processes.iter() {
        kill(process.pid, Signal::SIGCONT).unwrap();
    }

    println!("[{}] {}", job.as_ref().unwrap().borrow().job_id, job.as_ref().unwrap().borrow());

    Ok(())
}

pub fn alias(command: &SimpleCommand) -> Result<(), std::io::Error> {
    if command.suffix.is_none() {
        shell::display_aliases();
        return Ok(());
    }
    if command.suffix.as_ref().unwrap().word[0].contains("-p") {
        shell::display_aliases();
        return Ok(());
    }
    
    for word in command.suffix.as_ref().unwrap().word.iter() {
        if !word.contains('=') {
            continue;//TODO: make this read argument
        }
        shell::add_alias(word.as_str());
    }

    Ok(())
}
pub fn unalias(command: &SimpleCommand) -> Result<(), std::io::Error> {
    if command.suffix.is_none() {
        return Err(std::io::Error::new(std::io::ErrorKind::Other, "unalias needs an argument"));
    }
    if command.suffix.as_ref().unwrap().word.len() > 1 {
        return Err(std::io::Error::new(std::io::ErrorKind::Other, "unalias only takes one argument"));
    }
    if command.suffix.as_ref().unwrap().word[0].contains("-a") {
        shell::clear_aliases();
        return Ok(());
    }
    shell::remove_alias(command.suffix.as_ref().unwrap().word[0].as_str());
    Ok(())
}

pub fn export(command: &SimpleCommand) -> Result<(), std::io::Error> {
    if command.suffix.is_none() || command.suffix.as_ref().unwrap().word[0].contains("-p") {
        env::vars().for_each(|(key, value)| {
            println!("{}={}", key, value);
        });
    }
    
    for word in command.suffix.as_ref().unwrap().word.iter() {
        if !word.contains('=') {
            continue;//TODO: make this read argument
        }
        let mut split = word.split('=');
        let key = split.next().unwrap();
        let value = split.next().unwrap();
        shell::add_var(word,-1);
        env::set_var(key, value);
    }

    Ok(())
}

pub fn assignment(command: &SimpleCommand) -> Result<(), std::io::Error> {
    shell::add_var_context(command.prefix.as_ref().unwrap().assignment[0].as_str());
    Ok(())
}

pub fn return_cmd(command: &SimpleCommand) -> Result<(), std::io::Error> {
    if command.suffix.is_none() {
        return Err(std::io::Error::new(std::io::ErrorKind::Other, "return needs an argument"));
    }
    let mut chars = command.suffix.as_ref().unwrap().word[0].chars();
    chars.next();
    let code = chars.as_str().parse::<i32>().unwrap();

    exit(code);
}

