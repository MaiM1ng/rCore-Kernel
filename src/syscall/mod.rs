//! define syscall id and syscall entry

const SYSCALL_WRITE: usize = 64;
const SYSCALL_EXIT: usize = 93;
const SYSCALL_YIELD: usize = 124;
const SYSCALL_GET_TIME: usize = 169;
const SYSCALL_TASK_INFO: usize = 410;
const SYSCALL_MUNMAP: usize = 215;
const SYSCALL_MMAP: usize = 222;
#[allow(unused)]
const SYSCALL_SBRK: usize = 214;

mod fs;
mod process;

use fs::*;
use process::*;

use crate::task::do_update_current_task_syscall_times;

/// syscall entry
pub fn syscall(syscall_id: usize, args: [usize; 3]) -> isize {
    do_update_current_task_syscall_times(syscall_id);

    match syscall_id {
        SYSCALL_WRITE => sys_write(args[0], args[1] as *const u8, args[2]),
        SYSCALL_EXIT => sys_exit(args[0] as i32),
        SYSCALL_YIELD => sys_yield(),
        SYSCALL_GET_TIME => sys_get_time(args[0] as *mut TimeVal, args[1]),
        SYSCALL_TASK_INFO => sys_task_info(args[0] as *mut TaskInfo),
        SYSCALL_MUNMAP => sys_munmap(args[0], args[1]),
        SYSCALL_MMAP => sys_mmap(args[0], args[1], args[2]),
        _ => panic!("[Kernel] Unsupported syscall_id: {}", syscall_id),
    }
}
