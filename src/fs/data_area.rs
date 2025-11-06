use crate::disk::{BlockDevice, FileDisk, BLOCK_SIZE};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DataArea {
    pub blocks: Vec<u8>,   // 数据块
    pub total_blocks: u64, // 块总数
    pub start_block: u64,  // 起始块号
    #[serde(skip)] // 不序列化
    dirty: Vec<bool>, // 每个块是否被修改
}

impl DataArea {
    pub fn new(start_block: u64, total_blocks: u64) -> Self {
        Self {
            blocks: vec![0u8; (total_blocks as usize) * BLOCK_SIZE], // 扁平化存储
            total_blocks,
            start_block,
            dirty: vec![false; total_blocks as usize],
        }
    }

    pub fn write_block(&mut self, index: u64, buf: &[u8]) -> Result<(), String> {
        if index >= self.total_blocks {
            return Err("Block index out of range".to_string());
        }
        if buf.len() > BLOCK_SIZE {
            return Err("Data too large".to_string());
        }
        let start = (index as usize) * BLOCK_SIZE;
        self.blocks[start..start + buf.len()].copy_from_slice(buf);
        if buf.len() < BLOCK_SIZE {
            self.blocks[start + buf.len()..start + BLOCK_SIZE].fill(0);
        }
        self.dirty[index as usize] = true;
        Ok(())
    }

    pub fn read_block(&self, index: u64) -> Option<&[u8]> {
        if index >= self.total_blocks {
            return None;
        }
        let start = (index as usize) * BLOCK_SIZE;
        Some(&self.blocks[start..start + BLOCK_SIZE])
    }

    pub fn sync(&mut self, disk: &mut FileDisk) -> std::io::Result<()> {
        for i in 0..self.total_blocks {
            if self.dirty[i as usize] {
                let start = (i as usize) * BLOCK_SIZE;

                // 临时数组，写入 disk
                let mut buf = [0u8; BLOCK_SIZE];
                buf.copy_from_slice(&self.blocks[start..start + BLOCK_SIZE]);

                disk.write_block(self.start_block + i, &mut buf)?;
                self.dirty[i as usize] = false;
            }
        }
        Ok(())
    }

    pub fn load(&mut self, disk: &mut FileDisk) -> std::io::Result<()> {
        for i in 0..self.total_blocks {
            let start = (i as usize) * BLOCK_SIZE;

            // 临时数组，读取 disk
            let mut buf = [0u8; BLOCK_SIZE];
            disk.read_block(self.start_block + i, &mut buf)?;

            self.blocks[start..start + BLOCK_SIZE].copy_from_slice(&buf);
            self.dirty[i as usize] = false;
        }
        Ok(())
    }
}
