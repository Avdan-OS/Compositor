use json_tree::{JSONPath, Index};

use crate::{core::error::{TraceableError, Traceable}, config::config};

pub trait ConfigurationSection : Sized {
    ///
    /// The type the implementing section is built from. 
    /// Probably a hashmap.
    /// 
    type Raw : Sized;

    ///
    /// The absolute path to this section as a str.
    /// 
    const PATH : &'static str;

    ///
    /// Returns the absolute path to this section.
    /// Can be used in finding location of data.
    /// 
    fn path() -> JSONPath {
        Self::PATH.to_string().try_into().unwrap()
    }

    fn traceable(key : Option<bool>) -> Traceable {
        let loc = config::INDEX.unwrap().get(&Self::path()).unwrap();
        Traceable::combine(&config::PATH.to_string(), loc, key)
    }

    fn parse(trace: Traceable, raw : &Self::Raw, index: &Index) -> Result<Self, Vec<Box<dyn TraceableError>>>;
}