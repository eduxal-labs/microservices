use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, LitStr};

#[proc_macro]
pub fn key(input: TokenStream) -> TokenStream {
    let input_str = parse_macro_input!(input as LitStr).value();
    let bytes = input_str.as_bytes();
    let mut key_bytes = [0u8; 32];
    if !bytes.is_empty() {
        for i in 0..32 {
            key_bytes[i] = bytes[i % bytes.len()];
        }
    }

    let byte_literals = key_bytes.iter();
    let expanded = quote! {
        [#(#byte_literals),*]
    };

    expanded.into()
}
