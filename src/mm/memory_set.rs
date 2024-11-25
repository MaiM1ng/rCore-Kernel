//! Implementation of MapArea and MemorySet

use alloc::{collections::btree_map::BTreeMap, sync::Arc, vec::Vec};
use lazy_static::lazy_static;
use riscv::register::satp;

use crate::{
    config::{
        KERNEL_STACK_SIZE, MEMORY_END, PAGE_SIZE, TRAMPOLINE, TRAP_CONTEXT_BASE, USER_STACK_SIZE,
    },
    mm::address::StepByOne,
    sync::UPSafeCell,
};

use super::{
    address::{VPNRange, VirtAddr, VirtPageNum},
    frame_allocator::{frame_alloc, FrameTracker},
    page_table::{PTEFlags, PageTable, PageTableEntry},
    PhysAddr, PhysPageNum,
};

extern "C" {
    fn stext();
    fn etext();
    fn srodata();
    fn erodata();
    fn sdata();
    fn edata();
    fn sbss_with_stack();
    fn ebss();
    fn ekernel();
    fn strampoline();
}

/// 连续的Map地址
pub struct MapArea {
    vpn_range: VPNRange,
    data_frames: BTreeMap<VirtPageNum, FrameTracker>,
    map_type: MapType,
    map_perm: MapPermission,
}

#[derive(Copy, Clone, PartialEq, Debug)]
/// 描述Map方式
pub enum MapType {
    /// 恒等映射
    Identical,
    /// 重新映射
    Framed,
}

bitflags! {
    /// Map Permission corresponding to that in pte
    pub struct MapPermission: u8 {
        /// Readable
        const R = 1 << 1;
        /// Writable
        const W = 1 << 2;
        /// Excutable
        const X = 1 << 3;
        /// Accessible in U mode
        const U = 1 << 4;
    }
}

/// Address Space
pub struct MemorySet {
    page_table: PageTable,
    areas: Vec<MapArea>,
}

impl MemorySet {
    /// 创建一个新的地址空间
    pub fn new_bare() -> Self {
        Self {
            page_table: PageTable::new(),
            areas: Vec::new(),
        }
    }

    /// Push的时候会完成数据的拷贝
    fn push(&mut self, mut map_area: MapArea, data: Option<&[u8]>) {
        map_area.map(&mut self.page_table);
        if let Some(data) = data {
            map_area.copy_data(&self.page_table, data);
        }
        self.areas.push(map_area);
    }

    /// 插入一个framed map类型的区域
    pub fn insert_framed_area(
        &mut self,
        start_va: VirtAddr,
        end_va: VirtAddr,
        permission: MapPermission,
    ) {
        self.push(
            MapArea::new(start_va, end_va, MapType::Framed, permission),
            None,
        );
    }

    /// 创建内核地址空间
    /// 每个segment都是页对齐的，因此不会重叠
    pub fn new_kernel() -> Self {
        let mut memory_set = Self::new_bare();

        // map trampoline
        memory_set.map_trampoline();

        // map kernel section
        println!(
            "[Kernel] .text [{:#x}, {:#x})",
            stext as usize, etext as usize
        );

        println!(
            "[Kernel] .rodata [{:#x}, {:#x})",
            srodata as usize, erodata as usize
        );

        println!(
            "[Kernel] .data [{:#x}, {:#x})",
            sdata as usize, edata as usize
        );

        println!(
            "[Kernel] .bss [{:#x}, {:#x})",
            sbss_with_stack as usize, ebss as usize
        );

        println!("[Kernel] mapping .text section");
        memory_set.push(
            MapArea::new(
                (stext as usize).into(),
                (etext as usize).into(),
                MapType::Identical,
                MapPermission::R | MapPermission::X,
            ),
            None,
        );

        println!("[Kernel] mapping .rodata section");
        memory_set.push(
            MapArea::new(
                (srodata as usize).into(),
                (erodata as usize).into(),
                MapType::Identical,
                MapPermission::R,
            ),
            None,
        );

        println!("[Kernel] mapping .data section");
        memory_set.push(
            MapArea::new(
                (sdata as usize).into(),
                (edata as usize).into(),
                MapType::Identical,
                MapPermission::R,
            ),
            None,
        );

        println!("[Kernel] mapping .bss section");
        memory_set.push(
            MapArea::new(
                (sbss_with_stack as usize).into(),
                (ebss as usize).into(),
                MapType::Identical,
                MapPermission::R | MapPermission::W,
            ),
            None,
        );

        println!("[Kernel] mapping physical memory");
        memory_set.push(
            MapArea::new(
                (ekernel as usize).into(),
                MEMORY_END.into(),
                MapType::Identical,
                MapPermission::R | MapPermission::W,
            ),
            None,
        );

        // return
        memory_set
    }

    /// 给定elf数据创建地址空间
    /// 通常用于应用程序
    pub fn from_elf(elf_data: &[u8]) -> (Self, usize, usize) {
        // 创建一个新的memory set
        let mut memort_set = Self::new_bare();
        // map trampoline
        memort_set.map_trampoline();

        // map program headers of elf, with U flag
        let elf = xmas_elf::ElfFile::new(elf_data).unwrap();
        let elf_header = elf.header;
        let magic = elf_header.pt1.magic;

        assert_eq!(magic, [0x7f, 0x45, 0x4c, 0x46], "Invalid ELF!");

        let ph_count = elf_header.pt2.ph_count();
        let mut max_end_vpn = VirtPageNum(0);

        for i in 0..ph_count {
            let ph = elf.program_header(i).unwrap();
            if ph.get_type().unwrap() == xmas_elf::program::Type::Load {
                let start_va: VirtAddr = (ph.virtual_addr() as usize).into();
                let end_va: VirtAddr = ((ph.virtual_addr() + ph.mem_size()) as usize).into();

                let mut map_perm = MapPermission::U;

                let ph_flags = ph.flags();
                if ph_flags.is_read() {
                    map_perm |= MapPermission::R;
                }

                if ph_flags.is_write() {
                    map_perm |= MapPermission::W;
                }

                if ph_flags.is_execute() {
                    map_perm |= MapPermission::X;
                }

                let map_area = MapArea::new(start_va, end_va, MapType::Framed, map_perm);

                max_end_vpn = map_area.vpn_range.get_end();
                // 同时复制数据
                memort_set.push(
                    map_area,
                    Some(&elf.input[ph.offset() as usize..(ph.offset() + ph.file_size()) as usize]),
                );
            }
        }

        let max_end_va: VirtAddr = max_end_vpn.into();
        // 处理栈空间
        // 2024-11-12 看起来是约定好的
        // 2024-11-25 entry是0 用户栈实际上是在恢复上下文的时候直接写入到sp中
        let mut user_stack_bottom: usize = max_end_va.into();

        // 保护页面
        user_stack_bottom += PAGE_SIZE;
        let user_stack_top = user_stack_bottom + USER_STACK_SIZE;

        // 迭代器左闭右开，因此user_stack_top并没映射上去
        memort_set.push(
            MapArea::new(
                user_stack_bottom.into(),
                user_stack_top.into(),
                MapType::Framed,
                MapPermission::R | MapPermission::W | MapPermission::U,
            ),
            None,
        );

        // used for sbrk
        memort_set.push(
            MapArea::new(
                user_stack_top.into(),
                user_stack_top.into(),
                MapType::Framed,
                MapPermission::R | MapPermission::W | MapPermission::U,
            ),
            None,
        );

        // map trapcontext
        memort_set.push(
            MapArea::new(
                TRAP_CONTEXT_BASE.into(),
                TRAMPOLINE.into(),
                MapType::Framed,
                MapPermission::R | MapPermission::W,
            ),
            None,
        );

        (
            memort_set,
            user_stack_top,
            elf_header.pt2.entry_point() as usize,
        )
    }

    /// 当前地址空间的token，通常用于设置satp寄存器
    pub fn token(&self) -> usize {
        self.page_table.token()
    }

    /// remove all
    pub fn recycle_data_pages(&mut self) {
        self.areas.clear();
    }

    /// 切换当前地址空间对应的页表
    pub fn activate(&self) {
        let satp = self.page_table.token();
        unsafe {
            satp::write(satp);
            // 这条指令以后启用SV39虚拟内存
            // 由于恒等映射 后面的指令虚拟地址相同
            core::arch::asm!("sfence.vma");
            // 两个进程的页表切换的时候，要做一次fence
        }
    }

    /// 映射跳板函数
    fn map_trampoline(&mut self) {
        self.page_table.map(
            VirtAddr::from(TRAMPOLINE).into(),
            PhysAddr::from(strampoline as usize).into(),
            PTEFlags::R | PTEFlags::X,
        );
    }

    /// translate a vpn to a pte
    pub fn translate(&self, vpn: VirtPageNum) -> Option<PageTableEntry> {
        self.page_table.translate(vpn)
    }

    /// 保证区域一定都在页表当中
    pub fn munmap_area(&mut self, start_va: VirtAddr, end_va: VirtAddr) {
        let start_vpn = start_va.floor();
        let end_vpn = end_va.ceil();

        self.areas.retain_mut(|area| {
            if end_vpn <= area.get_end() && start_vpn >= area.get_start() {
                area.unmap(&mut self.page_table);
                false
            } else {
                true
            }
        })
    }

    /// remove a area
    pub fn remove_area_with_start_vpn(&mut self, start_vpn: VirtPageNum) {
        // 找到一个MapArea， start_vpn与该MapArea的start相同，然后删去该MapArea
        if let Some((idx, area)) = self
            .areas
            .iter_mut()
            .enumerate()
            .find(|(_, area)| area.vpn_range.get_start() == start_vpn)
        {
            area.unmap(&mut self.page_table);
            self.areas.remove(idx);
        }
    }

    /// 从另外一个MemorySet构造一个新的MemorySet
    pub fn from_existed_user(user_space: &MemorySet) -> MemorySet {
        let mut memory_set = Self::new_bare();

        memory_set.map_trampoline();

        for area in user_space.areas.iter() {
            // 构建新的MapArea
            let new_area = MapArea::from_another(area);
            // 将新复制的MapArea放入到新的MemorySet中
            // 在页表中映射MapArea, 但是还不用复制数据，所以data为None
            memory_set.push(new_area, None);
            // 将刚刚构建出的MapArea的数据复制一份到新的中
            // 复制数据
            for vpn in area.vpn_range {
                let src_ppn = user_space.translate(vpn).unwrap().ppn();
                let dst_ppn = memory_set.translate(vpn).unwrap().ppn();
                dst_ppn
                    .get_bytes_array()
                    .copy_from_slice(src_ppn.get_bytes_array());
            }
        }

        memory_set
    }

    /// shrink the area to new_end
    pub fn shrink_to(&mut self, start: VirtAddr, new_end: VirtAddr) -> bool {
        if let Some(area) = self
            .areas
            .iter_mut()
            .find(|area| area.vpn_range.get_start() == start.floor())
        {
            area.shrink_to(&mut self.page_table, new_end.ceil());
            true
        } else {
            false
        }
    }

    /// append the area to new_end
    pub fn append_to(&mut self, start: VirtAddr, new_end: VirtAddr) -> bool {
        if let Some(area) = self
            .areas
            .iter_mut()
            .find(|area| area.vpn_range.get_start() == start.floor())
        {
            area.append_to(&mut self.page_table, new_end.ceil());
            true
        } else {
            false
        }
    }
}

impl MapArea {
    /// 创建一个新的区域
    /// 但是是连续的
    pub fn new(
        start_va: VirtAddr,
        end_va: VirtAddr,
        map_type: MapType,
        map_perm: MapPermission,
    ) -> Self {
        // 向下取整
        let start_vpn: VirtPageNum = start_va.floor();
        // 向上取整
        let end_vpn = end_va.ceil();
        // 确保start_va和end_va在非对齐的时候 所在页面都可以被映射
        Self {
            vpn_range: VPNRange::new(start_vpn, end_vpn),
            data_frames: BTreeMap::new(),
            map_type,
            map_perm,
        }
    }

    /// map
    /// 将self映射到给定的page_table中
    pub fn map(&mut self, page_table: &mut PageTable) {
        // 对于连续的VA，由于范围是[floor(l), ceil(r)]
        // 因此进行连续分配，如果是framed形式的map，则会为每一个page分配物理页面
        for vpn in self.vpn_range {
            self.map_one(page_table, vpn);
        }
    }

    /// unmap
    #[allow(unused)]
    pub fn unmap(&mut self, page_table: &mut PageTable) {
        for vpn in self.vpn_range {
            self.unmap_one(page_table, vpn);
        }
    }

    /// map one
    pub fn map_one(&mut self, page_table: &mut PageTable, vpn: VirtPageNum) {
        let ppn: PhysPageNum;

        // 确定PPN
        match self.map_type {
            MapType::Identical => {
                // 恒等映射
                // VA = PA
                ppn = PhysPageNum(vpn.0);
            }
            MapType::Framed => {
                let frame = frame_alloc().unwrap();
                ppn = frame.ppn;
                self.data_frames.insert(vpn, frame);
            }
        }

        // 使用给定的权限生成页表权限
        let pte_flags = PTEFlags::from_bits(self.map_perm.bits).unwrap();
        // 在页表中修改PTE，真正修改映射
        page_table.map(vpn, ppn, pte_flags);
    }

    /// unmap one
    #[allow(unused)]
    pub fn unmap_one(&mut self, page_table: &mut PageTable, vpn: VirtPageNum) {
        match self.map_type {
            MapType::Framed => {
                self.data_frames.remove(&vpn);
            }
            _ => {}
        }

        page_table.unmap(vpn);
    }

    /// 将data复制到page_table所对应的物理地址中
    pub fn copy_data(&mut self, page_table: &PageTable, data: &[u8]) {
        assert_eq!(self.map_type, MapType::Framed);
        let mut start: usize = 0;
        let mut current_vpn = self.vpn_range.get_start();

        let len = data.len();

        loop {
            // 一次最多只能复制一个PAGE_SIZE的数据
            let src = &data[start..len.min(start + PAGE_SIZE)];
            let dst = &mut page_table
                .translate(current_vpn)
                .unwrap()
                .ppn()
                .get_bytes_array()[..src.len()];
            dst.copy_from_slice(src);
            start += PAGE_SIZE;
            if start >= len {
                break;
            }
            current_vpn.step();
        }
    }

    /// 虚拟页面的数量
    pub fn vpn_len(&self) -> usize {
        self.vpn_range.get_end().0 - self.vpn_range.get_start().0 + 1
    }

    /// 检查是否被映射过
    pub fn check_mapping(&self, page_table: &PageTable) -> bool {
        for vpn in self.vpn_range {
            if page_table.find_vpn(vpn) {
                return true;
            }
        }
        false
    }

    /// 检查是否存在没被映射过的区域
    pub fn check_unmapping(&self, page_table: &PageTable) -> bool {
        for vpn in self.vpn_range {
            if !page_table.find_vpn(vpn) {
                return true;
            }
        }
        false
    }

    /// 获取start
    pub fn get_start(&self) -> VirtPageNum {
        self.vpn_range.get_start()
    }

    /// 获取end
    pub fn get_end(&self) -> VirtPageNum {
        self.vpn_range.get_end()
    }

    /// 复制另外一个MapArea
    pub fn from_another(another: &MapArea) -> Self {
        Self {
            vpn_range: VPNRange::new(another.vpn_range.get_start(), another.vpn_range.get_end()),
            data_frames: BTreeMap::new(),
            map_type: another.map_type,
            map_perm: another.map_perm,
        }
    }

    /// 减少地址空间
    pub fn shrink_to(&mut self, page_table: &mut PageTable, new_end: VirtPageNum) {
        for vpn in VPNRange::new(new_end, self.vpn_range.get_end()) {
            self.unmap_one(page_table, vpn);
        }
        // 左闭右开
        self.vpn_range = VPNRange::new(self.vpn_range.get_start(), new_end);
    }

    /// 增加
    pub fn append_to(&mut self, page_table: &mut PageTable, new_end: VirtPageNum) {
        for vpn in VPNRange::new(self.vpn_range.get_end(), new_end) {
            self.map_one(page_table, vpn);
        }
        self.vpn_range = VPNRange::new(self.vpn_range.get_start(), new_end);
    }
}

lazy_static! {
    /// 全局地址空间 使用ARC的共享引用与UPSafeCell的互斥访问
    pub static ref KERNEL_SPACE: Arc<UPSafeCell<MemorySet>> =
        Arc::new(unsafe { UPSafeCell::new(MemorySet::new_kernel()) });
}

/// return (bottom, top) of a kernel stack in kernel space
pub fn kernel_stack_position(app_id: usize) -> (usize, usize) {
    // TRAMPOLINE: 跳板代码的bottom，已经页对齐
    // 同时加上保护页面
    let top = TRAMPOLINE - app_id * (KERNEL_STACK_SIZE + PAGE_SIZE);
    let bottom = top - KERNEL_STACK_SIZE;
    (bottom, top)
}

/// remap test in kernel_space
#[allow(unused)]
pub fn remap_test() {
    let mut kernel_space = KERNEL_SPACE.exclusive_access();
    let mid_text: VirtAddr = ((stext as usize + etext as usize) / 2).into();
    let mid_rodata: VirtAddr = ((srodata as usize + erodata as usize) / 2).into();
    let mid_data: VirtAddr = ((sdata as usize + edata as usize) / 2).into();

    // 文本段不可写
    assert!(!kernel_space
        .page_table
        .translate(mid_text.floor())
        .unwrap()
        .is_writable());

    // rodata段不可写
    assert!(!kernel_space
        .page_table
        .translate(mid_rodata.floor())
        .unwrap()
        .is_writable());

    // data段不可执行
    assert!(!kernel_space
        .page_table
        .translate(mid_data.floor())
        .unwrap()
        .is_executable());

    println!("[Kernel] remap_test passed!");
}
