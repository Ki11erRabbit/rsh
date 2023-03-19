use std::fs;

use crate::shell;

use std::borrow::Cow::{self, Borrowed, Owned};

use rustyline::completion::FilenameCompleter;
use rustyline::highlight::{Highlighter, MatchingBracketHighlighter};
use rustyline::hint::HistoryHinter;
use rustyline::validate::MatchingBracketValidator;
use rustyline::{Completer, Helper, Hinter, Validator};

use rustyline::{Context,Result};
use rustyline::completion::{Pair, Completer};
use rustyline::completion::Candidate;


#[derive(Helper, Completer, Hinter, Validator)]
pub struct CompletionHelper {
    #[rustyline(Completer)]
    completer: PathCompleter,
    highlighter: MatchingBracketHighlighter,
    #[rustyline(Validator)]
    validator: MatchingBracketValidator,
    #[rustyline(Hinter)]
    hinter: HistoryHinter,
}

impl Highlighter for CompletionHelper {
    fn highlight_prompt<'b, 's: 'b, 'p: 'b>(&'s self, prompt: &'p str, default: bool,) -> Cow<'b, str> {
        if default {
            Borrowed(prompt)
        } else {
            Borrowed(prompt)
        }
    }

    fn highlight_hint<'h>(&self, hint: &'h str) -> Cow<'h, str> {
        Owned("\x1b[1m".to_owned() + hint + "\x1b[m")
    }

    fn highlight<'l>(&self, line: &'l str, pos: usize) -> Cow<'l, str> {
        self.highlighter.highlight(line, pos)
    }

    fn highlight_char(&self, line: &str, pos: usize) -> bool {
        self.highlighter.highlight_char(line, pos)
    }
}

impl Default for CompletionHelper {
    fn default() -> CompletionHelper {
        Self {
            completer: PathCompleter::new(),//FilenameCompleter::new(),
            highlighter: MatchingBracketHighlighter::new(),
            validator: MatchingBracketValidator::new(),
            hinter: HistoryHinter {},
        }
    }
}


pub struct PathCompleter;

impl Default for PathCompleter {
    fn default() -> Self {
        Self {}
    }
}

impl PathCompleter {
    fn new() -> Self {
        Self {}
    }

    pub fn complete_cmd(&self, line: &str, pos: usize) -> Result<(usize, Vec<Pair>)> {
        let (start, mut matches) = self.complete_cmd_unsorted(line, pos)?;

        matches.sort_by(|a, b| a.display().cmp(b.display()));
        Ok((start, matches))
    }

    pub fn complete_cmd_unsorted(&self, line: &str, pos: usize) -> Result<(usize,Vec<Pair>)> {
        let matches = command_complete(line);

        Ok((0, matches))
    }

}

impl Completer for PathCompleter {
    type Candidate = Pair;

    fn complete(&self, line: &str, pos: usize, _ctx: &Context<'_>) -> Result<(usize, Vec<Pair>)> {
        self.complete_cmd(line, pos)
    }
}

fn command_complete(line: &str) -> Vec<Pair> {
    let path = {
        shell::expand_var("PATH").unwrap()
    };
    let paths = path.split(":");
    let mut entries = Vec::new();

    for path in paths {
        for entry in fs::read_dir(path).unwrap() {
            let entry = entry.unwrap();
            let name = entry.file_name().into_string().unwrap();

            if !name.starts_with(line) {
                continue;
            }
            entries.push(Pair {
                display: name.clone(),
                replacement: name,
            });
        }
    }

    entries
}

