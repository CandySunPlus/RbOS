use core::arch::asm;

use log::error;

pub unsafe fn print_stack_trace() {
    let mut fp: *const usize;
    asm!("mv {}, fp", out(reg) fp);

    error!("== Begin stack trace ==");

    while fp.is_null() {
        let saved_ra = *fp.sub(1);
        let saved_fp = *fp.sub(2);

        error!("0x{:016x}, fp = 0x{:016x}", saved_ra, saved_fp);

        fp = saved_fp as *const usize;
    }

    error!("== End stack trace ==");
}
