use std::{fs::OpenOptions, sync::mpsc::Sender, thread, time::Duration};

use crate::{fs::FileSystem, shell::BootProgress};

pub fn perform_disk_initialization(tx: Sender<BootProgress>) {
    // 定义磁盘参数
    const DISK_PATH: &str = "disk.img";
    const TOTAL_BLOCKS: u64 = 4096;
    const BLOCK_SIZE: u64 = 4 * 1024; // 4 KB

    const DISK_SIZE: u64 = BLOCK_SIZE * TOTAL_BLOCKS; // 4KB * 4096 = 16MB

    // 初始化虚拟磁盘
    tx.send(BootProgress::Step("🧠 Initializing virtual disk..."))
        .unwrap();

    // 创建文件
    let file_result = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(DISK_PATH);

    let file = match file_result {
        Ok(f) => f,
        Err(e) => {
            tx.send(BootProgress::Finished(Err(Box::new(e)))).unwrap();
            return;
        }
    };

    // 如果文件是新创建的，需要设置其大小，这个过程可以绑定到进度条
    if file.metadata().unwrap().len() < DISK_SIZE {
        // 将创建文件的过程与进度条的前 50% 绑定
        file.set_len(DISK_SIZE).unwrap(); // 顶分配空间
        for i in 0..50 {
            tx.send(BootProgress::Progress(i)).unwrap();
            thread::sleep(Duration::from_millis(5)); // 模拟耗时
        }
    } else {
        // 如果文件已存在，直接跳过这部分进度
        tx.send(BootProgress::Progress(50)).unwrap();
    }

    // 第二阶段：挂载文件系统
    tx.send(BootProgress::Step("⚙️ Mounting file system..."))
        .unwrap();

    // 将挂载/格式化的过程与进度条的后 50% 绑定
    // let mount_result = FileSystem::mount(DISK_PATH, TOTAL_BLOCKS, &tx);

    // 无论挂载成功与否，都将最终结果发送回去
    // tx.send(BootProgress::Finished(mount_result)).unwrap();
}
