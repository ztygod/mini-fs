use std::sync::mpsc::Sender;

use crate::{disk::file_disk::FileDisk, shell::BootProgress};

pub fn perform_disk_initialization(tx: Sender<BootProgress>) {
    const DISK_PATH: &str = "disk.img";

    tx.send(BootProgress::Step("🧠 Initializing virtual disk..."))
        .unwrap();

    // 初始化 FileDisk
    let _disk = match FileDisk::new(DISK_PATH, &tx) {
        Ok(d) => d,
        Err(e) => {
            tx.send(BootProgress::Finished(Err(Box::new(e)))).unwrap();
            return;
        }
    };

    tx.send(BootProgress::Step("⚙️ Mounting file system..."))
        .unwrap();

    // 在此挂载文件系统
    // let fs = FileSystem::mount(Box::new(disk));
    // fs.init_root_directory();

    for i in 50..=100 {
        tx.send(BootProgress::Progress(i)).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(5));
    }

    // tx.send(BootProgress::Finished(Ok(()))).unwrap();
}
