use crate::{disk::file_disk::FileDisk, fs::FileSystem, shell::BootProgress};
use std::sync::mpsc::Sender;

pub fn perform_disk_initialization(tx: Sender<BootProgress>) {
    const DISK_PATH: &str = "disk.img";

    tx.send(BootProgress::Step("🧠 Initializing virtual disk..."))
        .unwrap();

    let disk_exists = std::path::Path::new(DISK_PATH).exists();

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

    let mut fs = FileSystem::new(disk);

    if !disk_exists {
        // 只有“明确是新磁盘”才格式化
        tx.send(BootProgress::Step(
            "🔧 No disk found, formatting new file system...",
        ))
        .unwrap();

        if let Err(e) = fs.format() {
            tx.send(BootProgress::Finished(Err(Box::new(e)))).unwrap();
            return;
        }
    }

    // 不论是否新盘，最终都要 mount
    if let Err(e) = fs.mount() {
        tx.send(BootProgress::Finished(Err(Box::new(e)))).unwrap();
        return;
    }

    for i in 50..=100 {
        tx.send(BootProgress::Progress(i)).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(5));
    }

    tx.send(BootProgress::Finished(Ok(fs))).unwrap();
}
