mod shell;
//mod parser;
mod lexer;
mod ast;
mod eval;
mod jobs;
mod builtins;
mod trap;
//mod eval_alt;
//mod exec;
//mod process;
use lalrpop_util::lalrpop_mod;

use lexer::Lexer;
use std::io::{self, Write};



lalrpop_mod!(pub grammar);



fn main() {
   
    trap::set_signal(17);
    trap::set_signal(20);
    trap::set_signal(2);
    loop {
        print!("$ ");
        io::stdout().flush().unwrap();

        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();

        if input.len() == 0 {
            break;
        }

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
