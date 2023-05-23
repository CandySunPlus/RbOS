use alloc::sync::Arc;
use core::mem;

use spin::Mutex;

use crate::bitmap::Bitmap;
use crate::block_cache::{block_cache_sync_all, get_block_cache};
use crate::block_dev::BlockDevice;
use crate::layout::{DataBlock, DiskInode, DiskInodeType, SuperBlock};
use crate::vfs::Inode;
use crate::BLOCK_SZ;

pub struct EasyFileSystem {
    pub block_device: Arc<dyn BlockDevice>,
    pub inode_bitmap: Bitmap,
    pub data_bitmap: Bitmap,
    inode_area_start_block: u32,
    data_area_start_block: u32,
}

impl EasyFileSystem {
    pub fn create(
        block_device: Arc<dyn BlockDevice>,
        total_blocks: u32,
        inode_bitmap_blocks: u32,
    ) -> Arc<Mutex<Self>> {
        let inode_bitmap = Bitmap::new(1, inode_bitmap_blocks as usize);
        let inode_num = inode_bitmap.maximum();

        let inode_area_blocks =
            ((inode_num * mem::size_of::<DiskInode>() + BLOCK_SZ - 1) / BLOCK_SZ) as u32;

        let inode_total_blocks = inode_area_blocks + inode_bitmap_blocks;
        let data_total_blocks = total_blocks - inode_total_blocks - 1;

        let data_bitmap_blocks = (data_total_blocks + 4096) / 4097;
        let data_area_blocks = data_total_blocks - data_bitmap_blocks;

        let data_bitmap = Bitmap::new(
            (1 + inode_total_blocks + inode_area_blocks) as usize,
            data_bitmap_blocks as usize,
        );

        let efs = Self {
            block_device: block_device.clone(),
            inode_bitmap,
            data_bitmap,
            inode_area_start_block: 1 + inode_bitmap_blocks,
            data_area_start_block: 1 + inode_total_blocks + data_bitmap_blocks,
        };

        for i in 0..total_blocks {
            get_block_cache(i as usize, block_device.clone())
                .lock()
                .modify(0, |data_block: &mut DataBlock| {
                    for byte in data_block.iter_mut() {
                        *byte = 0;
                    }
                });
        }

        get_block_cache(0, block_device.clone()).lock().modify(
            0,
            |super_block: &mut SuperBlock| {
                super_block.initialize(
                    total_blocks,
                    inode_bitmap_blocks,
                    inode_area_blocks,
                    data_bitmap_blocks,
                    data_area_blocks,
                );
            },
        );

        assert_eq!(efs.alloc_inode(), 0);

        let (root_inode_block_id, root_inode_offset) = efs.get_disk_inode_pos(0);
        get_block_cache(root_inode_block_id as usize, block_device.clone())
            .lock()
            .modify(root_inode_offset, |disk_inode: &mut DiskInode| {
                disk_inode.initialize(DiskInodeType::Directory);
            });

        block_cache_sync_all();

        Arc::new(Mutex::new(efs))
    }

    pub fn open(block_device: Arc<dyn BlockDevice>) -> Arc<Mutex<Self>> {
        get_block_cache(0, block_device.clone())
            .lock()
            .read(0, |super_block: &SuperBlock| {
                assert!(super_block.is_valid(), "Error loading EFS!");

                let inode_total_blocks =
                    super_block.inode_bitmap_blocks + super_block.inode_area_blocks;
                let efs = Self {
                    block_device,
                    inode_bitmap: Bitmap::new(1, super_block.inode_bitmap_blocks as usize),
                    data_bitmap: Bitmap::new(
                        (1 + inode_total_blocks) as usize,
                        super_block.data_bitmap_blocks as usize,
                    ),
                    inode_area_start_block: 1 + super_block.inode_bitmap_blocks,
                    data_area_start_block: 1 + inode_total_blocks + super_block.data_bitmap_blocks,
                };
                Arc::new(Mutex::new(efs))
            })
    }

    pub fn get_disk_inode_pos(&self, inode_id: u32) -> (u32, usize) {
        let inode_size = mem::size_of::<DiskInode>();
        let inodes_pre_block = (BLOCK_SZ / inode_size) as u32;
        let block_id = self.inode_area_start_block + inode_id / inodes_pre_block;
        (
            block_id,
            (inode_id % inodes_pre_block) as usize * inode_size,
        )
    }

    pub fn get_data_block_id(&self, data_block_id: u32) -> u32 {
        self.data_area_start_block + data_block_id
    }

    pub fn alloc_inode(&self) -> u32 {
        self.inode_bitmap.alloc(&self.block_device).unwrap() as u32
    }

    pub fn alloc_data(&self) -> u32 {
        self.inode_bitmap.alloc(&self.block_device).unwrap() as u32 + self.data_area_start_block
    }

    pub fn dealloc_data(&self, block_id: u32) {
        get_block_cache(block_id as usize, self.block_device.clone())
            .lock()
            .modify(0, |data_block: &mut DataBlock| {
                data_block.iter_mut().for_each(|v| *v = 0);
            });
        self.data_bitmap.dealloc(
            &self.block_device,
            (block_id - self.data_area_start_block) as usize,
        )
    }

    pub fn root_inode(efs: &Arc<Mutex<Self>>) -> Inode {
        let block_device = efs.lock().block_device.clone();
        let (block_id, block_offset) = efs.lock().get_disk_inode_pos(0);

        Inode::new(block_id, block_offset, efs.clone(), block_device)
    }
}
