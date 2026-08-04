use heck::ToSnakeCase;
use proc_macro_error::proc_macro_error;
use proc_macro2::{Span, TokenStream};
use quote::{ToTokens, quote};
use syn::{
    Ident, Token, Type, parse::Parse, parse_macro_input, parse2, punctuated::Punctuated,
    token::Paren,
};

extern crate proc_macro;

struct Arg {
    ident: Ident,
    colon: Token![:],
    ty: Type,
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

struct SyscallMeta {
    var_ident: Ident,
    _paren: Paren,
    params: Punctuated<Arg, Token![,]>,
    _rarrow: Token![->],
    ret_ty: Type,
}

impl Parse for SyscallMeta {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let content;
        Ok(Self {
            var_ident: input.parse()?,
            _paren: syn::parenthesized!(content in input),
            params: Punctuated::parse_terminated(&content)?,
            _rarrow: input.parse()?,
            ret_ty: input.parse()?,
        })
    }
}

struct EnumValue {
    ty_ident: Ident,
    path_sep: Token![::],
    val_ident: Ident,
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

struct SysMeta {
    sys_ty: Type,
    _comma: Token![,],
    err_val: EnumValue,
    _semi: Token![;],
    syscalls: Punctuated<SyscallMeta, Token![;]>,
}

impl Parse for SysMeta {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        Ok(Self {
            sys_ty: input.parse()?,
            _comma: input.parse()?,
            err_val: input.parse()?,
            _semi: input.parse()?,
            syscalls: Punctuated::parse_terminated(input)?,
        })
    }
}

fn generate_slice_cast_from_usize(slice: &Ident, idx: usize, ty: &Type) -> TokenStream {
    match ty {
        Type::Path(p) if p.path.is_ident("usize") => quote! { #slice[#idx] },
        Type::Ptr(_) => quote! { #slice[#idx] as #ty },
        _ => quote! { #slice[#idx].into() },
    }
}

fn generate_cast_to_usize(ident: &Ident, ty: &Type) -> TokenStream {
    match ty {
        Type::Path(p) if p.path.is_ident("usize") => ident.to_token_stream(),
        Type::Ptr(_) => quote! { #ident as usize },
        _ => quote! { #ident.into() },
    }
}

#[proc_macro]
#[proc_macro_error]
pub fn gen_syscall_dispatch(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input);
    expand(input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

fn expand(input: TokenStream) -> syn::Result<TokenStream> {
    let meta = parse2::<SysMeta>(input)?;
    let sys_ty = &meta.sys_ty;
    let err_val = &meta.err_val;
    let err_ty = &err_val.ty_ident;
    let mut match_branches = vec![];

    let arg_slice = Ident::new("args", Span::call_site());
    for m in meta.syscalls.iter() {
        let sys_val = &m.var_ident;
        let handler_name = Ident::new(&sys_val.to_string().to_snake_case(), Span::call_site());
        let ret_ty = &m.ret_ty;
        let args = m
            .params
            .iter()
            .enumerate()
            .map(|(i, a)| generate_slice_cast_from_usize(&arg_slice, i, &a.ty))
            .collect::<Vec<_>>();
        match_branches.push(match ret_ty {
            Type::Never(_) => quote! {
                #sys_ty::#sys_val => { #handler_name(#(#args),*); }
            },
            Type::Tuple(t) if t.elems.is_empty() => quote! {
                #sys_ty::#sys_val => #handler_name(#(#args),*).map(|_| 0)
            },
            Type::Path(p) if p.path.is_ident("usize") => quote! {
                #sys_ty::#sys_val => #handler_name(#(#args),*)
            },
            _ => {
                let ret_name = Ident::new("val", Span::call_site());
                let ret_cast = generate_cast_to_usize(&ret_name, ret_ty);
                quote! {
                    #sys_ty::#sys_val => #handler_name(#(#args),*).map(|#ret_name| #ret_cast)
                }
            }
        });
    }

    let doc = [
        "# Safety".to_string(),
        format!(
            "Each argument passed in `{}` has to be convertible to their corresponding argument types for the syscall.",
            arg_slice
        ),
    ];
    Ok(quote! {
        #[inline]
        #(#[doc = #doc])*
        pub unsafe fn dispatch_syscall(sys: #sys_ty, #arg_slice: &[usize; 6]) -> Result<usize, #err_ty> {
            match sys {
                #(#match_branches),*,
                _ => Err(#err_val),
            }
        }
    })
}
