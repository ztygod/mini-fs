use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// 目录项类型
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub enum DirEntryType {
    File,
    Directory,
}

// 一个目录项
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DirEntry {
    pub name: String,
    pub inode_index: usize,
    pub entry_type: DirEntryType,
}

// 目录结构
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Directory {
    pub inode_index: usize,
    pub entries: Vec<DirEntry>,
    #[serde(skip)]
    pub index_map: HashMap<String, usize>, // name -> entries 索引
}

impl Directory {
    pub fn new(inode_index: usize) -> Self {
        Self {
            inode_index,
            entries: Vec::new(),
            index_map: HashMap::new(),
        }
    }

    // 从字节数组加载目录，自动重建 index_map
    pub fn load_from_bytes(bytes: &[u8]) -> Result<Self, Box<dyn std::error::Error>> {
        let mut dir: Directory = bincode::deserialize(bytes)?;
        dir.rebuild_index_map();
        Ok(dir)
    }

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
        println!("Directory: {:?}", self);
        self.index_map
            .get(name)
            .map(|&idx| self.entries[idx].inode_index)
    }

    pub fn get(&self, name: &str) -> Option<&DirEntry> {
        self.index_map.get(name).map(|&idx| &self.entries[idx])
    }

    pub fn list_sorted(&self) -> Vec<String> {
        let mut entries = self.entries.clone();
        entries.sort_by(|a, b| match (&a.entry_type, &b.entry_type) {
            (DirEntryType::Directory, DirEntryType::File) => std::cmp::Ordering::Less,
            (DirEntryType::File, DirEntryType::Directory) => std::cmp::Ordering::Greater,
            _ => a.name.cmp(&b.name),
        });
        entries.into_iter().map(|e| e.name).collect()
    }
}
