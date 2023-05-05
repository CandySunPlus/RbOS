mod context;

use core::arch::global_asm;

pub use context::TrapContext;
use log::error;
use riscv::register::{mtvec, scause, sie, stval, stvec};

use crate::syscall::syscall;
use crate::task::{
    exit_current_and_run_next, suspend_current_and_run_next, user_time_end, user_time_start,
};
use crate::timer::set_next_trigger;

global_asm!(include_str!("trap.asm"));

pub fn init() {
    extern "C" {
        fn __alltraps();
    }
    unsafe { stvec::write(__alltraps as usize, mtvec::TrapMode::Direct) }
}

#[no_mangle]
pub fn trap_handler(cx: &mut TrapContext) -> &mut TrapContext {
    user_time_end();
    let scause = scause::read();
    let stval = stval::read();

    match scause.cause() {
        scause::Trap::Exception(scause::Exception::UserEnvCall) => {
            cx.sepc += 4;
            cx.x[10] = syscall(cx.x[17], [cx.x[10], cx.x[11], cx.x[12]]) as usize;
        }
        scause::Trap::Exception(scause::Exception::StoreFault)
        | scause::Trap::Exception(scause::Exception::StorePageFault) => {
            error!("[kernel] PageFault in application, kernel killed it.");
            exit_current_and_run_next();
            // run_next_app();
        }
        scause::Trap::Exception(scause::Exception::IllegalInstruction) => {
            error!("[kernel] IllegalInstruction in application, kernel killed it.");
            exit_current_and_run_next();
            // run_next_app();
        }
        scause::Trap::Interrupt(scause::Interrupt::SupervisorTimer) => {
            set_next_trigger();
            suspend_current_and_run_next();
        }
        _ => {
            panic!(
                "Unsupported trap {:?}, stval = {:#x}!",
                scause.cause(),
                stval
            );
        }
    }
    user_time_start();
    cx
}

pub fn enable_timer_interrupt() {
    unsafe {
        sie::set_stimer();
    }
}
