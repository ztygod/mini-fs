use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct SuperBlock {
    pub fs_type: String, // 文件系统标识
    /** 数据块块信息 */
    pub block_size: u64, // 每块大小（字节）
    pub total_blocks: u64, // 文件系统总块数
    pub free_blocks: u64, // 当前空闲块数
    pub data_block_start: u64, // 数据区起始块号
    /** inode 信息 */
    pub total_inodes: u64, // 总 inode 数
    pub free_inode: u64, // 当前空闲 inode 数
    pub inode_table_start: u64, // inode 表起始块号
    /** 位图信息 */
    pub inode_bitmap_start: u64, // inode 位图起始块
    pub block_bitmap_start: u64, // 数据块位图起始块
    /** 文件系统状态 */
    pub mounted: bool, // 是否挂载
    pub dirty: bool,     // 是否有未写回的修改
    /** 其他元信息 */
    pub magic: u64, //魔数，用于识别文件系统
}

impl SuperBlock {
    fn new(total_inodes: u64) -> Self {
        let block_size: u64 = 4096; // 4KB
        let total_blocks = 64 * 1024 * 1024 / block_size; // 64MB / 4KB = 16384 块

        let superblock_size = 1; // 超级块占 1 块

        // inode 位图占用的块数 = ceil(total_inodes / 8 / block_size)
        let inode_bitmap_size = (total_inodes + 8 * block_size - 1) / (8 * block_size);
        // 数据块位图占用的块数 = ceil(total_blocks / 8 / block_size)
        let block_bitmap_size = (total_blocks + 8 * block_size - 1) / (8 * block_size);

        let inode_table_size = (total_inodes * 128 + block_size - 1) / block_size; // 每个 inode 128B

        let inode_bitmap_start = superblock_size;
        let block_bitmap_start = inode_bitmap_start + inode_bitmap_size;
        let inode_table_start = block_bitmap_start + block_bitmap_size;
        let data_block_start = inode_table_start + inode_table_size;

        Self {
            fs_type: "MiNiFS".to_string(),
            block_size,
            total_blocks,
            free_blocks: total_blocks,
            data_block_start,
            total_inodes,
            free_inode: total_inodes,
            inode_table_start,
            inode_bitmap_start,
            block_bitmap_start,
            mounted: false,
            dirty: false,
            magic: 0xDEADBEEF,
        }
    }
}
