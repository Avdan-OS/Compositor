use std::{
    error::Error, 
    fs::{self, File},
    io::BufReader,
    collections::HashMap
};

use lazy_static::lazy_static;


pub(crate) use json_comments::StripComments;

use json_tree::{Index,};
use serde::Deserialize;

use crate::{CONST::{
    CONFIG_FOLDER,
    CONFIG_FILE,
}, config::errors::UnexpectedToken};

use super::sections::{keybinds::Keybinds};

lazy_static! {
    pub static ref PATH : String = CONFIG_FOLDER.join(*CONFIG_FILE).to_string_lossy().to_string();
}
static mut INDEX : Option<Index> = None;

static mut CONFIG : Option<Config> = None;

#[derive(Deserialize, Debug)]
pub struct Config {
    pub keybinds: Keybinds
}

impl Config {
    
    pub fn path() -> String {
        PATH.to_string()
    }

    ///
    /// Returns the config file's JSON index.
    /// 
    pub fn index<'a>() -> &'a Index {
        unsafe {
            INDEX.as_ref().unwrap()
        }
    }

    ///
    /// Returns the Global Configuration Object.
    /// 
    pub fn config<'a>() -> &'a Self {
        unsafe {
            CONFIG.as_ref().unwrap()
        }
    }


    ///
    /// Loads the config.
    /// 
    /// THIS FUNCTION SHOULD BE NEAR THE TOP OF `main.rs`
    /// 
    pub fn load() -> Result<(), Box<dyn Error>> {
        let path = PATH.to_string();
        fs::create_dir_all(*CONFIG_FOLDER)
            .expect("Error while creating the AvdanOS config directory!");
    
        // TODO: If config file not found, either download config
        // or use a pre-bundled copy.
        let file: File = fs::OpenOptions::new()
            .read(true).write(true).create(true)
            .open(&path)?;
            
        let reader: BufReader<File> = BufReader::new(file);

        let stripped: StripComments<_> = StripComments::new(reader);

        let src_map = {
            use json_tree::{Tokenizer, Value, Indexer, Source,};


            let mut src = Source::new(
                // TODO: @Sammy99jsp Prettier Error
                fs::read_to_string(&path).expect("Missing AvdanOS config!")
            );

            let tokens = match Tokenizer::tokenize(&mut src) {
                Ok(tkns) => tkns,
                Err(err) => {
                    let a = UnexpectedToken::from_parser(
                        err
                    );

                    println!("{}", a);

                    panic!()
                }
            };
            let root = match Value::parse(&mut tokens.iter().peekable()) {
                Ok(r) => r,
                Err(err) => {
                    let a = UnexpectedToken::from_parser(
                        err
                    );

                    println!("{}", a);

                    panic!()
                }
            };

            let mut index = HashMap::new();
            Indexer::index(&root, &mut index, None);

            index
        };

        unsafe {
            INDEX = Some(src_map);
        }

        let o = serde_json::from_reader(stripped)?;
        
        
        unsafe {
            CONFIG = Some(o);
        };

        Ok(())
    }
}