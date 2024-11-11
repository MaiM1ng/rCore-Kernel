use alloc::vec;
use alloc::vec::Vec;
use bitflags::*;

use super::{
    address::{PhysPageNum, StepByOne, VirtPageNum},
    frame_allocator::{frame_alloc, FrameTracker},
    VirtAddr,
};

bitflags! {
    /// Page Table Entry Flags
    pub struct PTEFlags: u8 {
        const V = 1 << 0;
        const R = 1 << 1;
        const W = 1 << 2;
        const X = 1 << 3;
        const U = 1 << 4;
        const G = 1 << 5;
        const A = 1 << 6;
        const D = 1 << 7;
    }
}

#[derive(Copy, Clone)]
#[repr(C)]
/// Page Table Entry Structure
pub struct PageTableEntry {
    /// bits of pte
    pub bits: usize,
}

impl PageTableEntry {
    /// 根据PPN和PTEFlags创建PTE
    pub fn new(ppn: PhysPageNum, flags: PTEFlags) -> Self {
        PageTableEntry {
            bits: ppn.0 << 10 | flags.bits as usize,
        }
    }

    /// 创建一个空的PTE
    pub fn empty() -> Self {
        PageTableEntry { bits: 0 }
    }

    /// 获取PTE中的PPN字段
    pub fn ppn(&self) -> PhysPageNum {
        (self.bits >> 10 & ((1usize << 44) - 1)).into()
    }

    /// 获取PTE中的Flags字段
    pub fn flags(&self) -> PTEFlags {
        PTEFlags::from_bits(self.bits as u8).unwrap()
    }

    /// 判断该PTE对应PPN是否合法
    pub fn is_valid(&self) -> bool {
        (self.flags() & PTEFlags::V) != PTEFlags::empty()
    }

    /// 判断当前PTE对应的PPN是否可读
    pub fn is_readable(&self) -> bool {
        (self.flags() & PTEFlags::R) != PTEFlags::empty()
    }

    /// 判断当前PTE对应PPN是否可写
    pub fn is_writable(&self) -> bool {
        (self.flags() & PTEFlags::W) != PTEFlags::empty()
    }

    /// 判断当前PTE对应的PPN是否可执行
    pub fn is_executable(&self) -> bool {
        (self.flags() & PTEFlags::X) != PTEFlags::empty()
    }
}

/// Page Table Structure
pub struct PageTable {
    /// root ppn of root page table
    root_ppn: PhysPageNum,
    /// RAII
    frames: Vec<FrameTracker>,
}

impl PageTable {
    /// 创建一个新的空页表
    /// 分配一个frame用于放root
    pub fn new() -> Self {
        let frame = frame_alloc().unwrap();
        PageTable {
            root_ppn: frame.ppn,
            frames: vec![frame],
        }
    }

    /// 查找当前VPN所对应的PTE, 如果路径上不存在则创建
    fn find_pte_create(&mut self, vpn: VirtPageNum) -> Option<&mut PageTableEntry> {
        let idxs = vpn.indexes();
        let mut ppn = self.root_ppn;
        let mut result: Option<&mut PageTableEntry> = None;

        // 由于idx[i] & 511 所以idx[i]的取值范围为[0, 512)
        // 这样保证了在pte_array不会越界
        // SV39 寻址过程
        // 使用VPN0 在root_ppn中找到二级页表
        // 使用VPN1 在二级页表中找到三级页表
        // 使用VPN2 在三级页表中找到目标ppn
        // 如果正确某一级不存在，则分配一个物理内存页
        for i in 0..3 {
            let pte = &mut ppn.get_pte_array()[idxs[i]];
            if i == 2 {
                // 到达叶子结点
                result = Some(pte);
                break;
            }

            if !pte.is_valid() {
                let frame = frame_alloc().unwrap();
                *pte = PageTableEntry::new(frame.ppn, PTEFlags::V);
                self.frames.push(frame);
            }
            ppn = pte.ppn();
        }

        result
    }

    /// 在页表中建立VPN-PPN的映射, 权限为flags
    #[allow(unused)]
    pub fn map(&mut self, vpn: VirtPageNum, ppn: PhysPageNum, flags: PTEFlags) {
        let pte = self.find_pte_create(vpn).unwrap();
        assert!(!pte.is_valid(), "VPN {:?} is mapped before mapping", vpn);
        *pte = PageTableEntry::new(ppn, flags | PTEFlags::V);
    }

    /// 在页表中取消VPN所在的映射
    /// VPN必须合法
    #[allow(unused)]
    pub fn unmap(&mut self, vpn: VirtPageNum) {
        let pte = self.find_pte_create(vpn).unwrap();
        assert!(pte.is_valid(), "VPN {:?} is invalid before unmapping", vpn);
        *pte = PageTableEntry::empty();
    }

    /// 从给定token中建立页表，但是实际上不会控制任何页面
    pub fn from_token(satp: usize) -> Self {
        Self {
            root_ppn: PhysPageNum::from(satp & ((1usize << 44) - 1)),
            // 实际上不控制任何资源
            frames: Vec::new(),
        }
    }

    /// 查找给定VPN的PTE，但是不会创建
    pub fn find_pte(&self, vpn: VirtPageNum) -> Option<&PageTableEntry> {
        let idxs = vpn.indexes();
        let mut ppn = self.root_ppn;
        let mut result: Option<&PageTableEntry> = None;

        for i in 0..3 {
            let pte = &ppn.get_pte_array()[idxs[i]];
            if i == 2 {
                result = Some(pte);
                break;
            }

            if !pte.is_valid() {
                return None;
            }
            ppn = pte.ppn();
        }

        result
    }

    /// 在当前pt中找到给定vpn对应的ppn
    pub fn translate(&self, vpn: VirtPageNum) -> Option<PageTableEntry> {
        self.find_pte(vpn).map(|pte| pte.clone())
    }

    /// 按照SATP的要求返回数据
    /// 其中最高四位为8 代表了启用SV39虚拟页表
    /// 最低位为root_ppn的根页表
    /// ASID暂时未考虑
    pub fn token(&self) -> usize {
        8usize << 60 | self.root_ppn.0
    }
}

/// 使用给定token建立页表，然后在给定页表中找到[ptr, ptr + len)
/// 所对应的物理地址，以Byte数组的形式返回
pub fn translated_byte_buffer(token: usize, ptr: *const u8, len: usize) -> Vec<&'static mut [u8]> {
    let page_table = PageTable::from_token(token);
    let mut start = ptr as usize;
    let end = start + len;

    let mut v = Vec::new();

    while start < end {
        // 计算起点VA
        let start_va = VirtAddr::from(start);
        // 计算start_va所在的VPN
        let mut vpn = start_va.floor();
        // 找到该VPN真实对应的PPN
        let ppn = page_table.translate(vpn).unwrap().ppn();
        vpn.step();
        // 计算终止VA
        let mut end_va: VirtAddr = vpn.into();
        // 判断一下end_va 和 end谁小，end_va取最小
        end_va = end_va.min(VirtAddr::from(end));
        // page_offset获得页内偏移
        // 这里有两种情况
        // 1. end_va < end：说明还有一页，page_offset是0
        // 2. end_va = end：取页内偏移
        if end_va.page_offset() == 0 {
            v.push(&mut ppn.get_bytes_array()[start_va.page_offset()..]);
        } else {
            v.push(&mut ppn.get_bytes_array()[start_va.page_offset()..end_va.page_offset()]);
        }
        start = end_va.into();
    }

    v
}
