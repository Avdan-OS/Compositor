use proc_macro2::TokenStream;

use quote::quote;

use syn::{
    parse::{Lookahead1, ParseBuffer, ParseStream},
    punctuated::Punctuated,
    token::Brace,
    Error, Ident, LitBool, LitFloat, LitInt, LitStr, Result, Token,
};

pub enum AvValue {
    String(LitStr),
    Integer(LitInt),
    Float(LitFloat),
    Null(Ident),
    Boolean(LitBool),
    AvKeys(Punctuated<AvKey, Token![+]>),
    List(Punctuated<AvValue, Token![,]>),
}

pub enum AvKey {
    Key(Ident),
    Parameter(Brace, Ident),
}

impl AvValue {
    pub fn get_type(&self) -> TokenStream {
        use AvValue::*;
        match self {
            String(_) => quote! { String },
            Integer(_) => quote! { i64 },
            Float(_) => quote! { f64 },
            Null(_) => panic!("Null token is not supported for deserialization!"),
            Boolean(_) => quote! { bool },
            AvKeys(_) => quote! { AvKeys },
            List(_) => panic!("List tokens are not supported for deserialization yet!"),
        }
    }

    pub fn value(&self) -> TokenStream {
        let t: TokenStream = self.get_type();
        let v: TokenStream = match self {
            AvValue::String(s) => quote! { #s.into() },
            AvValue::Integer(s) => quote! { #s.into() },
            AvValue::Float(s) => quote! { #s.into() },
            AvValue::Null(_) => panic!("Null token is not supported for deserialization!"),
            AvValue::Boolean(s) => quote! { #s.into() },
            AvValue::AvKeys(s) => {
                let t = s.iter().map(|k: &AvKey| match k {
                    AvKey::Key(k) => {
                        let k: String = k.to_string();

                        quote! {
                            AvKey::Key(#k.into())
                        }
                    }
                    AvKey::Parameter(_, p) => {
                        let p: String = p.to_string();

                        quote! {
                            AvKey::Parameter(#p.try_into().unwrap())
                        }
                    }
                });

                quote! {
                    AvKeys ( vec![#(#t),*] )
                }
            }

            AvValue::List(_) => panic!("List tokens are not supported for deserialization yet!"),
        };

        quote! { AvValue::#t(#v) }
    }
}

impl syn::parse::Parse for AvKey {
    fn parse(input: ParseStream) -> Result<Self> {
        let look: Lookahead1 = input.lookahead1();

        return match look.peek(syn::Ident) {
            true => Ok(AvKey::Key(input.parse()?)),

            false => {
                let content: ParseBuffer;

                Ok(AvKey::Parameter(
                    syn::braced!(content in input),
                    content.parse()?,
                ))
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
            let ident: syn::Ident = input.parse()?;
            if ident.to_string().eq("null") {
                return Ok(Self::Null(ident));
            }
        }

        if input.peek(syn::token::Paren) {
            let content: ParseBuffer;

            let _ = syn::parenthesized!(content in input);

            return Ok(Self::AvKeys(content.parse_terminated(AvKey::parse)?));
        }

        if input.peek(syn::token::Bracket) {
            let content: ParseBuffer;

            let _ = syn::bracketed!(content in input);

            return Ok(Self::List(content.parse_terminated(AvValue::parse)?));
        }

        // let mut e = Diagnostic::new(Level::Error, "Expected a bool, float/int, string, null, or AvKey collection.");
        // e.set_spans(input.span().unwrap());
        // e.emit();
        Err(Error::new(
            input.span(),
            "Expected a bool, float/int, string, null, or AvKey collection.",
        ))
    }
}
