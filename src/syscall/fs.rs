use core::isize;

use crate::batch::{get_app_address_space, get_user_stack_sp_space};

const FD_STDOUT: usize = 1;

pub fn sys_write(fd: usize, buf: *const u8, len: usize) -> isize {
    match fd {
        FD_STDOUT => {
            let (sp_base, sp_top) = get_user_stack_sp_space();
            let app_as = get_app_address_space();
            let sp_range = sp_base..sp_top;
            let app_as_range = app_as.0..app_as.1;

            // 不仅要包含text段 还需要包含heap，因此需要检查整个app address space
            if (!sp_range.contains(&(buf as usize)) || !sp_range.contains(&(buf as usize + len)))
                && (!app_as_range.contains(&(buf as usize))
                    || !app_as_range.contains(&(buf as usize + len)))
            {
                return -1;
            }

            let slice = unsafe { core::slice::from_raw_parts(buf, len) };
            let str = core::str::from_utf8(slice).unwrap();
            print!("{}", str);
            len as isize
        }
        _ => {
            -1 as isize
            // panic!("Unsupport fd in sys_write!");
        }
    }
}
