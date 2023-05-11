#![allow(unused)]

use sbi_rt::{legacy, system_reset, NoReason, Shutdown, SystemFailure};

pub fn console_putchar(c: usize) {
    #[allow(deprecated)]
    legacy::console_putchar(c);
}

pub fn console_getchar() -> usize {
    #[allow(deprecated)]
    legacy::console_getchar()
}

pub fn shutdown(failure: bool) -> ! {
    if !failure {
        system_reset(Shutdown, NoReason);
    } else {
        system_reset(Shutdown, SystemFailure);
    }
    unreachable!("It should shutdown");
}
