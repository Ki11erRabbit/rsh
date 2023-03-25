use std::sync::atomic::{AtomicBool, Ordering};

static mut PRINT_OUT: AtomicBool = AtomicBool::new(false);

pub fn set_print_out(value: bool) {
    unsafe {
	PRINT_OUT.store(value, Ordering::Relaxed);
    }
}

pub fn get_print_out() -> bool {
    unsafe {
	PRINT_OUT.load(Ordering::Relaxed)
    }
}

/// if PRINT_OUT is true, print to stderr
/// otherwise, print out to the log file
/// # TODO: add log file
#[macro_export]
macro_rules! log {
    ($($arg:tt)*) => ({
	if crate::log::get_print_out() {
	    eprintln!($($arg)*);
	}
	
    })
}
