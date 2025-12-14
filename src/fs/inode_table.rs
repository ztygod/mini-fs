use crate::{
    disk::{BlockDevice, FileDisk},
    fs::inode_bitmap::InodeBitmap,
    utils::{current_timestamp, generate_uuid},
};
use serde::{Deserialize, Serialize};

pub const DIRECT_PTRS: usize = 12;
pub const PTRS_PER_BLOCK: usize = 1024;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub enum InodeType {
    File,
    Directory,
    Symlink,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InodeTable {
    pub inodes: Vec<Inode>,
    pub start_block: u64,
    pub total_inodes: u64,
    pub allocated_inodes: u64,
}

impl InodeTable {
    pub fn new(start_block: u64, total_inodes: u64) -> Self {
        Self {
            inodes: vec![Inode::empty(); total_inodes as usize],
            start_block,
            total_inodes,
            allocated_inodes: 0,
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
        if self.allocated_inodes >= self.total_inodes {
            return None;
        }
        if let Some(index) = inode_bitmap.alloc() {
            let inode = Inode::new(inode_type, uid, gid, perm);
            self.inodes[index as usize] = inode;
            self.allocated_inodes += 1;
            Some(index as usize)
        } else {
            None
        }
    }

    pub fn free_inode(&mut self, inode_bitmap: &mut InodeBitmap, inode_index: u64) {
        self.inodes[inode_index as usize] = Inode::empty();
        inode_bitmap.free(inode_index);
        self.allocated_inodes -= 1;
    }

    pub fn get_inode(&self, index: u64) -> Option<&Inode> {
        self.inodes.get(index as usize)
    }

    pub fn get_inode_mut(&mut self, index: u64) -> Option<&mut Inode> {
        self.inodes.get_mut(index as usize)
    }

    pub fn sync(&self, disk: &mut FileDisk) -> std::io::Result<()> {
        let bytes = bincode::serialize(&self.inodes)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        let total_blocks = (bytes.len() as u64 + 8 + 4095) / 4096;
        let mut block_buf = [0u8; 4096];
        let len_bytes = (bytes.len() as u64).to_le_bytes();
        block_buf[..8].copy_from_slice(&len_bytes);
        let first_chunk = std::cmp::min(4096 - 8, bytes.len());
        block_buf[8..8 + first_chunk].copy_from_slice(&bytes[..first_chunk]);
        disk.write_block(self.start_block, &block_buf)?;
        let mut offset = first_chunk;
        for i in 1..total_blocks {
            let mut block_buf = [0u8; 4096];
            let chunk = std::cmp::min(4096, bytes.len() - offset);
            block_buf[..chunk].copy_from_slice(&bytes[offset..offset + chunk]);
            disk.write_block(self.start_block + i, &block_buf)?;
            offset += chunk;
        }
        Ok(())
    }

    pub fn load(disk: &mut FileDisk, start_block: u64) -> std::io::Result<Self> {
        let mut block_buf = [0u8; 4096];
        disk.read_block(start_block, &mut block_buf)?;
        let mut len_bytes = [0u8; 8];
        len_bytes.copy_from_slice(&block_buf[..8]);
        let serialized_len = u64::from_le_bytes(len_bytes) as usize;
        let total_blocks = (serialized_len + 8 + 4095) / 4096;
        let mut bytes = Vec::with_capacity(serialized_len);
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
        let total_inodes = inodes.len() as u64;
        let allocated_inodes = inodes
            .iter()
            .filter(|inode| inode.id != Inode::empty().id)
            .count() as u64;
        Ok(Self {
            inodes,
            start_block,
            total_inodes,
            allocated_inodes,
        })
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct Inode {
    pub id: String,
    pub inode_type: InodeType,
    pub size: u64,
    pub permissions: u16,
    pub uid: u32,
    pub gid: u32,
    pub link_count: u32,
    pub atime: u64,
    pub mtime: u64,
    pub ctime: u64,
    pub direct_blocks: [u64; DIRECT_PTRS],
    pub indirect_block: Option<u64>,
    pub double_indirect_block: Option<u64>,
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

    pub fn touch(&mut self) {
        self.atime = current_timestamp();
        self.mtime = current_timestamp();
        self.ctime = current_timestamp();
    }

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
            count += 1;
        }
        if self.double_indirect_block.is_some() {
            count += 1;
        }
        count
    }
}
