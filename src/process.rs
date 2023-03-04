use nix::unistd::Pid;


pub enum CommandExitStatus {
    ExitedWith(i32),
    Running(Pid),
    Break,
    Continue,
    Return,
    NoExec,
}
