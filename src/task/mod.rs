//! Implement of Task Manager

mod context;
mod manager;
mod pid;
mod processor;
mod scheduler;
mod switch;
#[allow(clippy::module_inception)]
mod task;

use crate::fs::{open_file, OpenFlags};
use alloc::sync::Arc;
pub use context::TaskContext;
use lazy_static::*;
pub use manager::add_task;
pub use processor::{
    current_task, current_task_info_inner, current_trap_cx, current_user_token,
    mapping_address_space_for_current_task, run_tasks, schedule, take_current_task,
    unmapping_address_space_for_current_task, update_current_task_syscall_times,
};
pub use scheduler::BIG_STRIDE;
pub use task::{exit_current_and_run_next, TaskControlBlock, TaskStatus};

lazy_static! {
    /// initproc的初始PCB
    /// INITPROC进程在全局变量区
    pub static ref INITPROC: Arc<TaskControlBlock> = Arc::new({
        let inode = open_file("ch6b_initproc", OpenFlags::RDONLY).unwrap();
        let v = inode.read_all();
        TaskControlBlock::new(v.as_slice())
    }
    );
}

/// 暂停当前进程 执行另外一个进程
pub fn suspend_current_and_run_next() {
    // 获取当前Process上正在执行的任务
    let task = take_current_task().unwrap();

    let mut task_inner = task.inner_exclusive_access();
    let task_cx_ptr = &mut task_inner.task_cx as *mut TaskContext;

    // 还没有执行完，状态改为Ready
    task_inner.task_status = TaskStatus::Ready;
    drop(task_inner);

    // 将当前任务放入任务管理器队尾
    add_task(task);

    // 调用schedule函数 切换回idle进程 调用执行下一个任务
    // 如果只有一个任务，那么将继续执行
    schedule(task_cx_ptr);
}

/// add init process to the task manager
pub fn add_initproc() {
    add_task(INITPROC.clone());
}
