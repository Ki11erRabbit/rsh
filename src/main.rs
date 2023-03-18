mod shell;
//mod parser;
mod lexer;
mod ast;
mod eval;
mod jobs;
mod function;
mod builtins;
mod trap;
mod var;
//mod eval_alt;
//mod exec;
//mod process;
use lalrpop_util::lalrpop_mod;

use lexer::Lexer;

use std::io;
use std::env;
use std::fs::File;
use std::io::BufReader;
use std::io::prelude::*;

lalrpop_mod!(pub grammar);

use nix::errno::Errno;
use rustyline::error::ReadlineError;
use rustyline::{Editor, Cmd};


fn main() {

    let args: Vec<String> = env::args().collect();

    let input = parse_args(args);

  
    shell::set_arg_0();
   
    shell::push_var_stack();

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
                _ => {
                    //error
                }
            }
        }
    }

    read_from_args
}

fn interactive_loop() {

    let rl = shell::get_readline();

    //rl.borrow_mut().bind_sequence(rustyline::Event::KeySeq(vec![rustyline::KeyEvent::ctrl('z')]), rustyline::Cmd::Suspend);

    loop {
        let input;

        let readline = rl.borrow_mut().readline("$ ");
        match readline {
            Ok(line) => {
                rl.borrow_mut().add_history_entry(line.as_str());
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
                println!("Redline Error: {:?}", err);
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
        let mut ast = grammar::CompleteCommandParser::new()
            .parse(&input,lexer)
            .unwrap();
       
        //eprintln!("{:?}", ast);

        let _result = eval::eval(&mut ast);

        //let ast = parser::parse_input(&input).unwrap();

        //let result = eval::eval(&ast, 0, 1, 2);
    }


    shell::save_history();
}

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

fn read_from_args(input: &str) {
    let lexer = Lexer::new(&input);        
    let mut ast = grammar::CompleteCommandParser::new()
        .parse(&input,lexer)
        .unwrap();

    let _result = eval::eval(&mut ast);
}

fn read_profile() {

}
