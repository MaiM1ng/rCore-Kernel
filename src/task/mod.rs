//! Implement of Task Manager

mod context;
mod switch;
#[allow(clippy::module_inception)]
mod task;

use alloc::vec::Vec;
pub use context::TaskContext;
use lazy_static::*;
use switch::__switch;
pub use task::TaskControlBlock;
pub use task::{TaskInfoInner, TaskStatus};

use crate::loader::get_app_data;
use crate::loader::get_num_app;
use crate::mm::{MapPermission, VirtAddr};
use crate::sync::UPSafeCell;
use crate::timer::get_time_ms;
use crate::trap::TrapContext;

/// struct of task manager
pub struct TaskManager {
    /// total number of tasks
    num_app: usize,
    /// use inner value to get mutable access
    inner: UPSafeCell<TaskManagerInner>,
}

/// inner struct of task manager
pub struct TaskManagerInner {
    /// task list
    tasks: Vec<TaskControlBlock>,
    /// id of current Running Task
    current_task: usize,
}

lazy_static! {
    /// global var : TaskManager
    pub static ref TASK_MANAGER: TaskManager = {
        println!("[Kernel] init TASK_MANAGER");
        let num_app = get_num_app();
        println!("[Kernel] num_app = {}", num_app);

        let mut tasks = Vec::new();

        for i in 0..num_app {
            tasks.push(TaskControlBlock::new(get_app_data(i), i));
        }

        TaskManager {
            num_app,
            inner: unsafe {
                UPSafeCell::new(TaskManagerInner {
                    tasks,
                    current_task: 0,
                })
            },
        }
    };
}

impl TaskManager {
    fn run_first_task(&self) -> ! {
        let mut inner = self.inner.exclusive_access();
        let task0 = &mut inner.tasks[0];

        task0.task_status = TaskStatus::Running;
        task0.task_info_inner.first_run_flag = false;
        task0.task_info_inner.first_run_time = get_time_ms();

        let next_task_cx_ptr = &task0.task_cx as *const TaskContext;
        drop(inner);

        let mut _unused = TaskContext::zero_init();
        unsafe {
            __switch(&mut _unused as *mut TaskContext, next_task_cx_ptr);
        }

        panic!("unreachable in run_first_task!");
    }

    fn mark_current_suspended(&self) {
        let mut inner = self.inner.exclusive_access();
        let current = inner.current_task;
        inner.tasks[current].task_status = TaskStatus::Ready;
    }

    fn mark_current_exited(&self) {
        let mut inner = self.inner.exclusive_access();
        let current = inner.current_task;
        inner.tasks[current].task_status = TaskStatus::Exited;
    }

    fn find_next_task(&self) -> Option<usize> {
        let inner = self.inner.exclusive_access();
        let current = inner.current_task;
        (current + 1..current + self.num_app + 1)
            .map(|id| id % self.num_app)
            .find(|id| inner.tasks[*id].task_status == TaskStatus::Ready)
    }

    fn run_next_task(&self) {
        if let Some(next) = self.find_next_task() {
            let mut inner = self.inner.exclusive_access();
            let current = inner.current_task;

            inner.tasks[next].task_status = TaskStatus::Running;
            inner.current_task = next;

            let current_task_cx_ptr = &mut inner.tasks[current].task_cx as *mut TaskContext;
            let next_task_cx_ptr = &inner.tasks[next].task_cx as *const TaskContext;

            if inner.tasks[next].task_info_inner.first_run_flag {
                inner.tasks[next].task_info_inner.first_run_flag = false;
                inner.tasks[next].task_info_inner.first_run_time = get_time_ms();
            }

            drop(inner);

            unsafe {
                __switch(current_task_cx_ptr, next_task_cx_ptr);
            }
        } else {
            panic!("All applications completed!");
        }
    }

    fn update_current_task_syscall_times(&self, syscall_id: usize) {
        let mut inner = self.inner.exclusive_access();
        let current = inner.current_task;
        inner.tasks[current].task_info_inner.syscall_times[syscall_id] += 1;
    }

    fn current_task_task_info_inner(&self) -> TaskInfoInner {
        let inner = self.inner.exclusive_access();
        let current = inner.current_task;
        inner.tasks[current].task_info_inner
    }

    fn get_current_trap_cx(&self) -> &'static mut TrapContext {
        let inner = self.inner.exclusive_access();
        inner.tasks[inner.current_task].get_trap_cx()
    }

    fn get_current_token(&self) -> usize {
        let inner = self.inner.exclusive_access();
        inner.tasks[inner.current_task].get_user_token()
    }

    fn mapping_address_for_current_task(
        &self,
        start_va: VirtAddr,
        end_va: VirtAddr,
        map_perm: MapPermission,
    ) {
        let mut inner = self.inner.exclusive_access();
        let current = inner.current_task;
        inner.tasks[current]
            .memory_set
            .insert_framed_area(start_va, end_va, map_perm);
    }

    fn unmapping_address_for_current_task(&self, start_va: VirtAddr, end_va: VirtAddr) {
        let mut inner = self.inner.exclusive_access();
        let current = inner.current_task;
        inner.tasks[current]
            .memory_set
            .munmap_area(start_va, end_va);
    }
}

/// warp function: run first task
pub fn run_first_task() {
    TASK_MANAGER.run_first_task();
}

/// warp function: run_next_task
pub fn run_next_task() {
    TASK_MANAGER.run_next_task();
}

/// warp function: mark_current_suspended
fn marked_current_suspended() {
    TASK_MANAGER.mark_current_suspended();
}

fn marked_current_exited() {
    TASK_MANAGER.mark_current_exited();
}

/// warp function: suspended_current_and_run_next
pub fn suspended_current_and_run_next() {
    marked_current_suspended();
    run_next_task();
}

/// warp function: exit_current_and_run_next
pub fn exit_current_and_run_next() {
    marked_current_exited();
    run_next_task();
}

/// warp funciton: do_update_current_task_syscall_times
pub fn do_update_current_task_syscall_times(syscall_id: usize) {
    TASK_MANAGER.update_current_task_syscall_times(syscall_id);
}

/// warp function: get_current_task_task_info_inner
pub fn get_current_task_task_info_inner() -> TaskInfoInner {
    TASK_MANAGER.current_task_task_info_inner()
}

/// pub api get current trapcontext
pub fn current_trap_cx() -> &'static mut TrapContext {
    TASK_MANAGER.get_current_trap_cx()
}

/// get the current running task token
pub fn current_user_token() -> usize {
    TASK_MANAGER.get_current_token()
}

/// 给当前任务映射一片内存
pub fn mapping_address_for_current_task(
    start_va: VirtAddr,
    end_va: VirtAddr,
    map_perm: MapPermission,
) {
    TASK_MANAGER.mapping_address_for_current_task(start_va, end_va, map_perm);
}

/// 给当前任务取消映射一块内存
pub fn unmapping_address_for_current_task(start_va: VirtAddr, end_va: VirtAddr) {
    TASK_MANAGER.unmapping_address_for_current_task(start_va, end_va);
}
