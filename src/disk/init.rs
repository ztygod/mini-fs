use crate::{disk::file_disk::FileDisk, fs::FileSystem, shell::BootProgress};
use std::sync::mpsc::Sender;

pub fn perform_disk_initialization(tx: Sender<BootProgress>) {
    const DISK_PATH: &str = "disk.img";

    tx.send(BootProgress::Step("🧠 Initializing virtual disk..."))
        .unwrap();

    // 初始化 FileDisk
    let disk = match FileDisk::new(DISK_PATH, &tx) {
        Ok(d) => d,
        Err(e) => {
            tx.send(BootProgress::Finished(Err(Box::new(e)))).unwrap();
            return;
        }
    };

    tx.send(BootProgress::Step("⚙️ Mounting file system..."))
        .unwrap();

    // 创建 FileSystem 实例
    let mut fs = FileSystem::new(disk);

    // 尝试挂载，如果失败则格式化
    if let Err(_) = fs.mount() {
        tx.send(BootProgress::Step("🔧 Formatting new file system..."))
            .unwrap();

        if let Err(e) = fs.format() {
            tx.send(BootProgress::Finished(Err(Box::new(e)))).unwrap();
            return;
        }

        // 格式化完成后再挂载一次，保证内存对象同步
        if let Err(e) = fs.mount() {
            tx.send(BootProgress::Finished(Err(Box::new(e)))).unwrap();
            return;
        }
    }

    for i in 50..=100 {
        tx.send(BootProgress::Progress(i)).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(5));
    }

    // 返回 FileSystem 实例
    tx.send(BootProgress::Finished(Ok(fs))).unwrap();
}
