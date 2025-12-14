use crate::{
    disk::{BlockDevice, FileDisk},
    fs::{
        data_area::DataArea,
        data_block_bitmap::DataBlockBitmap,
        directory::{DirEntry, DirEntryType, Directory},
        inode_bitmap::InodeBitmap,
        inode_table::{Inode, InodeTable, InodeType},
        super_block::SuperBlock,
    },
    utils::{current_timestamp, split_path},
};

pub mod config;
pub mod data_area;
pub mod data_block_bitmap;
pub mod directory;
pub mod error;
pub mod inode_bitmap;
pub mod inode_table;
pub mod super_block;

bitflags::bitflags! {
    #[derive(Debug)]
    pub struct OpenFlags: u32 {
        const READ   = 0b0001;
        const WRITE  = 0b0010;
        const CREATE = 0b0100;
        const TRUNC  = 0b1000;
        const APPEND = 0b1_0000;
    }
}

#[derive(Debug)]
pub struct FileHandle {
    pub inode_id: u64,
    pub offset: u64,
    pub flags: OpenFlags,
}

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

        let inode_bitmap =
            InodeBitmap::new(super_block.total_inodes, super_block.inode_bitmap_start);

        let data_bitmap = DataBlockBitmap::new(
            super_block.total_blocks - super_block.data_block_start,
            super_block.block_bitmap_start,
        );

        let inode_table = InodeTable::new(super_block.inode_table_start, super_block.total_inodes);

        let data_area = DataArea::new(
            super_block.data_block_start,
            super_block.total_blocks - super_block.data_block_start,
        );

        Self {
            disk,
            super_block,
            inode_bitmap,
            data_bitmap,
            inode_table,
            data_area,
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

    /// 格式化文件系统
    pub fn format(&mut self) -> Result<(), std::io::Error> {
        println!("💾 Formatting virtual disk...");

        // 初始化 super_block、位图、inode_table、data_area
        self.super_block = SuperBlock::new(4096);
        self.super_block.mounted = true;
        self.super_block.dirty = true;

        self.inode_bitmap = InodeBitmap::new(
            self.super_block.total_inodes,
            self.super_block.inode_bitmap_start,
        );

        self.data_bitmap = DataBlockBitmap::new(
            self.super_block.total_blocks - self.super_block.data_block_start,
            self.super_block.block_bitmap_start,
        );

        self.inode_table = InodeTable::new(
            self.super_block.inode_table_start,
            self.super_block.total_inodes,
        );

        self.data_area = DataArea::new(
            self.super_block.data_block_start,
            self.super_block.total_blocks - self.super_block.data_block_start,
        );

        // 分配 root inode
        let root_index = 0;
        self.inode_bitmap
            .alloc_specific(root_index)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        self.super_block.free_inode -= 1;

        // 分配 root 数据块
        let root_block = self
            .data_bitmap
            .alloc()
            .map(|b| b + self.data_area.start_block) // 加上偏移
            .ok_or_else(|| {
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "Failed to allocate block for root",
                )
            })?;

        self.super_block.free_blocks -= 1;
        println!("Allocated root block id: {}", root_block);

        // 创建 root inode 并挂载数据块
        let inode = Inode::new(InodeType::Directory, 0, 0, 0o755);
        let inode = Inode {
            link_count: 2,
            direct_blocks: {
                let mut arr = [0u64; 12];
                arr[0] = root_block;
                arr
            },
            ..inode
        };
        self.inode_table.inodes[root_index] = inode.clone();
        println!("Root inode after creation: {:?}", inode);

        // 创建 root 目录结构
        let mut root_dir = Directory::new(root_index);
        root_dir
            .add(root_index, ".", DirEntryType::Directory)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        root_dir
            .add(root_index, "..", DirEntryType::Directory)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        let dir_bytes = bincode::serialize(&root_dir)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

        // 写入数据块
        self.data_area
            .write_block(root_block, &dir_bytes)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        println!("Root directory written, size: {} bytes", dir_bytes.len());

        // 更新 inode size
        let inode = self.inode_table.get_inode_mut(root_index as u64).unwrap();
        inode.size = dir_bytes.len() as u64;

        println!("Root inode final state: {:?}", inode);

        // 同步 inode_table 和 super_block 到磁盘
        self.sync()?;

        Ok(())
    }

    /// 创建目录
    pub fn create_dir(&mut self, parent_path: &str, name: &str) -> Result<u64, String> {
        println!(
            "--- Creating directory '{}' under '{}' ---",
            name, parent_path
        );

        let parent_inode_id = self.find_inode(parent_path)?;
        let parent_inode = self
            .inode_table
            .get_inode(parent_inode_id)
            .ok_or("Parent inode not found")?;
        println!("Parent inode before adding entry: {:?}", parent_inode);

        // 分配inode
        let inode_id = self
            .inode_table
            .alloc_inode(&mut self.inode_bitmap, InodeType::Directory, 0, 0, 0o755)
            .ok_or("Failed to allocate inode")?;
        println!("Allocated inode_id: {}", inode_id);

        // 创建目录结构
        let mut new_dir = Directory::new(inode_id);
        new_dir.add(inode_id, ".", DirEntryType::Directory).unwrap();
        new_dir
            .add(inode_id, "..", DirEntryType::Directory)
            .unwrap();
        let dir_bytes = bincode::serialize(&new_dir).unwrap();

        // 分配数据块
        let block_id = self
            .data_bitmap
            .alloc()
            .ok_or("Failed to allocate data block")?;
        self.super_block.free_blocks -= 1;
        self.data_area.write_block(block_id, &dir_bytes).unwrap();

        // 挂到 inode
        let inode = self.inode_table.get_inode_mut(inode_id as u64).unwrap();
        inode.add_block(block_id).unwrap();
        inode.size = dir_bytes.len() as u64;
        inode.touch();
        println!("New directory inode: {:?}", inode);

        // 更新父目录
        self.add_directory_entry(parent_path, name, inode_id, DirEntryType::Directory)?;
        self.super_block.free_inode -= 1;
        self.super_block.dirty = true;

        Ok(inode_id as u64)
    }

    /// 创建文件  
    pub fn create_file(&mut self, parent_path: &str, name: &str) -> Result<u64, String> {
        // 0. 检查文件是否已存在
        let full_path = format!("{}/{}", parent_path, name);
        if self.find_inode(&full_path).is_ok() {
            return Err("File already exists".to_string());
        }

        // 1. 分配 inode
        let inode_id = self
            .inode_table
            .alloc_inode(&mut self.inode_bitmap, InodeType::File, 0, 0, 0o644)
            .ok_or("Failed to allocate inode")?;

        let now = current_timestamp();

        // 2. 初始化 inode
        if let Some(inode) = self.inode_table.get_inode_mut(inode_id as u64) {
            inode.size = 0;
            inode.ctime = now;
            inode.mtime = now;
            // atime 不动
        }

        // 3. 添加目录项
        self.add_directory_entry(parent_path, name, inode_id, DirEntryType::File)?;

        // 4. 更新父目录 inode
        let parent_inode_id = self.find_inode(parent_path)?;
        if let Some(parent_inode) = self.inode_table.get_inode_mut(parent_inode_id) {
            parent_inode.mtime = now;
            parent_inode.ctime = now;
        }

        // 5. 更新超级块
        self.super_block.free_inode -= 1;
        self.super_block.dirty = true;

        Ok(inode_id as u64)
    }

    pub fn write_file(&mut self, path: &str, content: &[u8]) -> Result<(), String> {
        let inode_id = self.find_inode(path)?;
        let now = current_timestamp();

        // 1. 回收旧数据块
        self.free_file_blocks(inode_id)?;

        // 2. 写新数据
        let mut blocks_used = 0;
        if !content.is_empty() {
            let block_id = self.data_bitmap.alloc().ok_or("No free data blocks")?;

            self.data_area.write_block(block_id, content)?;

            if let Some(inode) = self.inode_table.get_inode_mut(inode_id) {
                inode.add_block(block_id)?;
                inode.size = content.len() as u64;
                inode.mtime = now;
            }

            blocks_used = 1;
        }

        // 3. ctime 不变（只是内容写）
        self.super_block.free_blocks -= blocks_used;
        self.super_block.dirty = true;

        Ok(())
    }

    pub fn create_or_write_file(
        &mut self,
        parent_path: &str,
        name: &str,
        content: &[u8],
    ) -> Result<u64, String> {
        let full_path = format!("{}/{}", parent_path, name);

        match self.find_inode(&full_path) {
            Ok(inode_id) => {
                self.write_file(&full_path, content)?;
                Ok(inode_id)
            }
            Err(_) => {
                let inode_id = self.create_file(parent_path, name)?;
                self.write_file(&full_path, content)?;
                Ok(inode_id)
            }
        }
    }

    /// 列出目录内容  
    pub fn list_dir(&self, path: &str) -> Result<Vec<DirEntry>, String> {
        // 获取目录 inode
        let inode_id = self.find_inode(path)?;
        let inode = self
            .inode_table
            .get_inode(inode_id)
            .ok_or("Inode not found")?;

        if !matches!(inode.inode_type, InodeType::Directory) {
            return Err("Not a directory".to_string());
        }

        // 读取所有 block，把所有目录项收集起来
        let mut result = Vec::new();

        for &block_id in &inode.direct_blocks {
            if block_id == 0 {
                break;
            }

            if let Some(block_data) = self.data_area.read_block(block_id) {
                let mut dir: Directory =
                    bincode::deserialize(block_data).map_err(|_| "Corrupted directory block")?;

                // 必须重建 index_map（因为 skip 了）
                dir.rebuild_index_map();

                // 追加目录项
                result.extend(dir.entries);
            }
        }

        result.sort_by(|a, b| {
            match (&a.entry_type, &b.entry_type) {
                (DirEntryType::Directory, DirEntryType::File) => std::cmp::Ordering::Less, // 文件夹在前
                (DirEntryType::File, DirEntryType::Directory) => std::cmp::Ordering::Greater, // 文件在后
                _ => a.name.cmp(&b.name), // 同类型按名字排序
            }
        });

        Ok(result)
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
        let parent_inode = self
            .inode_table
            .get_inode_mut(parent_inode_id)
            .ok_or("Parent inode not found")?;

        let block_id = parent_inode.direct_blocks[0];
        if block_id == 0 {
            // 添加更详细的错误信息
            return Err(format!(
                "Parent directory has no data block. inode_id={}, path={}",
                parent_inode_id, parent_path
            ));
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

    /// 删除文件    
    pub fn delete_file(&mut self, path: &str, name: &str) -> Result<(), String> {
        // 1. 查找文件inode
        let file_inode_id = self.find_inode(&format!("{}/{}", path, name))?;

        // 2. 释放文件占用的数据块
        let inode = self
            .inode_table
            .get_inode(file_inode_id)
            .ok_or("File inode not found")?;

        for &block_id in &inode.direct_blocks {
            if block_id != 0 {
                self.data_bitmap.free(block_id);
                // DataArea 不需要 remove_block，位图已经管理分配
            }
        }

        // 3. 释放inode
        self.inode_bitmap.free(file_inode_id);

        // 4. 从父目录中移除条目
        self.remove_directory_entry(path, name)?;

        // 5. 更新计数器
        self.super_block.free_inode += 1;
        self.super_block.dirty = true;

        Ok(())
    }

    /// 删除目录    
    pub fn delete_dir(&mut self, path: &str, name: &str) -> Result<(), String> {
        // 类似delete_file，但需要检查目录是否为空
        let dir_inode_id = self.find_inode(&format!("{}/{}", path, name))?;

        // 检查目录是否为空
        let entries = self.list_dir(&format!("{}/{}", path, name))?;
        if entries.len() > 2 {
            // 包含 . 和 ..
            return Err("Directory not empty".to_string());
        }

        // 释放目录数据块和inode
        let inode = self
            .inode_table
            .get_inode(dir_inode_id)
            .ok_or("Directory inode not found")?;

        if inode.direct_blocks[0] != 0 {
            // 检查是否为 0 而不是 Some
            let block_id = inode.direct_blocks[0];
            self.data_bitmap.free(block_id);
            // DataArea 不需要 remove_block
        }

        self.inode_bitmap.free(dir_inode_id);
        self.remove_directory_entry(path, name)?;

        self.super_block.free_inode += 1;
        self.super_block.dirty = true;

        Ok(())
    }

    /// 读取文件内容    
    pub fn read_file(&self, path: &str, name: &str) -> Result<Vec<u8>, String> {
        let file_inode_id = self.find_inode(&format!("{}/{}", path, name))?;
        let inode = self
            .inode_table
            .get_inode(file_inode_id)
            .ok_or("File inode not found")?;

        // 读取文件数据块
        let block_id = inode.direct_blocks[0];
        if block_id != 0 {
            // 改为检查是否为 0，而不是使用 Some
            if let Some(data) = self.data_area.read_block(block_id) {
                return Ok(data[..inode.size as usize].to_vec());
            }
        }

        Ok(Vec::new())
    }

    /// 获取文件状态信息  
    pub fn stat(&self, path: &str, name: &str) -> Result<Inode, String> {
        let inode_id = self.find_inode(&format!("{}/{}", path, name))?;
        let inode = self
            .inode_table
            .get_inode(inode_id)
            .ok_or("File inode not found")?;

        Ok(inode.clone())
    }

    // 辅助方法：从目录中移除条目
    fn remove_directory_entry(&mut self, parent_path: &str, name: &str) -> Result<(), String> {
        let parent_inode_id = self.find_inode(parent_path)?;
        let parent_inode = self
            .inode_table
            .get_inode_mut(parent_inode_id)
            .ok_or("Parent inode not found")?;

        let block_id = parent_inode.direct_blocks[0];
        if block_id == 0 {
            return Err("Parent directory has no data block".to_string());
        }

        let block_data = self
            .data_area
            .read_block(block_id)
            .ok_or("Failed to read directory block")?;

        let mut parent_dir: Directory =
            bincode::deserialize(block_data).map_err(|_| "Failed to deserialize directory")?;

        // 关键：重建 index_map
        parent_dir.rebuild_index_map();

        // 删除条目
        parent_dir
            .remove(name)
            .ok_or("Entry not found in directory")?;

        let dir_bytes = bincode::serialize(&parent_dir).map_err(|e| e.to_string())?;
        self.data_area
            .write_block(block_id, &dir_bytes)
            .map_err(|e| e.to_string())?;

        parent_inode.size = dir_bytes.len() as u64;
        parent_inode.touch();

        Ok(())
    }

    pub fn find_inode(&self, path: &str) -> Result<u64, String> {
        println!("🔍 find_inode called with path: {:?}", path);

        if path == "/" {
            return Ok(0);
        }

        let normalized_path = path.trim_start_matches('/').trim();
        if normalized_path.is_empty() {
            return Ok(0);
        }

        let components: Vec<&str> = normalized_path
            .split('/')
            .filter(|s| !s.is_empty())
            .collect();
        println!("Debug: path components = {:?}", components);

        let mut current_inode = 0u64; // 从根目录开始

        for component in components {
            println!("Debug: resolving component: {}", component);
            let inode = self
                .inode_table
                .get_inode(current_inode)
                .ok_or("Inode not found")?;

            if !matches!(inode.inode_type, InodeType::Directory) {
                return Err("Path component is not a directory".to_string());
            }

            let block_id = inode.direct_blocks[0];
            if block_id == 0 {
                return Err("Directory has no data block".to_string());
            }

            let block_data = self
                .data_area
                .read_block(block_id)
                .ok_or("Failed to read directory block")?;

            let mut directory = Directory::load_from_bytes(block_data)
                .map_err(|_| "Failed to deserialize directory")?;

            if let Some(inode_index) = directory.find(component) {
                println!(
                    "Debug: component '{}' resolved to inode {}",
                    component, inode_index
                );
                current_inode = inode_index as u64;
            } else {
                println!(
                    "❌ component '{}' not found in current directory",
                    component
                );
                return Err(format!("Path component not found: {}", component));
            }
        }

        println!("✅ find_inode resolved to inode {}", current_inode);
        Ok(current_inode)
    }

    pub fn open(&mut self, path: &str, flags: OpenFlags) -> Result<FileHandle, String> {
        let inode_id = match self.find_inode(path) {
            Ok(id) => {
                // 文件存在
                if flags.contains(OpenFlags::TRUNC) && flags.contains(OpenFlags::WRITE) {
                    self.truncate_file(id)?;
                }
                id
            }
            Err(_) => {
                // 文件不存在
                if flags.contains(OpenFlags::CREATE) {
                    self.create_file_from_path(path)?
                } else {
                    return Err("File not found".to_string());
                }
            }
        };

        // 类型检查：不能 open 目录
        let inode = self
            .inode_table
            .get_inode(inode_id)
            .ok_or("Inode not found")?;

        if inode.inode_type != InodeType::File {
            return Err("Cannot open directory as file".into());
        }

        // 权限检查（简化版）
        self.check_open_permissions(&inode, &flags)?;

        // offset 初始化
        let offset = if flags.contains(OpenFlags::APPEND) {
            inode.size
        } else {
            0
        };

        Ok(FileHandle {
            inode_id,
            offset,
            flags,
        })
    }

    fn check_open_permissions(&self, inode: &Inode, flags: &OpenFlags) -> Result<(), String> {
        if flags.contains(OpenFlags::READ) && inode.permissions & 0o400 == 0 {
            return Err("Permission denied: read".into());
        }

        if flags.contains(OpenFlags::WRITE) && inode.permissions & 0o200 == 0 {
            return Err("Permission denied: write".into());
        }

        Ok(())
    }

    pub fn free_file_blocks(&mut self, inode_id: u64) -> Result<(), String> {
        let inode = self
            .inode_table
            .get_inode_mut(inode_id)
            .ok_or("Inode not found")?;

        let mut freed = 0;

        // 1. 释放 direct blocks
        for block in inode.direct_blocks.iter_mut() {
            if *block != 0 {
                self.data_bitmap.free(*block);
                *block = 0;
                freed += 1;
            }
        }

        // 2. 释放 indirect block（注意：你现在只是“单个块”）
        if let Some(block_id) = inode.indirect_block.take() {
            self.data_bitmap.free(block_id);
            freed += 1;
        }

        // 3. double indirect（你目前还没用到，可以先占位）
        if let Some(block_id) = inode.double_indirect_block.take() {
            self.data_bitmap.free(block_id);
            freed += 1;
        }

        // 4. 更新 inode
        inode.size = 0;

        // 注意：mtime 在 write_file 里更新
        // ctime 不变（内容变化不算元数据变化）

        // 5. 更新超级块
        self.super_block.free_blocks += freed;
        self.super_block.dirty = true;

        Ok(())
    }

    pub fn truncate_file(&mut self, inode_id: u64) -> Result<(), String> {
        self.free_file_blocks(inode_id)?;

        if let Some(inode) = self.inode_table.get_inode_mut(inode_id) {
            inode.size = 0;
            inode.mtime = current_timestamp();
        }
        Ok(())
    }

    fn create_file_from_path(&mut self, path: &str) -> Result<u64, String> {
        let (parent, name) = split_path(path)?;
        self.create_file(parent, name)
    }
}
