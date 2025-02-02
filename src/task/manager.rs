use core::usize::MAX;

use crate::sync::UPSafeCell;

use super::TaskControlBlock;
use alloc::sync::Arc;
use alloc::vec::Vec;
use lazy_static::*;

pub struct TaskManager {
    /// 就绪队列
    ready_queue: Vec<Arc<TaskControlBlock>>,
}

impl TaskManager {
    pub fn new() -> Self {
        Self {
            ready_queue: Vec::new(),
        }
    }

    pub fn add(&mut self, task: Arc<TaskControlBlock>) {
        self.ready_queue.push(task);
    }

    pub fn fetch(&mut self) -> Option<Arc<TaskControlBlock>> {
        let mut min_stride = MAX;
        let mut min_stride_idx = MAX;

        for (cur_idx, res) in self.ready_queue.iter().enumerate() {
            let current_stride = res.inner_exclusive_access().stride;
            if min_stride > current_stride {
                min_stride_idx = cur_idx;
                min_stride = current_stride;
            }
        }

        if min_stride_idx == MAX {
            None
        } else {
            self.ready_queue[min_stride_idx].update_stride();
            Some(self.ready_queue.remove(min_stride_idx))
        }
    }
}

lazy_static! {
    pub static ref TASK_MANAGER: UPSafeCell<TaskManager> =
        unsafe { UPSafeCell::new(TaskManager::new()) };
}

/// 在就绪队列中添加Ready的进程
pub fn add_task(task: Arc<TaskControlBlock>) {
    TASK_MANAGER.exclusive_access().add(task);
}

/// 从就绪队列中取出一个就绪进程
pub fn fetch_task() -> Option<Arc<TaskControlBlock>> {
    TASK_MANAGER.exclusive_access().fetch()
}
