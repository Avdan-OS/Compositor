use std::fs;
use std::io::BufReader;
use std::error::Error;

use serde::Deserialize;

use json_comments::{CommentSettings, StripComments};

use crate::CONST::{CONFIG_FOLDER, CONFIG_FILE};

#[derive(Deserialize, Debug)]
pub struct Config {
    test: String
}

impl Config {
    pub fn from_file() -> Result<Config, Box<dyn Error>> {
        fs::create_dir_all(*CONFIG_FOLDER)
            .expect("Error while creating the AvdanOS config directory!");
    
        let file = fs::OpenOptions::new()
            .read(true).write(true).create(true)
            .open(CONFIG_FOLDER.join(*CONFIG_FILE))?;
            
        let reader = BufReader::new(file);

        let stripped = StripComments::new(reader);

        let parsed = serde_json::from_reader(stripped)?;
        Ok(parsed)
    }
}


