//! Type related to task manager

use super::TaskContext;
use crate::config::MAX_SYSCALL_NUM;

#[derive(Copy, Clone)]
/// struct of TCB
pub struct TaskControlBlock {
    /// TCB: task status
    pub task_status: TaskStatus,
    /// TCB: task context
    pub task_cx: TaskContext,
    /// TCB: task info inner
    pub task_info_inner: TaskInfoInner,
}

#[derive(Copy, Clone)]
/// struct of TCB Inner
pub struct TaskInfoInner {
    /// TII: syscall_times
    pub syscall_times: [u32; MAX_SYSCALL_NUM],
    /// TII: first run time
    pub first_run_time: usize,
    /// TII: first run flag
    pub first_run_flag: bool,
}

impl TaskInfoInner {
    /// function: zero_init
    pub fn zero_init() -> Self {
        Self {
            syscall_times: [0; MAX_SYSCALL_NUM],
            first_run_time: 0,
            first_run_flag: true,
        }
    }
}

#[derive(Copy, Clone, PartialEq)]
/// enum: TaskStatus
pub enum TaskStatus {
    /// Uninit
    UnInit,
    /// Ready to run
    Ready,
    /// Running
    Running,
    /// Exited
    Exited,
}
