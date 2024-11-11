//! Syscall: Process management syscalls

use crate::{
    config::MAX_SYSCALL_NUM,
    task::{
        exit_current_and_run_next, get_current_task_task_info_inner,
        suspended_current_and_run_next, TaskStatus,
    },
    timer::{get_time_ms, get_time_us},
};

#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

/// Task Info
#[allow(dead_code)]
pub struct TaskInfo {
    /// Task Status
    status: TaskStatus,
    /// the numbers of syscall called by task
    syscall_times: [u32; MAX_SYSCALL_NUM],
    /// total Running time of task
    time: usize,
}

/// task exit and submit an exit code
pub fn sys_exit(exit_code: i32) -> ! {
    trace!("[Kernel] Application exited with code {}", exit_code);
    exit_current_and_run_next();
    panic!("Unreachable in sys_exit!");
}

/// current task gives up resources for other task
pub fn sys_yield() -> isize {
    trace!("[Kernel] sys_yield");
    suspended_current_and_run_next();
    0
}

/// get time
pub fn sys_get_time(ts: *mut TimeVal, _tz: usize) -> isize {
    trace!("[Kernel] sys_get_time");
    let us = get_time_us();
    unsafe {
        *ts = TimeVal {
            sec: us / 1_000_000,
            usec: us % 1_000_000,
        };
    }
    0
}

/// get task info
pub fn sys_task_info(ti: *mut TaskInfo) -> isize {
    trace!("[Kernel] sys_task_info");
    let current_task_info = get_current_task_task_info_inner();

    unsafe {
        *ti = TaskInfo {
            status: TaskStatus::Running,
            syscall_times: current_task_info.syscall_times,
            time: get_time_ms() - current_task_info.first_run_time,
        }
    }
    0
}
