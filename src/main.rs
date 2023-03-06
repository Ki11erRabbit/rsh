mod shell;
//mod parser;
mod lexer;
mod ast;
mod eval;
mod jobs;
mod builtins;
//mod eval_alt;
//mod exec;
//mod process;
use lalrpop_util::lalrpop_mod;

use lexer::Lexer;
use std::io::{self, Write};


use std::thread::sleep;

use signal_hook::{iterator::Signals};

lalrpop_mod!(pub grammar);

fn sig_int_handler() {
        sleep(std::time::Duration::from_secs(4));
    unsafe {
        NAME = 0;
    }
}

fn sig_child_handler() {
    println!("SIGCHILD");
    shell::display_jobs();
}


static mut NAME: i32 = 42;


fn main() {
    /*unsafe {
        signal_hook::low_level::register(2, sig_int_handler).unwrap();
    }



    while unsafe {NAME == 42} {
        println!("Looping");
        sleep(std::time::Duration::from_secs(1));
    }
    println!("Hello, world!");*/

    unsafe {
        signal_hook::low_level::register(17, sig_child_handler).unwrap();
    }

    loop {
        print!("$ ");
        io::stdout().flush().unwrap();

        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();

        /*if input.as_str() == "\n" {
            continue;
        }*/

        let lexer = Lexer::new(&input);        
        let ast = grammar::CompleteCommandParser::new()
            .parse(&input,lexer)
            .unwrap();

        let result = eval::eval(&ast);

        //let ast = parser::parse_input(&input).unwrap();

        //let result = eval::eval(&ast, 0, 1, 2);
    }
}
