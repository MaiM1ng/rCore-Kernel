//! Global Config Define

#[allow(unused)]

/// size of user stack
pub const USER_STACK_SIZE: usize = 4096;
/// size of kernel stack
pub const KERNEL_STACK_SIZE: usize = 4096 * 2;
/// max number of application
pub const MAX_APP_NUM: usize = 16;
/// the base address of first application
pub const APP_BASE_ADDRESS: usize = 0x80400000;
/// app memory region size
pub const APP_SIZE_LIMIT: usize = 0x20000;

/// max number of syscall
pub const MAX_SYSCALL_NUM: usize = 500;
/// freq of platform clock
pub const CLOCK_FREQ: usize = 12500000;
