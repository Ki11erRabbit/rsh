use std::os::unix::io::RawFd;
use crate::lexer::Lexer;
use lalrpop_util::lalrpop_mod;

#[derive(Debug,Clone,PartialEq)]
pub struct CompleteCommand {
    list: List,
}

#[derive(Debug,Clone,PartialEq)]
pub struct List(Vec<AndOr>);

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
    and_or: Option<Box<AndOr>>,
    conditional_exec: Option<ConditionalExec>,
    pipeline: Pipeline,
}

#[derive(Debug,Clone,PartialEq)]
pub struct Pipeline {
    bang: bool,
    pipe_sequence: PipeSequence,
}

#[derive(Debug,Clone,PartialEq)]
pub struct PipeSequence(Vec<Command>);

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
    compound_list: CompoundList,
}

#[derive(Debug,Clone,PartialEq)]
pub struct CompoundList(Term);

#[derive(Debug,Clone,PartialEq)]
pub struct Term(Vec<AndOr>);


#[derive(Debug,Clone,PartialEq)]
pub enum ForType {
    ForClauseReg(ForClauseReg),
    ForClauseList(ForClauseList),
}

#[derive(Debug,Clone,PartialEq)]
pub struct ForClauseReg {
    name: String,
}

#[derive(Debug,Clone,PartialEq)]
pub struct ForClauseList {
    name: String,
    word_list: WordList,
}

#[derive(Debug,Clone,PartialEq)]
pub struct ForClause {
    for_type: ForType,
    do_group: DoGroup,
}

#[derive(Debug,Clone,PartialEq)]
pub struct WordList(Vec<String>);

#[derive(Debug,Clone,PartialEq)]
pub struct CaseClause {
    word: String,
    case_list: Option<CaseList>,
}

#[derive(Debug,Clone,PartialEq)]
pub struct CaseList(Vec<CaseItem>);

#[derive(Debug,Clone,PartialEq)]
pub struct CaseItem {
    pattern: Pattern,
    compound_list: Option<CompoundList>,
}

#[derive(Debug,Clone,PartialEq)]
pub struct Pattern(Vec<String>);

#[derive(Debug,Clone,PartialEq)]
pub struct IfClause {
    condition: CompoundList,
    then: CompoundList,
    else_part: Vec<ElsePart>,
}

#[derive(Debug,Clone,PartialEq)]
pub struct ElsePart {
    condition: Option<CompoundList>,
    then: CompoundList,
}

#[derive(Debug,Clone,PartialEq)]
pub struct WhileClause {
    condition: CompoundList,
    do_group: DoGroup,
}

#[derive(Debug,Clone,PartialEq)]
pub struct UntilClause {
    condition: CompoundList,
    do_group: DoGroup,
}

#[derive(Debug,Clone,PartialEq)]
pub struct FunctionDefinition {
    name: String,
    function_body: FunctionBody,
}

#[derive(Debug,Clone,PartialEq)]
pub struct FunctionBody {
    compound_command: CompoundCommand,
    redirect_list: Option<RedirectList>,
}

#[derive(Debug,Clone,PartialEq)]
pub struct BraceGroup(CompoundList);

#[derive(Debug,Clone,PartialEq)]
pub struct DoGroup(CompoundList);

#[derive(Debug,Clone,PartialEq)]
pub struct SimpleCommand {
    prefix: Option<Prefix>,
    name: String,
    suffix: Option<Suffix>,
}

#[derive(Debug,Clone,PartialEq)]
pub struct Prefix {
    io_redirect: Vec<IoRedirect>,
    assignment: Vec<String>
}

#[derive(Debug,Clone,PartialEq)]
pub struct Suffix {
    io_redirect: Vec<IoRedirect>,
    word: Vec<String>,
}

#[derive(Debug,Clone,PartialEq)]
pub struct RedirectList(Vec<IoRedirect>);


#[derive(Debug,Clone,PartialEq)]
pub struct IoRedirect {
    io_number: Option<RawFd>,
    io_file: Option<IoFile>,
    io_here: Option<IoHere>,
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
    redirect_type: RedirectType,
    filename: String,
}

#[derive(Debug,Clone,PartialEq)]
pub struct IoHere {
    here: String,
}

#[derive(Debug,Clone,PartialEq)]
pub struct NewlineList {
    list: Vec<String>,
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
