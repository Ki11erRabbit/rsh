

pub struct CompleteCommand {
    list: List,
}

pub struct List(Vec<AndOr>);


pub enum ConditionalExec {
    And,
    Or,
}

pub struct AndOr {
    and_or: Option<Box<AndOr>>,
    conditional_exec: Option<ConditionalExec>,
    pipeline: Pipeline,
}

pub struct Pipeline {
    bang: bool,
    pipe_sequence: PipeSequence,
}

pub struct PipeSequence(Vec<Command>);

pub enum Command {
    SimpleCommand(SimpleCommand),
    CompoundCommand(CompoundCommand, Option<RedirectList>),
    FunctionDefinition(FunctionDefinition),
}

pub enum CompoundCommand {
    BraceGroup(BraceGroup),
    SubShell(Subshell),
    ForClause(ForClause),
    CaseClause(CaseClause),
    IfClause(IfClause),
    WhileClause(WhileClause),
    UntilClause(UntilClause),
}

pub struct Subshell {
    compound_list: CompoundList,
}

pub struct CompoundList(Term);

pub struct Term(Vec<AndOr>);


pub enum ForType {
    ForClauseReg(ForClauseReg),
    ForClauseList(ForClauseList),
}

pub struct ForClauseReg {
    name: String,
}

pub struct ForClauseList {
    name: String,
    word_list: WordList,
}

pub struct ForClause {
    for_type: ForType,
    do_group: DoGroup,
}

pub struct WordList(Vec<String>);

pub struct CaseClause {
    word: String,
    case_list: Option<CaseList>,
}

pub struct CaseList(Vec<CaseItem>);

pub struct CaseItem {
    pattern: Pattern,
    compound_list: Option<CompoundList>,
}

pub struct Pattern(Vec<String>);

pub struct IfClause {
    condition: CompoundList,
    then: CompoundList,
    else_part: Vec<ElsePart>,
}

pub struct ElsePart {
    condition: Option<CompoundList>,
    then: CompoundList,
}

pub struct WhileClause {
    condition: CompoundList,
    do_group: DoGroup,
}

pub struct UntilClause {
    condition: CompoundList,
    do_group: DoGroup,
}

pub struct FunctionDefinition {
    name: String,
    function_body: FunctionBody,
}

pub struct FunctionBody {
    compound_command: CompoundCommand,
    redirect_list: Option<RedirectList>,
}

pub struct BraceGroup(CompoundList);

pub struct DoGroup(CompoundList);

pub struct SimpleCommand {
    prefix: Option<Prefix>,
    name: String,
    suffix: Option<Suffix>,
}

pub struct Prefix {
    io_redirect: Vec<IoRedirect>,
    assignment: Vec<String>
}

pub struct Suffix {
    io_redirect: Vec<IoRedirect>,
    word: Vec<String>,
}

pub struct RedirectList(Vec<IoRedirect>);


pub struct IoRedirect {
    io_number: Option<RawFd>,
    io_file: Option<IoFile>,
    io_here: Option<IoHere>,
}

pub enum RedirectType {
    Input,
    Output,
    Append,
    Clobber,
}

pub struct IoFile {
    redirect_type: RedirectType,
    filename: String,
}

pub struct IoHere {
    here: String,
}

pub struct NewlineList {
    list: Vec<String>,
}



lalrpop_mod!(pub parser_alt);

#[test]
fn test_parser() {
    let input = "echo Hello, world!";
    let ast = parser_alt::CompleteCommandParser::new()
        .parser(input)
        .unwrap();
    println!("{:#?}", ast);
}
