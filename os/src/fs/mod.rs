use crate::mm::UserBuffer;

mod inode;
pub mod stdio;

pub use inode::{list_apps, open_file, OSInode, OpenFlags};

pub trait File: Send + Sync {
    fn readable(&self) -> bool;
    fn writable(&self) -> bool;
    fn read(&self, buf: UserBuffer) -> usize;
    fn write(&self, buf: UserBuffer) -> usize;
}
