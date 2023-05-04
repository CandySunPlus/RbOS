use std::env;
use std::fs::{read_dir, File};
use std::io::{Read, Result, Write};
use std::path::PathBuf;

static BASE_ADDRESS: usize = 0x80400000;
static STEP: usize = 0x20000;

static LINK_SCRIPT: &str = "src/linker.ld";

fn main() {
    println!("cargo:rerun-if-changed=src");
    set_app_link_script().unwrap();
}

fn set_app_link_script() -> Result<()> {
    let dst = PathBuf::from(env::var("OUT_DIR").unwrap());

    let mut linker_temp = File::open(LINK_SCRIPT)?;

    let mut linker_temp_str = String::new();

    linker_temp.read_to_string(&mut linker_temp_str)?;

    let mut apps = read_dir("src/bin")
        .unwrap()
        .map(|dir_entry| {
            let mut name_with_ext = dir_entry.unwrap().file_name().into_string().unwrap();
            name_with_ext.drain(name_with_ext.find('.').unwrap()..name_with_ext.len());
            name_with_ext
        })
        .collect::<Vec<_>>();

    apps.sort();

    for (app_id, app) in apps.iter().enumerate() {
        let linker_content = linker_temp_str.replace(
            format!("{:x}", BASE_ADDRESS).as_str(),
            format!("{:x}", BASE_ADDRESS + STEP * app_id).as_str(),
        );

        let linker_file_path = dst.join(format!("linker_{app}.ld"));
        let mut linker_file = File::create(&linker_file_path)?;
        linker_file.write_all(linker_content.as_bytes())?;

        println!(
            "cargo:rustc-link-arg-bin={app}=-T{}",
            linker_file_path.to_str().unwrap()
        );
    }

    Ok(())
}
