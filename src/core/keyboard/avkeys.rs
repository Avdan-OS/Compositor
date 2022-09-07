use std::convert::TryFrom;

use compositor_macros::AvError;

use crate::{config::templating::{MacroParameter, AvMacro}, core::error::{AvError,}};

#[derive(Debug, PartialEq)]
pub enum AvKey {
    Key(String),
    Parameter(MacroParameter)
}

impl TryFrom<String> for AvKey {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.as_str() {
            "" => Err(format!("Expected Key, got empty string.")),
            v if v.ends_with("}") && v.starts_with("{") => {
                let t = &v[1..(v.len() - 1)];
                return Ok(AvKey::Parameter(
                    MacroParameter::try_from(t)?
                ))
            },
            a => Ok(AvKey::Key(a.to_string()))
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct AvKeys(Vec<AvKey>);

impl TryFrom<String> for AvKeys {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let vec : Vec<_> = value
            .split("+")
            .map(|s| AvKey::try_from(s.to_string()))
            .collect();


        let mut v = vec![];
        for val in vec {
            v.push(val?);
        }

        Ok(Self(v))
    }
}


#[AvError(INTERNAL_KEY_PARSE, "Internal: Key Parser")]
struct TestErr(String);

impl std::error::Error for TestErr {}

impl<'de> serde::Deserialize<'de> for AvKeys {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where D: serde::Deserializer<'de>
    {
        use serde::de::Error;
        
        let v : String = serde::Deserialize::deserialize(deserializer)?;
        AvKeys::try_from(v)
            .map_err(|s| {
                D::Error::custom(TestErr(s))
            })
    }
}
#[cfg(test)]
mod tests {
    use std::convert::TryFrom;

    use crate::config::templating::MacroParameter;

    use super::{AvKeys, AvKey};


    #[test]
    fn deserialize() {
        let j = "Ctrl+{d}".to_string();

        let v = AvKeys::try_from(j);

        assert_eq!(v, Ok(AvKeys(vec![AvKey::Key("Ctrl".to_string()), AvKey::Parameter(MacroParameter::DigitKey)])))

    }
}