use alloc::boxed::Box;
use devices::get_blk_device;
use vfscore::BlockDevice;

pub struct BlockDev(usize);
impl BlockDevice for BlockDev {
    fn read_block(&self, block: usize, buffer: &mut [u8]) -> vfscore::VfsResult<usize> {
        get_blk_device(self.0).unwrap().read_blocks(block, buffer);
        Ok(buffer.len())
    }

    fn write_block(&self, block: usize, buffer: &[u8]) -> vfscore::VfsResult<usize> {
        get_blk_device(self.0).unwrap().write_blocks(block, buffer);
        Ok(buffer.len())
    }

    fn capacity(&self) -> vfscore::VfsResult<u64> {
        Ok(get_blk_device(0).unwrap().capacity() as _)
    }
}

#[inline]
pub fn get_block_dev(id: usize) -> Box<dyn BlockDevice> {
    Box::new(BlockDev(id))
}
