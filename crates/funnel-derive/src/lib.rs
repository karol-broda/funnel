use proc_macro::TokenStream;
use quote::quote;
use syn::{DeriveInput, parse_macro_input};

/// derives `Enveloped` by converting the struct name to snake_case.
///
/// `User` becomes `"user"`, `TunnelInfo` becomes `"tunnel_info"`.
///
/// override with `#[kind = "custom"]` if the default doesn't fit.
#[proc_macro_derive(Enveloped, attributes(kind))]
pub fn derive_enveloped(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    let kind = extract_kind_attr(&input).unwrap_or_else(|| to_snake_case(&name.to_string()));

    let expanded = quote! {
        impl crate::api::envelope::Enveloped for #name {
            const KIND: &'static str = #kind;
        }
    };

    TokenStream::from(expanded)
}

fn extract_kind_attr(input: &DeriveInput) -> Option<String> {
    for attr in &input.attrs {
        if attr.path().is_ident("kind") {
            let value: syn::LitStr = attr.parse_args().ok()?;
            return Some(value.value());
        }
    }
    None
}

fn to_snake_case(name: &str) -> String {
    let mut result = String::with_capacity(name.len() + 4);
    for (i, ch) in name.chars().enumerate() {
        if ch.is_uppercase() {
            if i > 0 {
                result.push('_');
            }
            result.push(ch.to_lowercase().next().unwrap_or(ch));
        } else {
            result.push(ch);
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snake_case_simple() {
        assert_eq!(to_snake_case("User"), "user");
    }

    #[test]
    fn snake_case_multi_word() {
        assert_eq!(to_snake_case("TunnelInfo"), "tunnel_info");
    }

    #[test]
    fn snake_case_consecutive_caps() {
        assert_eq!(to_snake_case("APIKey"), "a_p_i_key");
    }

    #[test]
    fn snake_case_already_lower() {
        assert_eq!(to_snake_case("tunnel"), "tunnel");
    }

    #[test]
    fn snake_case_single_char() {
        assert_eq!(to_snake_case("A"), "a");
    }
}
