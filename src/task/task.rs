//! Type related to task manager

use super::TaskContext;
use crate::config::{MAX_SYSCALL_NUM, TRAP_CONTEXT_BASE};
use crate::mm::{
    kernel_stack_position, MapPermission, MemorySet, PhysPageNum, VirtAddr, KERNEL_SPACE,
};
use crate::trap::{trap_handler, TrapContext};

/// struct of TCB
pub struct TaskControlBlock {
    /// TCB: task status
    pub task_status: TaskStatus,
    /// TCB: task context
    pub task_cx: TaskContext,
    /// TCB: task info inner
    pub task_info_inner: TaskInfoInner,
    /// VM module
    pub memory_set: MemorySet,
    /// trap cx ppn
    pub trap_cx_ppn: PhysPageNum,
    /// base_size
    pub base_size: usize,
}

#[derive(Copy, Clone)]
/// struct of TCB Inner
pub struct TaskInfoInner {
    /// TII: syscall_times
    pub syscall_times: [u32; MAX_SYSCALL_NUM],
    /// TII: first run time
    pub first_run_time: usize,
    /// TII: first run flag
    pub first_run_flag: bool,
}

impl TaskInfoInner {
    /// function: zero_init
    pub fn zero_init() -> Self {
        Self {
            syscall_times: [0; MAX_SYSCALL_NUM],
            first_run_time: 0,
            first_run_flag: true,
        }
    }
}

#[derive(Copy, Clone, PartialEq)]
/// enum: TaskStatus
pub enum TaskStatus {
    /// Uninit
    UnInit,
    /// Ready to run
    Ready,
    /// Running
    Running,
    /// Exited
    Exited,
}

impl TaskControlBlock {
    /// 给定elf_data和appid, 构造TCB
    /// elf_data用于构造页表
    /// appid获取系统栈
    pub fn new(elf_data: &[u8], app_id: usize) -> Self {
        // 解析ELF
        // 解析出来实际上是虚拟地址
        let (memory_set, user_sp, entry_point) = MemorySet::from_elf(elf_data);
        // 从构造的页表中查找trap上下文实际的位置
        // 因为trapContext的VA是约定好的，但是PA不知道在哪
        let trap_cx_ppn = memory_set
            .translate(VirtAddr::from(TRAP_CONTEXT_BASE).into())
            .unwrap()
            .ppn();

        let task_status = TaskStatus::Ready;

        // 该应用内核栈的虚拟地址，在Kernel address space
        let (kernel_stack_bottom, kernel_stack_top) = kernel_stack_position(app_id);

        // map kernel stack fo app(app_id) in kernel space
        // 真实的内存也在os的heap上
        KERNEL_SPACE.exclusive_access().insert_framed_area(
            kernel_stack_bottom.into(),
            kernel_stack_top.into(),
            MapPermission::R | MapPermission::W,
        );

        let task_control_block = Self {
            task_status,
            task_cx: TaskContext::goto_trap_return(kernel_stack_top),
            task_info_inner: TaskInfoInner::zero_init(),
            memory_set,
            trap_cx_ppn,
            base_size: user_sp,
            // heap_bottom: user_sp,
            // program_brk: user_sp,
        };

        // TrapContext实际上在task_cx_ppn所在物理页上
        // 且TrapContext是页对齐的
        let trap_cx = task_control_block.get_trap_cx();
        *trap_cx = TrapContext::app_init_context(
            entry_point,
            user_sp,
            KERNEL_SPACE.exclusive_access().token(),
            kernel_stack_top,
            trap_handler as usize,
        );

        task_control_block
    }

    /// get the trap context
    pub fn get_trap_cx(&self) -> &'static mut TrapContext {
        self.trap_cx_ppn.get_mut()
    }

    /// get token
    pub fn get_user_token(&self) -> usize {
        self.memory_set.token()
    }
}
