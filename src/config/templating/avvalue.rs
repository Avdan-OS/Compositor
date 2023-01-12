//! A short enum wrapper of the value of a macro in a configuration section
//! 
//! Implemented for different types with the macro: `AvValue!([<types>])`
//! the types should have the `AvDeserialize` trait.
//! 

use colored::Colorize;

use compositor_macros::{
    AvError,
    AvValue,
    description,
    location,
};

use crate::core::{
    error::{
        AvError,
        Traceable,
        TraceableError,
    },
    keyboard::{
        AvKeys,
        avkeys::AvKey,
    },
};

use serde_json::Value;

use super::{
    AvMacro,
    r#macro::{AvKeysMismatch, Difference}, MacroParameter,
};

///
/// # UnexpectedType
/// 
/// Where the config expects one type, but gets another.
/// 
/// ## Members
/// * Location
/// * Expected type
/// * Received type
/// 
#[AvError(TraceableError, CONFIG_UNEXPECTED_TYPE, "Config: Unexpected Type")]
pub struct UnexpectedType(Traceable, String, String);

impl TraceableError for UnexpectedType {
    description!(("Expected a {}, got a {}.", &self.1.blue(), &self.2.blue()));
    location!(&self.0);
}

impl UnexpectedType {
    fn type_name(v : &Value) -> String {
        match v {
            Value::Array(_)  => "Array",
            Value::Object(_) => "Object",
            Value::String(_) => "String",
            Value::Number(_) => "Number",
            Value::Bool(_)   => "Boolean",
            Value::Null      => "Null",
        }.to_string()
    }

    pub fn from(loc : Traceable, e: &str, v : Value) -> UnexpectedType {
        Self(loc, e.to_string(), Self::type_name(&v))
    }
}

AvValue!([String, i64, f64, bool, AvKeys]);

impl AvValue {
    pub fn parse_same_type(&self, loc: Traceable, val : Value) -> Result<Self, Box<dyn TraceableError>> {

        match &self {
            AvValue::String(_)  => String::deserialize(loc, val),
            AvValue::i64(_)     => i64::deserialize(loc, val),
            AvValue::f64(_)     => f64::deserialize(loc, val),
            AvValue::bool(_)    => bool::deserialize(loc, val),
            AvValue::AvKeys(_)  => AvKeys::deserialize(loc, val),
        }
    }

    pub fn consistent_with_macro (
        &self,
        loc: Traceable,
        m: &AvMacro
    ) -> Result<(), Box<dyn TraceableError>> {
        match &self {
            AvValue::String(_)  => Ok(()),
            AvValue::i64(_)     => Ok(()),
            AvValue::f64(_)     => Ok(()),
            AvValue::bool(_)    => Ok(()),
            AvValue::AvKeys(k)  => {
                let p : Vec<_> = k.0.iter()
                    .filter_map(|k: &AvKey| match k {
                        AvKey::Parameter(k) => Some(k),
                        _ => None
                    })
                    .collect();

                m.has_parameters(p.iter().map(|k: &&MacroParameter| (*k).clone()).collect())
                    .map_err(|e: (Difference, Vec<MacroParameter>)| {
                        Box::new(AvKeysMismatch(loc, k.to_string(), e)) as Box<dyn TraceableError>
                    })
            }
        }
    }
}

///
/// Allows for conversion of `serde_json::Value` to `AvValue` for a given type
/// 
pub trait AvDeserialize {
    fn deserialize (
        loc: Traceable,
        val: Value
    ) -> Result<AvValue, Box<dyn TraceableError>>;
}

impl AvDeserialize for String {
    fn deserialize (
        loc: Traceable,
        val: Value
    ) -> Result<AvValue, Box<dyn TraceableError>> {
        match val {
            Value::String(s) => Ok(AvValue::String(s)),

            v => Err(Box::new(UnexpectedType::from(loc, "String", v)))
        }
    }
}

impl AvDeserialize for i64 {
    fn deserialize (
        loc: Traceable,
        val: Value
    ) -> Result<AvValue, Box<dyn TraceableError>> {
        match val {
            Value::Number(ref v) => {
                match v.as_i64() {
                    Some(v) => Ok(AvValue::i64(v.clone())),
                    None => Err(Box::new(UnexpectedType::from(loc, "Integer", Into::<Value>::into(v.clone()))))
                }
            },

            v => Err(Box::new(UnexpectedType::from(loc, "Integer", v.clone())))
        }
    }

}

impl AvDeserialize for f64 {
    fn deserialize (
        loc: Traceable,
        val: Value
    ) -> Result<AvValue, Box<dyn TraceableError>> {
        match val {
            Value::Number(ref v) => {
                match v.as_f64() {
                    Some(v) => Ok(AvValue::f64(v)),
                    None => Err(Box::new(UnexpectedType::from(loc, "Float",val.clone())))
                }
            },

            v => Err(Box::new(UnexpectedType::from(loc, "Float", v)))
        }
    }
}

impl AvDeserialize for bool {
    fn deserialize (
        loc: Traceable,
        val: Value
    ) -> Result<AvValue, Box<dyn TraceableError>> {
        match val {
            Value::Bool(v) => Ok(AvValue::bool(v)),

            v => Err(Box::new(UnexpectedType::from(loc, "Boolean",  v)))
        }
    }

}
