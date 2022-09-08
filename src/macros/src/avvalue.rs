use proc_macro2::TokenStream;
use syn::{Token, parse::ParseStream, Error};
use quote::quote;

pub enum AvValue {
    String(syn::LitStr),
    Integer(syn::LitInt),
    Float(syn::LitFloat),
    Null(syn::Ident),
    Boolean(syn::LitBool),
    AvKeys(syn::punctuated::Punctuated<AvKey, Token![+]>),
    List(syn::punctuated::Punctuated<AvValue, Token![,]>)
}

pub enum AvKey {
    Key(syn::Ident),
    Parameter(syn::token::Brace, syn::Ident)
}

impl AvValue {
    pub fn get_type(&self) -> TokenStream {
        use AvValue::*;
        match self {
            String(_) => quote!{ String },
            Integer(_) => quote!{ i64 },
            Float(_) => quote! { f64 },
            Null(_) => panic!("Null token is not supported for deserialization!"),
            Boolean(_) => quote! { bool },
            AvKeys(_) => quote! { crate::core::keyboard::AvKeys },
            List(_) => panic!("List tokens are not supported for deserialization yet!"),
        }
    }
}

impl syn::parse::Parse for AvKey {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let look = input.lookahead1();
        return match look.peek(syn::Ident) {
            true => {
                Ok(
                    AvKey::Key(input.parse()?)
                )
            },
            false => {
                let content;

                Ok(
                    AvKey::Parameter(
                        syn::braced!(content in input),
                        content.parse()?
                    )
                )
            }
        };
    }
}

impl syn::parse::Parse for AvValue {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        if input.peek(syn::LitBool) {
            return Ok(Self::Boolean(input.parse()?));
        }

        if input.peek(syn::LitInt) {
            return Ok(Self::Integer(input.parse()?));
        }

        if input.peek(syn::LitFloat) {
            return Ok(Self::Float(input.parse()?));
        }

        if input.peek(syn::LitStr) {
            return Ok(Self::String(input.parse()?));
        }

        if input.peek(syn::Ident) {
            let ident : syn::Ident = input.parse()?;
            if ident.to_string().eq("null") {
                return Ok(Self::Null(ident));
            }
        }

        if input.peek(syn::token::Paren) {
            let content;

            let _ = syn::parenthesized!(content in input);

            return Ok(Self::AvKeys(content.parse_terminated(AvKey::parse)?))
        }

        if input.peek(syn::token::Bracket) {
            let content;

            let _ = syn::bracketed!(content in input);

            return Ok(Self::List(content.parse_terminated(AvValue::parse)?));
        }

        // let mut e = Diagnostic::new(Level::Error, "Expected a bool, float/int, string, null, or AvKey collection.");
        // e.set_spans(input.span().unwrap());
        // e.emit();

        Err(
            Error::new(input.span(), "Expected a bool, float/int, string, null, or AvKey collection.")
        )
    }
}