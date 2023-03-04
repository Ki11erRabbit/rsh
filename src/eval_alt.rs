use crate::parser::{ParseError};

use pest::Parser;
use pest_derive::Parser;
use pest::iterators::Pair;
use pest::iterators::Pairs;






#[derive(Parser)]
#[grammar = "shell_grammar.pest"] // relative to project `src`
struct PestShellParser;


macro_rules! wsnl {
    ($pairs:expr) => {
        if let Some(next) = $pairs.next() {
            match next.as_rule() {
                Rule::newline => {
                    newline(next);
                    $pairs.next()
                },
                _ => Some(next),
            }
        }
        else {
            None
        }
    };
}



fn command(pair: Pair<Rule>) -> i32 {
    let inner = pair.into_inner().next().unwrap();

    match inner.as_rule() {
        Rule::simple_command => simple_command(inner),
        /*Rule::if_command => if_command(inner),
        Rule::case_command => case_command(inner),
        Rule::while_command => while_command(inner),
        Rule::for_command => for_command(inner),
        Rule::arithmetic_for_command => for_arithmetic_command(inner),
        Rule::group => group_command(inner),
        Rule::subshell_group => subshell_group_command(inner),
        Rule::break_command => Command::Break,
        Rule::continue_command => Command::Continue,
        Rule::return_command => return_command(inner),
        Rule::local_definition => local_definition(inner),
        Rule::function_definition => function_definition(inner),
        Rule::assignment_command => assignment_command(inner),*/
        //Rule::conditional_expression => self.conditional_expression(inner),
        _ => unreachable!(),
    }
    0
}

// pipeline = { command ~ ((!("||") ~ "|") ~ wsnl? ~ command)* }
fn pipeline(pair: Pair<Rule>) -> i32 {
    let mut commands = Vec::new();
    let mut inner = pair.into_inner();
    while let Some(command) = wsnl!(inner) {
        command(command);
    }

    0
}

fn and_or_list(pair: Pair<Rule>, run_if: RunIf) -> i32{
    let mut terms = Vec::new();
    let mut inner = pair.into_inner();

    if let Some(pipeline) = inner.next() {
        let commands = self.pipeline(pipeline);
        terms.push(Pipeline{ commands, run_if });

        let next_run_if = inner
            .next()
            .map(|sep| match sep.as_span().as_str() {
                "||" => RunIf::Failure,
                "&&" => RunIf::Success,
                _ => RunIf::Always,
            })
        .unwrap_or(RunIf::Always);

        if let Some(rest) = wsnl!(self, inner) {
            terms.extend(self.and_or_list(rest, next_run_if));
        }
    }

    0
}


fn compound_list(pair: Pair<Rule>) -> i32 {
    let mut terms = Vec::new();
    let mut inner = pair.into_inner();

    if let Some(and_or_list) = inner.next() {
        let mut background = false;
        let mut rest = None;
        while let Some(sep_or_rest) = wsnl!(inner) {
            match sep_or_rest.as_rule() {
                Rule::compound_list => {
                    rest = Some(sep_or_rest);
                    break;
                },
                _ => {
                    let sep = sep_or_rest.into_inner().next().unwrap();
                    match sep.as_rule() {
                        Rule::background => background = true,
                        Rule::newline => newline(sep),
                        Rule::sequence_separator => (),
                        _ => (),
                    }
                }
            }
        }

        if and_or_list.as_rule() == Rule::and_or_list {
            let code = and_or_list.as_str().to_owned().trim().to_owned();
            let pipelines = and_or_list(and_or_list, RunIf::Always);
        }

    }


    0
}

pub fn eval(script: &str) -> Result<i32, ParseError> {

    match PestShellParser::parse(Rule::script, script) {
        Ok(mut pairs) => {
            //println!("\npairs:\n{:?}\n\n", pairs);
            let terms = compound_list(pairs.next().unwrap());
            //println!("\nterms:\n{:?}\n\n", terms); 
            if terms < -1 {
                Err(ParseError::Empty)
            } else {
                Ok(terms)
            }
        },
        Err(e) => Err(ParseError::Fatal(e.to_string())),
    }

}

fn newline(pair: Pair<Rule>) {
    
}
