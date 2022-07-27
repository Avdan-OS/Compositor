use std::fs;
use std::io::BufReader;
use std::error::Error;

use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct Config {
    test: String
}

pub fn read_config() -> Result<Config, Box<dyn Error>> {
    fs::create_dir_all("/etc/AvdanOS")
        .expect("Error while create AvdanOS config directory");

    let file = fs::OpenOptions::new().read(true).write(true).create(true)
        .open("/etc/AvdanOS/Compositor.json")?;
    let reader = BufReader::new(file);
    let u = serde_json::from_reader(reader)?;
    Ok(u)
}

