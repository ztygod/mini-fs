use std::io::empty;

use serde::{Deserialize, Serialize};

use crate::{
    disk::{BlockDevice, FileDisk},
    fs::inode_bitmap::{self, InodeBitmap},
    utils::{current_timestamp, generate_uuid},
};

pub const DIRECT_PTRS: usize = 12; // 12个直接块指针（经典设计）
pub const PTRS_PER_BLOCK: usize = 1024; // 每个间接块可以指向多少个数据块（假设每个指针4字节）

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum InodeType {
    File,      // 文件
    Directory, // 目录项
    Symlink,   // 符号链接，指向另一个文件路径
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InodeTable {
    pub inodes: Vec<Inode>,
    pub start_block: u64,
    pub total_inodes: u64,
}

impl InodeTable {
    pub fn new(start_block: u64, total_inodes: u64) -> Self {
        Self {
            inodes: Vec::new(),
            start_block,
            total_inodes,
        }
    }

    pub fn alloc_inode(
        &mut self,
        inode_bitmap: &mut InodeBitmap,
        inode_type: InodeType,
        uid: u32,
        gid: u32,
        perm: u16,
    ) -> Option<usize> {
        if self.inodes.len() >= self.total_inodes as usize {
            return None;
        }

        if let Some(index) = inode_bitmap.alloc() {
            let inode = Inode::new(inode_type, uid, gid, perm);
            self.inodes[index as usize] = inode;
            Some(index as usize)
        } else {
            None
        }
    }

    pub fn free_inode(&mut self, inode_bitmap: &mut InodeBitmap, inode_index: u64) {
        self.inodes[inode_index as usize] = Inode::empty();
        inode_bitmap.free(inode_index);
    }

    pub fn get_inode(&self, index: u64) -> Option<&Inode> {
        self.inodes.get(index as usize)
    }

    pub fn get_inode_mut(&mut self, index: u64) -> Option<&mut Inode> {
        self.inodes.get_mut(index as usize)
    }

    pub fn sync(&self, disk: &mut FileDisk) -> std::io::Result<()> {
        // 1. 序列化
        let bytes = bincode::serialize(&self.inodes)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        let total_blocks = (bytes.len() as u64 + 8 + 4095) / 4096; // +8 for length prefix

        // 2. 把 length 写在第一个块的前 8 字节
        let mut block_buf = [0u8; 4096];
        // zero already by init
        let len_bytes = (bytes.len() as u64).to_le_bytes();
        block_buf[..8].copy_from_slice(&len_bytes);

        // copy first chunk after prefix
        let first_chunk = std::cmp::min(4096 - 8, bytes.len());
        block_buf[8..8 + first_chunk].copy_from_slice(&bytes[..first_chunk]);
        disk.write_block(self.start_block, &block_buf)?;

        // 写剩余块
        let mut offset = first_chunk;
        for i in 1..total_blocks {
            let mut block_buf = [0u8; 4096]; // 清零
            let chunk = std::cmp::min(4096, bytes.len() - offset);
            block_buf[..chunk].copy_from_slice(&bytes[offset..offset + chunk]);
            disk.write_block(self.start_block + i, &block_buf)?;
            offset += chunk;
        }
        Ok(())
    }

    pub fn load(disk: &mut FileDisk, start_block: u64) -> std::io::Result<Self> {
        // 先读第一个块，取得序列化长度
        let mut block_buf = [0u8; 4096];
        disk.read_block(start_block, &mut block_buf)?;
        let mut len_bytes = [0u8; 8];
        len_bytes.copy_from_slice(&block_buf[..8]);
        let serialized_len = u64::from_le_bytes(len_bytes) as usize;

        let total_blocks = (serialized_len + 8 + 4095) / 4096;

        let mut bytes = Vec::with_capacity(serialized_len);
        // first block: 从 8 开始取
        let first_chunk = std::cmp::min(4096 - 8, serialized_len);
        bytes.extend_from_slice(&block_buf[8..8 + first_chunk]);
        let mut read = first_chunk;

        for i in 1..total_blocks {
            disk.read_block(start_block + i as u64, &mut block_buf)?;
            let chunk = std::cmp::min(4096, serialized_len - read);
            bytes.extend_from_slice(&block_buf[..chunk]);
            read += chunk;
        }

        let inodes: Vec<Inode> = bincode::deserialize(&bytes)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

        let total_inodes = inodes.clone().len();
        Ok(Self {
            inodes,
            start_block,
            total_inodes: total_inodes as u64,
        })
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Inode {
    pub id: String,            // inode编号
    pub inode_type: InodeType, // 文件类型
    pub size: u64,             // 文件大小（字节）
    pub permissions: u16,      // 权限位（类似Unix: rwxr-xr-x）
    pub uid: u32,              // 所属用户
    pub gid: u32,              // 所属组
    pub link_count: u32,       // 硬链接数（有多少目录项链接到该 inode）
    pub atime: u64,            // 最后访问时间（Access Time）
    pub mtime: u64,            // 最后修改时间（Modify Time）
    pub ctime: u64,            // 状态改变时间（Change Time）

    // 块索引区
    pub direct_blocks: [u64; DIRECT_PTRS],  // 直接块指针
    pub indirect_block: Option<u64>,        // 一级间接块
    pub double_indirect_block: Option<u64>, // 二级间接块
}

impl Inode {
    pub fn new(inode_type: InodeType, uid: u32, gid: u32, permissions: u16) -> Self {
        Self {
            id: generate_uuid(),
            inode_type,
            size: 0,
            permissions,
            uid,
            gid,
            link_count: 1,
            atime: current_timestamp(),
            mtime: current_timestamp(),
            ctime: current_timestamp(),
            direct_blocks: [0; DIRECT_PTRS],
            indirect_block: None,
            double_indirect_block: None,
        }
    }

    pub fn empty() -> Self {
        Self {
            id: String::new(),
            inode_type: InodeType::File,
            size: 0,
            permissions: 0,
            uid: 0,
            gid: 0,
            link_count: 0,
            atime: 0,
            mtime: 0,
            ctime: 0,
            direct_blocks: [0; DIRECT_PTRS],
            indirect_block: None,
            double_indirect_block: None,
        }
    }

    // 更新时间戳
    pub fn touch(&mut self) {
        self.atime = current_timestamp();
        self.mtime = current_timestamp();
        self.ctime = current_timestamp();
    }

    // 增加/减少硬链接计数
    pub fn inc_link(&mut self) {
        self.link_count += 1;
    }

    pub fn dec_link(&mut self) {
        if self.link_count > 0 {
            self.link_count -= 1;
        }
    }

    // 块管理
    pub fn add_block(&mut self, block_id: u64) -> Result<(), String> {
        for ptr in self.direct_blocks.iter_mut() {
            if *ptr == 0 {
                *ptr = block_id;
                return Ok(());
            }
        }

        if self.indirect_block.is_none() {
            self.indirect_block = Some(block_id);
            Ok(())
        } else {
            Err("No space in inode block pointers".to_string())
        }
    }

    pub fn block_count(&self) -> u64 {
        let mut count = self.direct_blocks.iter().filter(|&&b| b != 0).count() as u64;
        if self.indirect_block.is_some() {
            count += 1
        }
        if self.double_indirect_block.is_some() {
            count += 1
        }
        count
    }
}
