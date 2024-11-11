//! Memory Management Implementation
//! SV39 Page-Based VM For RV64

mod address;
mod frame_allocator;
mod heap_allocator;
mod memory_set;
mod page_table;

pub use address::{PhysAddr, PhysPageNum, VirtAddr, VirtPageNum};
pub use memory_set::{kernel_stack_position, MapPermission, MemorySet, KERNEL_SPACE};
pub use page_table::translated_byte_buffer;

/// mm subsystem init
pub fn init() {
    // Init Rust runtime heap
    heap_allocator::init_heap();
    // Init Physical Frame allocator
    frame_allocator::init_frame_allocator();
    // Init Kernel address space
    KERNEL_SPACE.exclusive_access().activate();
}
