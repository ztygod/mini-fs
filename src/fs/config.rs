pub const SUPER_BLOCK_BLOCK_ID: u32 = 0;
pub const INODE_BITMAP_BLOCK_ID: u32 = 1;
pub const DATA_BLOCK_BITMAP_BLOCK_ID: u32 = 2;
pub const INODE_TABLE_START_BLOCK_ID: u32 = 3;

// 假设每个 Inode 128 字节，一个 4KB 块可以存 32 个 Inode
pub const INODES_PER_BLOCK: u32 = 32;

// 假设总共 4096 个 Inode
pub const TOTAL_INODES: u32 = 4096;

// Inode 表占用的块数
pub const INODE_TABLE_BLOCKS: u32 = TOTAL_INODES / INODES_PER_BLOCK;

// 数据区的起始块号
pub const DATA_AREA_START_BLOCK_ID: u32 = INODE_TABLE_START_BLOCK_ID + INODE_TABLE_BLOCKS;
