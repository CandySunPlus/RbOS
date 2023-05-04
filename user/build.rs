use std::env;
use std::fs::{read_dir, File};
use std::io::{BufReader, BufWriter, Read, Result, Write};
use std::path::Path;

static BASE_ADDRESS: usize = 0x80400000;
static STEP: usize = 0x20000;

static LINK_SCRIPT: &str = "src/linker.ld";

fn main() {
    println!("cargo:rerun-if-changed=src");
    set_app_link_script().unwrap();
}

fn set_app_link_script() -> Result<()> {
    let out_dir = env::var("OUT_DIR").unwrap();
    let dst = Path::new(&out_dir);

    let mut linker_temp = BufReader::new(File::open(LINK_SCRIPT)?);
    let mut linker_temp_str = String::new();

    linker_temp.read_to_string(&mut linker_temp_str)?;

    read_dir("src/bin")?
        .map(|dir_entry| {
            let mut name_with_ext = dir_entry.unwrap().file_name().to_string_lossy().to_string();
            name_with_ext.drain(name_with_ext.find('.').unwrap()..);
            name_with_ext
        })
        .enumerate()
        .try_for_each(|(app_id, app)| {
            let linker_content = linker_temp_str.replace(
                &format!("{:x}", BASE_ADDRESS),
                &format!("{:x}", BASE_ADDRESS + STEP * app_id),
            );

            let linker_file_path = dst.join(format!("linker_{}.ld", app));
            let mut linker_file = BufWriter::new(File::create(&linker_file_path)?);
            linker_file.write_all(linker_content.as_bytes())?;

            println!(
                "cargo:rustc-link-arg-bin={app}=-T{}",
                linker_file_path.to_str().unwrap(),
                app = app
            );

            Ok(())
        })
}
