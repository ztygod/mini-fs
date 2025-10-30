/// 每个逻辑块（Block）的大小：4KB
/// 文件系统以“块”为最小读写单位。
pub const BLOCK_SIZE: usize = 4096;

/// 磁盘中包含的块总数：64MB / 4KB = 16384 块
/// 即磁盘被划分为 16384 个逻辑块。
pub const BLOCK_COUNT: usize = 64 * 1024 * 1024 / 4096;

/// 虚拟磁盘总大小（单位：字节）
/// 用于创建固定大小的 disk.img 文件。
pub const DISK_SIZE: u64 = (BLOCK_SIZE * BLOCK_COUNT) as u64;

/// 定义一个逻辑块类型（每块 4KB 的字节数组）
/// 所有磁盘读写都以 Block 为单位进行。
pub type Block = [u8; BLOCK_SIZE];
