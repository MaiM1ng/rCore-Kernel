//! File INode

use super::{File, Stat, StatMode};
use crate::drivers::BLOCK_DEVICE;
use crate::mm::UserBuffer;
use crate::sync::UPSafeCell;
use alloc::sync::Arc;
use alloc::vec::Vec;
use easy_fs::{EasyFileSystem, Inode, InodeType};
use lazy_static::*;

/// OS看见的Inode接口，在内存中
pub struct OSInode {
    readable: bool,
    writable: bool,
    inner: UPSafeCell<OSInodeInner>,
}

/// OSINode的可变部分
pub struct OSInodeInner {
    offset: usize,
    inode: Arc<Inode>,
}

impl OSInode {
    /// 根据Inode和Flag生成一个OSInode
    pub fn new(readable: bool, writable: bool, inode: Arc<Inode>) -> Self {
        Self {
            readable,
            writable,
            inner: unsafe { UPSafeCell::new(OSInodeInner { offset: 0, inode }) },
        }
    }

    /// 从这个Inode中读取全部
    pub fn read_all(&self) -> Vec<u8> {
        let mut inner = self.inner.exclusive_access();
        let mut buffer = [0u8; 512];
        let mut v: Vec<u8> = Vec::new();

        loop {
            let len = inner.inode.read_at(inner.offset, &mut buffer);
            if len == 0 {
                break;
            }
            inner.offset += len;
            v.extend_from_slice(&buffer[..len]);
        }

        v
    }

    /// 获取偏移
    pub fn get_offset(&self) -> usize {
        let inner = self.inner.exclusive_access();
        inner.offset
    }

    /// Dump metadata
    pub fn dump_metadata(&self) {
        let inner = self.inner.exclusive_access();
        let (block_id, block_offset) = inner.inode.get_block_metadata();

        println!(
            "[Kernel] OSInode: size = {} block id {}, block offset {}, offset {}",
            inner.inode.get_size(),
            block_id,
            block_offset,
            inner.offset
        );
    }
}

lazy_static! {
    /// 根目录
    pub static ref ROOT_INODE: Arc<Inode> = {
        let efs = EasyFileSystem::open(BLOCK_DEVICE.clone());
        Arc::new(EasyFileSystem::root_inode(&efs))
    };
}

/// 列出所有APP
pub fn list_apps() {
    println!("/**** APPS ****/");
    for app in ROOT_INODE.ls() {
        println!("{}", app);
    }
    println!("/**************/");
}

bitflags! {
    /// 用于sys_open系统调用
    pub struct OpenFlags: u32 {
        /// 只读
        const RDONLY = 0;
        /// 只写
        const WRONLY = 1 << 0;
        /// 读写
        const RDWR = 1 << 1;
        /// 创建文件
        const CREATE = 1 << 9;
        /// 清空文件
        const TRUNC = 1 << 10;
    }
}

impl OpenFlags {
    /// 获取文件的读写权限, 读写权限目前一定合法
    pub fn read_write(&self) -> (bool, bool) {
        if self.is_empty() {
            (true, false)
        } else if self.contains(Self::WRONLY) {
            (false, true)
        } else {
            (true, true)
        }
    }
}

/// 打开一个文件
pub fn open_file(name: &str, flags: OpenFlags) -> Option<Arc<OSInode>> {
    let (readable, writable) = flags.read_write();

    if flags.contains(OpenFlags::CREATE) {
        if let Some(inode) = ROOT_INODE.find(name) {
            inode.clear();
            Some(Arc::new(OSInode::new(readable, writable, inode)))
        } else {
            ROOT_INODE
                .create(name)
                .map(|inode| Arc::new(OSInode::new(readable, writable, inode)))
        }
    } else {
        ROOT_INODE.find(name).map(|inode| {
            if flags.contains(OpenFlags::TRUNC) {
                inode.clear();
            }
            Arc::new(OSInode::new(readable, writable, inode))
        })
    }
}

/// OSInode需要实现File Trait
impl File for OSInode {
    fn readable(&self) -> bool {
        self.readable
    }

    fn writable(&self) -> bool {
        self.writable
    }

    fn read(&self, mut buf: UserBuffer) -> usize {
        let mut inner = self.inner.exclusive_access();
        let mut total_read_size = 0usize;

        for slice in buf.buffers.iter_mut() {
            let read_size = inner.inode.read_at(inner.offset, *slice);
            if read_size == 0 {
                break;
            }
            inner.offset += read_size;
            total_read_size += read_size;
        }

        total_read_size
    }

    fn write(&self, buf: UserBuffer) -> usize {
        let mut inner = self.inner.exclusive_access();
        let mut total_write_size = 0usize;

        for slice in buf.buffers.iter() {
            let write_size = inner.inode.write_at(inner.offset, *slice);
            assert_eq!(write_size, slice.len());
            inner.offset += write_size;
            total_write_size += write_size;
        }

        total_write_size
    }

    fn get_stat(&self) -> Stat {
        let inner = self.inner.exclusive_access();

        let nlink = inner.inode.get_nlink();
        let (blk_id, blk_offset) = inner.inode.get_block_metadata();
        let ino = ROOT_INODE
            .find_inode_id_by_block(blk_id as u32, blk_offset)
            .unwrap();
        let file_type = inner.inode.find_file_type();

        let stat = Stat {
            dev: 0u64,
            ino: ino as u64,
            mode: match file_type {
                InodeType::FILE => StatMode::FILE,
                InodeType::DIR => StatMode::DIR,
            },
            nlink: nlink,
            pad: [0u64; 7],
        };

        stat
    }
}
