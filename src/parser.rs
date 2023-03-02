
use pest::Parser;
use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "script_grammar.pest"] // relative to project `src`
struct ShellParser;



/*pub enum Command {
    Pipeline {
        argv: Vec<Word>,
        redirect: Vec<Redirect>,
    }
}

pub struct Word(String);

pub enum RedirectType {
    Pipe,

}
*/
