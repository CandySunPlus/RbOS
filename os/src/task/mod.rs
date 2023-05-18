mod context;
mod manager;
mod pid;
mod processor;
mod switch;
mod task;

use alloc::sync::Arc;

pub use context::TaskContext;
use lazy_static::lazy_static;
use log::info;
pub use manager::add_task;
use processor::schedule;
pub use processor::{
    current_task, current_trap_cx, current_user_token, run_tasks, take_current_task,
};
pub use task::TaskStatus;

use self::task::TaskControlBlock;
use crate::loader::get_app_data_by_name;

pub fn suspend_current_and_run_next() {
    let task = take_current_task().unwrap();
    let mut task_inner = task.inner_exclusive_access();
    let task_cx_ptr = &mut task_inner.task_cx as *mut TaskContext;

    task_inner.task_status = TaskStatus::Ready;

    drop(task_inner);

    add_task(task);

    schedule(task_cx_ptr);
}

pub const IDLE_PID: usize = 0;

pub fn exit_current_and_run_next(exit_code: i32) {
    let task = take_current_task().unwrap();

    let pid = task.getpid();
    if pid == IDLE_PID {
        info!("[kernel] Idle process exit with exit_code {}...", exit_code);
        panic!("All application completed!");
    }

    let mut inner = task.inner_exclusive_access();

    inner.task_status = TaskStatus::Zombie;
    inner.exit_code = exit_code;

    // TODO: INITPROC

    inner.children.clear();
    inner.memory_set.recycle_data_pages();

    drop(inner);
    drop(task);

    let mut _unused = TaskContext::zero_init();
    schedule(&mut _unused as *mut _);
}

lazy_static! {
    pub static ref INITPROC: Arc<TaskControlBlock> = Arc::new(TaskControlBlock::new(
        get_app_data_by_name("ch5b_initproc").unwrap()
    ));
}

pub fn add_initproc() {
    add_task(INITPROC.clone());
}
