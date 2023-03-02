
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

pub enum RedirectionType {
    Pipe,
    File(String),
    Normal,
    Fd(i32),
}

pub enum RedirectionMode {
    Append,
    Normal,
}

pub struct Redirection {
    pub stdin: RedirectionType,
    pub stdout: (RedirectionType, RedirectionMode),
    pub stderr: (RedirectionType, RedirectionMode),
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
        redirects: Vec<Redirection>,
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
    ArithmeticFor {
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
