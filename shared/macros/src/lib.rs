use argon2::{Algorithm, Argon2, Params, Version};
use proc_macro::TokenStream;
use quote::quote;
use syn::parse_macro_input;

const M_COST: u32 = 19456;
const T_COST: u32 = 8;
const P_COST: u32 = 4;
const OUTPUT_LEN: usize = 32;

#[proc_macro]
pub fn key(input: TokenStream) -> TokenStream {
    let input_str = parse_macro_input!(input as syn::LitStr).value();
    let value = std::env::var(&input_str).unwrap_or_else(|_| input_str.clone());
    let pwd = value.as_bytes();
    let salt = b"PASETO_KEY_PASSWORD";
    let argon2 = argon2();
    let mut key = [0u8; OUTPUT_LEN];
    if argon2.hash_password_into(pwd, salt, &mut key).is_ok() {
        let byte_literals = key.iter();
        return quote! { [#(#byte_literals),*] }.into();
    }
    quote! { compile_error!("could not hash the password to generate the key") }.into()
}

fn argon2() -> Argon2<'static> {
    Argon2::new(Algorithm::Argon2id, Version::V0x13, params())
}

fn params() -> Params {
    Params::new(M_COST, T_COST, P_COST, Some(OUTPUT_LEN)).unwrap()
}
