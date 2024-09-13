#![no_std]
#![no_main]
#![feature(panic_info_message)]

#[macro_use]
mod console;
mod lang_items;
mod logging;
mod sbi;

#[allow(unused)]
use core::{arch::global_asm, panic};
#[allow(unused)]
use log::{debug, error, info, trace, warn};

global_asm!(include_str!("entry.asm"));

#[no_mangle]
pub fn rust_main() -> ! {
    clear_bss();

    logging::init_log();

    show_os_sections();

    info!("[Kernel] Hello, World!");

    // panic!("Shutdown Machine!");
    info!("[Kernel] Kernel Shutdown!");
    sbi::shutdown(false);
}

fn clear_bss() {
    extern "C" {
        fn sbss();
        fn ebss();
    }
    (sbss as usize..ebss as usize).for_each(|a| unsafe { (a as *mut u8).write_volatile(0) });
}

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
