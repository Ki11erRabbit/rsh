mod shell;
mod completion;
mod context;
//mod parser;
mod lexer;
mod ast;
mod eval;
mod jobs;
mod builtins;
mod trap;
#[macro_use]
mod log;
mod var;
//mod eval_alt;
//mod exec;
//mod process;
use lalrpop_util::lalrpop_mod;
lalrpop_mod!(pub grammar);

use lexer::Lexer;

use std::error::Error;

use std::io;
use std::env;
use std::fs::File;
use std::io::BufReader;
use std::io::prelude::*;


use nix::errno::Errno;
use rustyline::error::ReadlineError;


fn main() {

    let args: Vec<String> = env::args().collect();

    let input = parse_args(args);

    /*match read_profile() {
        Ok(_) => {},
        Err(_) => {
            
        }
    }*/

    
  
   
    shell::push_context_new();
    shell::set_arg_0();

    
    match read_rc() {
        Ok(_) => {},
        Err(_) => {
            
        }
    }

    match read_user_profile() {
        Ok(_) => {},
        Err(_) => {
            
        }
    }


    if shell::is_interactive() && input.is_none() {
        trap::set_signal(17);
        trap::set_signal(20);
        trap::set_signal(2);

        shell::set_history_location("history.txt");

        interactive_loop();
    }
    else if input.is_some() {
        read_from_args(&input.unwrap());
    }
    else if input.is_none() {
        script_loop(&shell::get_script_name());
    }

}

/// This function reads the commandline arguments passed into the shell and parses them.
/// The function returns a Some(String) if -c was passed in, meaning that the shell will
/// evaluate all commandline arguments as a script.
/// The function returns a None if -c was not passed in, meaning that the shell will either
/// run in interactive mode or run a script.
fn parse_args(args: Vec<String>) -> Option<String> {
   
    let mut args = args;

    if args.len() == 1 {
        return None;
    }    
    
    let mut non_interactive_mode = false;
    let mut read_from_args = false;
    let mut load_args = false;
    let mut index = 1;
    let mut pos = 0;

    for arg in args.iter() {
            println!("arg: {}", arg);
        if arg.chars().nth(0).unwrap() == '-' && !non_interactive_mode {
            read_from_args = parse_dash_arg(&arg);
            if read_from_args {
                break;
            }
        }
        else if non_interactive_mode && !load_args {
            shell::set_script_name(&arg);
            load_args = true;
        }
        else if load_args {
            shell::set_input_args(&arg, index);
            index += 1;
        }
        else if pos != 0 {
            non_interactive_mode = true;
            shell::set_script_name(&arg);
            load_args = true;
        }
        pos += 1;
    }
    
    if read_from_args {
        shell::set_interactive(false);

        args.drain(0..=pos);

        println!("{:?}", args);

        let output = args.join(" ");

        return Some(output);
    }
    
    None

}

/// This function parses a command line argument.
/// If the argument is -c, the function returns true.
/// If the argument is not -c, the function returns false.
/// This is how we check to see if the we are going to evaluate the commandline arguments as a script.
fn parse_dash_arg(arg: &str) -> bool {
    let mut read_from_args = false;
    if arg.chars().nth(0).unwrap() == '-' {
        let mut chars = arg.chars();
        chars.next();
        for c in chars {
            match c {
                'c' => {
                    read_from_args = true;

                    //make it change the name of shell
                    //set all args following -c to be the args $0, $1, $2, etc
                    
                },
		'l' => {
		    log::set_print_out(true);
		},
                _ => {
                    //error
                }
            }
        }
    }

    read_from_args
}

/// This function is what the user interacts with when the shell is in interactive mode.
/// It is a simple REPL that uses the readline from the Shell struct singleton.
fn interactive_loop() {


    let rl = shell::get_readline();
    //rl.borrow_mut().bind_sequence(rustyline::Event::KeySeq(vec![rustyline::KeyEvent::ctrl('z')]), rustyline::Cmd::Suspend);

    loop {
        let input;

        let readline = rl.borrow_mut().readline(shell::expand_var("PS1").unwrap().as_str());
        match readline {
            Ok(line) => {
                //rl.borrow_mut().add_history_entry(line.as_str());
                input = line;
            },
            Err(ReadlineError::Interrupted) => {
                continue;
            },
            Err(ReadlineError::Eof) => {
                break;
            },
            /*Err(ReadlineError::Errno(error)) => {
                continue;
            },*/
            Err(err) => {
                println!("Readline Error: {:?}", err);
                break;
            }
        }

        if input.len() == 0 {
            continue;
        }

        /*if input.as_str() == "\n" {
            continue;
        }*/

        let lexer = Lexer::new(&input);        
        let mut ast = match grammar::CompleteCommandParser::new()
            .parse(&input,lexer) {
                Ok(ast) => ast,
                Err(err) => {
                    println!("Error: {:?}", err);
                    continue;
                }
            };
       
        //eprintln!("{:?}", ast);

        let _result = eval::eval(&mut ast);

        //let ast = parser::parse_input(&input).unwrap();

        //let result = eval::eval(&ast, 0, 1, 2);
    }


    shell::save_history();
}


/// This function needs to be changed since it won't work with how the parser works.
/// This function takes a file name and puts the contents of the file into a buffer.
/// The buffer is then passed into the parser and evaluated.
fn script_loop(script_name: &str) {
    let file = File::open(script_name).unwrap();
    let mut buf_reader = BufReader::new(file);
    loop {

        let mut input = String::new();
        buf_reader.read_line(&mut input).unwrap();


        let lexer = Lexer::new(&input);        
        let mut ast = grammar::CompleteCommandParser::new()
            .parse(&input,lexer)
            .unwrap();

        let _result = eval::eval(&mut ast);

    }
}

/// This function takes in the commandline arguments as a &str and evaluates it.
fn read_from_args(input: &str) {
    let lexer = Lexer::new(&input);        
    let mut ast = grammar::CompleteCommandParser::new()
        .parse(&input,lexer)
        .unwrap();

    let _result = eval::eval(&mut ast);
}


/// This function reads the system profile file located at /etc/profile.
/// The file is is opened and evaluated as a script.
/// This function is always called in main in order to load the system profile.
/// The return value is Ok(()) if the parser is successful.
/// The return value is Err(err) if the parser fails.
fn read_profile() -> Result<(), Box<dyn Error>>{
    let file = File::open("/etc/profile");
    if file.is_err() {
        return Ok(());
    }
    let mut file = file.unwrap();
    let mut system_profile = String::new();
    match file.read_to_string(&mut system_profile) {
        Ok(_) => {},
        Err(err) => {
            println!("Error reading /etc/profile: {:?}", err);
        }
    }

    let lexer = Lexer::new(&system_profile);
    let mut ast = match grammar::CompleteCommandParser::new()
        .parse(&system_profile,lexer) {
            Ok(ast) => ast,
            Err(err) => {
                println!("Error: {:?}", err);
                return Ok(());
            }
        };

    eval::eval(&mut ast).unwrap();


    Ok(())
}

/// This function is similar to read_profile but it instead reads the user profile file located at ~/.profile.
/// The file is is opened and evaluated as a script.
/// This function is always called in main in order to load the user profile.
/// The return value is Ok(()) if the parser is successful.
/// The return value is Err(err) if the parser fails.
fn read_user_profile() -> Result<(), Box<dyn Error>> {
    let file = File::open("~/.profile");
    if file.is_err() {
        return Ok(());
    }
    let mut file = file.unwrap();
    let mut user_profile = String::new();
    match file.read_to_string(&mut user_profile) {
        Ok(_) => {},
        Err(err) => {
            println!("Error reading ~/.profile: {:?}", err);
        }
    }

    let lexer = Lexer::new(&user_profile);
    let mut ast = match grammar::CompleteCommandParser::new()
        .parse(&user_profile,lexer) {
            Ok(ast) => ast,
            Err(err) => {
                println!("Error: {:?}", err);
                return Ok(());
            }
        };

    eval::eval(&mut ast).unwrap();

    Ok(())
}

/// This function reads the user's rshrc file that is located at ~/.rshrc.
/// The file is is opened and evaluated as a script.
/// This function is always called in main in order to load the user's rshrc file.
/// The return value is Ok(()) if the parser is successful.
/// The return value is Err(err) if the parser fails.
fn read_rc() -> Result<(), Box<dyn Error>> {
    let file = File::open("~/.rshrc");
    if file.is_err() {
        return Ok(());
    }
    let mut file = file.unwrap();
    let mut rc = String::new();
    match file.read_to_string(&mut rc) {
        Ok(_) => {},
        Err(err) => {
            println!("Error reading ~/.rshrc: {:?}", err);
        }
    }
    
    let lexer = Lexer::new(&rc);
    let mut ast = match grammar::CompleteCommandParser::new()
        .parse(&rc,lexer) {
            Ok(ast) => ast,
            Err(err) => {
                println!("Error: {:?}", err);
                return Ok(());
            }
        };

    eval::eval(&mut ast).unwrap();

    Ok(())
}
