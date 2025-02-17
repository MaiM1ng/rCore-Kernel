//! Type related to task manager

use core::cell::RefMut;

use alloc::sync::{Arc, Weak};
use alloc::vec;
use alloc::vec::Vec;

use super::pid::{pid_alloc, KernelStack, PidHandle};
use super::{schedule, take_current_task, TaskContext, BIG_STRIDE, INITPROC};
use crate::config::{MAX_SYSCALL_NUM, TRAP_CONTEXT_BASE};
use crate::fs::{File, Stdin, Stdout};
use crate::mm::{MapPermission, MemorySet, PhysPageNum, VirtAddr, KERNEL_SPACE};
use crate::sync::UPSafeCell;
use crate::trap::{trap_handler, TrapContext};

/// struct of TCB
pub struct TaskControlBlock {
    // immutable
    /// Pid
    pub pid: PidHandle,
    /// 内核栈
    pub kernel_stack: KernelStack,
    // mutable
    /// 可变信息
    inner: UPSafeCell<TaskControlBlockInner>,
}

/// struct of TCB inner
pub struct TaskControlBlockInner {
    /// 上下文所在的ppn
    pub trap_cx_ppn: PhysPageNum,
    /// base size
    pub base_size: usize,
    /// 任务上下文
    pub task_cx: TaskContext,
    /// 任务状态
    pub task_status: TaskStatus,
    /// address space
    pub memory_set: MemorySet,
    /// 父进程
    // Weak不会影响父进程的引用计数
    pub parent: Option<Weak<TaskControlBlock>>,
    /// 子进程
    pub child: Vec<Arc<TaskControlBlock>>,
    /// 进程退出code
    pub exit_code: i32,
    /// task info inner
    pub task_info_inner: TaskInfoInner,
    /// heap bottom
    pub heap_bottom: usize,
    /// program break
    pub program_brk: usize,
    /// 优先级
    pub prio: usize,
    /// Stride优先级
    pub stride: usize,
    /// 打开文件表
    pub fd_table: Vec<Option<Arc<dyn File + Send + Sync>>>,
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
    Zombie,
}

impl TaskControlBlock {
    /// 获取可变引用
    pub fn inner_exclusive_access(&self) -> RefMut<'_, TaskControlBlockInner> {
        self.inner.exclusive_access()
    }

    /// 获取 用户页表
    pub fn get_user_token(&self) -> usize {
        let inner = self.inner.exclusive_access();
        inner.memory_set.token()
    }

    /// 获取PID
    pub fn get_pid(&self) -> usize {
        self.pid.0
    }

    /// 给定elf数据, 新建进程
    pub fn new(elf_data: &[u8]) -> Self {
        // user_sp是用户栈的栈顶
        let (memory_set, user_sp, entry_point) = MemorySet::from_elf(elf_data);

        // 获得TRAP上下文的ppn
        let trap_cx_ppn = memory_set
            .translate(VirtAddr::from(TRAP_CONTEXT_BASE).into())
            .unwrap()
            .ppn();

        // 分配一个PID
        let pid_handle = pid_alloc();
        // 这里传引用 不能复制两次
        let kernel_stack = KernelStack::new(&pid_handle);
        // 获取栈顶
        let kernel_stack_top = kernel_stack.get_top();
        // 构建TCB
        let task_control_block = Self {
            pid: pid_handle,
            kernel_stack,
            inner: unsafe {
                UPSafeCell::new(TaskControlBlockInner {
                    trap_cx_ppn,
                    base_size: user_sp,
                    task_cx: TaskContext::goto_trap_return(kernel_stack_top),
                    task_status: TaskStatus::Ready,
                    memory_set,
                    parent: None,
                    child: Vec::new(),
                    exit_code: 0,
                    // 自建结构体 用于统计进程运行时数据
                    task_info_inner: TaskInfoInner::zero_init(),
                    heap_bottom: user_sp,
                    program_brk: user_sp,
                    stride: 0,
                    prio: 16,
                    fd_table: vec![
                        // 0 stdin
                        Some(Arc::new(Stdin)),
                        // 1 stdout
                        Some(Arc::new(Stdout)),
                        // 2 stderr
                        Some(Arc::new(Stdout)),
                    ],
                })
            },
        };

        // 在User Space构建Trap Context
        let trap_cx = task_control_block.inner_exclusive_access().get_trap_cx();
        *trap_cx = TrapContext::app_init_context(
            entry_point,
            user_sp,
            KERNEL_SPACE.exclusive_access().token(),
            kernel_stack_top,
            trap_handler as usize,
        );

        task_control_block
    }

    /// exec系统调用
    pub fn exec(&self, elf_data: &[u8]) {
        // 实际上user_sp应该不会变
        // 同时在构建的Memory Area中将数据拷贝过去
        let (memory_set, user_sp, entry_point) = MemorySet::from_elf(elf_data);
        let trap_cx_ppn = memory_set
            .translate(VirtAddr::from(TRAP_CONTEXT_BASE).into())
            .unwrap()
            .ppn();

        let mut inner = self.inner_exclusive_access();
        // 将from_elf生成的新的地址空间替换old
        // old的生命周期结束，回收物理页
        inner.memory_set = memory_set;
        inner.trap_cx_ppn = trap_cx_ppn;

        // 其他的都不变 只需要替换内存相关

        let trap_cx = inner.get_trap_cx();
        *trap_cx = TrapContext::app_init_context(
            entry_point,
            user_sp,
            KERNEL_SPACE.exclusive_access().token(),
            self.kernel_stack.get_top(),
            trap_handler as usize,
        );
    }

    /// fork系统调用
    pub fn fork(self: &Arc<Self>) -> Arc<Self> {
        // 获取父进程PCB
        let mut parent_inner = self.inner_exclusive_access();

        // 复制user space
        let memory_set = MemorySet::from_existed_user(&parent_inner.memory_set);
        let trap_cx_ppn = memory_set
            .translate(VirtAddr::from(TRAP_CONTEXT_BASE).into())
            .unwrap()
            .ppn();

        // 分配PID
        let pid_handle = pid_alloc();
        let kernel_stack = KernelStack::new(&pid_handle);
        let kernel_stack_top = kernel_stack.get_top();

        // 复制fd_table
        let mut new_fd_table: Vec<Option<Arc<dyn File + Send + Sync>>> = Vec::new();
        for fd in parent_inner.fd_table.iter() {
            if let Some(file) = fd {
                new_fd_table.push(Some(file.clone()));
            } else {
                new_fd_table.push(None);
            }
        }

        // 使用ARC，实际的内存分配在堆上
        let task_control_block = Arc::new(TaskControlBlock {
            pid: pid_handle,
            kernel_stack,
            inner: unsafe {
                UPSafeCell::new(TaskControlBlockInner {
                    trap_cx_ppn,
                    base_size: parent_inner.base_size,
                    task_cx: TaskContext::goto_trap_return(kernel_stack_top),
                    task_status: TaskStatus::Ready,
                    memory_set,
                    // 父亲引用为Self
                    parent: Some(Arc::downgrade(self)),
                    child: Vec::new(),
                    exit_code: 0,
                    task_info_inner: TaskInfoInner::zero_init(),
                    // 与父进程完全保持一致
                    heap_bottom: parent_inner.heap_bottom,
                    program_brk: parent_inner.program_brk,
                    stride: 0,
                    prio: parent_inner.prio,
                    fd_table: new_fd_table,
                })
            },
        });

        // add child
        // 创建一个新引用
        parent_inner.child.push(task_control_block.clone());
        let trap_cx = task_control_block.inner_exclusive_access().get_trap_cx();

        // 这里与new不同之处在于，fork需要完整保存寄存器状态
        // 此时复制出来的trap_cx所在ppn上已经存了之前的状态，但是需要修改kernel_stack为新分配的KernelStack
        trap_cx.kernel_sp = kernel_stack_top;

        task_control_block
    }

    /// 获取TaskInfoInner结构体
    pub fn get_task_info_inner(&self) -> TaskInfoInner {
        self.inner_exclusive_access().task_info_inner
    }

    /// change the location of the program break. return None if failed
    pub fn change_program_brk(&self, size: i32) -> Option<usize> {
        let mut inner = self.inner_exclusive_access();
        let heap_bottom = inner.heap_bottom;
        let old_break = inner.program_brk;
        let new_brk = inner.program_brk as isize + size as isize;

        // 如果新的位置小于heap底部
        if new_brk < heap_bottom as isize {
            return None;
        }

        let result = if size < 0 {
            // 回收
            inner
                .memory_set
                .shrink_to(VirtAddr(heap_bottom), VirtAddr(new_brk as usize))
        } else {
            // 增加
            inner
                .memory_set
                .append_to(VirtAddr(heap_bottom), VirtAddr(new_brk as usize))
        };

        if result {
            inner.program_brk = new_brk as usize;
            Some(old_break)
        } else {
            None
        }
    }

    /// 更新Stride
    pub fn update_stride(&self) {
        let mut inner = self.inner_exclusive_access();
        let pass = BIG_STRIDE / inner.prio;
        inner.stride += pass;
    }

    /// spwan=fork+exec
    pub fn spwan(self: &Arc<Self>, elf_data: &[u8]) -> Arc<Self> {
        let mut parent_inner = self.inner_exclusive_access();
        let (memory_set, user_sp, entry_point) = MemorySet::from_elf(elf_data);
        let trap_cx_ppn = memory_set
            .translate(VirtAddr::from(TRAP_CONTEXT_BASE).into())
            .unwrap()
            .ppn();
        let pid_handle = pid_alloc();
        let kernel_stack = KernelStack::new(&pid_handle);
        let kernel_stack_top = kernel_stack.get_top();
        // copy fd table
        let mut new_fd_table: Vec<Option<Arc<dyn File + Send + Sync>>> = Vec::new();
        for fd in parent_inner.fd_table.iter() {
            if let Some(file) = fd {
                new_fd_table.push(Some(file.clone()));
            } else {
                new_fd_table.push(None);
            }
        }

        let task_control_block = Arc::new(TaskControlBlock {
            pid: pid_handle,
            kernel_stack,
            inner: unsafe {
                UPSafeCell::new(TaskControlBlockInner {
                    task_status: TaskStatus::Ready,
                    trap_cx_ppn,
                    base_size: user_sp,
                    task_cx: TaskContext::goto_trap_return(kernel_stack_top),
                    task_info_inner: TaskInfoInner::zero_init(),
                    memory_set,
                    parent: Some(Arc::downgrade(self)),
                    child: Vec::new(),
                    exit_code: 0,
                    heap_bottom: user_sp,
                    program_brk: user_sp,
                    fd_table: new_fd_table,
                    stride: 0,
                    prio: 16,
                })
            },
        });
        let trap_cx = task_control_block.inner_exclusive_access().get_trap_cx();
        *trap_cx = TrapContext::app_init_context(
            entry_point,
            user_sp,
            KERNEL_SPACE.exclusive_access().token(),
            kernel_stack_top,
            trap_handler as usize,
        );
        parent_inner.child.push(task_control_block.clone());
        task_control_block
    }
}

impl TaskControlBlockInner {
    /// get the trap context
    pub fn get_trap_cx(&self) -> &'static mut TrapContext {
        self.trap_cx_ppn.get_mut()
    }

    /// get token
    pub fn get_user_token(&self) -> usize {
        self.memory_set.token()
    }

    /// 获取进程状态
    fn get_status(&self) -> TaskStatus {
        self.task_status
    }

    /// 判断当前进程是不是僵尸进程
    pub fn is_zombie(&self) -> bool {
        self.get_status() == TaskStatus::Zombie
    }

    /// 更新当前系统调用计数器
    pub fn update_syscall_times(&mut self, syscall_id: usize) {
        self.task_info_inner.syscall_times[syscall_id] += 1;
    }

    /// 映射地址空间
    pub fn mapping_address_space(
        &mut self,
        start_va: VirtAddr,
        end_va: VirtAddr,
        map_perm: MapPermission,
    ) {
        self.memory_set
            .insert_framed_area(start_va, end_va, map_perm);
    }

    /// 取消一块地址空间的映射
    pub fn unmapping_address_space(&mut self, start_va: VirtAddr, end_va: VirtAddr) {
        self.memory_set.munmap_area(start_va, end_va);
    }

    /// 分配一个fd
    pub fn alloc_fd(&mut self) -> usize {
        if let Some(fd) = (0..self.fd_table.len()).find(|fd| self.fd_table[*fd].is_none()) {
            fd
        } else {
            // 如果前面都有 那么在尾部增加一个
            self.fd_table.push(None);
            self.fd_table.len() - 1
        }
    }
}

/// 退出当前任务 执行下一个任务
pub fn exit_current_and_run_next(exit_code: i32) {
    // take当前任务
    let task = take_current_task().unwrap();
    // 获取TCB
    let mut inner = task.inner_exclusive_access();
    // 更改任务状态为僵尸进程
    inner.task_status = TaskStatus::Zombie;

    // 记录exit_code
    inner.exit_code = exit_code;

    {
        // 将子进程挂在INITPROC下
        let mut initproc_inner = INITPROC.inner_exclusive_access();
        for child in inner.child.iter() {
            // downgrade: 将INITPROC降级为Weak指针，而不增加引用
            child.inner_exclusive_access().parent = Some(Arc::downgrade(&INITPROC));
            initproc_inner.child.push(child.clone());
        }
    }

    inner.child.clear();
    // 用于存放数据的物理页回收
    // 但不是很必要
    inner.memory_set.recycle_data_pages();

    drop(inner);
    drop(task);

    // 该进程不会返回，因此不需要保存当前进程的上下文了
    let mut _unused = TaskContext::zero_init();
    schedule(&mut _unused as *mut _);
}
