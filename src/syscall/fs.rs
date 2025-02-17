//! Syscall: File and filesystem-related syscalls

use crate::fs::{open_file, OpenFlags, Stat, ROOT_INODE};
use crate::mm::{translated_and_write_bytes, translated_byte_buffer, translated_str, UserBuffer};
use crate::task::{current_task, current_user_token};

/// sys_write handler
pub fn sys_write(fd: usize, buf: *const u8, len: usize) -> isize {
    trace!("[Kernel] pid[{}] sys_write", current_task().unwrap().pid.0);

    let token = current_user_token();
    let task = current_task().unwrap();
    let inner = task.inner_exclusive_access();

    if fd >= inner.fd_table.len() {
        return -1;
    }

    if let Some(file) = &inner.fd_table[fd] {
        let file = file.clone();
        drop(inner);
        file.write(UserBuffer::new(translated_byte_buffer(token, buf, len))) as isize
    } else {
        -1
    }
}

/// sys_read
pub fn sys_read(fd: usize, buf: *const u8, len: usize) -> isize {
    trace!("[Kernel] pid[{}] sys_read", current_task().unwrap().pid.0);

    let token = current_user_token();
    let task = current_task().unwrap();
    let inner = task.inner_exclusive_access();

    if fd >= inner.fd_table.len() {
        return -1;
    }

    if let Some(file) = &inner.fd_table[fd] {
        let file = file.clone();
        drop(inner);
        file.read(UserBuffer::new(translated_byte_buffer(token, buf, len))) as isize
    } else {
        -1
    }
}

pub fn sys_open(path: *const u8, flags: u32) -> isize {
    trace!("[Kernel] pid[{}] sys_open", current_task().unwrap().pid.0);

    let task = current_task().unwrap();
    let token = current_user_token();
    let path = translated_str(token, path);

    if let Some(inode) = open_file(path.as_str(), OpenFlags::from_bits(flags).unwrap()) {
        let mut inner = task.inner_exclusive_access();
        let fd = inner.alloc_fd();
        inner.fd_table[fd] = Some(inode);
        fd as isize
    } else {
        -1
    }
}

pub fn sys_close(fd: usize) -> isize {
    trace!("[Kernel] pid[{}] sys_close", current_task().unwrap().pid.0);

    let task = current_task().unwrap();
    let mut inner = task.inner_exclusive_access();

    if fd >= inner.fd_table.len() {
        return -1;
    }

    if inner.fd_table[fd].is_none() {
        return -1;
    }

    inner.fd_table[fd].take();

    0
}

pub fn sys_fstat(fd: usize, st: *mut Stat) -> isize {
    trace!("[Kernel] pid[{}] sys_fstat", current_task().unwrap().pid.0);

    let task = current_task().unwrap();
    let os_inode = {
        // 获取 TCB 的可变引用，仅在此作用域内有效
        let inner = task.inner_exclusive_access();
        // 检查 fd 有效性
        if fd >= inner.fd_table.len() || inner.fd_table[fd].is_none() {
            return -1;
        }
        // 克隆 OSInode 的 Arc，确保所有权转移到外部
        inner.fd_table[fd].clone()
    }; // 此处 inner 的作用域结束，自动调用 drop（显式或隐式）

    let token = current_user_token();

    let stat = os_inode.unwrap().get_stat();

    let st_ptr = &stat as *const Stat as *const u8;
    let st_len = core::mem::size_of::<Stat>();

    translated_and_write_bytes(token, st as usize as *const u8, st_ptr, st_len);

    0
}

pub fn sys_linkat(old_path: *const u8, new_path: *const u8) -> isize {
    trace!("[Kernel] pid[{}] sys_linkat", current_task().unwrap().pid.0);

    let token = current_user_token();

    let old_path_str = translated_str(token, old_path);
    let new_path_str = translated_str(token, new_path);
    if old_path_str == new_path_str {
        // 不允许同名链接
        return -1;
    }

    ROOT_INODE.linkat(old_path_str.as_str(), new_path_str.as_str())
}

pub fn sys_unlinkat(path: *const u8) -> isize {
    trace!(
        "[Kernel] pid[{}] sys_unlinkat",
        current_task().unwrap().pid.0
    );

    let token = current_user_token();
    let path_str = translated_str(token, path);

    ROOT_INODE.unlinkat(path_str.as_str())
}
