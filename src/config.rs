//! Global Config Define

use core::usize;

#[allow(unused)]

/// size of user stack
pub const USER_STACK_SIZE: usize = 4096 * 2;
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

/// PAGE SIZE
pub const PAGE_SIZE: usize = 4096;
/// PAGE_SIZE_BITS
pub const PAGE_SIZE_BITS: usize = 12;

/// MEMORY END
pub const MEMORY_END: usize = 0x88000000;
/// KERNEL HEAP SIZE
pub const KERNEL_HEAP_SIZE: usize = 0x200_0000;

/// the VA of trapoline
// 可用地址范围[0, 2^64 - 1]
// 举例：第一页的地址为[0, 2^12 - 1]
// 因此 start_va是一个页对齐的地址
pub const TRAMPOLINE: usize = usize::MAX - PAGE_SIZE + 1;
/// the VA of trap context
// TRAMPOLINE已经页对齐了
pub const TRAP_CONTEXT_BASE: usize = TRAMPOLINE - PAGE_SIZE;
