use proc_macro2::TokenStream as TokenStream2;
// use proc_macro_error::*;
use quote::{quote, quote_spanned};
use syn::{DataEnum, Ident};
use syn::spanned::Spanned;

pub fn menu_impl(ty: &Ident, data: DataEnum) -> TokenStream2 {
    let options = data.variants.iter().map(|v| {
        let value = v.ident.to_string();
        quote_spanned! { v.span() =>
            ::discorsd::model::components::SelectOption {
                label: #value.to_string(),
                value: #value.to_string(),
                description: std::option::Option::None,
                emoji: std::option::Option::None,
                default: false,
            }
        }
    });

    let branches_from = data.variants.iter().map(|v| {
        let str = v.ident.to_string();
        let ident = &v.ident;
        quote_spanned! { v.span() =>
            #str => std::option::Option::Some(Self::#ident)
        }
    });

    let branches_into = data.variants.iter().map(|v| {
        let ident = &v.ident;
        let str = v.ident.to_string();
        quote_spanned! { v.span() =>
            Self::#ident => #str.to_string()
        }
    });

    let tokens = quote! {
        impl ::discorsd::commands::MenuData for #ty {
            fn options() -> Vec<::discorsd::model::components::SelectOption> {
                vec![
                    #(#options),*
                ]
            }

            fn from_string(string: String) -> Option<Self> {
                match string.as_str() {
                    #(#branches_from,)*
                    _ => std::option::Option::None,
                }
            }

            fn into_string(self) -> String {
                match self {
                    #(#branches_into,)*
                }
            }
        }
    };
    tokens
}