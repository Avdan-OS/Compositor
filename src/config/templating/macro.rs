use std::{collections::HashSet, convert::TryFrom, iter::FromIterator, };

use colored::Colorize;
use compositor_macros::{AvError, description, location};
use crate::{core::error::{AvError, TraceableError, Traceable}, config::errors};

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

#[AvError(TraceableError, CONFIG_MACRO_PARAMETER_ERROR, "Config: Macro Parameter Error")]
pub struct ParameterError(Traceable, String);

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
#[derive(Debug)]
pub struct AvMacro {
    identifier: String,
    parameters: HashSet<MacroParameter>
}

impl AvMacro {
    pub fn parse(loc : Traceable, value : String) -> Result<AvMacro, Vec<Box<dyn TraceableError>>> {
        let mut ident = String::new();
        let mut parameters : Vec<String> = vec![];
        let mut parameter_locs : Vec<Traceable> = vec![];

        enum State {
            Identifier,
            Parameters,
            Finished
        }

        let mut current_token = "".to_string();

        let mut current_state = State::Identifier;

        // If this macro is empty.
        if value.len() == 0 {
            return Err(
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
                    
                    _ => return Err(
                        vec![Box::new(ParseError {
                            location: loc.at_index(index),
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

                    }
                    _ => return Err(
                        vec![Box::new(ParseError {
                            location: loc.at_index(index),
                            erroneous: ParseErrorType::UnexpectedToken(chr.to_string()),
                        })]
                    )
                },
                State::Finished => {
                    return Err(
                        vec![Box::new(ParseError {
                            location: loc.at_index(index),
                            erroneous: ParseErrorType::ExpectedToken("<End of Macro>".to_string()),
                        })]
                    )
                }
            }
        }

        match current_state {
            State::Parameters => {
                // Didn't finish the parameters with a ')'
                return Err(
                    vec![Box::new(ParseError {
                        location: loc.at_index(value.len()),
                        erroneous: ParseErrorType::ExpectedToken(")".to_string()),
                    })]
                )
            },
            _ => {}
        }

        let parameters = parameters.iter()
            .map(|v| MacroParameter::try_from(v.as_str()));

        let errs : Vec<_> = parameters.clone()
            .enumerate()
            .filter(|(i, r)| r.is_err())
            .map(|(i, r)| {
                let err = r.unwrap_err();
                let e = parameter_locs.get(i).unwrap();

                Box::new(
                    ParameterError(e.clone(), err) 
                ) as Box<dyn TraceableError>
            })
            .collect();
        

        if errs.len() > 0 {
            return Err(errs);
        }

        let parameters : Vec<_> = parameters
            .map(|r| r.unwrap())
            .collect();

        Ok(
            Self {
                identifier: ident,
                parameters: HashSet::from_iter(parameters.iter().map(|s| s.clone()))
            }
        )   
    }
}