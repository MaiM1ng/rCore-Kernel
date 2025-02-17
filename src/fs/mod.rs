//! File Trade and inode
mod inode;
mod stdio;

use crate::mm::UserBuffer;

pub use inode::{list_apps, open_file, OSInode, OpenFlags, ROOT_INODE};
pub use stdio::{Stdin, Stdout};

/// trait FIle for all file types
pub trait File: Send + Sync {
    /// 判断是否可读
    fn readable(&self) -> bool;
    /// 判断是否可写
    fn writable(&self) -> bool;
    /// 读取数据写入缓冲区，返回读到的字节数
    fn read(&self, buf: UserBuffer) -> usize;
    /// 从缓冲区写入数据，返回成功写入的字节数
    fn write(&self, buf: UserBuffer) -> usize;
    /// Stat
    fn get_stat(&self) -> Stat;
}

#[repr(C)]
#[derive(Debug)]
/// The Stat of a Inode
pub struct Stat {
    /// 文件所在磁盘驱动器号，rcore中默认为0
    pub dev: u64,
    /// inode文件所在inode编号
    pub ino: u64,
    /// 文件类型
    pub mode: StatMode,
    /// 硬链接数量，初始为1
    pub nlink: u32,
    /// unused pad
    pad: [u64; 7],
}

bitflags! {
    /// StatMode定义
    pub struct StatMode: u32 {
        /// NULL
        const NULL  = 0;
        /// directory
        const DIR   = 0o040000;
        /// ordinary regular file
        const FILE  = 0o100000;
    }
}
