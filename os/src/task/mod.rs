use alloc::vec;

use lazy_static::lazy_static;
use log::info;

use self::switch::__switch;
use self::task::TaskControlBlock;
use crate::config::MAX_SYSCALL_NUM;
use crate::loader::{get_app_data, get_num_app};
use crate::sbi::shutdown;
use crate::sync::UPSafeCell;
use crate::task::context::TaskContext;
pub use crate::task::task::TaskStatus;
use crate::timer::get_time_us;
use crate::trap::TrapContext;

mod context;
mod switch;
mod task;

pub struct TaskManager {
    num_app: usize,
    inner: UPSafeCell<TaskManagerInner>,
}

pub struct TaskManagerInner {
    tasks: vec::Vec<TaskControlBlock>,
    current_task: usize,
    stop_watch: usize,
}

#[allow(dead_code)]
#[derive(Clone)]
pub struct TaskInfo {
    status: TaskStatus,
    syscall_times: [u32; MAX_SYSCALL_NUM],
    time: usize,
}

lazy_static! {
    pub static ref TASK_MANAGER: TaskManager = {
        info!("init TASK_MANAGER");
        let num_app = get_num_app();
        info!("num_app = {num_app}");
        let mut tasks = vec::Vec::new();

        for i in 0..num_app {
            tasks.push(TaskControlBlock::new(get_app_data(i), i));
        }

        TaskManager {
            num_app,
            inner: unsafe {
                UPSafeCell::new(TaskManagerInner {
                    tasks,
                    current_task: 0,
                    stop_watch: 0,
                })
            },
        }
    };
}

impl TaskManagerInner {
    fn refresh_stop_watch(&mut self) -> usize {
        let start_time = self.stop_watch;
        self.stop_watch = get_time_us();
        self.stop_watch - start_time
    }
}

impl TaskManager {
    fn run_first_task(&self) -> ! {
        let mut inner = self.inner.exclusive_access();
        let task0 = &mut inner.tasks[0];
        task0.task_status = TaskStatus::Running;
        task0.task_time = get_time_us();
        let next_task_cx_ptr = &task0.task_cx as *const TaskContext;

        inner.refresh_stop_watch();
        drop(inner);

        let mut _c = TaskContext::zero_init();

        unsafe {
            __switch(&mut _c as *mut TaskContext, next_task_cx_ptr);
        }

        unreachable!("run_first_task");
    }

    fn mark_current_suspended(&self) {
        let mut inner = self.inner.exclusive_access();
        let current = inner.current_task;
        inner.tasks[current].kernel_time += inner.refresh_stop_watch();
        inner.tasks[current].task_status = TaskStatus::Ready;
    }

    fn mark_current_exited(&self) {
        let mut inner = self.inner.exclusive_access();
        let current = inner.current_task;
        inner.tasks[current].kernel_time += inner.refresh_stop_watch();
        info!(
            "[task {} exited, user_time: {}us, kernel_time: {}us]",
            current, inner.tasks[current].user_time, inner.tasks[current].kernel_time
        );
        inner.tasks[current].task_status = TaskStatus::Exited;
    }

    fn find_next_task(&self) -> Option<usize> {
        let inner = self.inner.exclusive_access();
        let current = inner.current_task;

        (current + 1..current + self.num_app + 1)
            .map(|id| id % self.num_app)
            .find(|&id| inner.tasks[id].task_status == TaskStatus::Ready)
    }

    fn run_next_task(&self) {
        if let Some(next) = self.find_next_task() {
            let mut inner = self.inner.exclusive_access();
            let current = inner.current_task;
            inner.tasks[next].task_status = TaskStatus::Running;
            inner.current_task = next;
            let current_task_cx_ptr = &mut inner.tasks[current].task_cx as *mut TaskContext;
            let next_task_cx_ptr = &inner.tasks[next].task_cx as *const TaskContext;

            let current_time = get_time_us();

            inner.tasks[current].task_time += current_time - inner.tasks[current].task_time;
            inner.tasks[next].task_time = current_time;

            drop(inner);

            unsafe {
                __switch(current_task_cx_ptr, next_task_cx_ptr);
            }
        } else {
            info!("All application completed!");
            shutdown(false);
        }
    }

    fn user_time_start(&self) {
        let mut inner = self.inner.exclusive_access();
        let current = inner.current_task;
        inner.tasks[current].kernel_time += inner.refresh_stop_watch();
    }

    fn user_time_end(&self) {
        let mut inner = self.inner.exclusive_access();
        let current = inner.current_task;
        inner.tasks[current].user_time += inner.refresh_stop_watch();
    }

    fn get_current_token(&self) -> usize {
        let inner = self.inner.exclusive_access();
        let current = inner.current_task;
        inner.tasks[current].get_user_token()
    }

    fn get_current_trap_cx(&self) -> &'static mut TrapContext {
        let inner = self.inner.exclusive_access();
        inner.tasks[inner.current_task].get_trap_cx()
    }

    fn inc_syscall_times(&self, syscall_id: usize) {
        let mut inner = self.inner.exclusive_access();
        let current = inner.current_task;
        inner.tasks[current].syscall_times[syscall_id] += 1;
    }

    fn get_current_taskinfo(&self) -> TaskInfo {
        let inner = self.inner.exclusive_access();
        let current_task = &inner.tasks[inner.current_task];
        TaskInfo {
            status: current_task.task_status,
            syscall_times: current_task.syscall_times,
            time: current_task.task_time / 1000,
        }
    }

    fn change_current_program_brk(&self, size: i32) -> Option<usize> {
        let mut inner = self.inner.exclusive_access();
        let current = inner.current_task;
        inner.tasks[current].change_program_brk(size)
    }
}

pub fn run_first_task() -> ! {
    TASK_MANAGER.run_first_task()
}

fn run_next_task() {
    TASK_MANAGER.run_next_task();
}

fn mark_current_suspended() {
    TASK_MANAGER.mark_current_suspended();
}

fn mark_current_exited() {
    TASK_MANAGER.mark_current_exited();
}

pub fn suspend_current_and_run_next() {
    mark_current_suspended();
    run_next_task();
}

pub fn exit_current_and_run_next() {
    mark_current_exited();
    run_next_task();
}

pub fn user_time_start() {
    TASK_MANAGER.user_time_start();
}

pub fn user_time_end() {
    TASK_MANAGER.user_time_end();
}

pub fn current_user_token() -> usize {
    TASK_MANAGER.get_current_token()
}

pub fn current_trap_cx() -> &'static mut TrapContext {
    TASK_MANAGER.get_current_trap_cx()
}

pub fn get_taskinfo() -> TaskInfo {
    TASK_MANAGER.get_current_taskinfo()
}

pub fn inc_syscall_times(syscall_id: usize) {
    TASK_MANAGER.inc_syscall_times(syscall_id);
}

pub fn change_program_brk(size: i32) -> Option<usize> {
    TASK_MANAGER.change_current_program_brk(size)
}
