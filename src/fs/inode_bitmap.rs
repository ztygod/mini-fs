use serde::{Deserialize, Serialize};

use crate::disk::{file_disk, Block, BlockDevice, FileDisk};

#[derive(Debug, Serialize, Deserialize)]
pub struct InodeBitmap {
    pub bits: Vec<u8>,     // 位图数据，每个 bit 表示一个 inode 的状态
    pub total_inodes: u64, // inode 总数
    pub free_inodes: u64,  // 当前空闲 inode 数
    pub start_block: u64,  // 位图在磁盘中的起始块号（用于持久化）
}

impl InodeBitmap {
    // 创建一个新的 inode 位图（所有位清零 = 空闲）
    pub fn new(total_inodes: u64, start_block: u64) -> Self {
        let byte_len = ((total_inodes + 7) / 8) as usize;
        Self {
            bits: vec![0; byte_len],
            total_inodes,
            free_inodes: total_inodes,
            start_block,
        }
    }

    // 分配一个空闲 inode，返回 inode 编号（从 0 开始）
    pub fn alloc(&mut self) -> Option<u64> {
        for (byte_index, byte) in self.bits.iter_mut().enumerate() {
            if *byte != 0xFF {
                for bit in 0..8 {
                    if (*byte & (1 << bit)) == 0 {
                        *byte |= 1 << bit;
                        self.free_inodes -= 1;
                        return Some((byte_index * 8 + bit) as u64);
                    }
                }
            }
        }
        None // 没有空闲 inode
    }

    // 释放一个 inode
    pub fn free(&mut self, inode_index: u64) {
        if inode_index >= self.total_inodes {
            return; // 防止越界
        }

        let byte_index = (inode_index / 8) as usize;
        let bit_index = (inode_index % 8) as u8;
        if self.bits[byte_index] & (1 << bit_index) != 0 {
            // 防止空释放
            self.bits[byte_index] &= !(1 << bit_index);
            self.free_inodes += 1;
        }
    }

    // 检查 inode 是否被占用
    pub fn is_used(&mut self, inode_index: u64) {
        let byte_index = (inode_index / 8) as usize;
        let bit_index = (inode_index % 8) as u8;
        (self.bits[byte_index] & (1 << bit_index)) != 0;
    }

    // 从磁盘加载 inode 位图
    pub fn load(
        disk: &mut FileDisk,
        start_block: u64,
        size_in_block: u64,
        total_inodes: u64,
    ) -> Self {
        let mut bits = Vec::with_capacity((size_in_block * 4096) as usize);
        let mut block_buf: Block = [0; 4096];

        for i in 0..size_in_block {
            disk.read_block(start_block + i, &mut block_buf).unwrap();
            bits.extend_from_slice(&block_buf);
        }

        // 截掉多余的字节
        let byte_len = ((total_inodes + 7) / 8) as usize;
        bits.truncate(byte_len);

        let free_inodes = total_inodes - bits.iter().map(|b| b.count_ones() as u64).sum::<u64>();

        Self {
            bits,
            total_inodes,
            free_inodes,
            start_block,
        }
    }

    // 将 inode 位图写回磁盘
    pub fn sync(&self, disk: &mut FileDisk) -> std::io::Result<()> {
        let mut bits_to_write = self.bits.clone();

        // 每块 4KB，不够的用 0 填充
        let total_blocks = (bits_to_write.len() as u64 + 4096 - 1) / 4096;

        bits_to_write.resize((total_blocks * 4096) as usize, 0);

        let mut block_buf: Block = [0; 4096];
        for i in 0..total_blocks {
            let start = (i * 4096) as usize;
            let end = start + 4096;
            block_buf.copy_from_slice(&bits_to_write[start..end]);
            disk.write_block(self.start_block + i, &block_buf)?;
        }

        Ok(())
    }
}
