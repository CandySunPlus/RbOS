use alloc::sync::Arc;

use log::info;

use crate::loader::get_app_data_by_name;
use crate::mm::{translated_mut, translated_str};
use crate::task::{
    add_task, current_task, current_user_token, exit_current_and_run_next,
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

pub fn sys_task_info(ti: *mut TaskInfo) -> isize {
    let ti = translated_mut(current_user_token(), ti);
    let task_info = current_task().unwrap().get_taskinfo();
    *ti = task_info;
    0
}

pub fn sys_sbrk(size: i32) -> isize {
    if let Some(old_brk) = current_task().unwrap().change_program_brk(size) {
        old_brk as isize
    } else {
        -1
    }
}

pub fn sys_mmap(start: usize, len: usize, port: usize) -> isize {
    if current_task()
        .unwrap()
        .inner_exclusive_access()
        .mmap(start, len, port)
    {
        0
    } else {
        -1
    }
}

pub fn sys_munmap(start: usize, len: usize) -> isize {
    if current_task()
        .unwrap()
        .inner_exclusive_access()
        .munmap(start, len)
    {
        0
    } else {
        -1
    }
}

pub fn sys_get_pid() -> isize {
    current_task().unwrap().pid.0 as isize
}

pub fn sys_spawn(path: *const u8) -> isize {
    let path = translated_str(current_user_token(), path);
    if let Some(elf_data) = get_app_data_by_name(path.as_str()) {
        let task = current_task().unwrap();
        task.spawn(elf_data);
        0
    } else {
        -1
    }
}

pub fn sys_set_priority(prio: isize) -> isize {
    if prio >= 2 {
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .set_priority(prio);
        prio
    } else {
        -1
    }
}

pub fn sys_fork() -> isize {
    let current_task = current_task().unwrap();
    let new_task = current_task.fork();
    let new_pid = new_task.pid.0;

    let trap_cx = new_task.inner_exclusive_access().get_trap_cx();

    trap_cx.x[10] = 0;
    add_task(new_task);
    new_pid as isize
}

pub fn sys_exec(path: *const u8) -> isize {
    let token = current_user_token();
    let path = translated_str(token, path);
    if let Some(data) = get_app_data_by_name(path.as_str()) {
        let task = current_task().unwrap();
        task.exec(data);
        0
    } else {
        -1
    }
}

pub fn sys_waitpid(pid: isize, exit_code_ptr: *mut i32) -> isize {
    let task = current_task().unwrap();

    let mut inner = task.inner_exclusive_access();

    if !inner
        .children
        .iter()
        .any(|p| pid == -1 || pid as usize == p.getpid())
    {
        return -1;
    }

    let pair = inner.children.iter().enumerate().find(|(_, p)| {
        p.inner_exclusive_access().is_zombie() && (pid == -1 || pid as usize == p.getpid())
    });

    if let Some((idx, _)) = pair {
        let child = inner.children.remove(idx);

        assert_eq!(Arc::strong_count(&child), 1);
        let found_pid = child.getpid();
        let exit_code = child.inner_exclusive_access().exit_code;

        *translated_mut(inner.memory_set.token(), exit_code_ptr) = exit_code;
        found_pid as isize
    } else {
        -2
    }
}
