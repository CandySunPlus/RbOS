use alloc::sync::Arc;
use core::fmt;

use crate::block_cache::get_block_cache;
use crate::block_dev::BlockDevice;
use crate::BLOCK_SIZE;

const EFS_MAGIC: u32 = 0x3b800001;
const INODE_DIRECT_COUNT: usize = 28;
const DIRECT_BOUND: usize = INODE_DIRECT_COUNT;
const INODE_INDIRECT1_COUNT: usize = BLOCK_SIZE / 4;
const INDIRECT1_BOUND: usize = DIRECT_BOUND + INODE_INDIRECT1_COUNT;

#[repr(C)]
pub struct SuperBlock {
    magic: u32,
    pub total_blocks: u32,
    pub inode_bitmap_blocks: u32,
    pub inode_area_blocks: u32,
    pub data_bitmap_blocks: u32,
    pub data_area_blocks: u32,
}

impl fmt::Debug for SuperBlock {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SuperBlock")
            .field("total_blocks", &self.total_blocks)
            .field("inode_bitmap_blocks", &self.inode_bitmap_blocks)
            .field("inode_area_blocks", &self.inode_area_blocks)
            .field("data_bitmap_blocks", &self.data_bitmap_blocks)
            .field("data_area_blocks", &self.data_area_blocks)
            .finish()
    }
}

impl SuperBlock {
    pub fn initialize(
        &mut self,
        total_blocks: u32,
        inode_bitmap_blocks: u32,
        inode_area_blocks: u32,
        data_bitmap_blocks: u32,
        data_area_blocks: u32,
    ) {
        *self = Self {
            magic: EFS_MAGIC,
            total_blocks,
            inode_bitmap_blocks,
            inode_area_blocks,
            data_bitmap_blocks,
            data_area_blocks,
        };
    }

    pub fn is_valid(&self) -> bool {
        self.magic == EFS_MAGIC
    }
}

#[derive(PartialEq)]
pub enum DiskInodeType {
    File,
    Directory,
}

type IndirectBlock = [u32; BLOCK_SIZE / 4];
type DataBlock = [u8; BLOCK_SIZE];

#[repr(C)]
pub struct DiskInode {
    pub size: u32,
    pub direct: [u32; INODE_DIRECT_COUNT],
    pub indirect1: u32,
    pub indirect2: u32,
    type_: DiskInodeType,
}

impl DiskInode {
    /// Initialize a disk inode, as well as all direct inodes under it indirect1 and indirect2
    /// block are allocated only when they are needed.
    pub fn new(type_: DiskInodeType) -> Self {
        Self {
            size: 0,
            direct: [0; INODE_DIRECT_COUNT],
            indirect1: 0,
            indirect2: 0,
            type_,
        }
    }

    /// Whether the inode is a directory.
    pub fn is_dir(&self) -> bool {
        self.type_ == DiskInodeType::Directory
    }

    /// Whether the inode is a file.
    pub fn is_file(&self) -> bool {
        self.type_ == DiskInodeType::File
    }

    /// Get the block id of the inner block with the given inner id.
    pub fn get_block_id(&self, inner_id: u32, block_device: &Arc<dyn BlockDevice>) -> u32 {
        let inner_id = inner_id as usize;
        if inner_id < INODE_DIRECT_COUNT {
            self.direct[inner_id]
        } else if inner_id < INDIRECT1_BOUND {
            get_block_cache(self.indirect1 as usize, block_device.clone())
                .lock()
                .read(0, |indirect_block: &IndirectBlock| {
                    indirect_block[inner_id - INODE_DIRECT_COUNT]
                })
        } else {
            let last = inner_id - INDIRECT1_BOUND;
            let indirect1 = get_block_cache(self.indirect2 as usize, block_device.clone())
                .lock()
                .read(0, |indirect2: &IndirectBlock| {
                    indirect2[last / INODE_INDIRECT1_COUNT]
                });
            get_block_cache(indirect1 as usize, block_device.clone())
                .lock()
                .read(0, |indirect1: &IndirectBlock| {
                    indirect1[last % INODE_INDIRECT1_COUNT]
                })
        }
    }

    /// Return block number correspond to size.
    pub fn data_blocks(&self) -> u32 {
        Self::_data_blocks(self.size)
    }

    fn _data_blocks(size: u32) -> u32 {
        (size + BLOCK_SIZE as u32 - 1) / BLOCK_SIZE as u32
    }

    /// Return number of blocks needed include indriect1 and indirect2.
    pub fn total_blocks(size: u32) -> u32 {
        let data_blocks = Self::_data_blocks(size) as usize;
        let mut total = data_blocks as usize;
        if data_blocks > INODE_DIRECT_COUNT {
            total += 1;
        }

        if data_blocks > INDIRECT1_BOUND {
            total += 1;
            total +=
                (data_blocks - INDIRECT1_BOUND + INODE_INDIRECT1_COUNT - 1) / INODE_INDIRECT1_COUNT;
        }

        total as u32
    }

    /// Get the number of data blocks that have to be allocated given the new size.
    pub fn block_num_needed(&self, new_size: u32) -> u32 {
        Self::total_blocks(new_size) - Self::total_blocks(self.size)
    }
}
