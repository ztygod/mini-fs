pub mod block_device;
pub mod file_disk;
pub mod init;
pub mod types;

// 对外导出常用类型，便于上层使用
pub use block_device::BlockDevice;
pub use file_disk::FileDisk;
pub use init::perform_disk_initialization;
pub use types::{Block, BLOCK_COUNT, BLOCK_SIZE, DISK_SIZE};

// src/disk/mod.rs 底部添加
#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{mpsc::channel, Arc};

    #[test]
    fn test_file_disk_read_write() {
        // 创建进度通道（模拟 UI 接收 BootProgress）
        let (tx, rx) = channel();

        // 初始化虚拟磁盘（路径可自定义）
        let disk = Arc::new(FileDisk::new("test_disk.img", &tx).unwrap());

        // 读取异步进度信息（非必须）
        while let Ok(msg) = rx.try_recv() {
            println!("{:?}", msg);
        }

        // 准备一个写入缓冲区（Block 大小为 4KB）
        let mut write_buf: Block = [0u8; BLOCK_SIZE];
        let content = b"hello tiny fs";
        write_buf[..content.len()].copy_from_slice(content);

        // 写入第 0 号块
        disk.write_block(0, &write_buf).unwrap();

        // 读取回来验证
        let mut read_buf: Block = [0u8; BLOCK_SIZE];
        disk.read_block(0, &mut read_buf).unwrap();

        // 转换为字符串并检查是否一致
        let read_str = String::from_utf8_lossy(&read_buf[..content.len()]);
        assert_eq!(read_str, "hello tiny fs");

        println!("✅ Disk read/write test passed! Read: {}", read_str);
    }
}
