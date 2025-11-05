use serde::{Deserialize, Serialize};

use crate::{
    disk::{BlockDevice, FileDisk},
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
        inode_type: InodeType,
        uid: u32,
        gid: u32,
        perm: u16,
    ) -> Option<usize> {
        if self.inodes.len() >= self.total_inodes as usize {
            return None;
        }

        let inode = Inode::new(inode_type, uid, gid, perm);
        self.inodes.push(inode.clone());
        Some(self.inodes.len())
    }

    pub fn free_inode(&mut self, index: u64) {
        // 从表中移除 inode
        self.inodes[index as usize] = None;
    }

    pub fn get_inode(&self, index: u64) -> Option<&Inode> {
        self.inodes.get(index as usize)
    }

    pub fn get_inode_mut(&mut self, index: u64) -> Option<&mut Inode> {
        self.inodes.get_mut(index as usize)
    }

    pub fn load(disk: &mut FileDisk, start_block: u64, total_inodes: u64) -> Self {
        // 每块 4KB
        let block_size = 4096;

        // 计算 inode 表占用的总块数
        let inode_size = std::mem::size_of::<Inode>(); // 每个 inode 的大小
        let total_bytes = (total_inodes as usize) * inode_size;
        let total_blocks = (total_bytes + block_size - 1) / block_size;

        // 读取所有块到一个 Vec<u8>
        let mut bytes = Vec::with_capacity(total_blocks * block_size);
        let mut block_buf: [u8; 4096] = [0; 4096];
        for i in 0..total_blocks {
            disk.read_block(start_block + i as u64, &mut block_buf)
                .unwrap();
            bytes.extend_from_slice(&block_buf);
        }

        // 截掉多余的字节（可能最后一块填充了 0）
        bytes.truncate(total_bytes);

        // 反序列化
        let inodes: Vec<Inode> = bincode::deserialize(&bytes).unwrap();

        Self {
            inodes,
            start_block,
            total_inodes,
        }
    }

    pub fn sync(&self, disk: &mut FileDisk) -> std::io::Result<()> {
        let bytes = bincode::serialize(&self.inodes).unwrap();
        let total_blocks = (bytes.len() as u64 + 4095) / 4096;
        let mut block_buf = [0u8; 4096];

        for i in 0..total_blocks {
            let start = (i * 4096) as usize;
            let end = std::cmp::min(start + 4096, bytes.len());
            block_buf[..end - start].copy_from_slice(&bytes[start..end]);
            disk.write_block(self.start_block + i, &block_buf)?;
        }
        Ok(())
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
                self.inc_link();
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
