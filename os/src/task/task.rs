use alloc::sync::{Arc, Weak};
use alloc::vec;
use core::cell::RefMut;

use super::context::TaskContext;
use super::pid::{kstack_alloc, pid_alloc, KernelStack, PidHandle};
use crate::config::{MAX_SYSCALL_NUM, TRAP_CONTEXT};
use crate::mm::{MemorySet, PhysPageNum, VirtAddr, KERNEL_SPACE};
use crate::sync::UPSafeCell;
use crate::trap::{trap_handler, TrapContext};

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
                    user_time: 0,
                    kernel_time: 0,
                    task_time: 0,
                    memory_set,
                    parent: None,
                    children: Default::default(),
                    trap_cx_ppn,
                    base_size: user_sp,
                    program_brk: user_sp,
                    exit_code: 0,
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
}

pub struct TaskControlBlockInner {
    pub task_status: TaskStatus,
    pub task_cx: TaskContext,
    pub user_time: usize,
    pub kernel_time: usize,
    pub task_time: usize,
    pub memory_set: MemorySet,
    pub parent: Option<Weak<TaskControlBlock>>,
    pub children: vec::Vec<Arc<TaskControlBlock>>,
    pub trap_cx_ppn: PhysPageNum,
    pub base_size: usize,
    pub program_brk: usize,
    pub exit_code: i32,
    pub heap_bottom: usize,
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
}

#[allow(unused)]
#[derive(Clone, Copy, PartialEq)]
pub enum TaskStatus {
    UnInit,
    Ready,
    Running,
    Zombie,
}
