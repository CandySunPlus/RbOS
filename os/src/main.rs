#![no_std]
#![no_main]
#![feature(panic_info_message)]
#![feature(alloc_error_handler)]

use core::arch::global_asm;
use core::panic::PanicInfo;

use log::{error, info};
use sbi::shutdown;
use stack_trace::print_stack_trace;

extern crate alloc;

mod config;
mod console;
mod fs;
pub mod loader;
mod logging;
mod mm;
mod sbi;
mod stack_trace;
mod sync;
pub mod syscall;
mod task;
mod timer;
pub mod trap;

global_asm!(include_str!("entry.asm"));
global_asm!(include_str!("link_app.S"));

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    if let Some(location) = info.location() {
        error!(
            "Panicked at {}:{} {}",
            location.file(),
            location.line(),
            info.message().unwrap()
        );
    } else {
        error!("Panicked: {}", info.message().unwrap());
    }

    unsafe {
        print_stack_trace();
    }

    shutdown(true)
}

#[no_mangle]
pub fn rust_main() -> ! {
    clear_bss();
    logging::init();
    info!("[kernel] Hello, world!");
    mm::init();
    task::add_initproc();
    info!("[kernel] after initproc!");
    trap::init();
    trap::enable_timer_interrupt();
    timer::set_next_trigger();
    loader::list_apps();
    task::run_tasks();
    unreachable!("rust_main");
}

fn clear_bss() {
    extern "C" {
        fn sbss();
        fn ebss();
    }

    (sbss as usize..ebss as usize).for_each(|a| unsafe { (a as *mut u8).write_volatile(0) });
}
