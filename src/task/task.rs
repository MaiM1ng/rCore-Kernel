use super::{context::TaskContext, MAX_SYSCALL_NUM};

#[derive(Copy, Clone, PartialEq)]
// Trait: Clone clone调用拷贝
// PartialEq: 调用==比较
// Copy: 采用复制语义，而不是移动
pub enum TaskStatus {
    UnInit,
    Ready,
    Running,
    Exited,
}

#[derive(Copy, Clone)]
pub struct TaskControlBlock {
    pub task_status: TaskStatus,
    pub task_cx: TaskContext,
    pub syscall_times: [u32; MAX_SYSCALL_NUM],
    pub first_run_time: usize,
    pub first_run_flag: bool,
}
