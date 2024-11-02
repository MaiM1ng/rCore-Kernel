//! Syscall: File and filesystem-related syscalls

const FD_STDOUT: usize = 1;

pub fn sys_write(fd: usize, buf: *const u8, len: usize) -> isize {
    trace!("[Kernel] sys_write");

    match fd {
        FD_STDOUT => {
            let slice = unsafe { core::slice::from_raw_parts(buf, len) };
            let str = core::str::from_utf8(slice).unwrap();
            print!("{}", str);
            len as isize
        }
        _ => {
            panic!("[Kernel] sys_write: unsupported fd: {}", fd);
        }
    }
}
