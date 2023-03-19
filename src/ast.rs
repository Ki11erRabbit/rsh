use std::os::unix::io::RawFd;
use nix::sys::wait::waitpid;
use std::fs::File;
use std::io::{Read, Write};
use std::os::unix::io::FromRawFd;
use crate::lexer::Lexer;
use crate::shell;
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
    pub fn iter_mut(&mut self) -> std::slice::IterMut<Command> {
        self.0.iter_mut()
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

lalrpop_mod!(pub grammar);
impl SimpleCommand {
    pub fn alias_lookup(&mut self) {
        match shell::lookup_alias(&self.name) {
            Some(alias) => {
                let name = alias.0;
                let args_opt = alias.1;
                let mut args: Vec<String>;
                if args_opt.is_some() {
                    args = args_opt.unwrap();
                } else {
                    args = Vec::new();
                }
                self.name = name;
                match &mut self.suffix {
                    Some(suffix) => {
                        suffix.word.append(&mut args);
                    }
                    None => {
                        self.suffix = Some(Suffix {
                            io_redirect: Vec::new(),
                            word: args,
                        });
                    }
                }
            }
            None => {}
        }
    }

    pub fn expand_vars(&mut self) {
        if self.name.starts_with("$") {
            let mut chars = self.name.chars();
            chars.next();
            match shell::expand_var(&chars.collect::<String>()) {
                Some(name) => {
                    self.name = name;
                }
                None => {}
            }
        }
        if self.suffix.is_none() {
            return;
        }
        for word in self.suffix.as_mut().unwrap().word.iter_mut() {
            if word.starts_with("$") {
                let mut chars = word.chars();
                chars.next();
                match shell::expand_var(&chars.collect::<String>()) {
                    Some(expanded) => {
                        *word = expanded;
                    }
                    None => {}
                }
            }
        }
        /*self.suffix.as_mut().unwrap().word.iter_mut().for_each(|word| {
            println!("word: {}", word);
            if word.starts_with("$") {
                match shell::expand_var(&word[1..]) {
                    Some(expanded) => {
                        *word = expanded;
                    }
                    None => {}
                }
            }
        });*/
    }


    fn create_subshell(chr: char, chars: &mut std::str::Chars) -> String {
        let mut subshell = String::new();

        if chr == '$' {
            let Some(c) = chars.next() else { panic!("Unexpected EOF") };
            subshell.push(chr);
            if c == '(' {
                subshell.push(c);
                while let Some(c) = chars.next() {
                    if c == ')' {
                        subshell.push(c);
                        break;
                    }
                    subshell.push(c);
                }
                

            }
        }
        else {
            subshell.push(chr);
            while let Some(c) = chars.next() {
                if c == '`' {
                    subshell.push(c);
                    break;
                }
                subshell.push(c);
            }
        }

        subshell
    }

    fn eval_subshell(subshell: &str) -> String {

        let mut chars = subshell.chars();

        if subshell.starts_with("$(") {
            chars.next();
            chars.next();
            chars.next_back();
        }
        else {
            chars.next();
            chars.next_back();
        }

        let subshell = &chars.collect::<String>();

        let lexer = Lexer::new(&subshell);
        let mut ast = grammar::CompleteCommandParser::new()
            .parse(&subshell,lexer)
            .unwrap();

        let pip: (RawFd,RawFd) = nix::unistd::pipe().unwrap();

        match unsafe {nix::unistd::fork().expect("failed to fork")} {
            nix::unistd::ForkResult::Parent { child } => {
                nix::unistd::close(pip.1).unwrap();
                let mut buf = String::new();
                let mut file = unsafe { File::from_raw_fd(pip.0) };
                waitpid(child, None).unwrap();
                file.read_to_string(&mut buf).unwrap();
                nix::unistd::close(pip.0).unwrap();
                buf.trim().to_string()
            },
            nix::unistd::ForkResult::Child => {
                nix::unistd::dup2(pip.1, 1).unwrap();
                nix::unistd::dup2(pip.1, 2).unwrap();
                nix::unistd::close(pip.1).unwrap();
                nix::unistd::close(pip.0).unwrap();
                let _ = crate::eval::eval(&mut ast);
                std::process::exit(0);
            }
        }
    }


    pub fn remove_whitespace(&mut self) {
        let temp = self.name.clone();
        let mut words = temp.split_whitespace().collect::<Vec<&str>>();
        self.name = words.remove(0).to_string();

        if self.suffix.is_none() {
            self.suffix = Some(Suffix {
                io_redirect: Vec::new(),
                word: words.iter().map(|word| word.to_string()).collect(),
            });
            return;
        }

        let mut words = words.iter().map(|word| word.to_string()).collect::<Vec<String>>();

        for word in self.suffix.as_ref().unwrap().word.iter() {
            let temp_words = word.split_whitespace().collect::<Vec<&str>>();
            
            words.append(&mut temp_words.iter().map(|word| word.to_string()).collect::<Vec<String>>());
        }

        self.suffix.as_mut().unwrap().word = words;

    }


    /*
     * The logic of this is disgusting since other shells will split at whitespace and we do it
     * here. We basically rip apart every string in both the name and suffix and then recombine
     * them.
     * 
     */
    pub fn expand_subshells(&mut self) {
        if (self.name.starts_with("$(") && self.name.ends_with(")")) || (self.name.starts_with("`") && self.name.ends_with("`")) {
            self.name = Self::eval_subshell(&self.name);
        }

        let temp = self.name.clone();
        let mut words = temp.split_whitespace().collect::<Vec<&str>>();
        self.name = words.remove(0).to_string();

        if self.suffix.is_none() {
            self.suffix = Some(Suffix {
                io_redirect: Vec::new(),
                word: words.iter().map(|word| word.to_string()).collect(),
            });
            return;
        }

        let mut words = words.iter().map(|word| word.to_string()).collect::<Vec<String>>();

        for word in self.suffix.as_mut().unwrap().word.iter_mut() {
            if (word.starts_with("$(") && word.ends_with(")")) || (word.starts_with("`") && word.ends_with("`")) {
                *word = Self::eval_subshell(word);
            }
        }

        for word in self.suffix.as_ref().unwrap().word.iter() {
            let temp_words = word.split_whitespace().collect::<Vec<&str>>();
            
            words.append(&mut temp_words.iter().map(|word| word.to_string()).collect::<Vec<String>>());
        }

        self.suffix.as_mut().unwrap().word = words;


    }

    fn cut_quotes(word: &mut String) {
        let mut chars = word.chars();
        let mut try_expand_subshell = false;
        if chars.next() == Some('"') {
            try_expand_subshell = true;
        }
        chars.next_back();


        if try_expand_subshell {
            let mut new_word = String::new();
            while let Some(chr) = chars.next()  {
                if chr == '$' || chr == '`' {
                    let subshell = Self::create_subshell(chr,&mut chars);
                    
                    new_word = new_word + &Self::eval_subshell(&subshell);
                }
                else {
                    new_word.push(chr);
                }
                
            }

            *word = new_word;
            return;
        }

        *word = chars.collect::<String>();
    }

    pub fn remove_quotes(&mut self) {
        if (self.name.starts_with("\"") || self.name.starts_with("'")) && (self.name.ends_with("\"") || self.name.ends_with("'")) {
            Self::cut_quotes(&mut self.name);
        }
        if self.suffix.is_none() {
            return;
        }
        for word in self.suffix.as_mut().unwrap().word.iter_mut() {
            if (word.starts_with("\"") || word.starts_with("'")) && (word.ends_with("\"") || word.ends_with("'")) {
                Self::cut_quotes(word);
            }
        }
    }



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

        /*if self.suffix.is_some() {
            for word in self.suffix.as_ref().unwrap().word.iter() {
                cmd.push_str(" ");
                cmd.push_str(&word);
            }
        }*/

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


mod test {
    use super::*;
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

    #[test]
    fn test_function() {
        let input = "foo() { echo Hello world; }";
        let lexer = Lexer::new(input);
        let ast = grammar::CompleteCommandParser::new()
            .parse(input,lexer)
            .unwrap();
        println!("{:#?}", ast);
    }

    #[test]
    fn test_function_alt() {
        let input = "foo () { echo Hello world }";
        let lexer = Lexer::new(input);
        let ast = grammar::CompleteCommandParser::new()
            .parse(input,lexer)
            .unwrap();
        println!("{:#?}", ast);
    }
}
