mod context;

use core::arch::global_asm;

pub use context::TrapContext;
use riscv::register::{mtvec, scause, stval, stvec};

global_asm!(include_str!("trap.asm"));

pub fn init() {
    extern "C" {
        fn __alltraps();
    }
    unsafe { stvec::write(__alltraps as usize, mtvec::TrapMode::Direct) }
}

#[no_mangle]
pub fn trap_handler(cx: &mut TrapContext) -> &mut TrapContext {
    let scause = scause::read();
    let stval = stval::read();

    match scause.cause() {
        scause::Trap::Exception(scause::Exception::UserEnvCall) => {}
        scause::Trap::Exception(scause::Exception::StoreFault) => {}
        scause::Trap::Exception(scause::Exception::IllegalInstruction) => {}
        _ => {
            panic!(
                "Unsupported trap {:?}, stval = {:#x}!",
                scause.cause(),
                stval
            );
        }
    }
    cx
}
