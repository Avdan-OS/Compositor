use colored::{
    Colorize,
};

use json_tree::Location;

use std::cmp::Ordering;

pub mod color {
    use colored::Color;

    pub const ERROR : Color = Color::TrueColor {
        r: 215,
        g: 38,
        b: 56
    };

    pub const WARNING : Color = Color::TrueColor {
        r: 246,
        g: 170,
        b: 28
    };

    pub const NEUTRAL : Color = Color::TrueColor {
        r: 5,
        g:142,
        b: 217
    };
}

pub trait Indentable
    where Self: Sized
{
    const BASE_UNIT: usize = 2;
    ///
    /// Indent all the lines by a specific number of spaces
    /// 
    fn indent(&self, levels: usize) -> String;
}

impl Indentable for String {
    fn indent(&self, level: usize) -> String {
        let v : Vec<_> = self
            .split("\n")
            .map(|s| format!("{}{}", " ".repeat(Self::BASE_UNIT * level), s))
            .collect();
        v
            .join("\n")
            
    }
}

impl Indentable for &str {
    fn indent(&self, level: usize) -> String {
        let v : Vec<_> = self
            .split("\n")
            .map(|s| format!("{}{}", " ".repeat(Self::BASE_UNIT * level), s))
            .collect();
        v
            .join("\n")
    }
}


///
/// ## Errors
/// 
/// All errors in AvdanOS should implement this trait.
/// 
/// `AvError`s allow for colorful, and descriptive error messages.
/// 
/// The `#[AvError]` macro can be used to make the process of creating
/// a new error easily.
/// 
/// #### Example - Generic Error
/// 
/// ```rust
/// use crate::core::error::AvError;
/// use compositor_macros::AvError;
/// //         ↓ Error code    ↓ Error Title
/// #[AvError(EXAMPLE_ERROR, "Example Error for Documentation")]
/// struct ExampleError;
/// 
/// let example = ExampleError;
/// println!("{}", example);
/// 
/// ```
pub trait AvError : std::fmt::Display + std::fmt::Debug + std::error::Error {
    ///
    /// The unique code for this error.
    /// 
    /// Convention is to use TRAIN_CASE
    /// 
    /// ### Examples
    /// ```
    /// "CONFIG_MACRO_NOT_FOUND"
    /// "CONFIG_MACRO_INVALID_PARAMETER"
    /// ```
    /// 
    fn code(&self) -> String;

    ///
    /// A title for this error (shared by
    /// all instances of this error).
    /// 
    /// Capitalize each word.
    /// 
    /// ### Examples
    /// ```
    /// "Macro Not Found"
    /// "Invalid Macro Parameter"
    /// ```
    /// 
    fn title(&self) -> String;

    ///
    /// The content of this error warning.
    /// 
    /// **NOTE**: do not override this, unless
    /// you are making a new error type,
    /// like `LocatableError` 
    /// 
    fn body(&self) -> String { "".into() }
}

///
/// Represents a location
/// of a traceable error.
/// 
#[derive(Debug, Clone)]
pub struct Traceable {
    ///
    /// The path to the file
    /// that caused this error.
    /// 
    path  : String,

    ///
    /// The line where this error
    /// occurred.
    /// 
    line  : usize,

    ///
    /// The column index where this
    /// error occurred.
    /// 
    column: usize, 
}

impl Traceable {
    ///
    /// Makes a new instance of [`Traceable`].
    /// 
    pub fn new(path: String, loc : (usize, usize)) -> Self {
        Self {
            path,
            line  : loc.0,
            column: loc.1
        }
    }

    ///
    /// Returns new [`Traceable`] at a string index
    /// relative to the current column pos.
    /// 
    pub fn at_index(&self, index: usize) -> Self {
        Self {
            path  : self.path.clone(),
            line  : self.line,
            column: self.column + 1 + index
            // Account for the ""  ^
        }
    }

    ///
    /// Returns a new [`Traceable`] with new 
    /// line and column numbers.
    /// 
    pub fn at_loc(&self, (line, column): (usize, usize)) -> Self {
        Self {
            path: self.path.clone(),
            line,
            column
        }
    }

    ///
    /// Makes a Locatable from a Location and a file path
    /// 
    /// key object determines if it should return the Locatable for the key or value  
    pub fn combine(path: &String, loc: &Location, key: Option<bool>) -> Self {
        let (line, column): (usize, usize) = match loc {
            Location::KeyValue(k, v) => match key {
                None => unimplemented!("Should pass in Some(bool) as last argument for KeyValue pairs"),
                Some(true) => k,
                Some(false) => v,
            },

            Location::Value(v) => v
        }.loc();

        Self {
            path: path.clone(),
            line,
            column
        }
    }
}


impl ToString for Traceable {
    fn to_string(&self) -> String {
        format!("{}:{}:{}", self.path, self.line, self.column)
    }
}

///
/// ## Traceable Error
/// Traceable Errors are errors that can be
/// traced to a specific location 
/// in a file such as a mis-configuration.
/// 
/// ### Macros
/// 
/// * `location!(&self <expr>)`
///     - Implements [`TraceableError::location`] by returning the value
///     inside the brackets
/// 
/// See [`location`].
/// 
/// 
/// * `description!((<tuple expr>))`
///     - Implements [`TraceableError::description`] by returning `format!(<macro contents>)`
///     which allows for easy string interpolation. 
/// 
/// See [`description`].
/// 
/// ### Example
/// 
/// ```rust
/// use crate::core::error::{LocatableError, Traceable};
/// use compositor_macros::{AvError, location, description};
/// 
/// #[AvError(TraceableError, EXAMPLE_ERROR, "Example Error for Documentation")]
/// struct ExampleError(Traceable);
/// 
/// 
/// 
/// impl TraceableError for ExampleError {
///     location!(&self.0);
///     description!(("Example traceable error", ));
/// }
/// 
/// let test = ExampleError(
///     Traceable {
///         path: "config.txt".to_string(),
///         line: 203,
///         column: 20
///     }
/// );
/// 
/// println!("{}", test);
/// 
/// ```
/// 
pub trait TraceableError : AvError {
    ///
    /// The error's location
    /// 
    fn location(&self) -> &Traceable;

    fn description(&self) -> String;

    fn body(&self) -> String {
        format! (
            "{}\nat {}\n",
            TraceableError::description(self),
            self.location().to_string().color(color::NEUTRAL)
        )
    }
}

pub fn compare_errors(s : &Box<dyn TraceableError>, o: &Box<dyn TraceableError>)
    -> Option<Ordering>  {
    let s: &Traceable = s.location();
    let o: &Traceable = o.location();

    if s.path != o.path {
        return None;
    }

    match s.line.partial_cmp(&o.line).unwrap() {
        Ordering::Equal => s.column.partial_cmp(&o.column),
        a => Some(a)
    }
}

#[cfg(test)]
mod tests {
    use compositor_macros::AvError;

    use crate::core::error::{
        Traceable,
        TraceableError,
    };

    use super::AvError;

    #[test]
    fn derive() {
        #[AvError(TEST_DERIVE_ERROR, "Test Error")]
        struct Error;

        let test: Error = Error;
        println!("{}", test);
    }

    #[test]
    fn locatable() {
        #[AvError(TraceableError, TEST_DERIVE_ERROR, "Test Error")]
        struct Locatable(Traceable);
        
        let test: Locatable = Locatable(
            Traceable {
                path: "src/core/error.rs".to_string(),
                line: 203,
                column: 20
            }
        );

        impl TraceableError for Locatable {
            fn location(&self) -> &Traceable { &self.0 }

            fn description(&self) -> String {
                "Test Traceable thing!".into()
            }
        }

        println!("{}", test);
    }
}
