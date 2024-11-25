use crate::{
    mm::{MapPermission, VirtAddr},
    sync::UPSafeCell,
    trap::TrapContext,
};

use super::{
    manager::fetch_task, switch::__switch, task::TaskInfoInner, TaskContext, TaskControlBlock,
    TaskStatus,
};
use alloc::sync::Arc;
use lazy_static::*;

/// 当前这个核的任务
pub struct Processor {
    current: Option<Arc<TaskControlBlock>>,
    idle_task_cx: TaskContext,
}

impl Processor {
    pub fn new() -> Self {
        Self {
            current: None,
            idle_task_cx: TaskContext::zero_init(),
        }
    }
}

lazy_static! {
    pub static ref PROCESSOR: UPSafeCell<Processor> = unsafe { UPSafeCell::new(Processor::new()) };
}

impl Processor {
    /// 取出当前值 并设置current = None
    pub fn take_current(&mut self) -> Option<Arc<TaskControlBlock>> {
        self.current.take()
    }

    /// 获取当前执行任务的一份拷贝
    pub fn current(&self) -> Option<Arc<TaskControlBlock>> {
        // 如果current是None
        // as_ref直接返回None
        self.current.as_ref().map(|task| Arc::clone(task))
    }
}

/// 包装函数
pub fn take_current_task() -> Option<Arc<TaskControlBlock>> {
    PROCESSOR.exclusive_access().take_current()
}

/// 包装函数
pub fn current_task() -> Option<Arc<TaskControlBlock>> {
    PROCESSOR.exclusive_access().current()
}

/// 包装函数
pub fn current_user_token() -> usize {
    let task = current_task().unwrap();
    let token = task.inner_exclusive_access().get_user_token();
    token
}

/// 包装函数
pub fn current_trap_cx() -> &'static mut TrapContext {
    current_task()
        .unwrap()
        .inner_exclusive_access()
        .get_trap_cx()
}

/// 包装函数
pub fn current_task_info_inner() -> TaskInfoInner {
    current_task().unwrap().get_task_info_inner()
}

/// 当前核心运行任务, 从idle控制流转移到某个任务开始执行
pub fn run_tasks() {
    loop {
        let mut processor = PROCESSOR.exclusive_access();
        // 获取下一个任务
        if let Some(task) = fetch_task() {
            // 取一个任务
            let idle_task_cx_ptr = processor.get_idle_task_cx_ptr();
            // 获取TCB中的可变部分
            let mut task_inner = task.inner_exclusive_access();
            let next_task_cx_ptr = &task_inner.task_cx as *const TaskContext;
            task_inner.task_status = TaskStatus::Running;

            // 手动归还
            drop(task_inner);
            // 由于是Arc引用，只会被引用一次
            // task从任务管理器转移到process中
            processor.current = Some(task);
            drop(processor);

            // switch
            unsafe {
                __switch(idle_task_cx_ptr, next_task_cx_ptr);
            }

            // idle任务返回处，继续找下一个任务调度
            // loop
        }
    }
}

/// 切换回idle线程
pub fn schedule(switched_task_cx_ptr: *mut TaskContext) {
    let mut processor = PROCESSOR.exclusive_access();
    let idle_task_cx_ptr = processor.get_idle_task_cx_ptr();

    drop(processor);

    unsafe {
        __switch(switched_task_cx_ptr, idle_task_cx_ptr);
    }
}

impl Processor {
    /// 获得idle_task_cx的可变引用
    fn get_idle_task_cx_ptr(&mut self) -> *mut TaskContext {
        &mut self.idle_task_cx as *mut _
    }
}

/// 更新当前任务的信息
pub fn update_current_task_syscall_times(syscall_id: usize) {
    let task = current_task().unwrap();
    let mut inner = task.inner_exclusive_access();
    inner.update_syscall_times(syscall_id);
    drop(inner);
    drop(task);
}

/// 给当前任务映射一块内存
pub fn mapping_address_space_for_current_task(
    start_va: VirtAddr,
    end_va: VirtAddr,
    map_perm: MapPermission,
) {
    let task = current_task().unwrap();
    let mut inner = task.inner_exclusive_access();
    inner.mapping_address_space(start_va, end_va, map_perm);
}

/// 给当前任务取消映射一块内存
pub fn unmapping_address_space_for_current_task(start_va: VirtAddr, end_va: VirtAddr) {
    let task = current_task().unwrap();
    let mut inner = task.inner_exclusive_access();
    inner.unmapping_address_space(start_va, end_va);
}
