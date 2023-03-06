use std::os::unix::io::RawFd;
use crate::lexer::Lexer;
use lalrpop_util::lalrpop_mod;
use std::ffi::CString;

#[derive(Debug,Clone,PartialEq)]
pub struct CompleteCommand {
    pub list: Option<List>,
}

#[derive(Debug,Clone,PartialEq)]
pub struct List(pub Vec<AndOr>);

impl List {
    pub fn new() -> Self {
        List(Vec::new())
    }

    pub fn push(&mut self, and_or: AndOr) {
        self.0.push(and_or);
    }

    pub fn iter(&self) -> std::slice::Iter<AndOr> {
        self.0.iter()
    }
}


#[derive(Debug,Clone,PartialEq)]
pub enum ConditionalExec {
    And,
    Or,
}

#[derive(Debug,Clone,PartialEq)]
pub struct AndOr {
    pub and_or: Option<Box<AndOr>>,
    pub conditional_exec: Option<ConditionalExec>,
    pub pipeline: Pipeline,
}

#[derive(Debug,Clone,PartialEq)]
pub struct Pipeline {
    pub bang: bool,
    pub pipe_sequence: PipeSequence,
    pub background: bool,
}

#[derive(Debug,Clone,PartialEq)]
pub struct PipeSequence(pub Vec<Command>);

impl PipeSequence {
    pub fn new() -> Self {
        PipeSequence(Vec::new())
    }

    pub fn push(&mut self, command: Command) {
        self.0.push(command);
    }

    pub fn iter(&self) -> std::slice::Iter<Command> {
        self.0.iter()
    }
}

#[derive(Debug,Clone,PartialEq)]
pub enum Command {
    SimpleCommand(SimpleCommand),
    CompoundCommand(CompoundCommand, Option<RedirectList>),
    FunctionDefinition(FunctionDefinition),
}

#[derive(Debug,Clone,PartialEq)]
pub enum CompoundCommand {
    BraceGroup(BraceGroup),
    SubShell(Subshell),
    ForClause(ForClause),
    CaseClause(CaseClause),
    IfClause(IfClause),
    WhileClause(WhileClause),
    UntilClause(UntilClause),
}

#[derive(Debug,Clone,PartialEq)]
pub struct Subshell {
    pub compound_list: CompoundList,
}

#[derive(Debug,Clone,PartialEq)]
pub struct CompoundList(pub Term);

#[derive(Debug,Clone,PartialEq)]
pub struct Term(pub Vec<AndOr>);


#[derive(Debug,Clone,PartialEq)]
pub enum ForType {
    ForClauseReg(ForClauseReg),
    ForClauseList(ForClauseList),
}

#[derive(Debug,Clone,PartialEq)]
pub struct ForClauseReg {
    pub name: String,
}

#[derive(Debug,Clone,PartialEq)]
pub struct ForClauseList {
    pub name: String,
    pub word_list: WordList,
}

#[derive(Debug,Clone,PartialEq)]
pub struct ForClause {
    pub for_type: ForType,
    pub do_group: DoGroup,
}

#[derive(Debug,Clone,PartialEq)]
pub struct WordList(pub Vec<String>);

#[derive(Debug,Clone,PartialEq)]
pub struct CaseClause {
    pub word: String,
    pub case_list: Option<CaseList>,
}

#[derive(Debug,Clone,PartialEq)]
pub struct CaseList(pub Vec<CaseItem>);

#[derive(Debug,Clone,PartialEq)]
pub struct CaseItem {
    pub pattern: Pattern,
    pub compound_list: Option<CompoundList>,
}

#[derive(Debug,Clone,PartialEq)]
pub struct Pattern(pub Vec<String>);

#[derive(Debug,Clone,PartialEq)]
pub struct IfClause {
    pub condition: CompoundList,
    pub then: CompoundList,
    pub else_part: Vec<ElsePart>,
}

#[derive(Debug,Clone,PartialEq)]
pub struct ElsePart {
    pub condition: Option<CompoundList>,
    pub then: CompoundList,
}

#[derive(Debug,Clone,PartialEq)]
pub struct WhileClause {
    pub condition: CompoundList,
    pub do_group: DoGroup,
}

#[derive(Debug,Clone,PartialEq)]
pub struct UntilClause {
    pub condition: CompoundList,
    pub do_group: DoGroup,
}

#[derive(Debug,Clone,PartialEq)]
pub struct FunctionDefinition {
    pub name: String,
    pub function_body: FunctionBody,
}

#[derive(Debug,Clone,PartialEq)]
pub struct FunctionBody {
    pub compound_command: CompoundCommand,
    pub redirect_list: Option<RedirectList>,
}

#[derive(Debug,Clone,PartialEq)]
pub struct BraceGroup(pub CompoundList);

#[derive(Debug,Clone,PartialEq)]
pub struct DoGroup(pub CompoundList);

#[derive(Debug,Clone,PartialEq)]
pub struct SimpleCommand {
    pub prefix: Option<Prefix>,
    pub name: String,
    pub suffix: Option<Suffix>,
}

impl SimpleCommand {
    pub fn argv(&self) -> Vec<CString> {
        let mut argv = Vec::new();
        argv.push(CString::new(self.name.clone()).unwrap());
        
        if self.suffix.is_some() {
            for word in self.suffix.as_ref().unwrap().word.iter() {
                argv.push(CString::new(word.clone()).unwrap());
            }
        }

        argv
    }

    pub fn cmd(&self) -> String {
        let mut cmd = String::new();
        cmd.push_str(&self.name);

        if self.suffix.is_some() {
            for word in self.suffix.as_ref().unwrap().word.iter() {
                cmd.push_str(" ");
                cmd.push_str(&word);
            }
        }

        cmd
    }

    pub fn prefix_suffix(&self) -> (Option<&Prefix>, Option<&Suffix>) {
        let prefix = self.prefix.as_ref();
        let suffix = self.suffix.as_ref();
        (prefix, suffix)
    }
}

#[derive(Debug,Clone,PartialEq)]
pub struct Prefix {
    pub io_redirect: Vec<IoRedirect>,
    pub assignment: Vec<String>
}

#[derive(Debug,Clone,PartialEq)]
pub struct Suffix {
    pub io_redirect: Vec<IoRedirect>,
    pub word: Vec<String>,
}

#[derive(Debug,Clone,PartialEq)]
pub struct RedirectList(pub Vec<IoRedirect>);


#[derive(Debug,Clone,PartialEq)]
pub struct IoRedirect {
    pub io_number: Option<RawFd>,
    pub io_file: Option<IoFile>,
    pub io_here: Option<IoHere>,
}

#[derive(Debug,Clone,PartialEq)]
pub enum RedirectType {
    Input,
    Output,
    Append,
    Clobber,
}

#[derive(Debug,Clone,PartialEq)]
pub struct IoFile {
    pub redirect_type: RedirectType,
    pub filename: String,
}

#[derive(Debug,Clone,PartialEq)]
pub struct IoHere {
    pub here: String,
}

#[derive(Debug,Clone,PartialEq)]
pub struct NewlineList {
    pub list: Vec<String>,
}



lalrpop_mod!(pub grammar);

#[test]
fn test_parser() {
    let input = "echo Hello world";
    let lexer = Lexer::new(input);
    let ast = grammar::CompleteCommandParser::new()
        .parse(input,lexer)
        .unwrap();
    println!("{:#?}", ast);
}

#[test]
fn test_parser_quote() {
    let input = "echo \"Hello world\"";
    let lexer = Lexer::new(input);
    let ast = grammar::CompleteCommandParser::new()
        .parse(input,lexer)
        .unwrap();
    println!("{:#?}", ast);
}

#[test]
fn test_parser_pipeline() {
    let input = "echo Hello world | cat";
    let lexer = Lexer::new(input);
    let ast = grammar::CompleteCommandParser::new()
        .parse(input,lexer)
        .unwrap();
    println!("{:#?}", ast);
}

#[test]
fn test_parser_pipeline_longer() {
    let input = "echo Hello world | cat | wc";
    let lexer = Lexer::new(input);
    let ast = grammar::CompleteCommandParser::new()
        .parse(input,lexer)
        .unwrap();
    println!("{:#?}", ast);
}

#[test]
fn test_parser_long_pipeline() {
    let input = "echo Hello world | cat | wc | grep world";
    let lexer = Lexer::new(input);
    let ast = grammar::CompleteCommandParser::new()
        .parse(input,lexer)
        .unwrap();
    println!("{:#?}", ast);
}

#[test]
fn test_parser_really_long_pipeline() {
    let input = "echo Hello world | cat | wc | grep world";
    let lexer = Lexer::new(input);
    let ast = grammar::CompleteCommandParser::new()
        .parse(input,lexer)
        .unwrap();
    println!("{:#?}", ast);
}

#[test]
fn test_parser_redirect() {
    let input = "echo Hello world > file.txt";
    let lexer = Lexer::new(input);
    let ast = grammar::CompleteCommandParser::new()
        .parse(input,lexer)
        .unwrap();
    println!("{:#?}", ast);
}

#[test]
fn test_parser_redirect_append() {
    let input = "echo Hello world >> file.txt";
    let lexer = Lexer::new(input);
    let ast = grammar::CompleteCommandParser::new()
        .parse(input,lexer)
        .unwrap();
    println!("{:#?}", ast);
}

#[test]
fn test_parser_redirect_input() {
    let input = "cat < file.txt";
    let lexer = Lexer::new(input);
    let ast = grammar::CompleteCommandParser::new()
        .parse(input,lexer)
        .unwrap();
    println!("{:#?}", ast);
}

#[test]
fn test_parser_redirect_pipe() {
    let input = "cat < file.txt | wc";
    let lexer = Lexer::new(input);
    let ast = grammar::CompleteCommandParser::new()
        .parse(input,lexer)
        .unwrap();
    println!("{:#?}", ast);
}

#[test]
fn test_parser_env_assignment() {
    let input = "FOO=bar echo Hello world";
    let lexer = Lexer::new(input);
    let ast = grammar::CompleteCommandParser::new()
        .parse(input,lexer)
        .unwrap();
    println!("{:#?}", ast);
}

#[test]
fn test_parser_env_assignment_pipe() {
    let input = "FOO=bar echo Hello world | cat";
    let lexer = Lexer::new(input);
    let ast = grammar::CompleteCommandParser::new()
        .parse(input,lexer)
        .unwrap();
    println!("{:#?}", ast);
}

#[test]
fn test_parser_env_assignment_redirect_pipe() {
    let input = "FOO=bar echo Hello world > file.txt | cat";
    let lexer = Lexer::new(input);
    let ast = grammar::CompleteCommandParser::new()
        .parse(input,lexer)
        .unwrap();
    println!("{:#?}", ast);
}
