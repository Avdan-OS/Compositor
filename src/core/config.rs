#![allow(dead_code)]
use std::{
    error::Error,
    fs::{
        self,
        File,
    },
    io::BufReader,
};

use regex::{
    Regex,
    Captures,
};

use serde::Deserialize;

use lazy_static::lazy_static;

use json_comments::StripComments;

use crate::CONST::{
    CONFIG_FOLDER,
    CONFIG_FILE,
};

#[derive(Deserialize, Debug)]
pub struct Config {
    test: String,
}

impl Config {
    pub fn from_file() -> Result<Config, Box<dyn Error>> {
        fs::create_dir_all(*CONFIG_FOLDER)
            .expect("Error while creating the AvdanOS config directory!");
    
        let file: File = fs::OpenOptions::new()
            .read(true).write(true).create(true)
            .open(CONFIG_FOLDER.join(*CONFIG_FILE))?;
            
        let reader: BufReader<File> = BufReader::new(file);

        let stripped: StripComments<BufReader<File>> = StripComments::new(reader);

        let parsed: Config = serde_json::from_reader(stripped)?;
        Ok(parsed)
    }
}

#[derive(Debug)]
struct TemplateString {
    raw: String,
    tokens: Vec<String>,
}

impl<'de> TemplateString {

    fn from_raw_string<'a> (
        raw_string: String,
    ) -> Result<Self, &'a str> {

        lazy_static! {
            static ref VARIABLES_REGEX : Regex = Regex::new(r"\{(.*?)\}").unwrap();
        }

        // Check for a brace - {} - mismatch
        let braces_count: [usize; 2] = ["{", "}"]
            .map(|c: &str| raw_string.matches(c).count());

        if braces_count[0] != braces_count[1] {
            return Err("Brace {} mismatch !");
        }

        let variables : Vec<String> = VARIABLES_REGEX
            .captures_iter(&raw_string)
            .map(|m: Captures| 
                m.get(1).unwrap().as_str().to_string()
            )
            .collect();
        
        Ok (
            Self {
                raw: raw_string,
                tokens : variables
            }
        )
    }
}

impl<'de> Deserialize<'de> for TemplateString {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where D: serde::Deserializer<'de> {
        let raw_string : String = String::deserialize(deserializer)?;
        Self::from_raw_string(raw_string).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::TemplateString;

    #[test]
    fn test_variable_extract() {
        let template: Result<TemplateString, &str> = TemplateString::from_raw_string (
            "Logo+{a}+{b}+{c}".to_string()
        );

        assert!(template.is_ok());

        let template: TemplateString = template.unwrap();

        assert_eq! (
            template.tokens, vec!["a", "b", "c"]
        );

    }

    #[test]
    fn test_braces_mismatch() {
        let template: Result<TemplateString, &str> = TemplateString::from_raw_string(
            "Logo+{n}}".to_string()
        );

        assert!(template.is_err());

        println!("{}", template.unwrap_err());
    }
}
