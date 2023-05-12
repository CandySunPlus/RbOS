use log::info;

use crate::mm::translated_mut;
use crate::task::{
    change_program_brk, current_user_token, exit_current_and_run_next, get_taskinfo,
    suspend_current_and_run_next, TaskInfo,
};
use crate::timer::get_time_us;

#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

pub fn sys_exit(exit_code: i32) -> ! {
    info!("[kernel] Application exit with code {}", exit_code);
    exit_current_and_run_next();
    unreachable!("sys_exit");
}

pub fn sys_yield() -> isize {
    suspend_current_and_run_next();
    0
}

pub fn sys_get_time(ts: *mut TimeVal, _tz: usize) -> isize {
    let ts = translated_mut(current_user_token(), ts);
    let us = get_time_us();
    *ts = TimeVal {
        sec: us / 1_000_000,
        usec: us % 1_000_000,
    };
    0
}

pub fn sys_task_info(ti: *mut TaskInfo) -> isize {
    let ti = translated_mut(current_user_token(), ti);
    *ti = get_taskinfo();
    0
}

pub fn sys_sbrk(size: i32) -> isize {
    if let Some(old_brk) = change_program_brk(size) {
        old_brk as isize
    } else {
        -1
    }
}
