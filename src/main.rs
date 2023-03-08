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


lalrpop_mod!(pub grammar);


use rustyline::error::ReadlineError;
use rustyline::Editor;


fn main() {
   
    trap::set_signal(17);
    trap::set_signal(20);
    trap::set_signal(2);

    shell::set_history_location("history.txt");

    let mut rl = shell::get_readline();

    loop {
        //print!("$ ");
        //io::stdout().flush().unwrap();

        let mut input = String::new();
        //let mut buf_reader = BufReader::new(io::stdin());
        //buf_reader.read_line(&mut input).unwrap();
        //io::stdin().read_line(&mut input).unwrap();

        //load history

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
            Err(err) => {
                println!("Redline Error: {:?}", err);
                break;
            }
        }

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

    shell::save_history();
}
