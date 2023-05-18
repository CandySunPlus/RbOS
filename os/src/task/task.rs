use alloc::sync::{Arc, Weak};
use alloc::vec;
use core::cell::RefMut;

use super::context::TaskContext;
use super::pid::{kstack_alloc, pid_alloc, KernelStack, PidHandle};
use crate::config::{MAX_SYSCALL_NUM, TRAP_CONTEXT};
use crate::mm::{MemorySet, PhysPageNum, VirtAddr, KERNEL_SPACE};
use crate::sync::UPSafeCell;
use crate::timer::get_time_us;
use crate::trap::{trap_handler, TrapContext};

#[allow(unused)]
pub struct TaskInfo {
    status: TaskStatus,
    syscall_times: [u32; MAX_SYSCALL_NUM],
    time: usize,
}
pub struct TaskControlBlock {
    pub pid: PidHandle,
    pub kernel_stack: KernelStack,
    inner: UPSafeCell<TaskControlBlockInner>,
}

impl TaskControlBlock {
    pub fn new(elf_data: &[u8]) -> Self {
        let (memory_set, user_sp, entry_point) = MemorySet::from_elf(elf_data);
        let trap_cx_ppn = memory_set
            .translate(VirtAddr::from(TRAP_CONTEXT).into())
            .unwrap()
            .ppn();

        let pid_handle = pid_alloc();
        let kernel_stack = kstack_alloc();
        let kstack_top = kernel_stack.get_top();

        let task_control_block = Self {
            pid: pid_handle,
            kernel_stack,
            inner: unsafe {
                UPSafeCell::new(TaskControlBlockInner {
                    task_status: TaskStatus::Ready,
                    task_cx: TaskContext::goto_trap_return(kstack_top),
                    start_time: 0,
                    memory_set,
                    parent: None,
                    children: Default::default(),
                    trap_cx_ppn,
                    base_size: user_sp,
                    program_brk: user_sp,
                    exit_code: 0,
                    stride: 0,
                    priority: 16,
                    heap_bottom: user_sp,
                    syscall_times: [0; MAX_SYSCALL_NUM],
                })
            },
        };

        let trap_cx = task_control_block.inner_exclusive_access().get_trap_cx();

        *trap_cx = TrapContext::app_init_context(
            entry_point,
            user_sp,
            KERNEL_SPACE.exclusive_access().token(),
            kstack_top,
            trap_handler as usize,
        );

        task_control_block
    }

    pub fn inner_exclusive_access(&self) -> RefMut<'_, TaskControlBlockInner> {
        self.inner.exclusive_access()
    }

    pub fn getpid(&self) -> usize {
        self.pid.0
    }

    pub fn change_program_brk(&self, size: i32) -> Option<usize> {
        let mut inner = self.inner_exclusive_access();
        let old_break = inner.program_brk;
        let heap_bottom = inner.heap_bottom;
        let new_break = inner.program_brk as isize + size as isize;
        if new_break < inner.heap_bottom as isize {
            return None;
        }

        let result = if size < 0 {
            inner
                .memory_set
                .shrink_to(VirtAddr(heap_bottom), VirtAddr(new_break as usize))
        } else {
            inner
                .memory_set
                .append_to(VirtAddr(heap_bottom), VirtAddr(new_break as usize))
        };

        if result {
            inner.program_brk = new_break as usize;
            Some(old_break)
        } else {
            None
        }
    }

    pub fn fork(self: &Arc<TaskControlBlock>) -> Arc<TaskControlBlock> {
        let mut parent_inner = self.inner_exclusive_access();
        let memory_set = MemorySet::from_existed_user(&parent_inner.memory_set);
        let trap_cx_ppn = memory_set
            .translate(VirtAddr::from(TRAP_CONTEXT).into())
            .unwrap()
            .ppn();

        let pid_handle = pid_alloc();
        let kernel_stack = kstack_alloc();
        let kernel_stack_top = kernel_stack.get_top();
        let task_control_block = Arc::new(TaskControlBlock {
            pid: pid_handle,
            kernel_stack,
            inner: unsafe {
                UPSafeCell::new(TaskControlBlockInner {
                    task_status: TaskStatus::Ready,
                    task_cx: TaskContext::goto_trap_return(kernel_stack_top),
                    start_time: 0,
                    memory_set,
                    parent: Some(Arc::downgrade(self)),
                    children: vec::Vec::new(),
                    trap_cx_ppn,
                    base_size: parent_inner.base_size,
                    program_brk: parent_inner.program_brk,
                    exit_code: 0,
                    stride: 0,
                    priority: 16,
                    heap_bottom: parent_inner.heap_bottom,
                    syscall_times: [0; MAX_SYSCALL_NUM],
                })
            },
        });

        parent_inner.children.push(task_control_block.clone());
        let trap_cx = task_control_block.inner_exclusive_access().get_trap_cx();
        trap_cx.kernel_sp = kernel_stack_top;
        task_control_block
    }

    pub fn exec(&self, elf_data: &[u8]) {
        let (memory_set, user_sp, entry_point) = MemorySet::from_elf(elf_data);

        let trap_cx_ppn = memory_set
            .translate(VirtAddr::from(TRAP_CONTEXT).into())
            .unwrap()
            .ppn();

        let mut inner = self.inner_exclusive_access();
        inner.memory_set = memory_set;
        inner.trap_cx_ppn = trap_cx_ppn;

        let trap_cx = inner.get_trap_cx();

        *trap_cx = TrapContext::app_init_context(
            entry_point,
            user_sp,
            KERNEL_SPACE.exclusive_access().token(),
            self.kernel_stack.get_top(),
            trap_handler as usize,
        );
    }

    pub fn spawn(self: &Arc<TaskControlBlock>, elf_data: &[u8]) -> Arc<TaskControlBlock> {
        let mut parent_inner = self.inner_exclusive_access();

        let (memory_set, user_sp, entry_point) = MemorySet::from_elf(elf_data);
        let trap_cx_ppn = memory_set
            .translate(VirtAddr::from(TRAP_CONTEXT).into())
            .unwrap()
            .ppn();

        let pid_handle = pid_alloc();
        let kernel_stack = kstack_alloc();
        let kernel_stack_top = kernel_stack.get_top();
        let task_cx = TaskContext::goto_trap_return(kernel_stack_top);
        let task_control_block = Arc::new(TaskControlBlock {
            pid: pid_handle,
            kernel_stack,
            inner: unsafe {
                UPSafeCell::new(TaskControlBlockInner {
                    task_status: TaskStatus::Ready,
                    task_cx,
                    start_time: 0,
                    memory_set,
                    parent: Some(Arc::downgrade(self)),
                    children: vec::Vec::new(),
                    trap_cx_ppn,
                    base_size: user_sp,
                    program_brk: parent_inner.program_brk,
                    stride: 0,
                    priority: 16,
                    exit_code: 0,
                    heap_bottom: parent_inner.heap_bottom,
                    syscall_times: [0; MAX_SYSCALL_NUM],
                })
            },
        });
        parent_inner.children.push(task_control_block.clone());

        let trap_cx = task_control_block.inner_exclusive_access().get_trap_cx();

        *trap_cx = TrapContext::app_init_context(
            entry_point,
            user_sp,
            KERNEL_SPACE.exclusive_access().token(),
            kernel_stack_top,
            trap_handler as usize,
        );

        task_control_block
    }

    pub fn record_syscall_times(&self, syscall_id: usize) {
        let mut parent_inner = self.inner_exclusive_access();
        parent_inner.syscall_times[syscall_id] += 1;
    }

    pub fn get_taskinfo(&self) -> TaskInfo {
        let inner = self.inner_exclusive_access();
        TaskInfo {
            status: inner.task_status,
            syscall_times: inner.syscall_times,
            time: get_time_us() - inner.start_time,
        }
    }
}

pub struct TaskControlBlockInner {
    pub task_status: TaskStatus,
    pub task_cx: TaskContext,
    pub start_time: usize,
    pub memory_set: MemorySet,
    pub parent: Option<Weak<TaskControlBlock>>,
    pub children: vec::Vec<Arc<TaskControlBlock>>,
    pub trap_cx_ppn: PhysPageNum,
    pub base_size: usize,
    pub program_brk: usize,
    pub exit_code: i32,
    pub heap_bottom: usize,
    pub stride: u8,
    pub priority: u8,
    pub syscall_times: [u32; MAX_SYSCALL_NUM],
}

impl TaskControlBlockInner {
    pub fn get_user_token(&self) -> usize {
        self.memory_set.token()
    }

    pub fn get_trap_cx(&self) -> &'static mut TrapContext {
        self.trap_cx_ppn.get_mut()
    }

    pub fn mmap(&mut self, start: usize, len: usize, port: usize) -> bool {
        self.memory_set.mmap(start, len, port)
    }

    pub fn munmap(&mut self, start: usize, len: usize) -> bool {
        self.memory_set.munmap(start, len)
    }

    pub fn is_zombie(&self) -> bool {
        self.task_status == TaskStatus::Zombie
    }

    pub fn set_priority(&mut self, priority: isize) {
        self.priority = priority as u8;
    }
}

#[allow(unused)]
#[derive(Clone, Copy, PartialEq)]
pub enum TaskStatus {
    UnInit,
    Ready,
    Running,
    Zombie,
}
