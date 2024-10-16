use super::context::TaskContext;

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
}
