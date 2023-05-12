mod context;

use core::arch::{asm, global_asm};

pub use context::TrapContext;
use log::error;
use riscv::register::{mtvec, scause, sie, stval, stvec};

use crate::config::{TRAMPOLINE, TRAP_CONTEXT};
use crate::syscall::syscall;
use crate::task::{
    current_trap_cx, current_user_token, exit_current_and_run_next, suspend_current_and_run_next,
    user_time_end, user_time_start,
};
use crate::timer::set_next_trigger;

global_asm!(include_str!("trap.asm"));

#[no_mangle]
pub fn trap_from_kernel() -> ! {
    panic!("a trap form kernel!");
}

pub fn set_kernel_trap_entry() {
    unsafe {
        stvec::write(trap_from_kernel as usize, mtvec::TrapMode::Direct);
    }
}

pub fn set_user_trap_entry() {
    unsafe {
        stvec::write(TRAMPOLINE, stvec::TrapMode::Direct);
    }
}

pub fn init() {
    set_kernel_trap_entry();
}

#[no_mangle]
pub fn trap_handler() -> ! {
    user_time_end();
    set_kernel_trap_entry();
    let cx = current_trap_cx();
    let scause = scause::read();
    let stval = stval::read();

    match scause.cause() {
        scause::Trap::Exception(scause::Exception::UserEnvCall) => {
            cx.sepc += 4;
            cx.x[10] = syscall(cx.x[17], [cx.x[10], cx.x[11], cx.x[12]]) as usize;
        }
        scause::Trap::Exception(scause::Exception::StoreFault)
        | scause::Trap::Exception(scause::Exception::StorePageFault)
        | scause::Trap::Exception(scause::Exception::LoadFault)
        | scause::Trap::Exception(scause::Exception::LoadPageFault) => {
            error!("[kernel] PageFault in application, bad addr = {:#x}, bad instruction = {:#x}, kernel killed it.", stval, cx.sepc);
            exit_current_and_run_next();
        }
        scause::Trap::Exception(scause::Exception::IllegalInstruction) => {
            error!("[kernel] IllegalInstruction in application, kernel killed it.");
            exit_current_and_run_next();
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
    trap_return()
}

#[no_mangle]
pub fn trap_return() -> ! {
    set_user_trap_entry();
    let trap_cx_ptr = TRAP_CONTEXT;
    let user_satp = current_user_token();
    extern "C" {
        fn __alltraps();
        fn __restore();
    }

    let restore_va = __restore as usize - __alltraps as usize + TRAMPOLINE;

    unsafe {
        asm!(
            "fence.i",
            "jr {restore_va}",
            restore_va = in(reg) restore_va,
            in("a0") trap_cx_ptr,
            in("a1") user_satp,
            options(noreturn)
        );
    }
}

pub fn enable_timer_interrupt() {
    unsafe {
        sie::set_stimer();
    }
}
