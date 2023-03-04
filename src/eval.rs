use crate::parser::{self, Assignment, Command,Ast, BinaryExpression, ConditionalExpression, Expression, Initializer, LocalDeclaration, RunIf, Word, RedirectionType};
use crate::process::{self, CommandExitStatus};
use crate::exec;
use std::os::unix::io::FromRawFd;
use std::os::unix::io::RawFd;
use nix::unistd::Pid;
use nix::sys::wait::wait;

pub type Result<I> = std::result::Result<I, &'static str>;

pub struct Context {
    pub stdin: RawFd,
    pub stdout: RawFd,
    pub stderr: RawFd,
    pub background: bool,
    pub pgid: Option<Pid>,
}








fn run_simple_command(argv: &[Word], redirects: &[parser::Redirection], assignments: &[Assignment], context: &Context) -> Result<CommandExitStatus> {
    //expand words
    if argv.is_empty() {
        return Ok(CommandExitStatus::ExitedWith(0));
    }

    let argv = argv.iter().map(|w| w.to_string()).collect::<Vec<String>>();

    //function


    //builtin
    

    exec::exec(argv, redirects, assignments, context)
}




fn run_command(command: &parser::Command, context: &Context) -> Result<CommandExitStatus> {
    let result = match command {
        parser::Command::SimpleCommand {argv,redirects, assignments} => run_simple_command(argv, redirects, assignments, context)?,
        _ => unimplemented!(),
    };

    wait().unwrap();


    Ok(result)
}

fn run_pipeline(code: &str, pipeline: &parser::Pipeline, pipeline_stdin: RawFd, pipeline_stdout: RawFd, stderr: RawFd, background: bool) -> CommandExitStatus {
    //let mut last_result = None;
    let mut iter = pipeline.commands.iter().peekable();
    //let mut childs = Vec::new();
    let mut stdin = pipeline_stdin;
    let mut pgid = None;    

    while let Some(command) = iter.next() {
        let mut stdout = pipeline_stdout;

        let result = run_command(command, &Context {
            stdin,
            stdout,
            stderr,
            background,
            pgid,
        });
    }

    CommandExitStatus::ExitedWith(0)

}


pub fn run_terms(terms: &[parser::Term], stdin: RawFd, stdout: RawFd, stderr: RawFd) -> CommandExitStatus {
    let mut last_status = CommandExitStatus::ExitedWith(0);
    for term in terms {
        for pipeline in &term.pipelines {
            match (&last_status, &pipeline.run_if) {
                (CommandExitStatus::ExitedWith(0), RunIf::Success) => (),
                (CommandExitStatus::ExitedWith(_), RunIf::Failure) => (),
                (CommandExitStatus::Break, _) => return CommandExitStatus::Break,
                (CommandExitStatus::Continue, _) => return CommandExitStatus::Continue,
                (CommandExitStatus::Return, _) => return CommandExitStatus::Return,
                (_, RunIf::Always) => (),
                _ => continue,
            }

            last_status = run_pipeline(&term.code, pipeline, stdin, stdout, stderr, term.background);
        }
    }

    last_status
}




pub fn eval(ast: &Ast, stdin: RawFd, stdout: RawFd, stderr: RawFd) -> CommandExitStatus {
    run_terms(&ast.terms,stdin,stdout,stderr)
}
