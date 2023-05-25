use std::fs::{read_dir, File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::sync::{Arc, Mutex};

use clap::Parser;
use easy_fs::{BlockDevice, EasyFileSystem};

const BLOCK_SZ: usize = 512;

struct BlockFile(Mutex<File>);

impl BlockDevice for BlockFile {
    fn read_block(&self, block_id: usize, buf: &mut [u8]) {
        let mut file = self.0.lock().unwrap();
        file.seek(SeekFrom::Start((block_id * BLOCK_SZ) as u64))
            .expect("Error when seeking!");
        assert_eq!(file.read(buf).unwrap(), BLOCK_SZ, "Not a complete block!");
    }

    fn write_block(&self, block_id: usize, buf: &[u8]) {
        let mut file = self.0.lock().unwrap();
        file.seek(SeekFrom::Start((block_id * BLOCK_SZ) as u64))
            .expect("Error when seeking!");
        assert_eq!(file.write(buf).unwrap(), BLOCK_SZ, "Not a complete block!");
    }
}

fn main() {
    easy_fs_pack().expect("Error when packing easy-fs");
}

#[derive(Parser)]
#[command(name = "EasyFileSystem Packer")]
#[command(version = "1.0")]
struct Cli {
    /// Excutable source dir(with backslash)
    #[arg(short, long)]
    source: String,
    /// Excutable target dir(with backslash)
    #[arg(short, long)]
    target: String,
}

fn easy_fs_pack() -> io::Result<()> {
    let cli = Cli::parse();
    let source_path = cli.source;
    let target_path = cli.target;

    println!("source_path = {source_path}\ntarget_path = {target_path}");

    let block_file = Arc::new(BlockFile(Mutex::new({
        let f = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(format!("{target_path}fs.img"))?;
        f.set_len(16 * 2048 * 512)?;
        f
    })));

    let efs = EasyFileSystem::create(block_file, 16 * 2048, 1);
    let root_inode = Arc::new(EasyFileSystem::root_inode(&efs));
    let apps = read_dir(source_path)?
        .map(|dir_entry| {
            let mut name_with_ext = dir_entry.unwrap().file_name().into_string().unwrap();
            name_with_ext.drain(name_with_ext.find('.').unwrap()..name_with_ext.len());
            name_with_ext
        })
        .collect::<Vec<_>>();

    for app in apps {
        let mut host_file = File::open(format!("{target_path}{app}")).unwrap();
        let mut all_data = Vec::new();
        host_file.read_to_end(&mut all_data).unwrap();
        let inode = root_inode.create(app.as_str()).unwrap();
        inode.write_at(0, all_data.as_slice());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use rand;

    use super::*;

    #[test]
    fn efs_test() -> io::Result<()> {
        let block_file = Arc::new(BlockFile(Mutex::new({
            let f = OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .open("target/fs.img")?;
            f.set_len(8192 * 512)?;
            f
        })));

        EasyFileSystem::create(block_file.clone(), 4096, 1);
        let efs = EasyFileSystem::open(block_file.clone());
        let root_inode = EasyFileSystem::root_inode(&efs);
        root_inode.create("filea");

        let filea = root_inode.find("filea").unwrap();
        let greet_str = "Hello, world!";
        filea.write_at(0, greet_str.as_bytes());

        let mut buffer = [0u8; 233];
        let len = filea.read_at(0, &mut buffer);

        assert_eq!(greet_str, core::str::from_utf8(&buffer[..len]).unwrap());

        let fileb = root_inode.create("fileb").unwrap();
        fileb.write_at(0, "This is file b!".as_bytes());

        for name in root_inode.ls() {
            println!("{name}");
        }

        let mut random_str_test = |len: usize| {
            filea.clear();
            assert_eq!(filea.read_at(0, &mut buffer), 0);
            let mut str = String::new();
            for _ in 0..len {
                str.push(char::from('0' as u8 + rand::random::<u8>() % 10));
            }
            filea.write_at(0, str.as_bytes());
            let mut read_buffer = [0u8; 127];
            let mut offset = 0usize;
            let mut read_str = String::new();
            loop {
                let len = filea.read_at(offset, &mut read_buffer);
                if len == 0 {
                    break;
                }
                assert_eq!(&str.as_bytes()[offset..offset + len], &read_buffer[..len]);
                offset += len;
                read_str.push_str(core::str::from_utf8(&read_buffer[..len]).unwrap());
            }
            assert_eq!(str, read_str);
        };

        random_str_test(4 * BLOCK_SZ);
        random_str_test(8 * BLOCK_SZ + BLOCK_SZ / 2);
        random_str_test(157 * BLOCK_SZ);
        random_str_test(70 * BLOCK_SZ + BLOCK_SZ / 7);
        random_str_test((12 + 128) * BLOCK_SZ);
        random_str_test(280 * BLOCK_SZ);
        random_str_test(1000 * BLOCK_SZ);
        random_str_test(2000 * BLOCK_SZ);

        Ok(())
    }
}
