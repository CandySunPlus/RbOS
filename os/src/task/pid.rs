use alloc::vec;

use lazy_static::lazy_static;

use crate::config::{KERNEL_STACK_SIZE, PAGE_SIZE, TRAMPOLINE};
use crate::sync::UPSafeCell;

pub struct PidHandle(pub usize);

#[derive(Default)]
struct PidAllocator {
    /// The current maximum process ID that is available for allocation
    current: usize,
    /// A list of recycled process IDs
    recycled: vec::Vec<usize>,
}

impl PidAllocator {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn alloc(&mut self) -> PidHandle {
        if let Some(pid) = self.recycled.pop() {
            PidHandle(pid)
        } else {
            self.current += 1;
            PidHandle(self.current - 1)
        }
    }

    /// Recycle a process ID so it can be used again later.
    ///
    /// # Panics
    ///
    /// Panics if attempting to recycle an unallocated process ID or if attempting to recycle the same
    /// process ID more than once.
    pub fn dealloc(&mut self, pid: usize) {
        assert!(pid < self.current);
        assert!(
            self.recycled.iter().find(|ppid| **ppid == pid).is_none(),
            "pid {} has been deallocated!",
            pid
        );
        self.recycled.push(pid);
    }
}

lazy_static! {
    static ref PID_ALLOCATOR: UPSafeCell<PidAllocator> =
        unsafe { UPSafeCell::new(PidAllocator::new()) };
}

pub fn pid_alloc() -> PidHandle {
    PID_ALLOCATOR.exclusive_access().alloc()
}

impl Drop for PidHandle {
    fn drop(&mut self) {
        PID_ALLOCATOR.exclusive_access().dealloc(self.0);
    }
}

pub struct KernelStack {
    pid: usize,
}

/// Calculates the position of the kernel stack for a given application ID.
///
/// # Arguments
///
/// * `app_id` - The ID of the application for which to calculate the kernel stack position.
///
/// # Returns
///
/// A tuple containing the bottom and top addresses of the kernel stack for the given application ID.
///
/// The bottom address is calculated as follows:
pub fn kernel_stack_position(app_id: usize) -> (usize, usize) {
    let top = TRAMPOLINE - app_id * (KERNEL_STACK_SIZE + PAGE_SIZE);
    let bottom = top - KERNEL_STACK_SIZE;
    (bottom, top)
}

impl KernelStack {}
