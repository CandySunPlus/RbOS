use log::info;

use crate::mm::translated_mut;
use crate::task::{
    current_task, current_user_token, exit_current_and_run_next, suspend_current_and_run_next,
};
use crate::timer::get_time_us;

#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

pub fn sys_exit(exit_code: i32) -> ! {
    info!(
        "[kernel] pid[{}] Application exit with code {}",
        current_task().unwrap().pid.0,
        exit_code
    );
    exit_current_and_run_next(exit_code);
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

// pub fn sys_task_info(ti: *mut TaskInfo) -> isize {
//     let ti = translated_mut(current_user_token(), ti);
//     *ti = get_taskinfo();
//     0
// }

pub fn sys_sbrk(size: i32) -> isize {
    if let Some(old_brk) = current_task().unwrap().change_program_brk(size) {
        old_brk as isize
    } else {
        -1
    }
}

pub fn sys_mmap(start: usize, len: usize, port: usize) -> isize {
    -1
}

pub fn sys_munmap(start: usize, len: usize) -> isize {
    -1
}

pub fn sys_fork() -> isize {
    0
}

pub fn sys_exec(path: *const u8) -> isize {
    0
}

pub fn sys_waitpid(pid: isize, exit_code_ptr: *mut i32) -> isize {
    0
}
