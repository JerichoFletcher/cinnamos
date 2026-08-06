use proc_macro_error::proc_macro_error;
use syn::parse_macro_input;

extern crate proc_macro;

mod syntax;
mod sys;

#[proc_macro]
#[proc_macro_error]
pub fn gen_syscall_dispatch(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input);
    sys::expand(input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}
