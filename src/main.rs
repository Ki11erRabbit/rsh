//mod shell;
mod parser;
extern crate pest;
#[macro_use]
extern crate pest_derive;


use std::thread::sleep;

use signal_hook::{iterator::Signals};

fn sig_int_handler() {
        sleep(std::time::Duration::from_secs(4));
    unsafe {
        NAME = 0;
    }
}


static mut NAME: i32 = 42;


fn main() {

    unsafe {
        signal_hook::low_level::register(2, sig_int_handler).unwrap();
    }


    while unsafe {NAME == 42} {
        println!("Looping");
        sleep(std::time::Duration::from_secs(1));
    }
    println!("Hello, world!");
}
