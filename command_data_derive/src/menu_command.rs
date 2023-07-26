use proc_macro2::TokenStream as TokenStream2;
use proc_macro_error::abort;
use quote::{quote, quote_spanned};
use syn::{DataEnum, Ident, LitStr};

pub fn menu_impl(ty: &Ident, data: DataEnum) -> TokenStream2 {
    let variants: Vec<_> = data.variants
        .into_iter()
        .map(Variant::from)
        .collect();

    let options = variants.iter().map(|v| {
        let value = &v.ident_str;
        let label = v.display();
        // todo allow for setting description, emoji, etc
        quote_spanned! { v.ident.span() =>
            ::discorsd::model::components::SelectOption {
                label: #label.to_string(),
                value: #value.to_string(),
                description: std::option::Option::None,
                emoji: std::option::Option::None,
                default: false,
            }
        }
    });

    let from_str_branches = variants.iter().map(|v| {
        let str = &v.ident_str;
        let ident = &v.ident;
        quote_spanned! { v.ident.span() =>
            #str => ::std::result::Result::Ok(Self::#ident)
        }
    });

    let display_branches = variants.iter().map(|v| {
        let ident = &v.ident;
        let display = v.display();
        quote_spanned! { v.ident.span() =>
            Self::#ident => f.write_str(#display)
        }
    });

    quote! {
        impl ::std::str::FromStr for #ty {
            type Err = ::std::boxed::Box<str>;

            fn from_str(s: &str) -> ::std::result::Result<Self, Self::Err> {
                match s {
                    #(#from_str_branches,)*
                    bad => ::std::result::Result::Err(bad.into())
                }
            }
        }

        impl ::std::fmt::Display for #ty {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                match self {
                    #(#display_branches,)*
                }
            }
        }

        impl ::discorsd::commands::MenuData for #ty {
            type Data = ::std::string::String;

            fn options() -> ::std::vec::Vec<::discorsd::model::components::SelectOption> {
                vec![
                    #(#options),*
                ]
            }
        }
    }
}

#[derive(Debug)]
pub struct Variant {
    ident: Ident,
    ident_str: String,
    pub label: Option<LitStr>,
}

impl Variant {
    fn display(&self) -> String {
        if let Some(lit) = &self.label {
            lit.value()
        } else {
            self.ident.to_string()
        }
    }
}

impl From<syn::Variant> for Variant {
    fn from(variant: syn::Variant) -> Self {
        if !variant.fields.is_empty() {
            abort!(variant, "Menu variants can't have fields")
        }
        if variant.discriminant.is_some() {
            abort!(variant, "Menu variants can't have discriminants (ex, `= 1`)")
        }
        let attrs = variant.attrs;
        let ident_str = variant.ident.to_string();
        let mut variant = Self {
            ident: variant.ident,
            ident_str,
            label: None,
        };
        attrs.iter()
            .filter(|a| a.path.is_ident("menu"))
            .for_each(|a| variant.handle_attribute(a));
        variant
    }
}