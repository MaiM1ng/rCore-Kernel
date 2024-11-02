//! Trap handler

mod context;

use crate::syscall::syscall;
use core::arch::global_asm;
use riscv::register::{
    scause::{self, Exception, Interrupt, Trap},
    sie, stval, stvec,
};

use crate::{
    task::{exit_current_and_run_next, suspended_current_and_run_next},
    timer::set_next_trigger,
};

global_asm!(include_str!("trap.S"));

/// trap init: for set trap_handler
pub fn init() {
    extern "C" {
        fn __alltraps();
    }

    unsafe {
        stvec::write(__alltraps as usize, stvec::TrapMode::Direct);
    }
}

/// enable supervisor time Interrupt
pub fn enable_timer_interrupt() {
    unsafe {
        sie::set_stimer();
    }
}

/// trap handler
#[no_mangle]
pub fn trap_handler(cx: &mut TrapContext) -> &mut TrapContext {
    let scause = scause::read();
    let stval = stval::read();

    match scause.cause() {
        Trap::Exception(Exception::UserEnvCall) => {
            cx.sepc += 4;
            cx.x[10] = syscall(cx.x[17], [cx.x[10], cx.x[11], cx.x[12]]) as usize;
        }
        Trap::Exception(Exception::StoreFault) | Trap::Exception(Exception::StorePageFault) => {
            println!("[Kernel] PageFault in application, bad addr = {:#x}, bad instruction = {:#x}, kernel killed it.", stval, cx.sepc);
            exit_current_and_run_next();
        }
        Trap::Exception(Exception::IllegalInstruction) => {
            println!("[Kernel] IllegalInstruction in application! Kernel killed it.");
            exit_current_and_run_next();
        }
        Trap::Interrupt(Interrupt::SupervisorTimer) => {
            set_next_trigger();
            suspended_current_and_run_next();
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
