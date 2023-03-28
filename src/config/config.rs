use crate::{
    config::errors::UnexpectedToken,
    CONST::{CONFIG_FILE, CONFIG_FOLDER},
};

pub(crate) use json_comments::StripComments;

use lazy_static::lazy_static;

use json_tree::{parser::Token, Index, Indexer, JSONPath, Location, Source, Tokenizer, Value};

use serde::Deserialize;

use std::{
    collections::HashMap,
    error::Error,
    fs::{self, File},
    io::BufReader, path::PathBuf,
};

use super::sections::keybinds::Keybinds;

lazy_static! {
    pub static ref PATH: PathBuf = CONFIG_FOLDER
        .join(*CONFIG_FILE);
}

static mut INDEX: Option<Index> = None;

static mut CONFIG: Option<Config> = None;

#[derive(Deserialize, Debug)]
pub struct Config {
    pub keybinds: Keybinds,
}

impl Config {
    pub fn path() -> String {
        PATH.to_str().unwrap().to_string()
    }

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
    /// A CALL TO THIS FUNCTION SHOULD BE NEAR THE TOP OF `main.rs`
    ///
    pub fn load() -> Result<(), Box<dyn Error>> {
        
        // Recursively crate config dir if it doesn't exist.
        fs::create_dir_all(&*CONFIG_FOLDER)
            .expect("Could not create config folder '$XDG_CONFIG_HOME/avdan'.");

        let file: File = match fs::OpenOptions::new().read(true).open(&*PATH) {
            Err(_) => {
                // File probs doesn't exist
                let default = include_str!("../../DefaultConfig.jsonc");
                fs::write(&*PATH, default).expect(&format!("{} not writeable!", PATH.to_str().unwrap()));

                fs::OpenOptions::new()
                    .read(true)
                    .open(&*PATH)
                    .expect("Couldn't read newly-written default config!")
            }
            Ok(o) => o,
        };

        let reader: BufReader<File> = BufReader::new(file);

        let stripped: StripComments<_> = StripComments::new(reader);

        let src_map: HashMap<JSONPath, Location> = {
            let mut src: Source = Source::new(
                // TODO: @Sammy99jsp Prettier Error
                fs::read_to_string(&*PATH).expect("Missing AvdanOS config!"),
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
