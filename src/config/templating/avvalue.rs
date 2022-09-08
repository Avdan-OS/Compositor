//! Value of a key in a configuration section


use colored::Colorize;
use compositor_macros::{AvError, description, location};
use serde_json::Value;

use crate::Nadva::{keyboard::AvKeys, error::{Traceable, TraceableError, AvError}};

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

    pub fn from<E : AvValue>(loc : Traceable, v : &Value) -> UnexpectedType {
        Self(loc, E::name.to_string(), Self::type_name(v))
    }
}

/// Different types a value could be.
/// 
/// Similar to the enum with the same name in the macros src.
pub trait AvValue 
    where Self : Sized
{
    const name : &'static str;
    fn deserialize(loc : Traceable, val : Value) -> Result<Self, Box<dyn TraceableError>>;
}

impl AvValue for String {
    const name : &'static str = "String";
    fn deserialize(loc : Traceable, val : Value) -> Result<Self, Box<dyn TraceableError>> {
        
        match val {
            Value::String(s) => Ok(s),
            v => Err(Box::new(UnexpectedType::from::<String>(loc, &v)))
        }
    }
}

impl AvValue for i64 {
    const name : &'static str = "Integer";
    fn deserialize(loc : Traceable, val : Value) -> Result<Self, Box<dyn TraceableError>> {
        match val {
            Value::Number(v) => {
                match v.as_i64() {
                    Some(v) => Ok(v),
                    None => Err(Box::new(UnexpectedType::from::<i64>(loc, &val)))
                }
            },
            v => Err(Box::new(UnexpectedType::from::<i64>(loc, &v)))
        }
    }
}

impl AvValue for f64 {
    const name : &'static str = "Float";

    fn deserialize(loc : Traceable, val : Value) -> Result<Self, Box<dyn TraceableError>> {
        match val {
            Value::Number(v) => {
                match v.as_f64() {
                    Some(v) => Ok(v),
                    None => Err(Box::new(UnexpectedType::from::<i64>(loc, &val)))
                }
            },
            v => Err(Box::new(UnexpectedType::from::<i64>(loc, &v)))
        }
    }
}

impl AvValue for bool {
    const name : &'static str = "Boolean";

    fn deserialize(loc : Traceable, val : Value) -> Result<Self, Box<dyn TraceableError>> {
        match val {
            Value::Bool(v) => Ok(v),
            v => Err(Box::new(UnexpectedType::from::<i64>(loc, &v)))
        }
    }
}

impl<V : AvValue> AvValue for Option<V> {
    const name : &'static str = "Null";

    fn deserialize(loc : Traceable, val : Value) -> Result<Self, Box<dyn TraceableError>> {
        match val {
            Value::Null => Ok(None),
            v => Ok(Some(V::deserialize(loc, v)?))
        }
    }
}

// pub enum AvValue {
//     String(syn::LitStr),
//     Integer(syn::LitInt),
//     Float(syn::LitFloat),
//     Null(syn::Ident),
//     Boolean(syn::LitBool),
//     AvKeys(syn::punctuated::Punctuated<AvKey, Token![+]>),
//     List(syn::punctuated::Punctuated<AvValue, Token![,]>)
// }
