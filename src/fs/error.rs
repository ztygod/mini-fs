use std::fmt;

/// 文件系统错误类型
#[derive(Debug)]
pub enum FileSystemError {
    Io(std::io::Error),        // 底层 I/O 错误
    DiskFull,                  // 磁盘已满
    InodeFull,                 // inode 已满
    NotFound(String),          // 文件或目录不存在，带路径
    AlreadyExists(String),     // 文件或目录已存在，带路径
    NotADirectory(String),     // 期望目录，实际不是
    IsADirectory(String),      // 期望文件，实际是目录
    DirectoryNotEmpty(String), // 目录非空
    InvalidPath(String),       // 路径非法
    InvalidInode(u32),         // inode 无效
    Corrupted(String),         // 文件系统损坏
                               // 可以继续扩展其他错误类型
}

impl From<std::io::Error> for FileSystemError {
    fn from(e: std::io::Error) -> Self {
        FileSystemError::Io(e)
    }
}

// 实现 Display trait，用于打印错误信息
impl fmt::Display for FileSystemError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(e) => write!(f, "Disk I/O error: {}", e),
            Self::DiskFull => write!(f, "Disk space is full"),
            Self::InodeFull => write!(f, "No free inode available"),
            Self::NotFound(path) => write!(f, "File or directory not found: {}", path),
            Self::AlreadyExists(path) => write!(f, "File or directory already exists: {}", path),
            Self::NotADirectory(path) => write!(f, "Expected a directory, found a file: {}", path),
            Self::IsADirectory(path) => write!(f, "Expected a file, found a directory: {}", path),
            Self::DirectoryNotEmpty(path) => write!(f, "Directory is not empty: {}", path),
            Self::InvalidPath(path) => write!(f, "Invalid path: {}", path),
            Self::InvalidInode(inode) => write!(f, "Invalid inode: {}", inode),
            Self::Corrupted(desc) => write!(f, "File system corrupted: {}", desc),
        }
    }
}

// 支持链式错误，方便追踪底层原因
impl std::error::Error for FileSystemError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            _ => None,
        }
    }
}

/// 文件系统统一结果类型
pub type Result<T> = std::result::Result<T, FileSystemError>;
