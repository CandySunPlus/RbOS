use alloc::borrow::ToOwned;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec;

use spin::{Mutex, MutexGuard};

use crate::block_cache::get_block_cache;
use crate::block_dev::BlockDevice;
use crate::efs::EasyFileSystem;
use crate::layout::{DirEntry, DiskInode, DiskInodeType, DIRENT_SIZE};

pub struct Inode {
    block_id: usize,
    block_offset: usize,
    fs: Arc<Mutex<EasyFileSystem>>,
    block_device: Arc<dyn BlockDevice>,
}

impl Inode {
    pub fn new(
        block_id: u32,
        block_offset: usize,
        fs: Arc<Mutex<EasyFileSystem>>,
        block_device: Arc<dyn BlockDevice>,
    ) -> Self {
        Self {
            block_id: block_id as usize,
            block_offset,
            fs,
            block_device,
        }
    }

    fn read_disk_inode<V>(&self, f: impl FnOnce(&DiskInode) -> V) -> V {
        get_block_cache(self.block_id, self.block_device.clone())
            .lock()
            .read(self.block_offset, f)
    }

    fn modify_disk_inode<V>(&self, f: impl FnOnce(&mut DiskInode) -> V) -> V {
        get_block_cache(self.block_id, self.block_device.clone())
            .lock()
            .modify(self.block_offset, f)
    }

    pub fn find(&self, name: &str) -> Option<Arc<Inode>> {
        let fs = self.fs.lock();
        self.read_disk_inode(|disk_inode| {
            self.find_inode_id(name, disk_inode).map(|inode_id| {
                let (block_id, block_offset) = fs.get_disk_inode_pos(inode_id);
                Arc::new(Inode::new(
                    block_id,
                    block_offset,
                    self.fs.clone(),
                    self.block_device.clone(),
                ))
            })
        })
    }

    pub fn find_inode_id(&self, name: &str, disk_inode: &DiskInode) -> Option<u32> {
        assert!(disk_inode.is_dir());
        let file_count = disk_inode.size as usize / DIRENT_SIZE;
        let mut dirent = DirEntry::empty();

        for i in 0..file_count {
            assert_eq!(
                disk_inode.read_at(DIRENT_SIZE * i, dirent.as_bytes_mut(), &self.block_device),
                DIRENT_SIZE
            );
            if dirent.name() == name {
                return Some(dirent.inode_number());
            }
        }

        None
    }

    /// List inodes from the current inode.
    pub fn ls(&self) -> vec::Vec<String> {
        let _fs = self.fs.lock();
        self.read_disk_inode(|disk_inode| {
            let file_count = disk_inode.size as usize / DIRENT_SIZE;
            let mut v = vec::Vec::new();

            for i in 0..file_count {
                let mut dirent = DirEntry::empty();

                assert_eq!(
                    disk_inode.read_at(DIRENT_SIZE * i, dirent.as_bytes_mut(), &self.block_device),
                    DIRENT_SIZE
                );
                v.push(dirent.name().to_owned());
            }
            v
        })
    }

    /// Increase the size of the inode.
    fn increase_size(
        &self,
        new_size: u32,
        disk_inode: &mut DiskInode,
        fs: &mut MutexGuard<EasyFileSystem>,
    ) {
        if new_size < disk_inode.size {
            return;
        }

        let blocks_needed = disk_inode.block_num_needed(new_size);
        let mut v = vec::Vec::new();
        for _ in 0..blocks_needed {
            v.push(fs.alloc_data());
        }

        disk_inode.increase_size(new_size, v, &self.block_device);
    }

    /// Create inode under the current inode with name.
    pub fn create(&self, name: &str) -> Option<Arc<Inode>> {
        let mut fs = self.fs.lock();

        let op = |root_inode: &DiskInode| {
            assert!(root_inode.is_dir());
            self.find_inode_id(name, root_inode)
        };
        if self.read_disk_inode(op).is_some() {
            return None;
        }

        let new_inode_id = fs.alloc_inode();
        let (new_inode_block_id, new_inode_block_offset) = fs.get_disk_inode_pos(new_inode_id);

        get_block_cache(new_inode_block_id as usize, self.block_device.clone())
            .lock()
            .modify(new_inode_block_offset, |new_inode: &mut DiskInode| {
                new_inode.initialize(DiskInodeType::File);
            });

        self.modify_disk_inode(|root_inode| {
            let file_count = root_inode.file_count();
            let new_size = (file_count + 1) * DIRENT_SIZE;
            self.increase_size(new_size as u32, root_inode, &mut fs);

            let dirent = DirEntry::new(name, new_inode_id);
            root_inode.write_at(
                file_count * DIRENT_SIZE,
                dirent.as_bytes(),
                &self.block_device,
            );
        });

        let (block_id, block_offset) = fs.get_disk_inode_pos(new_inode_id);

        Some(Arc::new(Self::new(
            block_id,
            block_offset,
            self.fs.clone(),
            self.block_device.clone(),
        )))
    }

    /// Clear the data in the inode.
    pub fn clear(&self) {
        let fs = self.fs.lock();
        self.modify_disk_inode(|disk_inode| {
            let size = disk_inode.size;
            let data_blocks_dealloc = disk_inode.clear_size(&self.block_device);
            assert!(data_blocks_dealloc.len() == DiskInode::total_blocks(size) as usize);
            for data_block in data_blocks_dealloc.into_iter() {
                fs.dealloc_data(data_block);
            }
        });
    }

    /// Read data from the inode.
    pub fn read_at(&self, offset: usize, buf: &mut [u8]) -> usize {
        let _fs = self.fs.lock();
        self.read_disk_inode(|disk_inode| disk_inode.read_at(offset, buf, &self.block_device))
    }

    /// Write data to the inode.
    pub fn write_at(&self, offset: usize, buf: &[u8]) -> usize {
        let mut fs = self.fs.lock();
        self.modify_disk_inode(|disk_inode| {
            self.increase_size((offset + buf.len()) as u32, disk_inode, &mut fs);
            disk_inode.write_at(offset, buf, &self.block_device)
        })
    }
}