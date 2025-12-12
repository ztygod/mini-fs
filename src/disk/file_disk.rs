use std::{
    fs::{File, OpenOptions},
    io::{Read, Result, Seek, SeekFrom, Write},
    sync::{mpsc::Sender, Mutex},
    thread,
    time::Duration,
};

use crate::{
    disk::{
        block_device::BlockDevice,
        types::{Block, BLOCK_SIZE, DISK_SIZE},
    },
    shell::BootProgress,
};
#[derive(Debug)]
pub struct FileDisk {
    file: Mutex<File>,
}

impl FileDisk {
    pub fn new(path: &str, tx: &Sender<BootProgress>) -> Result<Self> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(path)?;

        if file.metadata()?.len() < DISK_SIZE {
            tx.send(BootProgress::Step("🪶 Allocating disk space..."))
                .unwrap();

            file.set_len(DISK_SIZE)?;

            for i in 0..=100 {
                let _ = tx.send(BootProgress::Progress(i));
                thread::sleep(Duration::from_millis(20));
            }
        } else {
            tx.send(BootProgress::Progress(100)).unwrap();
        }

        Ok(Self {
            file: Mutex::new(file),
        })
    }
}

impl BlockDevice for FileDisk {
    fn read_block(&self, block_id: u64, buf: &mut Block) -> std::io::Result<()> {
        let mut file = self.file.lock().unwrap();
        file.seek(SeekFrom::Start(block_id * BLOCK_SIZE as u64))?;
        file.read_exact(buf)?;
        Ok(())
    }

    fn write_block(&self, block_id: u64, buf: &Block) -> std::io::Result<()> {
        let mut file = self.file.lock().unwrap();
        file.seek(SeekFrom::Start(block_id * BLOCK_SIZE as u64))?;
        file.write_all(buf)?;
        Ok(())
    }
}
