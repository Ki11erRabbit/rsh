
use pest::Parser;
use pest_derive::Parser;
use pest::iterators::Pair;

#[derive(Parser)]
#[grammar = "shell_grammar.pest"] // relative to project `src`
struct PestShellParser;








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

pub enum RedirectionType {
    Pipe,
    File(String, RedirectionMode),
    None,
    Fd(i32),
}

pub enum RedirectionMode {
    Append,
    Output,
    Input,
}

pub struct Redirection {
    pub stdin: RedirectionType,
    pub stdout: RedirectionType,
    pub stderr: RedirectionType,
}

pub enum RunIf {
    Always,
    // run if the previous command exited with status 0
    Success,
    // run if thet previous command exited with status != 0
    Failure,
}


pub struct CaseItem {
    pub patterns: Vec<Word>,
    pub body: Vec<Term>,
}

pub struct ElIf {
    pub condition: Vec<Term>,
    pub then_part: Vec<Term>,
}

pub enum Initializer {
    Array(Vec<Word>),
    String(Word),
}

pub struct Assignment {
    pub name: String,
    pub initializer: Initializer,
    pub index: Option<Expression>,
}

pub enum LocalDeclaration {
    // local foo=1
    Assignment(Assignment),
    // local foo
    Name(String),
}

pub enum Command {
    SimpleCommand {
        argv: Vec<Word>,
        redirect: Redirection,
        // Assingment Prefixes (e.g. FOO=bar ./foo)
        assignments: Vec<Assignment>,
    },
    // foo=1, bar="HEllo World"
    Assignment {
        assignments: Vec<Assignment>,
    },
    If {
        condition: Vec<Term>,
        then_part: Vec<Term>,
        elif_parts: Vec<ElIf>,
        else_part: Option<Vec<Term>>,
        redirects: Redirection,
    },
    While {
        condition: Vec<Term>,
        body: Vec<Term>,
    },
    For {
        var_name: String,
        words: Vec<Word>,
        body: Vec<Term>,
    },
    ForArithmetic {
        init: Expression,
        condition: Expression,
        update: Expression,
        body: Vec<Term>,
    },
    Break,
    Continue,
    Return {
        status: Option<i32>,
    },
    Case {
        word: Word,
        cases: Vec<CaseItem>,
    },
    FunctionDef {
        name: String,
        body: Box<Command>,
    },
    LocalDef {
        declarations: Vec<LocalDeclaration>,
    },
    Group {
        terms: Vec<Term>,
    },
    SubShellGroup {
        terms: Vec<Term>,
    },
    Conditional(Box<ConditionalExpression>),
}

pub enum ConditionalExpression {
    Or(Box<ConditionalExpression>, Box<ConditionalExpression>),
    And(Box<ConditionalExpression>, Box<ConditionalExpression>),
    // = or ==
    StringEq(Box<ConditionalExpression>, Box<ConditionalExpression>),
    // !=
    StringNe(Box<ConditionalExpression>, Box<ConditionalExpression>),
    // -eq
    Eq(Box<ConditionalExpression>, Box<ConditionalExpression>),
    Ne(Box<ConditionalExpression>, Box<ConditionalExpression>),
    Lt(Box<ConditionalExpression>, Box<ConditionalExpression>),
    Le(Box<ConditionalExpression>, Box<ConditionalExpression>),
    Gt(Box<ConditionalExpression>, Box<ConditionalExpression>),
    Ge(Box<ConditionalExpression>, Box<ConditionalExpression>),
    Word(Word),
}

pub struct Pipeline {
    pub run_if: RunIf,
    pub commands: Vec<Command>, // Separated by |
}

pub struct Term {
    pub code: String,
    pub pipelines: Vec<Pipeline>,
    pub background: bool,
}

pub struct Ast {
    pub terms: Vec<Term>, // separated by & or ;
}

pub enum ExpansionOp {
    Length,                                 // ${#foo}
    GetOrEmpty,                             // $foo and ${foo}
    GetOrDefault(Word),                     // ${foo:-default}
    GetNullableOrDefault(Word),             // ${foo-default}
    GetOrDefaultAndAssign(Word),            // ${foo:=default}
    GetNullableOrDefaultAndAssign(Word),    // ${foo=default}

    Substring {                             //${foo/pattern/replacement}
        pattern: Word,
        replacement: Word,
        replace_all: bool,
    }
}

pub struct BinaryExpression {
    lhs: Box<Expression>,
    rhs: Box<Expression>,
}

pub enum Expression {
    Add(BinaryExpression),
    Sub(BinaryExpression),
    Mul(BinaryExpression),
    Div(BinaryExpression),
    Mod(BinaryExpression),
    Assign {
        name: String,
        rhs: Box<Expression>,
    },
    Literal(i32),

    Parameter {name: String},

    Eq(Box<Expression>, Box<Expression>),
    Ne(Box<Expression>, Box<Expression>),
    Lt(Box<Expression>, Box<Expression>),
    Le(Box<Expression>, Box<Expression>),
    Gt(Box<Expression>, Box<Expression>),
    Ge(Box<Expression>, Box<Expression>),

    Inc(String),
    Dec(String),

    Expression(Box<Expression>),
}

pub enum LiteralChar {
    Normal(char),
    Escape(char),
}

pub enum Span {
    Literal(String),
    LiteralChar(Vec<LiteralChar>),
    // ~
    Tilde(Option<String>),
    // $foo, ${foo}, ${foo:- default}
    Parameter {
        name: String,
        op: ExpansionOp,
        quoted: bool,
    },
    // $${foo[1]}
    ArrayParameter {
        name: String,
        index: Expression,
        quoted: bool,
    },
    // $(echo hello && echo world)
    Command {
        body: Vec<Term>,
        quoted: bool,
    },
    // $((1 + 2))
    ArithmeticExpression {
        expression: Expression,
    },
    // *
    WildcardString {
        quoted: bool,
    },
    // ?
    WildcardChar {
        quoted: bool,
    },
}

pub struct Word(pub Vec<Span>);


pub enum ParseError {
    Fatal(String),
    Empty,
}


macro_rules! wsnl {
    ($self:expr, $pairs:expr) => {
        if let Some(next) = $pairs.next() {
            match next.as_rule() {
                Rule::newline => {
                    $self.newline(next);
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


pub struct ShellParser;

impl ShellParser {

    pub fn new() -> ShellParser {
        ShellParser {}
    }

    // word = ${ assign_like_prefix? ~ (tilde_span | span) ~ span* }
    // assign_like_prefix = { assign_like_prefix_var_name ~ "=" }
    // span = _{
    //     double_quoted_span
    //     | single_quoted_span
    //     | literal_span
    //     | any_string_span
    //     | any_char_span
    //     | expr_span
    //     | command_span
    //     | backtick_span
    //     | param_ex_span
    //     | param_span
    // }
    fn escaped_word(&mut self, pair: Pair<Rule>, literal_chars: bool) -> Word {
        assert_eq!(pair.as_rule(), Rule::word);

        let mut spans = Vec::new();
        for span in pair.into_inner() {
            match span.as_rule() {
                Rule::literal_span if literal_chars => {
                    let mut chars = Vec::new();
                    for chr in span.into_inner() {
                        match chr.as_rule() {
                            Rule::escaped_char => {
                                let lit_chr = chr.as_str().chars().nth(1).unwrap();
                                chars.push(LiteralChar::Escape(lit_chr));
                            },
                            Rule::unescaped_char => {
                                let lit_chr = chr.as_str().chars().next().unwrap();
                                chars.push(LiteralChar::Normal(lit_chr));
                            },
                            _ => unreachable!(),
                        }
                    }

                    spans.push(Span::LiteralChar(chars));
                },
                Rule::literal_span if !literal_chars => {
                    spans.push(Span::Literal(self.escape_sequences(span, None)));
                },
                Rule::double_quoted_span => {
                    for span_in_quote in span.into_inner() {
                        match span_in_quote.as_rule() {
                            Rule::literal_in_double_quoted_span => {
                                spans.push (Span::Literal(self.escape_sequences(span_in_quote, Some("\"`$"))));
                            },
                            Rule::backtick_span => {
                                spans.push(self.command_span(span_in_quote, true));
                            },
                            Rule::command_span => {
                                spans.push(self.command_span(span_in_quote, true));
                            },
                            Rule::parameter_span => {
                                spans.push(self.parameter_span(span_in_quote, true));
                            }
                            Rule::parameter_expansion_span => {
                                spans.push(self.parameter_expansion_span(span_in_quote, true));
                            },
                            _ => unreachable!(),
                        }
                    }
                }

                Rule::single_quoted_span => {
                    for span_in_quote in span.into_inner() {
                        match span_in_quote.as_rule() {
                            Rule::literal_in_single_quoted_span => {
                                spans.push(Span::Literal(span_in_quote.as_str().to_owned()));
                            },
                            _ => unreachable!(),
                        }
                    }
                }
                Rule::expression_span => spans.push(self.expression_span(span)),
            }
        }
    }


    // fd = { ASCII_DIGIT+ }
    // redirect_direction = { "<" | ">" | ">>" }
    // redirect_to_fd = ${ "&" ~ ASCII_DIGIT* }
    // redirect = { fd ~ redirect_direction ~ (word | redirect_to_fd) }
    fn redirect(&mut self, redirect: &mut Redirection, pair: Pair<Rule>) {
        let mut inner = pair.into_inner();
        let fd = inner.next().unwrap();
        let symbol = inner.next().unwrap();
        let target = inner.next().unwrap();

        let (direction, default_fd) = match symbol.as_span().as_str() {
            "<" => (RedirectionMode::Input, 0),
            ">" => (RedirectionMode::Output, 1),
            ">>" => (RedirectionMode::Append, 1),
            _ => unreachable!(),
        };

        let fd = fd.as_span().as_str().parse::<i32>().unwrap_or(default_fd);
        let target = match target.as_rule() {
            Rule::word => RedirectionType::File(self.word(target), direction),
            Rule::redirect_to_file_descriptor => {
                let target_fd = target
                    .into_inner()
                    .next()
                    .unwrap()
                    .as_span()
                    .as_str()
                    .parse::<i32>()
                    .unwrap();
                RedirectionType::Fd(target_fd)
            },
            _ => unreachable!(),
        };
        
        match fd {
            0 => redirect.stdin = target,
            1 => redirect.stdout = target,
            2 => redirect.stderr = target,
            _ => unreachable!(),
        }

    }

    fn word(&mut self, pair: Pair<Rule>) -> Word {
        self.escaped_word(pair, false)
    }

    // assignment = { var_name ~ index ~ "=" ~ initializer ~ WHITESPACE? }
    // index = { ("[" ~ expr ~ "]")? }
    // initializer = { array_initializer | string_initializer }
    // string_initializer = { word }
    // array_initializer = { ("(" ~ word* ~ ")") }
    fn assignment(&mut self, pair: Pair<Rule>) -> Assignment {
        let mut inner = pair.into_inner();

        let name = inner.next().unwrap().as_span().as_str().to_owned();
        let index = inner
            .next()
            .unwrap()
            .into_inner()
            .next()
            .map(|pair| self.expression(pair));

        let initializer = inner.next().unwrap().into_inner().next().unwrap();

        match initializer.as_rule() {
            Rule::string_initializer => {
                let word = Initializer::String(self.word(initializer.into_inner().next().unwrap()));
                Assignment { name, index, initializer: word }
            },
            Rule::array_initializer => {
                let word = Initializer::Array(initializer
                                              .into_inner()
                                              .map(|pair| self.word(pair))
                                              .collect()
                                              );
                let index = None;
                Assignment { name, index, initializer: word }
            },
            _ => unreachable!(),
        }
    }

    fn simple_command(&mut self, pair: Pair<Rule>) -> Command {
        assert_eq!(pair.as_rule(), Rule::simple_command);

        let mut argv = Vec::new();
        let mut redirect: Redirection;

        let mut inner = pair.into_inner();
        let assignment_pairs = inner.next().unwrap().into_inner();
        let argv0 = inner.next().unwrap().into_inner().next().unwrap();
        let args = inner.next().unwrap().into_inner();

        argv.push(self.word(argv0));
        for word_or_redirect in args {
            match word_or_redirect.as_rule() {
                Rule::word => argv.push(self.word(word_or_redirect)),
                Rule::redirection => self.redirection(&mut redirect, word_or_redirect),
                _ => unreachable!(),
            }
        }
        let mut assignments = Vec::new();
        for assignment in assignment_pairs {
            assignments.push(self.assignment(assignment));
        }

        Command::SimpleCommand {
            argv,
            redirect,
            assignments,
        }
    }

    // if_command = {
    //     "if" ~ compound_list ~
    //     "then" ~ compound_list ~
    //     elif_part* ~
    //     else_part? ~
    //     "fi"
    // }
    // elif_part = { "elif" ~ compound_list ~ "then" ~ compound_list }
    // else_part = { "else" ~ compound_list }
    fn if_command(&mut self, pair: Pair<Rule>) -> Command {
        assert_eq!(pair.as_rule(), Rule::if_command);

        let mut inner = pair.into_inner();
        let condition = self.compound_list(inner.next().unwrap());
        let then_part = self.compound_list(inner.next().unwrap());
        let mut elif_parts = Vec::new();
        let mut else_part = None;
        for elif in inner {
            match elif.as_rule() {
                Rule::elif_part => {
                    let mut inner = elif.into_inner();
                    let condition = self.compound_list(inner.next().unwrap());
                    let then_part = self.compound_list(inner.next().unwrap());
                    elif_parts.push(ElIf { condition, then_part });
                },
                Rule::else_part => {
                    let mut inner = elif.into_inner();
                    let body = self.compound_list(inner.next().unwrap());
                    else_part = Some(body);
                },
                _ => unreachable!(),
            }
        }
        Command::If {
            condition,
            then_part,
            elif_parts,
            else_part,
            redirects: Redirection { //TODO
                stdin: RedirectionType::None,
                stdout: RedirectionType::None,
                stderr: RedirectionType::None,
            },
        }
    }

    // patterns = { word ~ ("|" ~ patterns)* }
    // case_item = {
    //     !("esac") ~ patterns ~ ")" ~ compound_list ~ ";;"
    // }
    //
    // case_command = {
    //     "case" ~ word ~ "in" ~ (wsnl | case_item)* ~ "esac"
    // }
    fn case_command(&mut self, pair: Pair<Rule>) -> Command {
        let mut inner = pair.into_inner();
        let word = self.word(inner.next().unwrap());
        let mut cases = Vec::new();
        while let Some(case) = wsnl!(self, inner) {
            match case.as_rule() {
                Rule::case_item => {
                    let mut inner = case.into_inner();
                    let patterns = inner
                        .next()
                        .unwrap()
                        .into_inner()
                        .map(|pair| self.word(pair))
                        .collect();
                    let body = self.compound_list(inner.next().unwrap());
                    cases.push(CaseItem { patterns, body });
                }
                Rule::newline => self.newline(case),
                _ => unreachable!(),
            }
        }

        Command::Case { word, cases }
    }

    // while_command = {
    //     "while" ~ compound_list ~ "do" ~ compound_list ~ "done"
    // }
    fn while_command(&mut self, pair: Pair<Rule>) -> Command {
        let mut inner = pair.into_inner();
        let condition = self.compound_list(inner.next().unwrap());
        let body = self.compound_list(inner.next().unwrap());

        Command::While { condition, body, }
    }


    // word_list = { word* }
    // for_command = {
    //     "for" ~ var_name ~ "in" ~ word_list ~ "do" ~ compound_list ~ "done"
    // }

    fn for_command(&mut self, pair: Pair<Rule>) -> Command {
        let mut inner = pair.into_inner();
        let var_name = inner.next().unwrap().as_span().as_str().to_owned();
        let words = inner
            .next()
            .unwrap()
            .into_inner()
            .map(|word| self.word(word))
            .collect();
        let compound_list = wsnl!(self, inner).unwrap();
        let body = self.compound_list(compound_list);

        Command::For {
            var_name,
            words,
            body,
        }
    }

    // arith_for_exprs = { "((" ~ expr ~";" ~ expr ~ ";" ~ expr ~ "))" }
    // arith_for_command = {
    //     "for" ~ arith_for_exprs ~ (";" | wsnl)+ ~ "do" ~ compound_list ~ "done"
    // }
    fn for_arithmetic_command(&mut self, pair: Pair<Rule>) -> Command {
        let mut inner = pair.into_inner();
        let mut expression = inner.next().unwrap().into_inner();
        let compound_list = wsnl!(self, inner).unwrap();
        let body = self.compound_list(compound_list);

        let init = self.expression(expression.next().unwrap());
        let condition = self.expression(expression.next().unwrap());
        let update = self.expression(expression.next().unwrap());

        Command::ForArithmetic {
            init: Box::new(init),
            condition: Box::new(condition),
            update: Box::new(update),
            body,
        }
    }

    // function_definition = {
    //     ("function")? ~ var_name ~ "()" ~ wsnl ~ compound_list
    // }
    fn function_definition(&mut self, pair: Pair<Rule>) -> Command {
        let mut inner = pair.into_inner();
        let name = inner.next().unwrap().as_span().as_str().to_owned();
        let compound_list = wsnl!(self, inner).unwrap();
        let body = Box::new(self.command(compound_list));

        Command::FunctionDef { name, body }
    }


    // local_definition = { "local" ~ (assignment | var_name)+ }
    fn local_definition(&mut self, pair: Pair<Rule>) -> Command {
        let mut declarations = Vec::new();
        for inner in pair.into_inner() {
            declarations.push(match inner.as_rule() {
                Rule::assignment => LocalDeclaration::Assignment(self.assignment(inner)),
                Rule::var_name => LocalDeclaration::Name(inner.as_span().as_str().to_owned()),
                _ => unreachable!(),
            });
        }

        Command::LocalDef { declarations }
    }


    // assignment_command = { assignment+ }
    fn assignment_command(&mut self, pair: Pair<Rule>) -> Command {
        let assignments = pair 
            .into_inner()
            .map(|assignment| self.assignment(assignment))
            .collect();

        Command::Assignment { assignments }
    }

    // group = { "{" ~ compound_list ~ "}" }
    fn group_command(&mut self, pair: Pair<Rule>) -> Command {
        let mut inner = pair.into_inner();
        let terms = self.compound_list(inner.next().unwrap());
        Command::Group { terms }
    }

    // group = { "(" ~ compound_list ~ ")" }
    fn subshell_group_command(&mut self, pair: Pair<Rule>) -> Command {
        let mut inner = pair.into_inner();
        let terms = self.compound_list(inner.next().unwrap());
        Command::SubShellGroup { terms }
    }

    // return = { return ~ num? }
    fn return_command(&mut self, pair: Pair<Rule>) -> Command {
        let mut inner = pair.into_inner();
        let status = inner
            .next()
            .map(|status| status.as_span().as_str().parse().unwrap());

        Command::Return { status }
    }

    // command = {
    //     if_command
    //     | case_command
    //     | while_command
    //     | for_command
    //     | break_command
    //     | continue_command
    //     | return_command
    //     | local_definition
    //     | function_definition
    //     | group
    //     | cond_ex
    //     | simple_command
    //     | assignment_command
    // }

    fn command(&mut self, pair: Pair<Rule>) -> Command {
        let inner = pair.into_inner().next().unwrap();

        match inner.as_rule() {
            Rule::simple_command => self.simple_command(inner),
            Rule::if_command => self.if_command(inner),
            Rule::case_command => self.case_command(inner),
            Rule::while_command => self.while_command(inner),
            Rule::for_command => self.for_command(inner),
            Rule::for_arithmetic_command => self.for_arithmetic_command(inner),
            Rule::while_command => self.while_command(inner),
            Rule::group => self.group(inner),
            Rule::subshell_group => self.subshell_group_command(inner),
            Rule::break_command => Command::Break,
            Rule::continue_command => Command::Continue,
            Rule::return_command => self.return_command(inner),
            Rule::local_definition => self.local_definition(inner),
            Rule::function_definition => self.function_definition(inner),
            Rule::assignment_command => self.assignment_command(inner),
            Rule::cond_ex => self.cond_ex(inner),
            _ => unreachable!(),
        }
    }

    // pipeline = { command ~ ((!("||") ~ "|") ~ wsnl? ~ command)* }
    fn pipeline(&mut self, pair: Pair<Rule>) -> Vec<Command> {
        let mut commands = Vec::new();
        let mut inner = pair.into_inner();
        while let Some(command) = wsnl!(self, inner) {
            commands.push(self.command(command));
        }

        commands
    }

    fn and_or_list(&mut self, pair: Pair<Rule>, run_if: RunIf) -> Vec<Pipeline> {
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

        terms
    }


    // compound_list = { compound_list_inner ~ (compound_list_sep ~ wsnl? ~ compound_list)* }
    // compound_list_sep = { (!(";;") ~ ";") | !("&&") ~ "&" | "\n" }
    // empty_line = { "" }
    // compound_list_inner = _{ and_or_list | empty_line }
    fn compound_list(&mut self, pair: Pair<Rule>) -> Vec<Term> {
        let mut terms = Vec::new();
        let mut inner = pair.into_inner();
        
        if let Some(and_or_list) = inner.next() {
            let mut background = false;
            let mut rest = None;
            while let Some(sep_or_rest) = wsnl!(self, inner) {
                match sep_or_rest.as_rule() {
                    Rule::compound_list => {
                        rest = Some(sep_or_rest);
                        break;
                    },
                    _ => {
                        let sep = sep_or_rest.into_inner().next().unwrap();
                        match sep.as_rule() {
                            Rule::background => background = true,
                            Rule::newline => self.newline(sep),
                            Rule::sequence_separator => (),
                            _ => (),
                        }
                    }
                }
            }

            if and_or_list.as_rule() == Rule::and_or_list {
                let code = and_or_list.as_str().to_owned().trim().to_owned();
                let pipelines = self.and_or_list(and_or_list, RunIf::Always);
                terms.push(Term { code, pipelines, background });
            }

            if let Some(rest) = rest {
                terms.extend(self.compound_list(rest));
            }
        }

        terms
    }
    
    pub fn newline(&mut self, pair: Pair<Rule>) {
        // does nothing by default
    }


    pub fn parse(&mut self, script: &str) -> Result<Ast, ParseError> {
        match PestShellParser::parse(Rule::script, script) {
            Ok(mut pairs) => {
                let terms = self.compound_list(pairs.next().unwrap());

                if terms.is_empty() {
                    Err(ParseError::Empty)
                } else {
                    Ok(Ast { terms })
                }
            },
            Err(e) => Err(ParseError::Fatal(e.to_string())),
        }
    }

}


pub fn parse_input(script: &str) -> Result<Ast, ParseError> {
    let mut parser = ShellParser::new();
    parser.parse(script)
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::PestShellParser;

    #[test]
    fn test_pipeline() {
        let input = "ls -l | grep foo";
        let pairs = PestShellParser::parse(Rule::pipeline, input).unwrap_or_else(|e| panic!("{}", e));

        for pair in pairs {
            println!("Rule: {:?}", pair.as_rule());
            println!("Span: {:?}", pair.as_span());
            println!("Text: {:?}", pair.as_span().as_str());
        }
    }
    
    #[test]
    fn test_long_pipeline() {
        let input = "ls -l | grep foo | wc -l";
        let pairs = PestShellParser::parse(Rule::pipeline, input).unwrap_or_else(|e| panic!("{}", e));

        for pair in pairs {
            println!("Rule: {:?}", pair.as_rule());
            println!("Span: {:?}", pair.as_span());
            println!("Text: {:?}", pair.as_span().as_str());
        }
    }
    #[test]
    fn test_redirection() {
        let input = "ls -l > foo.txt";
        let pairs = PestShellParser::parse(Rule::pipeline, input).unwrap_or_else(|e| panic!("{}", e));

        for pair in pairs {
            println!("Rule: {:?}", pair.as_rule());
            println!("Span: {:?}", pair.as_span());
            println!("Text: {:?}", pair.as_span().as_str());
        }
    }
    #[test]
    fn test_pipe_with_redirection() {
        let input = "ls -l | grep foo > foo.txt";
        let pairs = PestShellParser::parse(Rule::pipeline, input).unwrap_or_else(|e| panic!("{}", e));

        for pair in pairs {
            println!("Rule: {:?}", pair.as_rule());
            println!("Span: {:?}", pair.as_span());
            println!("Text: {:?}", pair.as_span().as_str());
        }
    }

    #[test]
    fn test_if_statement() {
        let input = "if [ -f foo.txt ]; then echo foo; fi";
        let pairs = PestShellParser::parse(Rule::pipeline, input).unwrap_or_else(|e| panic!("{}", e));

        for pair in pairs {
            println!("Rule: {:?}", pair.as_rule());
            println!("Span: {:?}", pair.as_span());
            println!("Text: {:?}", pair.as_span().as_str());
        }
    }
    #[test]
    fn test_case_statement() {
        let input = "case foo in bar) echo bar;; esac";
        let pairs = PestShellParser::parse(Rule::pipeline, input).unwrap_or_else(|e| panic!("{}", e));

        for pair in pairs {
            println!("Rule: {:?}", pair.as_rule());
            println!("Span: {:?}", pair.as_span());
            println!("Text: {:?}", pair.as_span().as_str());
        }
    }

    #[test]
    fn test_long_shell_script() {
        let input = "if [ -f foo.txt ]; then echo foo; fi; case foo in bar) echo bar;; esac; ls -l | grep foo > foo.txt";
        let pairs = PestShellParser::parse(Rule::pipeline, input).unwrap_or_else(|e| panic!("{}", e));

        for pair in pairs {
            println!("Rule: {:?}", pair.as_rule());
            println!("Span: {:?}", pair.as_span());
            println!("Text: {:?}", pair.as_span().as_str());
        }
    }
    #[test]
    fn test_function(){
        let input = "foo() { echo foo; }";
        let pairs = PestShellParser::parse(Rule::pipeline, input).unwrap_or_else(|e| panic!("{}", e));

        for pair in pairs {
            println!("Rule: {:?}", pair.as_rule());
            println!("Span: {:?}", pair.as_span());
            println!("Text: {:?}", pair.as_span().as_str());
        }
    }
    #[test]
    fn test_calling_function(){

        let input = "foo() { echo foo; }\n foo";
        let pairs = PestShellParser::parse(Rule::pipeline, input).unwrap_or_else(|e| panic!("{}", e));

        for pair in pairs {
            println!("Rule: {:?}", pair.as_rule());
            println!("Span: {:?}", pair.as_span());
            println!("Text: {:?}", pair.as_span().as_str());
        }
    }

    #[test]
    fn test_calling_function_with_args() {
        let input = "foo() { echo $1; }\n foo bar";
        let pairs = PestShellParser::parse(Rule::pipeline, input).unwrap_or_else(|e| panic!("{}", e));

        for pair in pairs {
            println!("Rule: {:?}", pair.as_rule());
            println!("Span: {:?}", pair.as_span());
            println!("Text: {:?}", pair.as_span().as_str());
        }
    }

    #[test]
    fn test_while_loop() {
        let input = "while [ -f foo.txt ]; do echo foo; done";
        let pairs = PestShellParser::parse(Rule::pipeline, input).unwrap_or_else(|e| panic!("{}", e));

        for pair in pairs {
            println!("Rule: {:?}", pair.as_rule());
            println!("Span: {:?}", pair.as_span());
            println!("Text: {:?}", pair.as_span().as_str());
        }
    }
    #[test]
    fn test_directory_for_loop() {
        let input = "for file in /path/to/dir/*; do echo $file; done";
        let pairs = PestShellParser::parse(Rule::pipeline, input).unwrap_or_else(|e| panic!("{}", e));

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
        let pairs = PestShellParser::parse(Rule::pipeline, input).unwrap_or_else(|e| panic!("{}", e));

        for pair in pairs {
            println!("Rule: {:?}", pair.as_rule());
            println!("Span: {:?}", pair.as_span());
            println!("Text: {:?}", pair.as_span().as_str());
        }
    }
}
