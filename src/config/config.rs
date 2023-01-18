use crate::{
    config::errors::UnexpectedToken,
    CONST::{
        CONFIG_FOLDER,
        CONFIG_FILE,
    },
};

pub(crate) use json_comments::StripComments;

use lazy_static::lazy_static;

use json_tree::{
    JSONPath,
    Index,
    Indexer,
    Location,
    parser::Token,
    Source,
    Tokenizer,
    Value,
};

use serde::Deserialize;

use std::{
    collections::HashMap,
    error::Error, 
    fs::{
        File,
        self,
    },
    io::BufReader,
};

use super::sections::keybinds::Keybinds;

lazy_static! {
    pub static ref PATH: String = CONFIG_FOLDER.join(*CONFIG_FILE).to_string_lossy().to_string();
}

static mut INDEX: Option<Index> = None;

static mut CONFIG: Option<Config> = None;

#[derive(Deserialize, Debug)]
pub struct Config { pub keybinds: Keybinds }

impl Config {
    pub fn path() -> String { PATH.to_string() }

    ///
    /// Returns the config file's JSON index.
    /// 
    pub fn index<'a>() -> &'a Index {
        unsafe { INDEX.as_ref().unwrap() }
    }

    ///
    /// Returns the Global Configuration Object.
    /// 
    pub fn config<'a>() -> &'a Self {
        unsafe { CONFIG.as_ref().unwrap() }
    }

    ///
    /// Loads the config.
    /// 
    /// THIS FUNCTION SHOULD BE NEAR THE TOP OF `main.rs`
    /// 
    pub fn load() -> Result<(), Box<dyn Error>> {
        let path: String = PATH.to_string();


        // TODO: If config file not found, either download config
        // or use a pre-bundled copy.
        let file: File = match fs::OpenOptions::new()
            .read(true)
            .open(&path)
        {
            Err(_) => {
                // File probs doesn't exist
                let default = include_str!("../../DefaultConfig.jsonc");
                fs::write(&path, default)
                    .expect(&format!("{} not writeable!", path));

                fs::OpenOptions::new()
                    .read(true)
                    .open(&path)
                    .expect("Couldn't read newly-written default config!")      
            },
            Ok(o) => o
        };


            
        let reader: BufReader<File> = BufReader::new(file);

        let stripped: StripComments<_> = StripComments::new(reader);

        let src_map: HashMap<JSONPath, Location> = {

            let mut src: Source = Source::new (
                // TODO: @Sammy99jsp Prettier Error
                fs::read_to_string(&path).expect("Missing AvdanOS config!")
            );

            let tokens: Vec<Token> = match Tokenizer::tokenize(&mut src) {
                Ok(tkns) => tkns,
                Err(err) => {
                    let a: UnexpectedToken = UnexpectedToken::from_parser(err);

                    println!("{}", a);

                    panic!()
                }
            };
            let root: Value = match Value::parse(&mut tokens.iter().peekable()) {
                Ok(r) => r,

                Err(err) => {
                    let a: UnexpectedToken = UnexpectedToken::from_parser(err);

                    println!("{}", a);

                    panic!()
                }
            };

            let mut index: HashMap<JSONPath, Location> = HashMap::new();

            Indexer::index(&root, &mut index, None);

            index
        };

        unsafe { INDEX = Some(src_map) }

        let o: Config = serde_json::from_reader(stripped)?;
    
        unsafe { CONFIG = Some(o) }

        Ok(())
    }
}
