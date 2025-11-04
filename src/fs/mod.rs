use crate::{
    disk::FileDisk,
    fs::{
        data_area::DataArea, data_block_bitmap::DataBlockBitmap, inode_bitmap::InodeBitmap,
        inode_table::InodeTable, super_block::SuperBlock,
    },
};

pub mod config;
pub mod data_area;
pub mod data_block_bitmap;
pub mod error;
pub mod inode_bitmap;
pub mod inode_table;
pub mod super_block;

#[derive(Debug)]
pub struct FileSystem {
    pub disk: FileDisk,               // 底层磁盘抽象层
    pub super_block: SuperBlock,      // 文件系统总体信息
    pub inode_bitmap: InodeBitmap,    // inode 分配信息
    pub data_bitmap: DataBlockBitmap, // 数据块分配信息
    pub inode_table: InodeTable,      // 所有 inode 管理
    pub data_area: DataArea,          // 所有数据块内容管理
}

impl FileSystem {
    pub fn mount() {}
    pub fn unmount() {}
    pub fn format() {}
    pub fn create_file() {}
    pub fn update_file() {}
    pub fn list_dir() {}
    pub fn sync() {}
    pub fn alloc_inode() {}
    pub fn alloc_block() {}
}
