use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// 目录项类型：文件或子目录
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum DirEntryType {
    File,
    Directory,
}

// 一个目录项：文件名 → inode_index + 类型
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DirEntry {
    pub name: String,
    pub inode_index: usize,
    pub entry_type: DirEntryType,
}

// 目录结构
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Directory {
    pub inode_index: usize,     // 该目录对应的 inode
    pub entries: Vec<DirEntry>, // 保持顺序的目录项列表
    #[serde(skip)] // 不序列化，用于快速索引
    pub index_map: HashMap<String, usize>, // name -> entries 索引
}

impl Directory {
    // 新建目录
    pub fn new(inode_index: usize) -> Self {
        Self {
            inode_index,
            entries: Vec::new(),
            index_map: HashMap::new(),
        }
    }

    // 初始化 index_map（加载后使用）
    pub fn rebuild_index_map(&mut self) {
        self.index_map.clear();
        for (i, entry) in self.entries.iter().enumerate() {
            self.index_map.insert(entry.name.clone(), i);
        }
    }

    // 添加目录项
    pub fn add(
        &mut self,
        inode_index: usize,
        name: &str,
        entry_type: DirEntryType,
    ) -> Result<(), String> {
        if self.index_map.contains_key(name) {
            return Err(format!("Entry '{}' already exists", name));
        }

        self.entries.push(DirEntry {
            name: name.to_string(),
            inode_index,
            entry_type,
        });
        self.index_map
            .insert(name.to_string(), self.entries.len() - 1);
        Ok(())
    }

    // 删除目录项，返回 inode_index
    pub fn remove(&mut self, name: &str) -> Option<usize> {
        if let Some(&idx) = self.index_map.get(name) {
            let entry = self.entries.remove(idx);
            self.rebuild_index_map();
            Some(entry.inode_index)
        } else {
            None
        }
    }

    // 查找目录项，返回 inode_index
    pub fn find(&self, name: &str) -> Option<usize> {
        self.index_map
            .get(name)
            .map(|&idx| self.entries[idx].inode_index)
    }

    // 列出所有文件名
    pub fn list(&self) -> Vec<String> {
        self.entries.iter().map(|e| e.name.clone()).collect()
    }

    // 判断是否为目录
    pub fn is_directory(&self, name: &str) -> Option<bool> {
        self.index_map
            .get(name)
            .map(|&idx| matches!(self.entries[idx].entry_type, DirEntryType::Directory))
    }
}
