//! Memory Management Implementation
//! SV39 Page-Based VM For RV64

mod address;
mod frame_allocator;
mod heap_allocator;
mod memory_set;
mod page_table;

pub use address::{PhysAddr, PhysPageNum, VirtAddr, VirtPageNum};
use heap_allocator::heap_test;
pub use memory_set::{
    kernel_stack_position, remap_test, MapArea, MapPermission, MapType, MemorySet, KERNEL_SPACE,
};
pub use page_table::{
    check_map_area_mapping, check_map_area_unmapping, translated_and_write_bytes,
    translated_byte_buffer, translated_refmut, translated_str,
};

/// mm subsystem init
pub fn init() {
    // Init Rust runtime heap
    heap_allocator::init_heap();
    // Init Physical Frame allocator
    frame_allocator::init_frame_allocator();
    // Init Kernel address space
    KERNEL_SPACE.exclusive_access().activate();

    // test
    heap_test();
}
