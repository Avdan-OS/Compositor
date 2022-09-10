use std::convert::TryFrom;

use colored::Colorize;
use compositor_macros::{AvError, location, description};
use serde_json::Value;

use crate::{
    config::templating::{
        MacroParameter,
        avvalue::{
            UnexpectedType, AvValue
        }, 
        r#macro::ParameterError,
        AvDeserialize
    },
    core::error::{
        AvError,
    },
    Nadva::error::{
        TraceableError,
        Traceable
    }
};

#[derive(Debug, PartialEq, Clone)]
pub enum AvKey {
    Key(String),
    Parameter(MacroParameter)
}

impl TryFrom<String> for AvKey {
    type Error = (String, String);

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.as_str() {
            "" => Err(("key".to_string(), "".to_string())),
            v if v.ends_with("}") && v.starts_with("{") => {
                let t = &v[1..(v.len() - 1)];
                return Ok(AvKey::Parameter(
                    MacroParameter::try_from(t)
                        .map_err(|s| ("macro_param".to_string(), s))?
                ))
            },
            a => Ok(AvKey::Key(a.to_string()))
        }
    }
}

impl ToString for AvKey {
    fn to_string(&self) -> String {
        match self {
            Self::Key(k) => k.clone(),
            Self::Parameter(p) => format!("{{{}}}", p.to_string())
        }
    }
}

#[derive(Debug, PartialEq,)]
pub struct AvKeys(pub Vec<AvKey>);

impl TryFrom<String> for AvKeys {
    type Error = (String, String);

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

impl Clone for AvKeys {
    fn clone(&self) -> Self {
        Self(
            self.0.iter().map(|k| (*k).clone()).collect()
        )
    }
}

impl ToString for AvKeys {
    fn to_string(&self) -> String {
        let v : Vec<_> = self.0.iter().map(AvKey::to_string).collect();

        v.join("+")
    }
}

#[AvError(TraceableError, CONFIG_KEY_PARSE, "Config: Keyboard Shortcut Parsing Error")]
struct KeyParseError(Traceable, String);

impl TraceableError for KeyParseError {
    location!(&self.0);
    description!(("Error while parsing `{}` as a Keyboard Key", self.1.blue()));
}

impl AvDeserialize for AvKeys {
    fn deserialize(loc : Traceable, val : serde_json::Value) -> Result<AvValue, Box<dyn TraceableError>> {
        match val {
            Value::String(s) => Ok(
                AvValue::AvKeys(
                    AvKeys::try_from(s)
                        .map_err(|(t, v)| 
                            match t.as_str() {
                                "key" => Box::new(KeyParseError(loc, v)) as Box<dyn TraceableError>,
                                "macro_param" => Box::new(ParameterError(loc, v)) as Box<dyn TraceableError>,
                                _ => unreachable!()
                            }
                        )?
                )
            ),
            v => Err(Box::new(UnexpectedType::from(loc, "String", v)))
        }
    }
}

impl Default for AvKeys {
    fn default() -> Self {
        Self(vec![])
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