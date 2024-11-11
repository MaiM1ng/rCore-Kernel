//! Implementation of TCB

use crate::trap::trap_return;

#[derive(Copy, Clone)]
#[repr(C)]
/// TaskContext of an application
pub struct TaskContext {
    /// 返回地址寄存器
    ra: usize,
    /// Stack Pointer
    sp: usize,
    /// s0-11 register, callee saved
    s: [usize; 12],
}

impl TaskContext {
    /// init an zero task context
    pub fn zero_init() -> Self {
        Self {
            ra: 0,
            sp: 0,
            s: [0; 12],
        }
    }

    /// 构造返回上下文
    pub fn goto_trap_return(kstack_ptr: usize) -> Self {
        Self {
            ra: trap_return as usize,
            sp: kstack_ptr,
            s: [0; 12],
        }
    }
}
