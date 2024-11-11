//! Physical Page Frame Allocator的实现

use super::PhysPageNum;
use crate::{config::MEMORY_END, mm::PhysAddr, sync::UPSafeCell};
use alloc::vec::Vec;
use lazy_static::*;

trait FrameAllocator {
    fn new() -> Self;
    fn alloc(&mut self) -> Option<PhysPageNum>;
    fn dealloc(&mut self, ppn: PhysPageNum);
}

/// Frame Allocator，用于将内核剩余物理地址以页面形式分配
pub struct StackFrameAllocator {
    current: usize,
    end: usize,
    recycled: Vec<usize>,
}

impl FrameAllocator for StackFrameAllocator {
    fn new() -> Self {
        Self {
            current: 0,
            end: 0,
            recycled: Vec::new(),
        }
    }

    fn alloc(&mut self) -> Option<PhysPageNum> {
        if let Some(ppn) = self.recycled.pop() {
            // 先查找recycled中是否有之前回收的物理页
            Some(ppn.into())
        } else {
            if self.current == self.end {
                // 分配完了
                None
            } else {
                self.current += 1;
                Some((self.current - 1).into())
            }
        }
    }

    fn dealloc(&mut self, ppn: PhysPageNum) {
        let ppn = ppn.0;
        if ppn >= self.current || self.recycled.iter().find(|&v| *v == ppn).is_some() {
            panic!("Frame ppn {:#x} has not been allocated!", ppn);
        }
        // 回收
        self.recycled.push(ppn);
    }
}

impl StackFrameAllocator {
    pub fn init(&mut self, l: PhysPageNum, r: PhysPageNum) {
        self.current = l.0;
        self.end = r.0;
    }
}

// 定义了类型别名
type FrameAllocatorImpl = StackFrameAllocator;

lazy_static! {
    pub static ref FRAME_ALLOCATOR: UPSafeCell<FrameAllocatorImpl> =
        unsafe { UPSafeCell::new(FrameAllocatorImpl::new()) };
}

/// init 函数
pub fn init_frame_allocator() {
    extern "C" {
        fn ekernel();
    }

    // 可用数据范围
    // [ceil(ekernel as usize), floor(MEMORY_END))
    FRAME_ALLOCATOR.exclusive_access().init(
        PhysAddr::from(ekernel as usize).ceil(),
        PhysAddr::from(MEMORY_END).floor(),
    );
}

// RAII
/// 用于跟踪Physical Page Frame的生命周期
pub struct FrameTracker {
    pub ppn: PhysPageNum,
}

impl FrameTracker {
    /// 给定ppn构造一个FrameTracker
    /// 同时清空该PPN对应页面内数据
    pub fn new(ppn: PhysPageNum) -> Self {
        // 在分配的时候要清空数据
        let bytes_array = ppn.get_bytes_array();

        for i in bytes_array {
            *i = 0;
        }

        Self { ppn }
    }
}

// 当FrameTracker被回收时，调用该函数
// 实现RAII，从而将FrameTracker绑定的ppn进行回收
impl Drop for FrameTracker {
    fn drop(&mut self) {
        frame_dealloc(self.ppn);
    }
}

pub fn frame_alloc() -> Option<FrameTracker> {
    // 如果alloc返回None，那么Map也会返回None
    FRAME_ALLOCATOR
        .exclusive_access()
        .alloc()
        .map(|ppn| FrameTracker::new(ppn))
}

pub fn frame_dealloc(ppn: PhysPageNum) {
    FRAME_ALLOCATOR.exclusive_access().dealloc(ppn);
}
