use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::{Ident, Token, Type, parse::Parse};

pub struct Arg {
    pub ident: Ident,
    pub colon: Token![:],
    pub ty: Type,
}

impl Parse for Arg {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        Ok(Self {
            ident: input.parse()?,
            colon: input.parse()?,
            ty: input.parse()?,
        })
    }
}

impl ToTokens for Arg {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.ident.to_tokens(tokens);
        self.colon.to_tokens(tokens);
        self.ty.to_tokens(tokens);
    }
}

pub struct EnumValue {
    pub ty_ident: Ident,
    pub path_sep: Token![::],
    pub val_ident: Ident,
}

impl Parse for EnumValue {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        Ok(Self {
            ty_ident: input.parse()?,
            path_sep: input.parse()?,
            val_ident: input.parse()?,
        })
    }
}

impl ToTokens for EnumValue {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.ty_ident.to_tokens(tokens);
        self.path_sep.to_tokens(tokens);
        self.val_ident.to_tokens(tokens);
    }
}
