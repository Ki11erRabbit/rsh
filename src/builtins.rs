use std::env;
use std::io;
use std::fs::File;
use std::io::prelude::*;
use std::process::exit;
use std::io::Write;
use crate::ast::SimpleCommand;
use crate::shell;
use nix::unistd::Pid;
use nix::sys::signal::kill;
use nix::sys::signal::Signal;
use std::ffi::CString;
use crate::jobs;
use crate::trap;
use crate::eval;
use crate::log;
use crate::context::ContextUtils;
use crate::context::Context;
use crate::jobs::Process;

use std::rc::Rc;
use std::cell::RefCell;

use lalrpop_util::lalrpop_mod;
lalrpop_mod!(pub grammar);

use crate::lexer::Lexer;

enum IdType {
    Pid,
    Jid,
}

/// This function trims quotes off of its input.
fn trim(word: &str) -> String {
    if (word.starts_with("\"") && word.ends_with("\"")) || (word.starts_with("'") && word.ends_with("'")){
        let mut chars = word.chars();
        chars.next();
        chars.next_back();
        chars.collect::<String>()
    }
    else {
        word.to_string()
    }
}

/// This function is the 'cd' command of the shell.
/// We use the env::set_current_dir function to change the current directory.
/// this needs to change several variables when changing but for now we won't care
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

/// This is the 'exit' command of the shell.
/// Despite having a return value, this function will cause the shell to exit.
/// By default it will exit with a status code of 0, but if the user specifies a status code
/// it will exit with that status code.
pub fn quit(command: &SimpleCommand) -> Result<(), std::io::Error> {
    shell::save_history();
    if command.suffix.is_none() || command.suffix.as_ref().unwrap().word.is_empty() {
        let exit_code = eval::get_exit_code();
        exit(exit_code);
    }
    //let chars = command.suffix.as_ref().unwrap().word[0].chars();
    //chars.next();
    let code = command.suffix.as_ref().unwrap().word[0].parse::<i32>().unwrap();

    exit(code);
}

/// This is the 'return' command of the shell.
/// It returns from a function.
/// By default, it returns the last command's exit status.
/// It takes a SimpleCommand with a suffix that is a string of the form 'number'.
pub fn return_cmd(command: &SimpleCommand) -> Result<(), std::io::Error> {
    if command.suffix.is_none() || command.suffix.as_ref().unwrap().word.is_empty() {
        return Ok(());
    }
    //let chars = command.suffix.as_ref().unwrap().word[0].chars();
    //chars.next();
    let code = command.suffix.as_ref().unwrap().word[0].parse::<i32>().unwrap();

    eval::set_exit_status(code);
    Ok(())
}

/// This is the 'jobs' command of the shell.
/// It prints out all of the jobs that are currently running.
pub fn jobs() -> Result<(), std::io::Error> {
    print!("{}", shell::display_jobs());
    io::stdout().flush().unwrap();
    Ok(())
}

/// This is the 'fg' and 'bg' commands of the shell.
/// They are used to bring a job to the foreground or background respectively.
/// They take a SimpleCommand with a suffix that is either a valid Pid or a job id if it starts with a '%'.
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

/// This is the 'fg' command of the shell.
/// It is used to bring a stopped job to the foreground.
/// It takes a SimpleCommand with a suffix that is either a valid Pid or a job id if it starts with a '%'.
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

/// This is the 'bg' command of the shell.
/// It is used to bring a stopped job to the background.
/// It takes a SimpleCommand with a suffix that is either a valid Pid or a job id if it starts with a '%'.
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

/// This is the 'alias' command of the shell.
/// It is used to create aliases for commands.
/// It takes a SimpleCommand with a suffix that is a string of the form 'alias=command'.
/// If the suffix is empty, it will print out all of the current aliases.
/// If the suffix is '-p', it will print out all of the current aliases.
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

/// This is the 'unalias' command of the shell.
/// It is used to remove aliases for commands.
/// It takes a SimpleCommand with a suffix that is a string of the form 'alias'.
/// If the suffix is '-a', it will remove all of the current aliases.
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


/// This is an internal function that takes a &str which is the name of a file.
/// It will then open the file and read it into a string.
/// It will then parse the string into an AST.
/// It will then push a new context onto the context stack.
/// It will then evaluate the AST.
/// It will then pop the context off of the context stack.
/// It will then return the context.
fn create_context_from_file(file: &str) -> Result<Rc<RefCell<Context>>, std::io::Error> {
    let mut file = File::open(file)?;
    let mut string = String::new();
    file.read_to_string(&mut string)?;

    let lexer = Lexer::new(string.as_str());
    let mut ast = match grammar::CompleteCommandParser::new().parse(&string,lexer) {
        Ok(ast) => ast,
        Err(e) => {
            return Err(std::io::Error::new(std::io::ErrorKind::Other, format!("Error parsing file: {}", e)));
        }
    };

    shell::push_context_new();

    match eval::eval(&mut ast) {
        Ok(_) => {},
        Err(e) => {
            return Err(std::io::Error::new(std::io::ErrorKind::Other, format!("Error evaluating file: {}", e)));
        }
    }
    let context = shell::pop_context().unwrap();
    Ok(context)
}

/// This is the 'export' command of the shell.
/// It is used to export variables to the environment.
/// It takes a SimpleCommand with a suffix that is a string of the form 'variable=value'.
/// If the suffix is empty, it will print out all of the current environment variables.
/// If the suffix is '-p', it will print out all of the current environment variables.
/// If the first value of the Suffix is 'context', it will perform one of the following:
/// .    If the second value is 'self', it will export the current context to the environment using the namespace defined by $0.
/// .    If the second value contains an equal sign ('='), with the left side being the namespace and the right side either being a file or 'self'.
/// .        If the right side is a file, it will evaluate the as a new context and export it.
/// .        If the right side is 'self', it will export the current context to the environment using the namespace defined by the left side.
pub fn export(command: &SimpleCommand) -> Result<(), std::io::Error> {
    if (command.suffix.is_none() || command.suffix.as_ref().unwrap().word.len() == 0) || command.suffix.as_ref().unwrap().word[0].contains("-p") {
        env::vars().for_each(|(key, value)| {
            println!("{}={}", key, value);
        });
        return Ok(());
    }

    let suffix = command.suffix.as_ref().unwrap();

    if suffix.word[0].as_str() == "context" {
        if suffix.word.len() < 2 {
            return Err(std::io::Error::new(std::io::ErrorKind::Other, "export context needs an argument"));
        }
        if suffix.word[1].as_str().contains('=') {
            let mut split = suffix.word[1].split('=');
            let namespace = split.next().unwrap();
            let file = split.next().unwrap();
            
            if file == "self" {
                let context = shell::get_current_context();

                shell::add_context(&namespace, context.clone());
                return Ok(());
            }
            else {
                let result = create_context_from_file(file)?;
                shell::add_context(&namespace, result);
            }
        }
        else {
            if suffix.word[1].as_str() == "self" {
                let context = shell::get_current_context();
                let namespace = {
                    context.borrow().get_var("0").unwrap().borrow().value.clone()
                };
                shell::add_context(&namespace, context);
                return Ok(());
            }
            else {
                let result = create_context_from_file(suffix.word[0].as_str())?;
                shell::add_context(suffix.word[0].as_str(), result);
            }
        }
        return Ok(());
    }
    
    for word in command.suffix.as_ref().unwrap().word.iter() {
        if !word.contains('=') {
            let var = shell::expand_var(word);
            if var.is_none() {
                let function = shell::get_function(word);
                if function.is_some() {
                    shell::get_env_context().borrow_mut().add_function(word, function.unwrap());
                }
            }
            else {
                let temp = format!("{}={}",word,var.unwrap());
                shell::get_env_context().borrow_mut().add_var(temp.as_str());
            }
            continue;//TODO: make this read argument
        }
        let mut split = word.split('=');
        let key = split.next().unwrap();
        let value = split.next().unwrap();
        shell::add_var(word,0);
        env::set_var(key, &trim(value));
    }

    Ok(())
}

/// This evaluates an assignment which is a SimpleCommand with a prefix that is a string of the form 'variable=value'.
pub fn assignment(command: &SimpleCommand) -> Result<(), std::io::Error> {



    shell::add_var_context(command.prefix.as_ref().unwrap().assignment[0].as_str());
    Ok(())
}


/// This is the 'eval' command of the shell.
/// It evaluates a string as a command.
/// It takes a SimpleCommand with a suffix that is a string of the form 'string'.
pub fn eval_cmd(command: &SimpleCommand) -> Result<(), std::io::Error> {
    if command.suffix.is_none() {
        return Err(std::io::Error::new(std::io::ErrorKind::Other, "eval needs an argument"));
    }


    let string = command.suffix.as_ref().unwrap().word[0].as_str().to_string() + "\n";
    
    let mut lexer = Lexer::new(string.as_str());
    let mut ast = match grammar::CompleteCommandParser::new().parse(string.as_str(),lexer) {
        Ok(ast) => ast,
        Err(e) => {
            return Err(std::io::Error::new(std::io::ErrorKind::Other, format!("Error parsing file: {}", e)));
        }
    };

    log!("AST: {:?}", ast);
    match eval::eval(&mut ast) {
        Ok(_) => {},
        Err(e) => {
            return Err(std::io::Error::new(std::io::ErrorKind::Other, format!("Error evaluating string: {}", e)));
        }
    }

    
    Ok(())
}

/// This is the 'pwd' command of the shell.
/// It prints out the current working directory.
pub fn pwd() -> Result<(), std::io::Error> {
    let path = env::current_dir().unwrap();
    println!("{}", path.display());
    Ok(())
}

enum Flags {
    Variable,
    Function,
    Context,
}

pub fn unset(command: &SimpleCommand) -> Result<(), std::io::Error> {
    if command.suffix.is_none() || command.suffix.as_ref().unwrap().word.len() == 0 {
	return Err(std::io::Error::new(std::io::ErrorKind::Other, "unset needs an argument"));
    }

    let mut flags = Flags::Variable;
    let mut pos = 0;
    if command.suffix.as_ref().unwrap().word[0].as_str().contains('-') {
	if command.suffix.as_ref().unwrap().word[0].as_str().contains('f') {
	    flags = Flags::Function;
	    pos = 1;
	}
	else if command.suffix.as_ref().unwrap().word[0].as_str().contains('c') {
	    flags = Flags::Context;
	    pos = 1;
	}
	else if command.suffix.as_ref().unwrap().word[0].as_str().contains('v') {
	    flags = Flags::Variable;
	    pos = 1;
	}
	else {
	    return Err(std::io::Error::new(std::io::ErrorKind::Other, "unset: invalid option"));
	}
    }

    match flags {
        Flags::Variable => {
            shell::remove_var(command.suffix.as_ref().unwrap().word[pos].as_str());
        },
        Flags::Function => {
            shell::remove_function(command.suffix.as_ref().unwrap().word[pos].as_str());
        },
        Flags::Context => {
            shell::remove_context(command.suffix.as_ref().unwrap().word[pos].as_str());
        },
    }


    Ok(())

}

pub fn readonly(command: &SimpleCommand) -> Result<(), std::io::Error> {
    if command.suffix.is_none() || command.suffix.as_ref().unwrap().word.len() == 0 {
    return Err(std::io::Error::new(std::io::ErrorKind::Other, "readonly needs an argument"));
    }

    let mut flags = Flags::Variable;
    let mut pos = 0;
    let mut print = false;
    if command.suffix.as_ref().unwrap().word[0].as_str().contains('-') {
        if command.suffix.as_ref().unwrap().word[0].as_str().contains('f') {
            flags = Flags::Function;
            pos = 1;
        }
        else if command.suffix.as_ref().unwrap().word[0].as_str().contains('c') {
            flags = Flags::Context;
            pos = 1;
        }
        else if command.suffix.as_ref().unwrap().word[0].as_str().contains('v') {
            flags = Flags::Variable;
            pos = 1;
        }
        else if command.suffix.as_ref().unwrap().word[0].as_str().contains('p') {
            print = true;
            pos = 1;
        }
        else {
            return Err(std::io::Error::new(std::io::ErrorKind::Other, "readonly: invalid option"));
        }
    }
    if command.suffix.as_ref().unwrap().word[pos].as_str().contains('-') {
        if command.suffix.as_ref().unwrap().word[pos].as_str().contains('p') {
            print = true;
            pos += 1;
        }
        else {
            return Err(std::io::Error::new(std::io::ErrorKind::Other, "readonly: invalid option"));
        }
    }


    if print {
        match &flags {
            Flags::Variable | Flags::Context => {
                match flags {
                    Flags::Variable => {
                        shell::print_readonly_vars();
                    },
                    Flags::Context => {
                        if command.suffix.as_ref().unwrap().word.len() == pos {
                            return Err(std::io::Error::new(std::io::ErrorKind::Other, "readonly needs an argument"));
                        }
                        shell::print_readonly_vars_context(command.suffix.as_ref().unwrap().word[pos].as_str());
                    },
                    _ => {},
                }
                shell::print_readonly_vars();
            },
            Flags::Function => {
                shell::print_readonly_functions();
            },
        }
        return Ok(());
    }

    match flags {
        Flags::Function => {
            shell::print_readonly_functions();
            return Ok(());
        },
        _ => {},
    }

    if command.suffix.as_ref().unwrap().word.len() == pos {
        return Err(std::io::Error::new(std::io::ErrorKind::Other, "readonly needs an argument"));
    }
    if command.suffix.as_ref().unwrap().word[pos].as_str().contains('=') {
        shell::add_readonly_var(command.suffix.as_ref().unwrap().word[pos].as_str());
        return Ok(());
    }
    
    shell::set_readonly_var(command.suffix.as_ref().unwrap().word[pos].as_str());


    Ok(())

}

pub fn exec_cmd(command: &SimpleCommand) -> Result<(),std::io::Error> {

    if command.suffix.is_none() || command.suffix.as_ref().unwrap().word.len() < 1 {
        return Err(std::io::Error::new(std::io::ErrorKind::Other, "exec needs an argument"));
    }
    
    let suffix = command.suffix.as_ref().unwrap();

    let mut empty_env = false;
    let mut dash = false;
    let mut rename = false;
    let mut pos = 0;
    
    if suffix.word[0].contains('-') {
        if suffix.word[0].contains('c') {
            empty_env = true;
            pos = 1;
        }
        if suffix.word[0].contains('l') {
            dash = true;
            pos = 1;
        }
        if suffix.word[0].contains('a') {
            rename = true;
            pos = 2;
            if suffix.word.len() <= pos {
                return Err(std::io::Error::new(std::io::ErrorKind::Other, "exec -a needs an argument"));
            }
        }
    }
    if suffix.word[pos].contains('-') {
        if suffix.word[pos].contains('c') {
            empty_env = true;
            pos += 1;
        }
        if suffix.word[pos].contains('l') {
            dash = true;
            pos += 1;
        }
        if suffix.word[pos].contains('a') {
            rename = true;
            pos += 2;
            if suffix.word.len() <= pos {
                return Err(std::io::Error::new(std::io::ErrorKind::Other, "exec -a needs an argument"));
            }
        }
    }

    log!("empty_env: {}, dash: {}, rename: {}, pos: {}", empty_env, dash, rename, pos);

    let mut args = Vec::new();
    let mut cmd = String::new();
    for i in pos..suffix.word.len() {
        log!("i: {}, pos: {}", i, pos);
        log!("word: {}", suffix.word[i]);
        if i == pos && dash && rename {
            let temp = "-".to_string() + suffix.word[pos - 1].as_str();
            args.push(CString::new(temp.as_str()).unwrap());
            cmd += suffix.word[pos - 1].as_str();
            cmd += " ";
            continue;
        }
        else if i == pos && dash {
            let temp = "-".to_string() + suffix.word[i].as_str();
            args.push(CString::new(temp.as_str()).unwrap());
            cmd += suffix.word[i].as_str();
            cmd += " ";
            continue;
        }
        else if rename && i == pos {
            cmd += suffix.word[pos - 1].as_str();
            args.push(CString::new(suffix.word[pos - 1].as_str()).unwrap());
            cmd += " ";
            continue;
        }
        args.push(CString::new(suffix.word[i].as_str()).unwrap());
        cmd += suffix.word[i].as_str();
        cmd += " ";
    }
    log!("args: {:?}", args);
    

    let mut process = Process::new(args, suffix.word[pos].clone(),cmd);

    let env = if empty_env {
        Vec::new()
    }
    else {
        env::vars().map(|(k, v)| CString::new(k + "=" + &v).unwrap()).collect()
    };

    
    let result = eval::temp_execve(&mut process, &env);

    if result.is_err() {
        return Err(std::io::Error::new(std::io::ErrorKind::Other, format!("{}", result.unwrap_err())));
    }

    
    Ok(())
}

pub fn source(command: &SimpleCommand) -> Result<(),std::io::Error> {

    if command.suffix.is_none() || command.suffix.as_ref().unwrap().word.len() < 1 {
        return Err(std::io::Error::new(std::io::ErrorKind::Other, "source needs an argument"));
    }

    let suffix = command.suffix.as_ref().unwrap();

    let filename = suffix.word[0].clone();

    let args = if suffix.word.len() > 1 {
        let mut args = Vec::new();
        for i in 1..suffix.word.len() {
            args.push(suffix.word[i].clone());
        }
        args
    }
    else {
        Vec::new()
    };

    for (pos, arg) in args.iter().enumerate() {
        shell::add_var_context(&format!("{}={}", pos + 1, arg));
    }

    // Open the file directly if it has a slash
    let mut file = if filename.contains('/') {
        File::open(&filename)
    }// Open the file if it is in in the PATH
    else if shell::lookup_command(filename.as_str()).is_some() {
        File::open(shell::lookup_command(filename.as_str()).unwrap())
    }// Open the file in place
    else {
        File::open(&filename)
    };

    if file.is_err() {
        return Err(std::io::Error::new(std::io::ErrorKind::Other, format!("{} {}: No such file or directory",command.name ,filename)));
    }
    let mut file = file.unwrap();

    let mut contents = String::new();
    file.read_to_string(&mut contents).unwrap();

    let mut lexer = Lexer::new(contents.as_str());
    let mut ast = match grammar::CompleteCommandParser::new().parse(contents.as_str(),lexer) {
        Ok(ast) => ast,
        Err(e) => {
            return Err(std::io::Error::new(std::io::ErrorKind::Other, format!("Error parsing file: {}", e)));
        }
    };

    log!("AST: {:?}", ast);
    match eval::eval(&mut ast) {
        Ok(_) => {},
        Err(e) => {
            return Err(std::io::Error::new(std::io::ErrorKind::Other, format!("Error evaluating string: {}", e)));
        }
    }
    

    Ok(())
}
