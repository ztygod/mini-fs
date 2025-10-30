use std::io::Result;

use crate::disk::types::Block;

pub trait BlockDevice: Send + Sync {
    fn read_block(&self, block_id: u64, buf: &mut Block) -> Result<()>;
    fn write_block(&self, block_id: u64, buf: &Block) -> Result<()>;
}
