use colored::Colorize;

use compositor_macros::{
    AvError,
    description,
    location,
};

use crate::core::error::{
    AvError,
    Traceable,
    TraceableError,
};

use std::{
    convert::TryFrom,
    collections::{
        HashSet,
    },
    hash::Hash,
    iter::FromIterator,
};

#[derive(Clone)]
pub enum ParseErrorType {
    UnexpectedToken(String),
    ExpectedToken(String)
}

#[AvError(TraceableError, CONFIG_MACRO_PARSE_ERROR, "Config: Macro Parser Error")]
pub struct ParseError {
    ///
    /// The token that caused the failure.
    /// 
    erroneous: ParseErrorType, 

    ///
    /// Absolute location of the token.
    /// 
    location: Traceable,
}

impl TraceableError for ParseError {
    location!(&self.location);

    fn description(&self) -> String {
        match &self.erroneous {
            ParseErrorType::ExpectedToken(t) => {
                format!("Expected `{}`", t.blue())
            },
            ParseErrorType::UnexpectedToken(t) => {
                format!("Unexpected token `{}`", t.blue())
            }
        }
    }
}

///
/// Parameter types supported by
/// macros.
/// 
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum MacroParameter {
    ///
    /// Digit keys from 0...9
    /// 
    DigitKey,

    ///
    /// Function keys from 1...12
    /// 
    FunctionKey,
}

impl<'a> TryFrom<&'a str> for MacroParameter {
    type Error = String;

    ///
    /// Parse enum code into an enum member.
    /// 
    fn try_from(value: &'a str) -> Result<Self, Self::Error> {
        match value {
            "d" => Ok(Self::DigitKey),
            "F" => Ok(Self::FunctionKey),
            _   => Err(value.to_string()) 
        }
    }
}

impl From<MacroParameter> for String {
    fn from(p: MacroParameter) -> Self {
        match p {
            MacroParameter::DigitKey => "d",
            MacroParameter::FunctionKey => "F"
        }.to_string()
    }
}

impl ToString for MacroParameter {
    fn to_string(&self) -> String {
        <Self as Into<String>>::into(self.clone())
    }
}

#[AvError(TraceableError, CONFIG_MACRO_PARAMETER_ERROR, "Config: Macro Parameter Error")]
pub struct ParameterError(pub Traceable, pub String);

impl TraceableError for ParameterError {
    location!(&self.0);
    description!(("Invalid macro parameter `{}`", self.1.blue()));
}

#[AvError(TraceableError, CONFIG_MACRO_EMPTY, "Config: Expected Macro")]
pub struct MacroEmpty(Traceable);

impl TraceableError for MacroEmpty {
    location!(&self.0);
    description!(("Expected a macro here, got the silent treatment :(", ));
}

///
/// ## Structure of a Macro
/// All macros have an identifier.
/// Macro identifiers should use `camelCase`
/// and only contain ASCII alphanumeric characters.
/// 
/// 
/// Macro expressions can either have brackets (for parameters), or not.
/// * With parameters `macroNameHere(param1, param2, ...)`
/// * Without `macroNameHere` (equivalent to `macroNameHere()`)
/// 
/// ## Use 
/// 
/// Allows for macros in the configuration allow for things such as:
/// ```jsonc
/// {
///     "moveToWorkspace1": "Ctrl+Super+1",
///     "moveToWorkspace2": "Ctrl+Super+2",
///     "moveToWorkspace3": "Ctrl+Super+3",
///     /*             . . .            */       
///     "moveToWorkspace9": "Ctrl+Super+9",
/// }
/// ```
/// 
/// to be shortened to:
/// 
/// ```jsonc
/// {
///     "moveToWorkspace(d)": "Ctrl+Super+{d}"
/// }
/// ```
/// 
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct AvMacro {
    identifier: String,
    parameters: HashSet<MacroParameter>
}

impl Hash for AvMacro {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.cannonize().hash(state)
    }
}

#[derive(Clone)]
pub enum Difference {
    FromOriginal,
    FromNew
}

impl AvMacro {
    /// Two macros are said to have equal signature if
    /// they have the same parameters (the order doesn't matter).
    pub fn has_signature<'a> (
        &'a self,
        o: &'a Self
    ) -> Result<(), (Difference, Vec<&'a MacroParameter>)> {
        let u: HashSet<_> = self.parameters.union(&o.parameters).collect();

        match u.len() == self.parameters.len() && u.len() == o.parameters.len() {
            true => Ok(()),

            false => {
                let from_original : Vec<_> = self.parameters.difference(&o.parameters).collect();

                if from_original.len() > 0 {
                    return Err((Difference::FromOriginal, from_original))
                }

                return Err((
                    Difference::FromNew,
                    o.parameters.difference(&self.parameters).collect()
                ))
            }
        }
    }

    pub fn has_parameters(&self, v : Vec<MacroParameter>) -> Result<(), (Difference, Vec<MacroParameter>)> {
        match self.parameters.len() {
            l if l < v.len() => {
                Err (
                    (Difference::FromNew, 
                        v.iter().filter(|k: &&MacroParameter| !self.parameters.contains(k))
                            .map(|k: &MacroParameter| k.clone())
                            .collect()
                    )
                )
            },

            l if l == v.len() => Ok(()),

            l if l > v.len() => {
                Err(
                    (Difference::FromNew, 
                        self.parameters.iter().filter(|k: &&MacroParameter| !v.contains(k))
                            .map(|k: &MacroParameter| k.clone())
                            .collect()
                    )
                )
            },

            _ => unreachable!()
        }
    }

    pub fn has_same_id(&self, o : &Self) -> bool {
        self.identifier == o.identifier 
    }

    pub fn identifier(&self) -> String {
        self.identifier.clone()
    }
    
    fn cannonize(&self) -> String {
        format!("{}{:?}", self.identifier, self.parameters)
    }
    pub fn parse(loc : Traceable, value : String) -> Result<AvMacro, Vec<Box<dyn TraceableError>>> {
        let mut ident          : String = String::new();
        let mut parameters     : Vec<String> = vec![];
        let mut parameter_locs : Vec<Traceable> = vec![];

        enum State {
            Identifier,
            Parameters,
            Finished
        }

        let mut current_token: String = "".to_string();

        let mut current_state: State = State::Identifier;

        // If this macro is empty.
        if value.len() == 0 {
            return Err (
                vec![Box::new(MacroEmpty(loc))]
            );
        }

        // Parse each character.
        for (index, chr) in value.chars().enumerate() {
            if chr == ' ' {
                continue;
            }

            match current_state {
                State::Identifier => match chr {
                    c if c.is_ascii_alphanumeric() => {
                        current_token.push(c);
                    },

                    '(' => {
                        ident = current_token;
                        current_token = "".to_string();

                        parameter_locs.push(loc.at_index(index + 1));

                        current_state = State::Parameters;
                    },
                    
                    _ => return Err (
                        vec![Box::new(ParseError {
                            location : loc.at_index(index),
                            erroneous: ParseErrorType::UnexpectedToken(chr.to_string()),
                        })]
                    )
                },
                State::Parameters => match chr {
                    c if c.is_ascii_alphanumeric() => {
                        current_token.push(c);
                    },

                    ',' => {
                        parameters.push(current_token);
                        parameter_locs.push(loc.at_index(index + 1));

                        current_token = "".to_string();
                    },

                    ')' => {
                        parameter_locs.push(loc.at_index(index + 1));
                        parameters.push(current_token);
                        current_token = "".to_string();

                        current_state = State::Finished;

                    },

                    _ => return Err (
                        vec![Box::new(ParseError {
                            location: loc.at_index(index),
                            erroneous: ParseErrorType::UnexpectedToken(chr.to_string()),
                        })]
                    )
                },

                State::Finished => {
                    return Err (
                        vec![Box::new(ParseError {
                            location: loc.at_index(index),
                            erroneous: ParseErrorType::ExpectedToken("<End of Macro>".to_string()),
                        })]
                    )
                }
            }
        }

        match current_state {
            State::Identifier => {
                ident = current_token
            },

            State::Parameters => {
                // Didn't finish the parameters with a ')'
                return Err (
                    vec![Box::new(ParseError {
                        location: loc.at_index(value.len()),
                        erroneous: ParseErrorType::ExpectedToken(")".to_string()),
                    })]
                )
            },

            _ => {}
        }

        let parameters = parameters.iter()
            .map(|v: &String| MacroParameter::try_from(v.as_str()));

        let errs : Vec<_> = parameters.clone()
            .enumerate()
            .filter(|(_, r)| r.is_err())
            .map(|(i, r)| {
                let err: String     = r.unwrap_err();
                let e  : &Traceable = parameter_locs.get(i).unwrap();

                Box::new (
                    ParameterError(e.clone(), err) 
                ) as Box<dyn TraceableError>
            })
            .collect();
        
        if errs.len() > 0 {
            return Err(errs);
        }

        let parameters : Vec<_> = parameters
            .map(|r: Result<MacroParameter, String>| r.unwrap())
            .collect();

        Ok (
            Self {
                identifier: ident,
                parameters: HashSet::from_iter(parameters.iter().map(|s: &MacroParameter| s.clone()))
            }
        )   
    }
}

#[AvError(TraceableError, CONFIG_MACRO_SIGNATURE_MISMATCH, "Config: Macro Signature Mismatch")]
pub struct SignatureMismatchError(pub Traceable, pub String, pub (Difference, Vec<MacroParameter>));

impl TraceableError for SignatureMismatchError {
    location!(&self.0);
    description! (
        (
            "The macro parameter(s) of {} is not valid against its definition.\n\
            {}",
            self.1,
            match &self.2 {
                (p, v) => format!(
                    "{} {}",

                    match p {
                        Difference::FromNew => "Excess parameters:",
                        Difference::FromOriginal => "Missing:"
                    },

                    v.iter().map(|s| 
                        format!("`{}` ", s.to_string().blue()
                    )
                ).collect::<String>()),
            }
        )
    );
}
#[AvError(TraceableError, CONFIG_AVKEYS_PARAMETER_MISMATCH, "Config: Macro Value Mismatch")]
pub struct AvKeysMismatch(pub Traceable, pub String, pub (Difference, Vec<MacroParameter>));

impl TraceableError for AvKeysMismatch {
    location!(&self.0);
    description! (
        (
            "The macro parameter(s) of the Key expression {} are not valid against the macro holding it.\n\
            {}",

            self.1,

            match &self.2 {
                (p, v) => format!(
                    "{} {}",

                    match p {
                        Difference::FromNew => "Excess parameters:",
                        Difference::FromOriginal => "Missing:"
                    },

                    v.iter().map(|s| 
                        format!("`{}` ", s.to_string().blue()
                    )
                ).collect::<String>()),
            }
        )
    );
}

#[cfg(test)]
mod tests {
    use compositor_macros::traceable;

    use crate::core::error::TraceableError;

    use super::AvMacro;

    #[test]
    fn parsing_test() {
        let m: Result<AvMacro, Vec<Box<dyn TraceableError>>> = AvMacro::parse (
            traceable!(), 
            "helpMe(deez)".to_string()
        );

        let m: AvMacro = m.unwrap();

        println!("{m:?}");
    }
}
