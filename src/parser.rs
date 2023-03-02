
use pest::Parser;
use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "shell_grammar.pest"] // relative to project `src`
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


#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::ShellParser;

    #[test]
    fn test_pipeline() {
        let input = "ls -l | grep foo";
        let pairs = ShellParser::parse(Rule::pipeline, input).unwrap_or_else(|e| panic!("{}", e));

        for pair in pairs {
            println!("Rule: {:?}", pair.as_rule());
            println!("Span: {:?}", pair.as_span());
            println!("Text: {:?}", pair.as_span().as_str());
        }

    }
}
