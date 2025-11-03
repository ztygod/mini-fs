use serde::{Deserialize, Serialize};

const MAGIC_NUMBER: u32 = 0xDEADBEEF;
pub const FS_VERSION: u32 = 1;

#[derive(Debug, Serialize, Deserialize)]
pub struct SuperBlcok {
    magic: u32,
    pub total_blocks: u32,
    pub total_inodes: u32,
    pub inode_bitmap_block_id: u32,
    pub data_block_bitmap_block_id: u32,
    pub inode_table_start_block_id: u32,
    pub data_area_start_block_id: u32,
}

impl SuperBlcok {
    pub fn new(total_blocks: u32, total_inodes: u32) -> Self {
        use super::config::*;
        Self {
            magic: MAGIC_NUMBER,
            total_blocks,
            total_inodes,
            inode_bitmap_block_id: INODE_BITMAP_BLOCK_ID,
            data_block_bitmap_block_id: DATA_BLOCK_BITMAP_BLOCK_ID,
            inode_table_start_block_id: INODE_TABLE_START_BLOCK_ID,
            data_area_start_block_id: DATA_AREA_START_BLOCK_ID,
        }
    }

    pub fn is_valid(&self) -> bool {
        self.magic == MAGIC_NUMBER
    }
}
