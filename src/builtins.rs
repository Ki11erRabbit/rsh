use std::env;
use crate::ast::SimpleCommand;
use crate::shell;





// this needs to change several variables when changing but for now we won't care
pub fn change_directory(command: &SimpleCommand) -> Result<(), std::io::Error> {
    

    let path;
    if command.suffix.is_none() {
        path = env::var("HOME").unwrap();
    } else {
        path = command.suffix.as_ref().unwrap().word[0].to_string();
    }
    env::set_current_dir(path)?;
    Ok(())
}

pub fn quit() -> Result<(), std::io::Error> {
    shell::save_history();
    std::process::exit(0);
}

pub fn jobs() -> Result<(), std::io::Error> {
    println!("{}", shell::display_jobs());
    Ok(())
}
