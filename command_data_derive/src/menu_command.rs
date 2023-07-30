use proc_macro2::{TokenStream as TokenStream2, TokenStream};
use proc_macro_error::abort;
use quote::{quote, quote_spanned};
use syn::{Attribute, DataEnum, Ident, LitStr};

pub fn menu_impl(ty: &Ident, data: DataEnum, attrs: Vec<Attribute>) -> TokenStream2 {
    let mut enm = Enum { skip_display: false };
    attrs.iter()
        .filter(|a| a.path.is_ident("menu"))
        .for_each(|a| enm.handle_attribute(a));

    let variants: Vec<_> = data.variants
        .into_iter()
        .map(Variant::from)
        .collect();

    let into_branches = variants.iter().map(|v| {
        let ident = &v.ident;
        let value = &v.ident_str;
        let label = v.display();
        let desc = v.desc();
        // todo allow for setting description, emoji, etc
        quote_spanned! { v.ident.span() =>
            Self::#ident => ::discorsd::model::components::SelectOption {
                label: #label.to_string(),
                value: #value.to_string(),
                description: #desc,
                emoji: ::std::option::Option::None,
                default: false,
            }
        }
    });

    let all = variants.iter().map(|v| {
        let ident = &v.ident;
        quote! { Self::#ident }
    });

    let options = variants.iter().map(|v| {
        let value = &v.ident_str;
        let label = v.display();
        let desc = v.desc();
        // todo allow for setting description, emoji, etc
        quote_spanned! { v.ident.span() =>
            ::discorsd::model::components::SelectOption {
                label: #label.to_string(),
                value: #value.to_string(),
                description: #desc,
                emoji: ::std::option::Option::None,
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

    let display_impl = if enm.skip_display {
        TokenStream::new()
    } else {
        let display_branches = variants.iter().map(|v| {
            let ident = &v.ident;
            let display = v.display();
            quote_spanned! { v.ident.span() =>
                Self::#ident => f.write_str(#display)
            }
        });
        quote! {
            impl ::std::fmt::Display for #ty {
                fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                    match self {
                        #(#display_branches,)*
                    }
                }
            }
        }
    };

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

        #display_impl

        impl ::discorsd::commands::MenuData for #ty {
            type Data = ::std::string::String;

            fn into_option(self) -> ::discorsd::model::components::SelectOption {
                match self {
                    #(#into_branches),*
                }
            }
            fn all() -> ::std::vec::Vec<Self> {
                vec![#(#all),*]
            }
            fn options() -> ::std::vec::Vec<::discorsd::model::components::SelectOption> {
                vec![
                    #(#options),*
                ]
            }
        }
    }
}

pub struct Enum {
    pub skip_display: bool,
}

#[derive(Debug)]
pub struct Variant {
    ident: Ident,
    ident_str: String,
    pub label: Option<LitStr>,
    pub description: Option<LitStr>,
}

impl Variant {
    fn display(&self) -> String {
        if let Some(lit) = &self.label {
            lit.value()
        } else {
            self.ident.to_string()
        }
    }

    fn desc(&self) -> TokenStream2 {
        if let Some(desc) = &self.description {
            let desc = desc.value();
            quote! { ::std::option::Option::Some(#desc.into()) }
        } else {
            quote! { ::std::option::Option::None }
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
            description: None,
        };
        attrs.iter()
            .filter(|a| a.path.is_ident("menu"))
            .for_each(|a| variant.handle_attribute(a));
        variant
    }
}