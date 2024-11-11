//! Syscall: File and filesystem-related syscalls

use crate::mm::translated_byte_buffer;
use crate::task::current_user_token;

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
