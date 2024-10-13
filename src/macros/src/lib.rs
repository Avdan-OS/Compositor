#![feature(proc_macro_diagnostic)]

mod avvalue;
mod delcaration;

use delcaration::{AvMacro, ConfigDelcaration};

use proc_macro::{Diagnostic, Level};

use proc_macro2::{Ident, TokenStream};

use quote::quote;

use syn::{
    bracketed, parse::ParseBuffer, parse_macro_input, punctuated::Punctuated, AttributeArgs,
    ExprReference, ExprTuple, ItemStruct, Lit, LitStr, Meta, NestedMeta, Token, TypePath,
};

#[proc_macro]
#[allow(non_snake_case)]
pub fn AvValue(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    struct AvValueDeclaration {
        types: Punctuated<syn::TypePath, Token![,]>,
    }

    impl syn::parse::Parse for AvValueDeclaration {
        fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
            let content: ParseBuffer;

            let _ = bracketed!(content in input);

            Ok(Self {
                types: content.parse_terminated(syn::TypePath::parse)?,
            })
        }
    }

    let t: AvValueDeclaration = parse_macro_input!(input as AvValueDeclaration);

    let iter = t.types.iter();
    let variants = iter.clone().map(|t: &TypePath| {
        quote! {
            #t(#t)
        }
    });

    let impls = iter;

    let impls = impls.map(|t: &TypePath| {
        quote! {
            impl<'a> core::convert::TryFrom<&'a AvValue> for #t {
                type Error = ();

                fn try_from(value: &'a AvValue) -> Result<Self, Self::Error> {
                    match value {
                        AvValue::#t(k) => Ok(k.clone()),
                        _ => Err(())
                    }
                }
            }
        }
    });

    quote! {
        #[derive(Debug, PartialEq,)]
        #[allow(non_camel_case_types)]

        pub enum AvValue { #(#variants),* }

        #(#impls)*
    }
    .into()
}

#[proc_macro]
pub fn traceable(_: proc_macro::TokenStream) -> proc_macro::TokenStream {
    quote! {
        crate::core::error::Traceable::new (
            file!().to_string(),
            (line!() as usize, column!() as usize)
        )
    }
    .into()
}

#[proc_macro]
pub fn config_section(struc: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let parsed: ConfigDelcaration = parse_macro_input!(struc as ConfigDelcaration);

    // fs::write("./test.txt", format!("{:?}", parsed)).unwrap();

    let ident: Ident = parsed.ident;

    let mut q: TokenStream = quote! {};

    let iter = parsed.fields.iter();

    for field in iter.clone() {
        let (name, _): (String, _) = field.av_macro();
        let comments: String = field.description();

        let n: Ident = syn::Ident::new(&name, ident.span());

        let mut lines: TokenStream = quote! {};

        for l in comments.lines() {
            let tkns: proc_macro2::TokenStream = format!("/// {}", l.trim_start()).parse().unwrap();
            lines = quote! {
                #lines
                #tkns
            }
        }

        let typ: TokenStream = field.default().get_type();

        let z: TokenStream = quote! {
            #q
            #lines
            pub #n : #typ,
        };

        q = z;
    }

    let macro_registration = iter.clone().map(|m: &AvMacro| {
        let v: TokenStream = m.default().value();
        let (m_ident, m_params): (String, Vec<String>) = m.av_macro();
        let av_mac_raw: String = format!(
            "{m_ident}{}",
            match m_params.len() {
                0 => format!(""),
                _ => format!("({})", m_params.join(",")),
            }
        );

        quote! {
            let m = AvMacro::parse (
                traceable!(),
                // Insert full macro as string
                #av_mac_raw.to_string()
            ).unwrap();

            declared.insert(
                m.clone(),
                #v
            );

            ids.insert(#m_ident, m);
        }
    });

    let macro_idents = iter.map(|m: &AvMacro| m.av_macro().0).map(|k: String| {
        let n: Ident = syn::Ident::new(&k, ident.span());
        quote! {
            #n: m.get(ids.get(#k).unwrap()).unwrap().try_into().unwrap(),
        }
    });

    quote! {
        use compositor_macros::traceable;

        use crate::{
            config::{
                templating::{
                    AvMacro,
                    AvValue,
                },
            },
            core::keyboard::{
                AvKey,
                AvKeys,
            },
        };

        use std::collections::HashMap;

        #[allow(non_snake_case)]
        #[derive(Debug)]
        pub struct #ident {
           #q
        }

        impl<'de> serde::de::Deserialize<'de> for #ident {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
                where D: serde::Deserializer<'de>
            {
                let mut ids = HashMap::new();
                let declared = {
                    let mut declared: HashMap<AvMacro, AvValue> = HashMap::new();

                    #(#macro_registration)*

                    declared
                };

                let raw: HashMap<String, serde_json::Value> = serde::de::Deserialize::deserialize(deserializer)?;

                let m = <Self as ConfigurationSection>::from_map(declared, raw);

                Ok (
                    #ident { #(#macro_idents)* }
                )
            }
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
#[allow(non_snake_case)]
pub fn AvError(
    attributes: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let a: Vec<NestedMeta> = parse_macro_input!(attributes as AttributeArgs);

    let (parent, code, title): (Option<&NestedMeta>, &NestedMeta, &NestedMeta) = match a.len() {
        2 => (None, a.get(0).unwrap(), a.get(1).unwrap()),

        3 => (
            Some(a.get(0).unwrap()),
            a.get(1).unwrap(),
            a.get(2).unwrap(),
        ),

        _ => {
            return quote::quote! {
                compile_error!("Expected two or three elements: [`TYPE`], `ERROR CODE`, `TITLE`")
            }
            .into();
        }
    };

    let (code, title): (&Meta, &Lit) = match (code, title) {
        (syn::NestedMeta::Meta(l1), syn::NestedMeta::Lit(l2)) => (l1, l2),
        _ => unimplemented!("Code and title in invalid format!"),
    };

    let code: &syn::Path = match code {
        syn::Meta::Path(p) => p,
        _ => {
            return quote::quote! {
                compile_error!("Expected `ERROR CODE` to be a raw identifier (no \"\")")
            }
            .into();
        }
    };

    let title: &LitStr = match title {
        syn::Lit::Str(tkn) => tkn,
        _ => {
            return quote::quote! {
                compile_error!("Invalid format for title!")
            }
            .into();
        }
    };

    let code = match code.get_ident() {
        None => {
            return quote::quote! {
                compile_error!("`ERROR CODE` needs to be a raw identifier (no ::)")
            }
            .into();
        }

        Some(t) => t,
    };

    // // I wish diagnostics were on the stable channel :(
    // // 'Linting' the error code.
    match code.to_string() {
        s if !s.is_ascii() => {
            let mut w = Diagnostic::new(Level::Warning, "Error Codes should be ASCII only ");
            w.set_spans(code.span().unwrap());
            w.emit();
        }

        s if s.to_ascii_uppercase() != s => {
            let mut w = Diagnostic::new(Level::Warning, "Error Codes should be in TRAIN_CASE");
            w.set_spans(code.span().unwrap());
            w.emit();
        }

        s if s.ends_with("_") => {
            let mut w = Diagnostic::new(Level::Warning, "Remove the trailing `_`");
            w.set_spans(code.span().unwrap());
            w.emit();
        }

        _ => {}
    }

    let code: LitStr = LitStr::new(&code.to_string(), code.span());

    let input: ItemStruct = parse_macro_input!(input as ItemStruct);

    let ident: Ident = input.ident.clone();

    // if the parent error attribute is defined,
    // use it as the base trait instead of the default AvError
    // for AvError::body()...
    let line: TokenStream = match parent {
        Some(err_type) => {
            let err_type: &Meta = match err_type {
                syn::NestedMeta::Meta(l) => l,
                _ => {
                    (return quote::quote! {
                        compile_error!("`ERROR TYPE` needs to be a raw identifier (no ::)")
                    }
                    .into());
                }
            };

            let err_type: &syn::Path = match err_type {
                syn::Meta::Path(tkn) => tkn,

                _ => {
                    (return quote::quote! {
                        compile_error!("Expected `ERROR TYPE` to be a Trait!")
                    }
                    .into());
                }
            };

            quote! {
                <Self as #err_type>::body(&self).indent(1)
            }
        }

        None => {
            quote! {
                <Self as crate::core::error::AvError>::body(&self).indent(1)
            }
        }
    };

    quote! {
        #[derive(Clone)]
        #input

        impl AvError for #ident {
            fn code(&self) -> String { #code.to_string() }

            fn title(&self) -> String { #title.to_string() }
        }

        impl std::fmt::Display for #ident {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                use colored::Colorize;

                use crate::core::error::{
                    AvError,
                    color,
                    Indentable,
                };

                writeln! (
                    f,
                    "{} -- {}:",
                    format!("{}", self.code()).bold().color(color::ERROR),
                    self.title().color(color::ERROR),
                )?;

                write! (
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
    }
    .into()
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
pub fn description(attrs: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let args: ExprTuple = parse_macro_input!(attrs as syn::ExprTuple);

    quote! {
        fn description(&self) -> String { format!#args }
    }
    .into()
}

#[proc_macro]
pub fn name(attrs: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let args: LitStr = parse_macro_input!(attrs as syn::LitStr);
    quote! {
        fn name<'a>(&self) -> &'a str { #args }
    }
    .into()
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
pub fn location(attrs: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let args: ExprReference = parse_macro_input!(attrs as syn::ExprReference);

    quote! {
        fn location(&self) -> &crate::core::error::Traceable {
            #args
        }
    }
    .into()
}
