//! Trap handler

mod context;

use crate::{
    config::{TRAMPOLINE, TRAP_CONTEXT_BASE},
    syscall::syscall,
};
use core::arch::global_asm;
use riscv::register::{
    scause::{self, Exception, Interrupt, Trap},
    sie, stval, stvec,
};

use crate::{
    task::{
        current_trap_cx, current_user_token, exit_current_and_run_next,
        suspend_current_and_run_next,
    },
    timer::set_next_trigger,
};

global_asm!(include_str!("trap.S"));

/// trap init: for set trap_handler
pub fn init() {
    set_kernel_trap_entry();
}

/// enable supervisor time Interrupt
pub fn enable_timer_interrupt() {
    unsafe {
        sie::set_stimer();
    }
}

#[no_mangle]
/// Trap处理程序
pub fn trap_handler() -> ! {
    set_kernel_trap_entry();
    // 当前应用程序的TrapContext PPN
    // 由于内核是恒等映射的
    // 读取S态寄存器状态
    let scause = scause::read();
    let stval = stval::read();

    match scause.cause() {
        Trap::Exception(Exception::UserEnvCall) => {
            // 当前应用的Trap上下文
            let mut cx = current_trap_cx();
            cx.sepc += 4;
            let result = syscall(cx.x[17], [cx.x[10], cx.x[11], cx.x[12]]) as usize;
            // 对于exec系统调用：旧的cx上下文已经被回收了，此时需要重新获取新的cx
            // 对于fork系统调用：父进程的x10在syscall中被修改，但是子进程的还未修改
            // 子进程的第一步入口一样在这个位置，因此需要修改子进程的返回值
            cx = current_trap_cx();
            // 父进程相当于重复写入相同的值
            cx.x[10] = result as usize;
        }
        Trap::Exception(Exception::StoreFault)
        | Trap::Exception(Exception::StorePageFault)
        | Trap::Exception(Exception::InstructionFault)
        | Trap::Exception(Exception::InstructionPageFault)
        | Trap::Exception(Exception::LoadFault)
        | Trap::Exception(Exception::LoadPageFault) => {
            println!("[Kernel] trap_handler: {:?} in application, bad addr = {:#x}, bad instruction = {:#x} kernel killed it!", scause.cause(), stval, current_trap_cx().sepc);
            // page fault exit code
            exit_current_and_run_next(-2);
        }
        Trap::Exception(Exception::IllegalInstruction) => {
            println!("[Kernel] IllegalInstruction in application! Kernel killed it.");
            exit_current_and_run_next(-3);
        }
        Trap::Interrupt(Interrupt::SupervisorTimer) => {
            set_next_trigger();
            suspend_current_and_run_next();
        }
        _ => {
            panic!(
                "[Kernel] Unsupport trap {:?}, stval = {:#x}!",
                scause.cause(),
                stval
            );
        }
    }

    trap_return()
}

#[no_mangle]
/// Supervisor Trap Handler入口，暂时不处理
pub fn trap_from_kernel() -> ! {
    panic!("a trap from kernel!");
}

/// 将Supervisor-Mode的跳板地址设置为trap_from_kernel, 但是目前不用处理
fn set_kernel_trap_entry() {
    unsafe {
        stvec::write(trap_from_kernel as usize, stvec::TrapMode::Direct);
    }
}

/// 将User-Mode的trpa地址设置为跳板
fn set_user_trap_entry() {
    unsafe {
        // 实际上TRAMPOLINE映射的是__alltraps的物理地址
        stvec::write(TRAMPOLINE as usize, stvec::TrapMode::Direct);
    }
}

/// trap返回跳板函数
pub fn trap_return() -> ! {
    // 设置为APP同一的跳板函数虚拟地址，即最高页
    set_user_trap_entry();

    // 应用程序的TrapContext的VA也是固定的
    let trap_cx_ptr = TRAP_CONTEXT_BASE;
    let user_satp = current_user_token();

    extern "C" {
        fn __alltraps();
        fn __restore();
    }

    // 计算restore的虚拟地址在跳板位置的偏移
    let restore_va = __restore as usize - __alltraps as usize + TRAMPOLINE;
    unsafe {
        core::arch::asm!(
            // 讲义内容：在内核中进行的一些操作
            // 可能导致一些原先存放某个应用代码的物理页帧如今用来存放数据或者是其他应用的代码
            // i-cache 中可能还保存着该物理页帧的 错误快照。
            "fence.i",
            "jr {restore_va}",
            restore_va = in(reg) restore_va,
            in("a0") trap_cx_ptr,
            in("a1") user_satp,
            options(noreturn)
        );
    }
}

pub use context::TrapContext;
