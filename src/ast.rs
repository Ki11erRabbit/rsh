use std::os::unix::io::RawFd;
use nix::sys::wait::waitpid;
use std::fs::File;
use std::io::{Read, Write};
use std::os::unix::io::FromRawFd;
use crate::lexer::Lexer;
use crate::shell;
use crate::log;
use lalrpop_util::lalrpop_mod;
use std::ffi::CString;
use core::str::Split;

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

#[derive(Debug,Clone,PartialEq)]
enum QuoteType {
    None,
    Single,
    Double,
}

pub trait SplitWhitespaceIgnoringQuotes {
    fn split_whitespace_ig_qts(&self) -> Vec<&str>;
}

impl SplitWhitespaceIgnoringQuotes for str {
    fn split_whitespace_ig_qts<'input>(&'input self) -> Vec<&'input str> {
        //eprintln!("split_whitespace_ig_qts: {}", self);
        let mut quote_type = QuoteType::None;
        let mut start = 0;
        let mut end = 0;
        let mut splits: Vec<&'input str> = Vec::new();
        for (i, c) in self.char_indices() {
            match quote_type {
                QuoteType::None => {
                    if c == '\'' {
                        quote_type = QuoteType::Single;
                    } else if c == '"' {
                        quote_type = QuoteType::Double;
                    } else if c.is_whitespace() {
                        end = i;
                        splits.push(&self[start..end]);
                        start = i + 1;
                    }
                }
                QuoteType::Single => {
                    if c == '\'' {
                        quote_type = QuoteType::None;
                    }
                }
                QuoteType::Double => {
                    if c == '"' {
                        quote_type = QuoteType::None;
                    }
                }
            }
        }
        if start < self.len() {
            splits.push(&self[start..]);
        }
        splits

    }
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

        if self.prefix.is_some() {
            for word in self.prefix.as_mut().unwrap().assignment.iter_mut() {
                if word.contains("$") {
                    let mut split = word.split("=");
                    let var = split.next().unwrap();
                    let val = split.next().unwrap();
                    let val = match shell::expand_var(val) {
                        Some(expanded) => {
                            expanded
                        }
                        None => {
                            val.to_string()
                        }
                    };
                    *word = format!("{}={}", var, val);
                }
            }
        }

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
        else if subshell.starts_with("`"){
            chars.next();
            chars.next_back();
        }
        else {
            return subshell.to_string();
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

    fn recombine_double_quotes(words: &mut Vec<&str>) -> Vec<String>{
        let mut new_words = Vec::new();
        let mut in_quotes = false;
        let mut index = 0;
        for word in words.iter() {
            if word.starts_with("\"") {
                in_quotes = true;
                new_words.push(word.to_string());
            }
            else if in_quotes {
                new_words[index].push_str(" ");
                new_words[index].push_str(word);
                if word.ends_with("\"") {
                    in_quotes = false;
                }
            }
            else {
                index += 1;
                new_words.push(word.to_string());
            }
        }
        new_words
    }


    pub fn remove_whitespace(&mut self) {
        //eprintln!("suffix: {:?}", self.suffix);
        let temp = self.name.clone();
        let mut words;
        if !temp.contains('\''){
            words = temp.split_whitespace().collect::<Vec<&str>>();
            let mut new_words = if temp.contains("\"") {
		log!("recombine_double_quotes");
                Self::recombine_double_quotes(&mut words)
            } 
            else {
                words.iter().map(|word| word.to_string()).collect::<Vec<String>>()
            };

            self.name = new_words.remove(0);

            if self.suffix.is_none() {
                self.suffix = Some(Suffix {
                    io_redirect: Vec::new(),
                    word: new_words,
                });
                return;
            }
            else {
                let mut words = new_words;
                for word in self.suffix.as_ref().unwrap().word.iter() {
                    if word.contains('\'') {
                        words.push(word.to_string());
                    }
                    else {
                        let mut temp_words = word.split_whitespace().collect::<Vec<&str>>();
                        let mut new_words = if word.contains("\"") {
			    log!("recombine_double_quotes");
                            Self::recombine_double_quotes(&mut temp_words)
                        } 
                        else {
                            temp_words.iter().map(|word| word.to_string()).collect::<Vec<String>>()
                        };
                        words.append(&mut new_words);
                    }
                }

                self.suffix.as_mut().unwrap().word = words;
            }
        }
    }


    /*
     * The logic of this is disgusting since other shells will split at whitespace and we do it
     * here. We basically rip apart every string in both the name and suffix and then recombine
     * them.
     * 
     */
    pub fn expand_subshells(&mut self) {
        //eprintln!("expand_subshells");
        if self.prefix.is_some() {
            for word in self.prefix.as_mut().unwrap().assignment.iter_mut() {
                if (word.contains("$(") && word.ends_with(")")) || (word.contains("`") && word.ends_with("`")) {
                    let mut split = word.split("=");
                    let var = split.next().unwrap();
                    let val = split.next().unwrap();
                    let val = Self::eval_subshell(val);
                    *word = format!("{}={}", var, val);
                }
            }
        }

        if (self.name.starts_with("$(") && self.name.ends_with(")")) || (self.name.starts_with("`") && self.name.ends_with("`")) {
            self.name = Self::eval_subshell(&self.name);
        }

        //let temp = self.name.clone();
        //let mut words = temp.split_whitespace().collect::<Vec<&str>>();
        //self.name = words.remove(0).to_string();

        /*if self.suffix.is_none() {
            self.suffix = Some(Suffix {
                io_redirect: Vec::new(),
                word: words.iter().map(|word| word.to_string()).collect(),
            });
            return;
        }*/

        //let mut words = words.iter().map(|word| word.to_string()).collect::<Vec<String>>();

        if self.suffix.is_none() {
            return;
        }
        for word in self.suffix.as_mut().unwrap().word.iter_mut() {
            if (word.starts_with("$(") && word.ends_with(")")) || (word.starts_with("`") && word.ends_with("`")) {
                *word = Self::eval_subshell(word);
            }
        }

        /*for word in self.suffix.as_ref().unwrap().word.iter() {
            let temp_words = word.split_whitespace().collect::<Vec<&str>>();
            
            words.append(&mut temp_words.iter().map(|word| word.to_string()).collect::<Vec<String>>());
        }*/

        //self.suffix.as_mut().unwrap().word = words;


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

    pub fn eval(&mut self) {
        if self.name.as_str() == "" {
            return;
        }
        if self.name.starts_with('\'') {
            Self::cut_quotes(&mut self.name);
        }
        let mut words = Vec::new();
        for word in self.name.split_whitespace_ig_qts().iter() {
            if word.starts_with('"') {
                words.append(&mut Self::eval_double_quotes(word));
            }
            else {
                words.push(word.to_string());
            }
        }
        self.name = words.remove(0);
        
        if self.suffix.is_none() || self.suffix.as_ref().unwrap().word.is_empty() {
            self.suffix = Some(Suffix {
                io_redirect: Vec::new(),
                word: words,
            });
            return;
        }
        else {
            let mut new_words = words;
            for word in self.suffix.as_ref().unwrap().word.iter() {
                if word.starts_with('"') {
                    new_words.append(&mut Self::eval_double_quotes(word));
                }
                else if word.starts_with('\'') {
                    let mut new_word = word.to_string();
                    Self::cut_quotes(&mut new_word);
                    new_words.push(new_word);
                }
                else {
                    new_words.push(word.to_string());
                }
            }
            self.suffix.as_mut().unwrap().word = new_words;
        }

    }

    fn eval_double_quotes(word: &str) -> Vec<String> {
        if !word.contains("$(") && !word.contains("`") {
            let mut new_word = word.to_string();
            Self::cut_quotes(&mut new_word);
            return vec![new_word];
        }
        let mut new_word = word.to_string();
        Self::cut_quotes(&mut new_word);
        let mut ret = Vec::new();
        let eval = Self::eval_subshell(&new_word);
        if eval.contains(" ") {
            ret.append(&mut eval.split_whitespace().map(|word| word.to_string()).collect());
        }
        else {
            ret.push(eval);
        }
        ret
    }

    /*pub fn eval_double_quotes(&mut self) {

        eprintln!("\n\n{:?}", self.name.split_whitespace_ig_qts());


        if (self.name.starts_with("\"")) && (self.name.ends_with("\"")) {
            Self::cut_quotes(&mut self.name);

        }
        
        self.name = Self::eval_subshell(&self.name);
        let mut words;
        let name = self.name.clone();
        if self.name.contains(" ") {
            words = name.split_whitespace().collect::<Vec<&str>>();
            self.name = words.remove(0).to_string();
        }
        else {
            words = Vec::new();
        }
        if self.suffix.is_none() {
            if words.len() > 0 {
                self.suffix = Some(Suffix {
                    io_redirect: Vec::new(),
                    word: words.iter().map(|word| word.to_string()).collect(),
                });
            }
            return;
        }
        if self.suffix.is_some() {
            if self.suffix.as_ref().unwrap().word.len() > 0 {
                eprintln!("{:?}\n\n", self.suffix.as_ref().unwrap().word[0].split_whitespace_ig_qts());
            }
            else {
                eprintln!("\n\n");
            }
        }
        else {
            eprintln!("\n\n");
        }
        for word in self.suffix.as_mut().unwrap().word.iter_mut() {
            if (word.starts_with("\"")) && (word.ends_with("\"")) {
                Self::cut_quotes(word);
                if word.contains("$(") || word.contains("`") {
                    *word = Self::eval_subshell(word);
                    if word.contains(" ") {
                        let mut temp_words = word.split_whitespace().collect::<Vec<&str>>();
                        words.append(&mut temp_words);
                    }
                }
                else {
                    words.push(word);
                }
            }
            else {
                words.push(word);
            }
        }
        self.suffix.as_mut().unwrap().word = words.iter().map(|word| word.to_string()).collect();
    }*/

    pub fn remove_double_quotes(&mut self) {
        if (self.name.starts_with("\"")) && (self.name.ends_with("\"")) {
            Self::cut_quotes(&mut self.name);
        }
        if self.suffix.is_none() {
            return;
        }
        for word in self.suffix.as_mut().unwrap().word.iter_mut() {
            if (word.starts_with("\"")) && (word.ends_with("\"")) {
                Self::cut_quotes(word);
            }
        }
    }
    pub fn remove_single_quotes(&mut self) {
        if (self.name.starts_with("'")) && (self.name.ends_with("'")) {
            Self::cut_quotes(&mut self.name);
        }
        if self.suffix.is_none() {
            return;
        }
        for word in self.suffix.as_mut().unwrap().word.iter_mut() {
            if (word.starts_with("'")) && (word.ends_with("'")) {
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
