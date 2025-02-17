//! The main module and entrypoint

#![deny(warnings)]
#![deny(missing_docs)]
#![no_std]
#![no_main]
#![feature(panic_info_message)]
#![feature(alloc_error_handler)]

#[macro_use]
extern crate log;
#[macro_use]
extern crate bitflags;

extern crate alloc;

#[macro_use]
mod console;
pub mod config;
pub mod drivers;
pub mod fs;
pub mod lang_item;
pub mod logging;
pub mod mm;
pub mod sbi;
pub mod sync;
pub mod syscall;
pub mod task;
pub mod timer;
pub mod trap;

use task::add_initproc;

core::arch::global_asm!(include_str!("entry.asm"));
core::arch::global_asm!(include_str!("link_app.S"));

/// Clear BSS Segment
fn clear_bss() {
    extern "C" {
        fn sbss();
        fn ebss();
    }

    (sbss as usize..ebss as usize).for_each(|a| unsafe { (a as *mut u8).write_volatile(0) });
}

#[no_mangle]
/// os entry
pub fn rust_main() -> ! {
    clear_bss();
    logging::init();
    show_os_sections();
    println!("[Kernel] Hello, World!");

    mm::init();
    mm::remap_test();

    add_initproc();
    println!("[Kernel] After initproc!");

    trap::init();
    trap::enable_timer_interrupt();
    timer::set_next_trigger();
    fs::list_apps();
    task::run_tasks();

    panic!("unreachable in rust_main!");
}

/// show os-elf segment
fn show_os_sections() {
    extern "C" {
        fn stext();
        fn etext();
        fn srodata();
        fn erodata();
        fn sdata();
        fn edata();
        fn sbss();
        fn ebss();
        fn boot_stack_top();
        fn boot_stack_lower_bound();
    }

    info!(
        "Kernel Section Info: .text   : [0x{:x}, 0x{:x})",
        stext as usize, etext as usize
    );

    info!(
        "Kernel Section Info: .rodata : [0x{:x}, 0x{:x})",
        srodata as usize, erodata as usize
    );

    info!(
        "Kernel Section Info: .sdata  : [0x{:x}, 0x{:x})",
        sdata as usize, edata as usize
    );

    info!(
        "Kernel Section Info: .bss    : [0x{:x}, 0x{:x})",
        sbss as usize, ebss as usize
    );

    info!(
        "Kernel Stack Info: [0x{:x}, 0x{:x})",
        boot_stack_lower_bound as usize, boot_stack_top as usize
    );
}
