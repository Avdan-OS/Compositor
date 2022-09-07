use std::{
    error::Error, 
    fs::{self, File},
    io::BufReader,
    collections::HashMap
};

use lazy_static::lazy_static;


pub(crate) use json_comments::StripComments;

use json_tree::{Index, TokenContent};
use serde::Deserialize;

use crate::{CONST::{
    CONFIG_FOLDER,
    CONFIG_FILE,
}, core::error::Traceable, config::errors::UnexpectedToken};

use super::sections::{keybinds::Keybinds, section::ConfigurationSection};

lazy_static! {
    pub static ref INDEX : Option<Index> = None;
    pub static ref PATH : &'static str = CONFIG_FOLDER.join(*CONFIG_FILE).to_str().unwrap();
}


#[derive(Deserialize, Debug)]
pub struct Config {

    ///
    /// JSON Index of the config file itself.
    /// 
    #[serde(skip_deserializing)]
    ___index___ : Index,

    keybinds: <Keybinds as ConfigurationSection>::Raw,
}

impl Config {
    pub fn index(&self) -> &Index {
        &self.___index___
    }
    pub fn from_file() -> Result<Config, Box<dyn Error>> {
        let path = PATH.to_string();
        fs::create_dir_all(*CONFIG_FOLDER)
            .expect("Error while creating the AvdanOS config directory!");
    
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

        let mut parsed: Config = serde_json::from_reader(stripped)?;

       
        *INDEX = Some(src_map);

        parsed.___index___ = src_map;

        let result = Keybinds::parse(
            Keybinds::traceable(
                Some(true)
            ),
            &parsed.keybinds,
            parsed.index()
        );

        match result {
            Ok(k) => {
                return Ok(parsed)
            },
            Err(errs) => {
                for err in errs {
                    println!("{}\n", err);
                }

                panic!()
            }
        }
    }
}