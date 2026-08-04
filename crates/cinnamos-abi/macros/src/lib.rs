use heck::ToSnakeCase;
use proc_macro_error::{Diagnostic, Level, abort, proc_macro_error};
use proc_macro2::{Span, TokenStream};
use quote::{ToTokens, quote};
use syn::{
    Fields, Ident, ItemEnum, Token, Type, parse::Parse, parse_macro_input, parse2,
    punctuated::Punctuated, spanned::Spanned,
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

fn generate_cast_to_usize(ident: &Ident, ty: &Type) -> TokenStream {
    match ty {
        Type::Path(p) if p.path.is_ident("usize") => ident.to_token_stream(),
        Type::Ptr(_) => quote! { #ident as usize },
        _ => quote! { #ident.into() },
    }
}

#[proc_macro_derive(SyscallTable, attributes(err, args, returns))]
#[proc_macro_error]
pub fn syscall_table_derive(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let item = parse_macro_input!(item);
    expand(item)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

fn expand(item: TokenStream) -> syn::Result<TokenStream> {
    let mut funcs = vec![];
    let mut metas = vec![];
    let top_enum = parse2::<ItemEnum>(item)?;

    let mut err_val = top_enum
        .attrs
        .iter()
        .filter(|attr| attr.path().is_ident("err"));
    let err_val = match (err_val.next(), err_val.next()) {
        (None, _) => abort!(
            top_enum,
            "#[err(...)] attribute required on enum declaration"
        ),
        (Some(attr), None) => attr.parse_args::<EnumValue>()?,
        (Some(first), Some(second)) => Diagnostic::spanned(
            second.span(),
            Level::Error,
            "duplicate #[err(...)] attributes".to_string(),
        )
        .span_note(first.span(), "first attribute found here".to_string())
        .abort(),
    };

    let name = &top_enum.ident;
    for v in top_enum.variants {
        let Fields::Unit = v.fields else {
            abort!(
                v,
                "only unit-variants allowed";
                help = "remove associated data declaration";
            )
        };

        let mut params = v.attrs.iter().filter(|attr| attr.path().is_ident("args"));
        let params = match (params.next(), params.next()) {
            (None, _) => None,
            (Some(attr), None) => {
                Some(attr.parse_args_with(Punctuated::<Arg, Token![,]>::parse_terminated)?)
            }
            (Some(first), Some(second)) => Diagnostic::spanned(
                second.span(),
                Level::Error,
                "duplicate #[args(...)] attributes".to_string(),
            )
            .span_note(first.span(), "first attribute found here".to_string())
            .abort(),
        }
        .unwrap_or_default();

        let mut ret_ty = v
            .attrs
            .iter()
            .filter(|attr| attr.path().is_ident("returns"));
        let ret_ty = match (ret_ty.next(), ret_ty.next()) {
            (None, _) => None,
            (Some(attr), None) => Some(attr.parse_args_with(Type::parse)?),
            (Some(first), Some(second)) => Diagnostic::spanned(
                second.span(),
                Level::Error,
                "duplicate #[returns(...)] attributes".to_string(),
            )
            .span_note(first.span(), "first attribute found here".to_string())
            .abort(),
        };

        let call_name = Ident::new(
            match params.len() {
                0 => "syscall0",
                1 => "syscall1",
                2 => "syscall2",
                3 => "syscall3",
                4 => "syscall4",
                5 => "syscall5",
                6 => "syscall6",
                _ => abort! {
                    params,
                    "too many syscall arguments ({})", params.len();
                    help = "maximum number of arguments is 6";
                },
            },
            Span::call_site(),
        );

        let func_name = Ident::new(&v.ident.to_string().to_snake_case(), Span::call_site());
        let variant_name = &v.ident;
        let params = params.into_iter().collect::<Vec<_>>();
        let args = params
            .iter()
            .map(|a| generate_cast_to_usize(&a.ident, &a.ty))
            .collect::<Vec<_>>();
        funcs.push(match &ret_ty {
            Some(ret_ty) => match &ret_ty {
                Type::Never(_) => {
                    let noret_errmsg = format!("no-return syscall {} returned to caller", func_name);
                    quote! {
                        #[inline]
                        pub unsafe fn #func_name(#(#params),*) -> ! {
                            let _ = crate::abi::#call_name(Self::#variant_name, #(#args),*);
                            unreachable!(#noret_errmsg)
                        }
                    }
                },
                Type::Path(p) if p.path.is_ident("usize") => quote! {
                    #[inline]
                    pub unsafe fn #func_name(#(#params),*) -> Result<#ret_ty, SyscallError> {
                        crate::abi::#call_name(Self::#variant_name, #(#args),*)
                    }
                },
                _ => {
                    let ret_name = Ident::new("val", Span::call_site());
                    let ret_cast = generate_cast_to_usize(&ret_name, ret_ty);
                    quote! {
                        #[inline]
                        pub unsafe fn #func_name(#(#params),*) -> Result<#ret_ty, SyscallError> {
                            crate::abi::#call_name(Self::#variant_name, #(#args),*).map(|#ret_name| #ret_cast)
                        }
                    }
                },
            },
            None => quote! {
                #[inline]
                pub unsafe fn #func_name(#(#params),*) -> Result<(), SyscallError> {
                    crate::abi::#call_name(Self::#variant_name, #(#args),*)?;
                    Ok(())
                }
            },
        });

        metas.push(match &ret_ty {
            Some(ret_ty) => quote! {
                #variant_name(#(#params),*) -> #ret_ty;
            },
            None => quote! {
                #variant_name(#(#params),*) -> ();
            },
        });
    }

    Ok(quote! {
        #[cfg(not(target_os = "none"))]
        impl #name {
            #(#funcs)*
        }

        #[doc(hidden)]
        #[macro_export]
        macro_rules! __syscall_meta {
            ($m:ident) => {
                $m! {
                    #name, #err_val;
                    #(#metas)*
                }
            };
        }
    })
}
