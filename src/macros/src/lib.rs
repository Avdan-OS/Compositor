#![feature(proc_macro_diagnostic)]

use std::fs;

mod delcaration;
mod avvalue;

use avvalue::AvValue;
use delcaration::ConfigDelcaration;
use proc_macro::{Diagnostic, Level,};
use proc_macro2::TokenStream;
use quote::{quote, quote_spanned, TokenStreamExt};
use syn::{parse_macro_input, AttributeArgs, ItemStruct, LitStr};

#[proc_macro]
pub fn export_test(struc: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let parsed = parse_macro_input!(struc as ConfigDelcaration);

    // fs::write("./test.txt", format!("{:?}", parsed)).unwrap();

    let ident = parsed.ident;

    let mut q = quote! {};

    for f in parsed.fields.iter() {
        let (name, params) = f.av_macro();
        let comments = f.description();

        let n = syn::Ident::new(&name, ident.span());

        let mut lines = quote! {};

        for l in comments.lines() {
            let tkns : proc_macro2::TokenStream = format!("/// {}", l.trim_start()).parse().unwrap();
            lines = quote! {
                #lines
                #tkns
            }
        }

        let typ : TokenStream = f.default().get_type();

        let z  = quote! {
            #q
            #lines
            #n : #typ,
        };

        q = z;
    } 
    quote! {
        #[allow(non_snake_case)]
        struct #ident {
           #q 
        }

    }.into()
}

extern crate proc_macro;

///
/// ## AvError Macro
/// This macro acts like
/// `#[derive(AvError)]`
/// 
/// ### Parameters
/// 
/// 1. *(Optional)* Error Type - AvError (default), or a super trait of it.
/// 2. Error Code -- The error code as an identifier (in TRAIN_CASE)
/// 3. Error Title -- A user-friendly description of the error.
/// 
/// 
#[proc_macro_attribute]
pub fn AvError(attributes : proc_macro::TokenStream, input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    
    let a =  parse_macro_input!(attributes as AttributeArgs);

    let (parent, code, title) = match a.len() {
        2 => (None, a.get(0).unwrap(), a.get(1).unwrap()),
        3 => (Some(a.get(0).unwrap()), a.get(1).unwrap(), a.get(2).unwrap()), 
        _ => {
            return quote::quote! {
                compile_error!("Expected two or three elements: [`TYPE`], `ERROR CODE`, `TITLE`")
            }.into();
        }
    };

    let (code, title) = match (code, title) {
        (syn::NestedMeta::Meta(l1), syn::NestedMeta::Lit(l2)) => (l1, l2),
        _ => unimplemented!("Code and title in invalid format!"),
    };

    let code = match code {
        syn::Meta::Path(p) => p,
        _ => {
            return quote::quote! {
                compile_error!("Expected `ERROR CODE` to be a raw identifier (no \"\")")
            }.into();
        },
    };

    let title = match title {
        syn::Lit::Str(tkn) => tkn,
        _ => {
            return quote::quote! {
                compile_error!("Invalid format for title!")
            }.into();
        }
    };

    let code = match code.get_ident() {
        None => {
            return quote::quote! {
                compile_error!("`ERROR CODE` needs to be a raw identifier (no ::)")
            }.into();
        },
        Some(t) => t
    };

    
    // // I wish diagnostics were on the stable channel :(
    // // 'Linting' the error code. 
    match code.to_string() {
        s if !s.is_ascii() => {
            let mut w =Diagnostic::new(Level::Warning, "Error Codes should be ASCII only ");
            w.set_spans(code.span().unwrap());
            w.emit();
        },
        s if s.to_ascii_uppercase() != s => {
            let mut w = Diagnostic::new(Level::Warning, "Error Codes should be in TRAIN_CASE");
            w.set_spans(code.span().unwrap());
            w.emit();
        },
        s if s.ends_with("_") => {
            let mut w = Diagnostic::new(Level::Warning, "Remove the trailing `_`");
            w.set_spans(code.span().unwrap());
            w.emit();
        }
        _ => {}
    }

    let code : LitStr = LitStr::new(&code.to_string(), code.span());

    let input = parse_macro_input!(input as ItemStruct);

    let ident = input.ident.clone();

    // if the parent error attribute is defined, 
    // use it as the base trait instead of the default AvError
    // for AvError::body()...
    let line = match parent {
        Some(err_type) => {
            let err_type = match err_type {
                syn::NestedMeta::Meta(l) => l,
                _ => {
                    return quote::quote! {
                        compile_error!("`ERROR TYPE` needs to be a raw identifier (no ::)")
                    }.into();
                }
            };

            let err_type = match err_type {
                syn::Meta::Path(tkn) => tkn,
                _ => {
                    return quote::quote! {
                        compile_error!("Expected `ERROR TYPE` to be a Trait!")
                    }.into();
                }
            };

            quote! {
                <Self as #err_type>::body(&self).indent(1)
            }
        },
        None => {
            quote! {
                <Self as crate::core::error::AvError>::body(&self).indent(1)
            }
        }
    };

    quote! {
        #input

        impl AvError for #ident {
            fn code(&self) -> String {
                #code.to_string()
            }

            fn title(&self) -> String {
                #title.to_string()
            }
        }

        impl std::fmt::Display for #ident {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                use colored::Colorize;
                use crate::core::error::{color, Indentable, AvError};

                writeln!(f, 
                    "{} -- {}:",
                    format!("{}", self.code()).bold().color(color::ERROR),
                    self.title().color(color::ERROR),
                )?;
        
                write!(
                    f,
                    "{}",
                    #line
                )
            }
        }

        impl std::fmt::Debug for #ident {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                <Self as std::fmt::Display>::fmt(&self, f)
            }
        }

        impl std::error::Error for #ident {}

    }.into()
}

///
/// ## AvError Description
/// Generates a TraceableError::description implementation for
/// a new error struct.
/// 
/// This works exactly the same way as `format!(...)`
/// be sure to include an addition pair of brackets
/// as the contents of this macro must be a tuple expression.
/// 
/// ### Example
/// ```
/// impl TraceableError for MyCustomError {
///     /* . . .  */
///     description!("Invalid option `{}`", self.option.blue());
/// }
/// ```
/// 
/// 
#[proc_macro]
#[allow(non_snake_case)]
pub fn description(attrs : proc_macro::TokenStream) -> proc_macro::TokenStream {

    let args = parse_macro_input!(attrs as syn::ExprTuple);
    quote! {
        fn description(&self) -> String {
            format!#args
        }
    }.into()
}

///
/// ## TraceableError Location
/// Generates a TraceableError::Location implementation for
/// a new error struct.
/// 
/// The contents of this macro must be a reference to
/// field within `self`
/// 
/// ### Example
/// ```
/// impl TraceableError for MyCustomError {
///     location!(&self.location);
///     /* . . .  */
/// }
/// ```
/// 
/// 
#[proc_macro]
pub fn location(attrs : proc_macro::TokenStream) -> proc_macro::TokenStream {

    let args = parse_macro_input!(attrs as syn::ExprReference);
    quote! {
        fn location(&self) -> &crate::core::error::Traceable {
            #args
        }
    }.into()
}