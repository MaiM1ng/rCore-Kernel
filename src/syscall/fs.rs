//! Syscall: File and filesystem-related syscalls

use crate::mm::translated_byte_buffer;
use crate::sbi::console_getchar;
use crate::task::{current_user_token, suspend_current_and_run_next};

const FD_STDIN: usize = 0;
const FD_STDOUT: usize = 1;

/// sys_write handler
pub fn sys_write(fd: usize, buf: *const u8, len: usize) -> isize {
    trace!("[Kernel] sys_write");

    match fd {
        FD_STDOUT => {
            let buffers = translated_byte_buffer(current_user_token(), buf, len);
            for buffer in buffers {
                print!("{}", core::str::from_utf8(buffer).unwrap());
            }
            len as isize
        }
        _ => {
            panic!("[Kernel] sys_write: unsupported fd: {}", fd);
        }
    }
}

/// sys_read
pub fn sys_read(fd: usize, buf: *const u8, len: usize) -> isize {
    match fd {
        FD_STDIN => {
            assert_eq!(len, 1, "Only support len = 1 in sys_read!");
            let mut c: usize;
            loop {
                c = console_getchar();
                if c == 0 {
                    // 返回0说明还没有输入
                    // 阻塞输入貌似现在是在SBI中实现
                    suspend_current_and_run_next();
                    continue;
                } else {
                    break;
                }
            }

            let ch = c as u8;
            let mut buffers = translated_byte_buffer(current_user_token(), buf, len);
            unsafe {
                buffers[0].as_mut_ptr().write_volatile(ch);
            }
            1
        }
        _ => {
            panic!("[Kernel] unsupported fd in sys_read!");
        }
    }
}
