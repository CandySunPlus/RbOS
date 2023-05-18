use alloc::collections::VecDeque;
use alloc::sync::Arc;

use lazy_static::lazy_static;

use super::task::TaskControlBlock;
use crate::sync::UPSafeCell;

#[derive(Default)]
pub struct TaskManager {
    ready_queue: VecDeque<Arc<TaskControlBlock>>,
}

impl TaskManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, task: Arc<TaskControlBlock>) {
        self.ready_queue.push_back(task);
    }

    pub fn fetch(&mut self) -> Option<Arc<TaskControlBlock>> {
        // self.ready_queue.pop_front()
        if self.ready_queue.is_empty() {
            return None;
        }
        let mut min_stride = self
            .ready_queue
            .get(0)
            .unwrap()
            .inner_exclusive_access()
            .stride;
        let mut index = 0;
        for (i, task) in self.ready_queue.iter().enumerate() {
            let inner = task.inner_exclusive_access();
            let gap = (inner.stride - min_stride) as i8;
            if gap <= 0 {
                min_stride = inner.stride;
                index = i;
            }
        }

        self.ready_queue.remove(index)
    }
}

lazy_static! {
    pub static ref TASK_MANAGER: UPSafeCell<TaskManager> =
        unsafe { UPSafeCell::new(TaskManager::new()) };
}

pub fn add_task(task: Arc<TaskControlBlock>) {
    TASK_MANAGER.exclusive_access().add(task);
}

pub fn fetch_task() -> Option<Arc<TaskControlBlock>> {
    TASK_MANAGER.exclusive_access().fetch()
}
