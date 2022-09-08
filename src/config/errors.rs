use colored::Colorize;
use compositor_macros::AvError;
use json_tree::{ParserError, TokenContent};
use crate::core::error::{TraceableError, AvError, Traceable};

use super::config;


///
/// Error in parsing the config file.
/// 
#[AvError(TraceableError, CONFIG_UNEXPECTED_TOKEN, "Config: Unexpected Token")]
pub struct UnexpectedToken(pub String, pub Traceable);

impl UnexpectedToken 
{
    pub fn from_parser<T>(p: ParserError<T>) -> Self
        where T: Into<TokenContent> {
        match p {
            ParserError::UnexpectedToken(t) => {
                let to : TokenContent = t.into();

                let loc = Traceable::new(
                    config::PATH.to_string(),
                    to.loc()
                );

                // Convert to our friendlier error format
                UnexpectedToken(
                    // rust-analyzer (and I guess cargo check) gives up on the line below
                    // but it *does* compile -- smh my head
                    to.content,
                    loc
                )
            },
            ParserError::UnexpectedEnd(_, _) => unimplemented!(),
        }
    }
}

impl TraceableError for UnexpectedToken {
    fn location(&self) -> &Traceable {
        &self.1
    }

    fn description(&self) -> String {
        format!("Unexpected token `{}`", self.0.blue())
    }
}