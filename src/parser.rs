
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
    
    #[test]
    fn test_long_pipeline() {
        let input = "ls -l | grep foo | wc -l";
        let pairs = ShellParser::parse(Rule::pipeline, input).unwrap_or_else(|e| panic!("{}", e));

        for pair in pairs {
            println!("Rule: {:?}", pair.as_rule());
            println!("Span: {:?}", pair.as_span());
            println!("Text: {:?}", pair.as_span().as_str());
        }
    }
    #[test]
    fn test_redirection() {
        let input = "ls -l > foo.txt";
        let pairs = ShellParser::parse(Rule::pipeline, input).unwrap_or_else(|e| panic!("{}", e));

        for pair in pairs {
            println!("Rule: {:?}", pair.as_rule());
            println!("Span: {:?}", pair.as_span());
            println!("Text: {:?}", pair.as_span().as_str());
        }
    }
    #[test]
    fn test_pipe_with_redirection() {
        let input = "ls -l | grep foo > foo.txt";
        let pairs = ShellParser::parse(Rule::pipeline, input).unwrap_or_else(|e| panic!("{}", e));

        for pair in pairs {
            println!("Rule: {:?}", pair.as_rule());
            println!("Span: {:?}", pair.as_span());
            println!("Text: {:?}", pair.as_span().as_str());
        }
    }

    #[test]
    fn test_if_statement() {
        let input = "if [ -f foo.txt ]; then echo foo; fi";
        let pairs = ShellParser::parse(Rule::pipeline, input).unwrap_or_else(|e| panic!("{}", e));

        for pair in pairs {
            println!("Rule: {:?}", pair.as_rule());
            println!("Span: {:?}", pair.as_span());
            println!("Text: {:?}", pair.as_span().as_str());
        }
    }
    #[test]
    fn test_case_statement() {
        let input = "case foo in bar) echo bar;; esac";
        let pairs = ShellParser::parse(Rule::pipeline, input).unwrap_or_else(|e| panic!("{}", e));

        for pair in pairs {
            println!("Rule: {:?}", pair.as_rule());
            println!("Span: {:?}", pair.as_span());
            println!("Text: {:?}", pair.as_span().as_str());
        }
    }

    #[test]
    fn test_long_shell_script() {
        let input = "if [ -f foo.txt ]; then echo foo; fi; case foo in bar) echo bar;; esac; ls -l | grep foo > foo.txt";
        let pairs = ShellParser::parse(Rule::pipeline, input).unwrap_or_else(|e| panic!("{}", e));

        for pair in pairs {
            println!("Rule: {:?}", pair.as_rule());
            println!("Span: {:?}", pair.as_span());
            println!("Text: {:?}", pair.as_span().as_str());
        }
    }
    #[test]
    fn test_function(){
        let input = "foo() { echo foo; }";
        let pairs = ShellParser::parse(Rule::pipeline, input).unwrap_or_else(|e| panic!("{}", e));

        for pair in pairs {
            println!("Rule: {:?}", pair.as_rule());
            println!("Span: {:?}", pair.as_span());
            println!("Text: {:?}", pair.as_span().as_str());
        }
    }
    #[test]
    fn test_calling_function(){

        let input = "foo() { echo foo; }\n foo";
        let pairs = ShellParser::parse(Rule::pipeline, input).unwrap_or_else(|e| panic!("{}", e));

        for pair in pairs {
            println!("Rule: {:?}", pair.as_rule());
            println!("Span: {:?}", pair.as_span());
            println!("Text: {:?}", pair.as_span().as_str());
        }
    }

    #[test]
    fn test_calling_function_with_args() {
        let input = "foo() { echo $1; }\n foo bar";
        let pairs = ShellParser::parse(Rule::pipeline, input).unwrap_or_else(|e| panic!("{}", e));

        for pair in pairs {
            println!("Rule: {:?}", pair.as_rule());
            println!("Span: {:?}", pair.as_span());
            println!("Text: {:?}", pair.as_span().as_str());
        }
    }

    #[test]
    fn test_while_loop() {
        let input = "while [ -f foo.txt ]; do echo foo; done";
        let pairs = ShellParser::parse(Rule::pipeline, input).unwrap_or_else(|e| panic!("{}", e));

        for pair in pairs {
            println!("Rule: {:?}", pair.as_rule());
            println!("Span: {:?}", pair.as_span());
            println!("Text: {:?}", pair.as_span().as_str());
        }
    }
    #[test]
    fn test_directory_for_loop() {
        let input = "for file in /path/to/dir/*; do echo $file; done";
        let pairs = ShellParser::parse(Rule::pipeline, input).unwrap_or_else(|e| panic!("{}", e));

        for pair in pairs {
            println!("Rule: {:?}", pair.as_rule());
            println!("Span: {:?}", pair.as_span());
            println!("Text: {:?}", pair.as_span().as_str());
        }
    }

    #[test]
    fn test_range_for_loop() {
        let input = "for i in 0 1 2 3 4 5 6 7 8 9 10; do
    echo $i;
done";
        let pairs = ShellParser::parse(Rule::pipeline, input).unwrap_or_else(|e| panic!("{}", e));

        for pair in pairs {
            println!("Rule: {:?}", pair.as_rule());
            println!("Span: {:?}", pair.as_span());
            println!("Text: {:?}", pair.as_span().as_str());
        }
    }
}
