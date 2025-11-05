use crate::disk::{Block, BlockDevice, FileDisk};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct DataBlockBitmap {
    pub bits: Vec<u8>,     // 位图数据，每个 bit 表示一个数据块是否被使用
    pub total_blocks: u64, // 数据块总数
    pub free_blocks: u64,  // 当前空闲块数
    pub start_block: u64,  // 位图在磁盘中的起始块号
}

impl DataBlockBitmap {
    pub fn new(total_blocks: u64, start_block: u64) -> Self {
        let btye_len = ((total_blocks + 7) / 8) as usize;

        Self {
            bits: vec![0; btye_len],
            total_blocks,
            free_blocks: total_blocks,
            start_block,
        }
    }

    // 分配一个空闲的数据块，返回编号
    pub fn alloc(&mut self) -> Option<u64> {
        for (byte_index, byte) in self.bits.iter_mut().enumerate() {
            if *byte != 0xFF {
                for bit in 0..8 {
                    if *byte & (1 << bit) == 0 {
                        *byte |= 1 << bit;
                        self.free_blocks -= 1;
                        return Some((byte_index * 8 + bit) as u64);
                    }
                }
            }
        }
        None
    }

    // 释放一个数据块
    pub fn free(&mut self, block_index: u64) {
        if block_index > self.total_blocks {
            return; // 防止越界
        }

        let byte_index = (block_index / 8) as usize;
        let bit_index = (block_index % 8) as u8;

        if self.bits[byte_index] & (1 << bit_index) != 0 {
            self.bits[byte_index] &= !(1 << bit_index);
            self.free_blocks += 1;
        }
    }

    pub fn is_used(&self, block_index: u64) -> bool {
        let byte_index = (block_index / 8) as usize;
        let bit_index = (block_index % 8) as u8;
        (self.bits[byte_index]) & (1 << bit_index) != 0
    }

    // 从磁盘加载数据块位图
    pub fn load(disk: &mut FileDisk, start_block: u64, total_blocks: u64) -> Self {
        let size_in_block = ((total_blocks + 8 * 4096 - 1) / (8 * 4096)) as u64;
        let mut bits = Vec::with_capacity((size_in_block * 4096) as usize);
        let mut block_buf: Block = [0; 4096];

        for i in 0..size_in_block {
            disk.read_block(start_block + i, &mut block_buf).unwrap();
            bits.extend_from_slice(&block_buf);
        }

        // 截掉多余字节，只保留有效位
        let byte_len = ((total_blocks + 7) / 8) as usize;
        bits.truncate(byte_len);

        let free_blocks = total_blocks - bits.iter().map(|b| b.count_ones() as u64).sum::<u64>();

        Self {
            bits,
            total_blocks,
            free_blocks,
            start_block,
        }
    }

    // 将数据块位图写回磁盘
    pub fn sync(&self, disk: &mut FileDisk) -> std::io::Result<()> {
        let mut bits_to_write = self.bits.clone();

        // 每块 4KB，不够用 0 填充
        let total_blocks_in_bitmap = (bits_to_write.len() as u64 + 4096 - 1) / 4096;
        bits_to_write.resize((total_blocks_in_bitmap * 4096) as usize, 0);

        let mut block_buf: Block = [0; 4096];
        for i in 0..total_blocks_in_bitmap {
            let start = (i * 4096) as usize;
            let end = start + 4096;
            block_buf.copy_from_slice(&bits_to_write[start..end]);
            disk.write_block(self.start_block + i, &block_buf)?;
        }

        Ok(())
    }
}
