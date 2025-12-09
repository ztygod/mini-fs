use crate::{
    disk::{BlockDevice, FileDisk},
    fs::{
        data_area::DataArea,
        data_block_bitmap::DataBlockBitmap,
        directory::{DirEntryType, Directory},
        inode_bitmap::InodeBitmap,
        inode_table::{InodeTable, InodeType},
        super_block::SuperBlock,
    },
};

pub mod config;
pub mod data_area;
pub mod data_block_bitmap;
pub mod directory;
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
    /// 创建新的文件系统实例  
    pub fn new(disk: FileDisk) -> Self {
        let super_block = SuperBlock::new(4096);

        Self {
            disk,
            super_block,
            inode_bitmap: InodeBitmap::new(4096, 1),
            data_bitmap: DataBlockBitmap::new(16384 - 131, 2),
            inode_table: InodeTable::new(3, 4096),
            data_area: DataArea::new(131, 16384 - 131),
        }
    }

    /// 挂载文件系统：从磁盘加载所有组件  
    pub fn mount(&mut self) -> Result<(), std::io::Error> {
        let mut block_buf = [0u8; 4096];
        self.disk.read_block(0, &mut block_buf)?;

        self.super_block = bincode::deserialize(&block_buf)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

        // 加载各个组件
        self.inode_bitmap = InodeBitmap::load(
            &mut self.disk,
            self.super_block.inode_bitmap_start,
            self.super_block.total_inodes,
        );

        self.data_bitmap = DataBlockBitmap::load(
            &mut self.disk,
            self.super_block.block_bitmap_start,
            self.super_block.total_blocks - self.super_block.data_block_start,
        );

        self.inode_table = InodeTable::load(&mut self.disk, self.super_block.inode_table_start)?;

        self.data_area.load(&mut self.disk)?;

        self.super_block.mounted = true;
        Ok(())
    }

    /// 格式化文件系统：初始化所有结构  
    pub fn format(&mut self) -> Result<(), std::io::Error> {
        // 重置超级块
        self.super_block = SuperBlock::new(4096);
        self.super_block.mounted = true;
        self.super_block.dirty = true;

        // 重置位图
        self.inode_bitmap = InodeBitmap::new(4096, 1);
        self.data_bitmap = DataBlockBitmap::new(16384 - 131, 2);

        // 重置inode表和数据区
        self.inode_table = InodeTable::new(3, 4096);
        self.data_area = DataArea::new(131, 16384 - 131);

        // 创建根目录（inode 0）
        if let Some(root_inode) =
            self.inode_table
                .alloc_inode(&mut self.inode_bitmap, InodeType::Directory, 0, 0, 0o755)
        {
            // 创建根目录的目录结构
            let mut root_dir = Directory::new(root_inode);

            // 添加 "." 和 ".." 条目
            root_dir
                .add(root_inode, ".", DirEntryType::Directory)
                .unwrap();
            root_dir
                .add(root_inode, "..", DirEntryType::Directory)
                .unwrap();

            // 序列化目录到数据块
            let dir_bytes = bincode::serialize(&root_dir)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

            // 分配数据块存储根目录
            if let Some(block_id) = self.data_bitmap.alloc() {
                self.data_area.write_block(block_id, &dir_bytes).unwrap();

                // 更新inode指向数据块
                if let Some(inode) = self.inode_table.get_inode_mut(root_inode as u64) {
                    inode.add_block(block_id).unwrap();
                    inode.size = dir_bytes.len() as u64;
                }

                // 更新超级块计数
                self.super_block.free_inode -= 1;
                self.super_block.free_blocks -= 1;
            }
        }

        // 同步到磁盘
        self.sync()?;
        Ok(())
    }

    /// 创建文件  
    pub fn create_file(
        &mut self,
        parent_path: &str,
        name: &str,
        content: &[u8],
    ) -> Result<u64, String> {
        // 1. 分配inode
        let inode_id = self
            .inode_table
            .alloc_inode(&mut self.inode_bitmap, InodeType::File, 0, 0, 0o644)
            .ok_or("Failed to allocate inode")?;

        // 2. 分配数据块（如果需要）
        let mut blocks_used = 0;
        if !content.is_empty() {
            let block_id = self
                .data_bitmap
                .alloc()
                .ok_or("Failed to allocate data block")?;

            self.data_area
                .write_block(block_id, content)
                .map_err(|e| e)?;

            // 3. 更新inode
            if let Some(inode) = self.inode_table.get_inode_mut(inode_id as u64) {
                inode.add_block(block_id).map_err(|e| e)?;
                inode.size = content.len() as u64;
                inode.touch();
            }
            blocks_used = 1;
        }

        // 4. 更新父目录
        self.add_directory_entry(parent_path, name, inode_id, DirEntryType::File)?;

        // 5. 更新计数器
        self.super_block.free_inode -= 1;
        self.super_block.free_blocks -= blocks_used;
        self.super_block.dirty = true;

        Ok(inode_id as u64)
    }

    /// 创建目录  
    pub fn create_dir(&mut self, parent_path: &str, name: &str) -> Result<u64, String> {
        // 1. 分配inode
        let inode_id = self
            .inode_table
            .alloc_inode(&mut self.inode_bitmap, InodeType::Directory, 0, 0, 0o755)
            .ok_or("Failed to allocate inode")?;

        // 2. 创建目录结构
        let mut new_dir = Directory::new(inode_id);
        new_dir.add(inode_id, ".", DirEntryType::Directory).unwrap();
        new_dir
            .add(inode_id, "..", DirEntryType::Directory)
            .unwrap();

        // 3. 序列化并存储
        let dir_bytes = bincode::serialize(&new_dir).unwrap();
        let block_id = self
            .data_bitmap
            .alloc()
            .ok_or("Failed to allocate data block")?;

        self.data_area.write_block(block_id, &dir_bytes).unwrap();

        // 4. 更新inode
        if let Some(inode) = self.inode_table.get_inode_mut(inode_id as u64) {
            inode.add_block(block_id).unwrap();
            inode.size = dir_bytes.len() as u64;
            inode.touch();
        }

        // 5. 更新父目录
        self.add_directory_entry(parent_path, name, inode_id, DirEntryType::Directory)?;

        // 6. 更新计数器
        self.super_block.free_inode -= 1;
        self.super_block.free_blocks -= 1;
        self.super_block.dirty = true;

        Ok(inode_id as u64)
    }

    /// 列出目录内容  
    pub fn list_dir(&self, path: &str) -> Result<Vec<String>, String> {
        // 查找目录的inode
        let inode_id = self.find_inode(path)?;

        // 获取inode
        let inode = self
            .inode_table
            .get_inode(inode_id)
            .ok_or("Inode not found")?;

        if !matches!(inode.inode_type, InodeType::Directory) {
            return Err("Not a directory".to_string());
        }

        // 读取目录数据
        let mut entries = Vec::new();
        for &block_id in &inode.direct_blocks {
            if block_id == 0 {
                break;
            }

            if let Some(block_data) = self.data_area.read_block(block_id) {
                if let Ok(dir) = bincode::deserialize::<Directory>(block_data) {
                    entries = dir.list();
                    break;
                }
            }
        }

        Ok(entries)
    }

    /// 同步所有组件到磁盘  
    pub fn sync(&mut self) -> Result<(), std::io::Error> {
        // 同步各个组件
        self.inode_bitmap.sync(&mut self.disk)?;
        self.data_bitmap.sync(&mut self.disk)?;
        self.inode_table.sync(&mut self.disk)?;
        self.data_area.sync(&mut self.disk)?;

        // 同步超级块
        let super_block_bytes = bincode::serialize(&self.super_block)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        let mut block_buf = [0u8; 4096];
        block_buf[..super_block_bytes.len()].copy_from_slice(&super_block_bytes);
        self.disk.write_block(0, &block_buf)?;

        self.super_block.dirty = false;
        Ok(())
    }

    /// 卸载文件系统  
    pub fn unmount(&mut self) -> Result<(), std::io::Error> {
        if self.super_block.dirty {
            self.sync()?;
        }
        self.super_block.mounted = false;
        Ok(())
    }

    // 辅助方法：添加目录项
    fn add_directory_entry(
        &mut self,
        parent_path: &str,
        name: &str,
        inode_id: usize,
        entry_type: DirEntryType,
    ) -> Result<(), String> {
        let parent_inode_id = self.find_inode(parent_path)?;

        // 读取父目录
        let parent_inode = self
            .inode_table
            .get_inode_mut(parent_inode_id)
            .ok_or("Parent inode not found")?;

        // 获取目录数据块
        let block_id = parent_inode.direct_blocks[0];
        if block_id == 0 {
            return Err("Parent directory has no data block".to_string());
        }

        // 读取并反序列化目录
        let block_data = self
            .data_area
            .read_block(block_id)
            .ok_or("Failed to read directory block")?;

        let mut parent_dir: Directory =
            bincode::deserialize(block_data).map_err(|_| "Failed to deserialize directory")?;

        // 添加新条目
        parent_dir.add(inode_id, name, entry_type)?;

        // 序列化并写回
        let dir_bytes = bincode::serialize(&parent_dir).unwrap();
        self.data_area.write_block(block_id, &dir_bytes).unwrap();

        // 更新父目录inode
        parent_inode.size = dir_bytes.len() as u64;
        parent_inode.touch();

        Ok(())
    }

    // 辅助方法：查找路径对应的inode
    fn find_inode(&self, path: &str) -> Result<u64, String> {
        // 简化实现：只支持从根目录开始的绝对路径
        if path == "/" {
            return Ok(0); // 根目录inode
        }

        // TODO: 实现完整的路径解析
        // 这里需要递归或迭代地解析路径
        Err("Path resolution not implemented".to_string())
    }

    /// 分配inode（供内部使用）  
    pub fn alloc_inode(&mut self) -> Option<u64> {
        self.inode_bitmap.alloc()
    }

    /// 分配数据块（供内部使用）  
    pub fn alloc_block(&mut self) -> Option<u64> {
        self.data_bitmap.alloc()
    }
}
