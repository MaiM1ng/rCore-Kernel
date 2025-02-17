//! Stdin & Stdout
use super::File;
use super::Stat;
use crate::mm::UserBuffer;
use crate::sbi::console_getchar;
use crate::task::suspend_current_and_run_next;

/// 标准输入
pub struct Stdin;
/// 标准输出
pub struct Stdout;

impl File for Stdin {
    fn readable(&self) -> bool {
        true
    }

    fn writable(&self) -> bool {
        false
    }

    fn read(&self, mut user_buf: UserBuffer) -> usize {
        // 一次只读一字节
        assert_eq!(user_buf.len(), 1);

        let mut c: usize;

        loop {
            c = console_getchar();
            if c == 0 {
                suspend_current_and_run_next();
                continue;
            } else {
                break;
            }
        }

        let ch = c as u8;

        unsafe {
            user_buf.buffers[0].as_mut_ptr().write_volatile(ch);
        }

        1
    }

    fn write(&self, _buf: UserBuffer) -> usize {
        panic!("Cannot write to stdin!");
    }

    fn get_stat(&self) -> Stat {
        panic!("Cannot Access Stdin Stat!");
    }
}

impl File for Stdout {
    fn readable(&self) -> bool {
        false
    }

    fn writable(&self) -> bool {
        true
    }

    fn read(&self, _buf: UserBuffer) -> usize {
        panic!("Cannot read from stdout!");
    }

    fn write(&self, buf: UserBuffer) -> usize {
        for buffer in buf.buffers.iter() {
            // 循环读
            print!("{}", core::str::from_utf8(*buffer).unwrap());
        }

        buf.len()
    }

    fn get_stat(&self) -> Stat {
        panic!("Cannot Access Stdout Stat!");
    }
}
