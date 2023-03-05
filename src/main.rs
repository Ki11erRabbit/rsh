//mod shell;
mod parser;
mod lexer;
mod parser_alt;
mod eval_alt;
mod exec;
mod process;
extern crate pest;
#[macro_use]
extern crate pest_derive;


use std::io::{self, Write};


use std::thread::sleep;

use signal_hook::{iterator::Signals};

fn sig_int_handler() {
        sleep(std::time::Duration::from_secs(4));
    unsafe {
        NAME = 0;
    }
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


    loop {
        print!("$ ");
        io::stdout().flush().unwrap();

        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();

        let ast = parser::parse_input(&input).unwrap();

        //let result = eval::eval(&ast, 0, 1, 2);
    }
}
