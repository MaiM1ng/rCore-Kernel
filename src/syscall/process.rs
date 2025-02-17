//! Syscall: Process management syscalls
use crate::config::{MAX_SYSCALL_NUM, PAGE_SIZE};
use crate::fs::{open_file, OpenFlags};
use crate::mm::{
    check_map_area_mapping, check_map_area_unmapping, translated_and_write_bytes,
    translated_refmut, translated_str, MapArea, MapPermission, MapType, VirtAddr,
};
use crate::task::{
    add_task, current_task, current_task_info_inner, current_user_token, exit_current_and_run_next,
    mapping_address_space_for_current_task, suspend_current_and_run_next,
    unmapping_address_space_for_current_task, TaskControlBlock, TaskStatus,
};
use crate::timer::{get_time_ms, get_time_us};
use alloc::sync::Arc;

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
    trace!(
        "[Kernel] pid[{}] exited with code {}",
        current_task().unwrap().pid.0,
        exit_code
    );
    exit_current_and_run_next(exit_code);
    panic!("Unreachable in sys_exit!");
}

/// current task gives up resources for other task
pub fn sys_yield() -> isize {
    trace!("[Kernel] pid[{}] sys_yield", current_task().unwrap().pid.0);
    suspend_current_and_run_next();
    0
}

/// get time
pub fn sys_get_time(ts: *mut TimeVal, _tz: usize) -> isize {
    trace!(
        "[Kernel] pid[{}] sys_get_time",
        current_task().unwrap().pid.0
    );

    let us = get_time_us();
    let tv_inner = TimeVal {
        sec: us / 1_000_000,
        usec: us % 1_000_000,
    };

    let tv_inner_ptr = &tv_inner as *const TimeVal as *const u8;
    let tv_inner_len = core::mem::size_of::<TimeVal>();

    translated_and_write_bytes(
        current_user_token(),
        ts as usize as *const u8,
        tv_inner_ptr,
        tv_inner_len,
    );

    0
}

/// get task info
pub fn sys_task_info(ti: *mut TaskInfo) -> isize {
    trace!(
        "[Kernel] pid[{}] sys_task_info",
        current_task().unwrap().pid.0
    );
    let current_task_info = current_task_info_inner();

    let task_info = TaskInfo {
        status: TaskStatus::Running,
        syscall_times: current_task_info.syscall_times,
        time: get_time_ms() - current_task_info.first_run_time,
    };

    let ptr = &task_info as *const TaskInfo as *const u8;
    let len = core::mem::size_of::<TaskInfo>();

    translated_and_write_bytes(current_user_token(), ti as usize as *const u8, ptr, len);

    0
}

pub fn sys_mmap(start: usize, len: usize, port: usize) -> isize {
    trace!("[Kernel] pid[{}] sys_mmap", current_task().unwrap().pid.0);

    if port & 0x07 == 0 || port & !0x7 != 0 || start & (PAGE_SIZE - 1) != 0 {
        return -1;
    }

    let end = start + len;
    let start_va = VirtAddr::from(start);
    let end_va = VirtAddr::from(end);

    let mut map_perm = MapPermission::U;

    if port & 0x01 == 0x01 {
        map_perm |= MapPermission::R;
    }

    if port & 0x02 == 0x02 {
        map_perm |= MapPermission::W;
    }

    if port & 0x04 == 0x04 {
        map_perm |= MapPermission::X;
    }

    // 仅用于检查
    let map_area = MapArea::new(start_va, end_va, MapType::Framed, map_perm);

    if check_map_area_mapping(current_user_token(), map_area) {
        return -1;
    }

    mapping_address_space_for_current_task(start_va, end_va, map_perm);

    0
}

pub fn sys_munmap(start: usize, len: usize) -> isize {
    trace!("[Kernel] pid[{}] sys_munmap", current_task().unwrap().pid.0);

    if start % PAGE_SIZE != 0 {
        return -1;
    }

    let start_va = VirtAddr::from(start);
    let end_va = VirtAddr::from(start + len);

    let map_area = MapArea::new(start_va, end_va, MapType::Framed, MapPermission::U);

    if check_map_area_unmapping(current_user_token(), map_area) {
        return -1;
    }

    unmapping_address_space_for_current_task(start_va, end_va);

    0
}

pub fn sys_fork() -> isize {
    trace!("[Kernel] pid[{}] sys_fork", current_task().unwrap().pid.0);

    let current_task = current_task().unwrap();
    let new_task = current_task.fork();
    let new_pid = new_task.pid.0;
    let trap_cx = new_task.inner_exclusive_access().get_trap_cx();
    // 修改当前任务的返回值为0
    trap_cx.x[10] = 0;

    add_task(new_task);

    new_pid as isize
}

pub fn sys_exec(path: *const u8) -> isize {
    trace!("[Kernel] pid[{}] sys_exec", current_task().unwrap().pid.0);

    // 在用户地址空间中找到要执行的elf名字
    let token = current_user_token();
    let path_name = translated_str(token, path);

    if let Some(app_inode) = open_file(path_name.as_str(), OpenFlags::RDONLY) {
        let task = current_task().unwrap();
        let all_data = app_inode.read_all();
        task.exec(all_data.as_slice());
        0
    } else {
        -1
    }
}

/// sys_waitpid
pub fn sys_waitpid(pid: isize, exit_code_ptr: *mut i32) -> isize {
    trace!(
        "[Kernel] pid[{}] sys_waitpid",
        current_task().unwrap().pid.0
    );

    // 获取当前任务
    let task = current_task().unwrap();

    // 当pid == -1时，等待任意一个子进程即可
    let mut inner = task.inner_exclusive_access();
    if !inner
        .child
        .iter()
        .any(|p| p.get_pid() == pid as usize || pid == -1)
    {
        return -1;
        // release current tcb
    }

    let pair = inner.child.iter().enumerate().find(|(_, p)| {
        p.inner_exclusive_access().is_zombie() && (pid == -1 || p.get_pid() == pid as usize)
    });

    if let Some((idx, _)) = pair {
        // 这里就是清除资源
        let child = inner.child.remove(idx);
        assert_eq!(Arc::strong_count(&child), 1);
        let found_pid = child.get_pid();
        let exit_code = child.inner_exclusive_access().exit_code;
        // 将exit_code写入到进程数据中
        *translated_refmut(inner.memory_set.token(), exit_code_ptr) = exit_code;
        found_pid as isize
    } else {
        // 没有进程可以回收
        -2
    }
}

/// change data segment size
pub fn sys_sbrk(size: i32) -> isize {
    trace!(
        "[Kernel] pid[{}] sys_sbrk args[0] = {:x}",
        current_task().unwrap().pid.0,
        size
    );

    if let Some(old_brk) = current_task().unwrap().change_program_brk(size) {
        old_brk as isize
    } else {
        -1
    }
}

/// get pid
pub fn sys_getpid() -> isize {
    trace!("[Kernel] pid[{}] sys_getpid", current_task().unwrap().pid.0);

    current_task().unwrap().pid.0 as isize
}

/// sys_spawn
#[allow(unused)]
pub fn sys_spawn1(path: *const u8) -> isize {
    trace!("[Kernel] pid[{}] sys_spawn", current_task().unwrap().pid.0);

    let token = current_user_token();
    let path_name = translated_str(token, path);

    if let Some(inode) = open_file(path_name.as_str(), OpenFlags::RDONLY) {
        // 此时有这个app 需要检查进程池和内存是否足够分配
        inode.dump_metadata();
        let elf_data = inode.read_all();

        if elf_data.len() == 0 {
            println!("len = 0 ffuucckk");
            return -1;
        }
        let new_task_tcb = Arc::new(TaskControlBlock::new(elf_data.as_slice()));
        let new_pid = new_task_tcb.pid.0;
        // 当前的父进程
        let current_task_tcb = current_task().unwrap();
        let mut current_task_inner = current_task_tcb.inner_exclusive_access();

        // 配置父子关系
        let mut new_task_tcb_inner = new_task_tcb.inner_exclusive_access();
        new_task_tcb_inner.parent = current_task().as_ref().map(|arc| Arc::downgrade(arc));

        current_task_inner.child.push(new_task_tcb.clone());
        // spawn调用成功！
        // 返回子进程id
        drop(current_task_inner);
        drop(new_task_tcb_inner);
        add_task(new_task_tcb);

        new_pid as isize
    } else {
        // 没有这个APP 直接返回即可
        -1
    }
}

#[allow(unused)]
pub fn sys_spawn(path: *const u8) -> isize {
    let token = current_user_token();
    let path = translated_str(token, path);
    if let Some(app_inode) = open_file(path.as_str(), OpenFlags::RDONLY) {
        let all_data = app_inode.read_all();
        let task = current_task().unwrap();
        let new_task = task.spwan(all_data.as_slice());
        let new_pid = new_task.pid.0;
        // let trap_cx = new_task.inner_exclusive_access().get_trap_cx();
        // // we do not have to move to next instruction since we have done it before
        // // for child process, fork returns 0
        // trap_cx.x[10] = 0;
        // add new task to scheduler
        add_task(new_task);
        new_pid as isize
    } else {
        -1
    }
}

/// set prio
pub fn sys_set_prio(prio: isize) -> isize {
    trace!(
        "[Kernel] pid[{}] sys_set_prio",
        current_task().unwrap().pid.0
    );

    if prio <= 1 {
        return -1;
    }

    current_task().unwrap().inner_exclusive_access().prio = prio as usize;

    prio as isize
}
