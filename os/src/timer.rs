use riscv::register::time;
use sbi_rt::set_timer;

use crate::config::CLOCK_FREQ;

const TICKS_PRE_SEC: usize = 100;
const MICRO_PRE_SEC: usize = 1_000_000;
const MESC_PRE_SEC: usize = 1000;

pub fn get_time() -> usize {
    time::read()
}

pub fn set_next_trigger() {
    set_timer((get_time() + CLOCK_FREQ / TICKS_PRE_SEC) as u64);
}

pub fn get_time_us() -> usize {
    time::read() / (CLOCK_FREQ / MICRO_PRE_SEC)
}

#[allow(unused)]
pub fn get_time_ms() -> usize {
    time::read() / (CLOCK_FREQ / MESC_PRE_SEC)
}
