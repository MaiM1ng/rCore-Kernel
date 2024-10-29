use log::trace;

use crate::{
    config::MAX_SYSCALL_NUM,
    task::{
        exit_current_and_run_next, get_current_task_first_running_time,
        get_current_task_syscall_times, suspend_current_and_run_next, TaskStatus,
    },
    timer::{get_time_ms, get_time_us},
};

#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

// 结构体组织方式要与User一致
#[allow(dead_code)]
pub struct TaskInfo {
    status: TaskStatus,
    syscall_times: [u32; MAX_SYSCALL_NUM],
    time: usize,
}

pub fn sys_exit(xstate: i32) -> ! {
    println!("[Kernel] Application exited with code {}", xstate);
    exit_current_and_run_next();
    panic!("[Kernel] Unreachable! in sys_exit!");
}

pub fn sys_yield() -> isize {
    suspend_current_and_run_next();
    0
}

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

pub fn sys_task_info(ti: *mut TaskInfo) -> isize {
    trace!("[Kernel] sys_task_info");
    let first_time = get_current_task_first_running_time();
    let gap = get_time_ms() - first_time;
    let sc_times = get_current_task_syscall_times();

    unsafe {
        *ti = TaskInfo {
            status: TaskStatus::Running,
            syscall_times: sc_times,
            time: gap,
        };
    }

    0
}
