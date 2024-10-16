mod context;

use core::arch::global_asm;
use riscv::register::{
    mtvec::TrapMode,
    scause::{self, Exception, Trap},
    stval,
    stvec::{self},
};

use crate::syscall::syscall;

global_asm!(include_str!("trap.S"));

pub fn init() {
    extern "C" {
        fn __alltraps();
    }

    // set exception handler
    // 在RV中，发生异常时会跳转到stevc指向的地址执行
    // 将stvec指向alltraps 用于 u模式程序的上下文保存
    // 保存后会跳转到trap_handler进行异常服务程序的分发
    unsafe {
        // 在rCore中只涉及Direct模式
        stvec::write(__alltraps as usize, TrapMode::Direct);
    }
}

#[no_mangle]
pub fn trap_handler(cx: &mut TrapContext) -> &mut TrapContext {
    let scause = scause::read();
    let stval = stval::read();

    match scause.cause() {
        Trap::Exception(Exception::UserEnvCall) => {
            // 从下一条指令开始执行
            cx.sepc += 4;
            cx.x[10] = syscall(cx.x[17], [cx.x[10], cx.x[11], cx.x[12]]) as usize;
        }
        Trap::Exception(Exception::StoreFault) | Trap::Exception(Exception::StorePageFault) => {
            println!("[Kernel] PageFault in application, kernel killed it.");
            panic!("[Kernel] Cannot continue!");
        }
        Trap::Exception(Exception::IllegalInstruction) => {
            println!("[Kernel] IllegalInstruction in application, kernel killed it.");
            panic!("[Kernel] Cannot continue!");
        }
        _ => {
            panic!(
                "[Kernel] Unsupport trap {:?}, stval = {:#x}!",
                scause.cause(),
                stval
            );
        }
    }
    cx
}

pub use context::TrapContext;
