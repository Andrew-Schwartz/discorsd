use proc_macro2::{Ident, TokenStream};
use proc_macro_error::abort;
use quote::quote;
use syn::{Attribute, DataStruct, Fields, LitStr, Type};
use crate::utils::TypeExt;

#[allow(clippy::module_name_repetitions)]
pub fn modal_impl(ty: &Ident, data: DataStruct, attrs: &[Attribute]) -> TokenStream {
    let strukt = Struct::from(ty, data, attrs);

    let len = strukt.fields.len();
    let title = strukt.title();

    let text_inputs = strukt.fields.iter().map(|f| {
        let ident = &f.ident;
        let name = ident.to_string();
        let new = match f.long {
            false => quote!(new_short),
            true => quote!(new_paragraph),
        };
        let optional = match f.optional {
            None => quote!(),
            Some(_) => quote!(.optional())
        };
        quote! {
            ::discorsd::model::components::TextInput::#new(#name)
                #optional
        }
    });

    let let_vars = strukt.fields.iter().map(|f| {
        let ident = &f.ident;
        let err = format!("missing required field `{ident}`");
        let ty = &f.ty;
        let var = match f.optional {
            None => quote! { Err(#err.into()) },
            Some(_) => quote! { Ok(::core::option::Option::None) }
        };
        quote! {
            let mut #ident = ::core::result::Result::<#ty, _>::#var;
        }
    });

    let match_branches = strukt.fields.iter().enumerate().map(|(i, f)| {
        let ident = &f.ident;
        let ident_str = ident.to_string();
        let ty = &f.ty;
        let parse = |ty| quote! { s.parse::<#ty>().map_err(|e| e.from_str_err(&s, #ident_str)) };
        let val = match &f.optional {
            None => parse(ty),
            Some(ty) => {
                let parse = parse(ty);
                quote! {
                    if s.is_empty() {
                        ::core::result::Result::Ok(::core::option::Option::None)
                    } else {
                        #parse.map(::core::option::Option::Some)
                    }
                }
            },
        };
        quote! {
            #i => #ident = #val,
        }
    });

    let struct_fields = strukt.fields.iter().map(|f| {
        let ident = &f.ident;
        // let question = match f.optional {
        //     true => TokenStream::new(),
        //     false => quote! { ? },
        // };
        quote! {
            #ident: #ident?,
        }
    });

    quote! {
        impl ::discorsd::commands::modal_command::ArrayLen<#len> for #ty {}

        impl #ty {
            pub fn builder() -> ::discorsd::model::interaction_response::ModalBuilder<#len> {
                ::discorsd::model::interaction_response::ModalBuilder::with_inputs(
                    #title,
                    [
                        #(#text_inputs,)*
                    ]
                )
            }
        }

        impl ::discorsd::commands::modal_command::ModalValues for #ty {
            fn from_vec(vec: ::std::vec::Vec<::std::string::String>) -> ::core::result::Result<Self, ::std::borrow::Cow<'static, str>> {
                #(
                    #let_vars
                )*
                for (i, s) in vec.into_iter().enumerate() {
                    match i {
                        #(
                            #match_branches
                        )*
                        _ => unreachable!(),
                    }
                }
                ::core::result::Result::Ok(Self {
                    #(
                        #struct_fields
                    )*
                })
            }
        }
    }
}

pub struct Struct {
    ident: String,
    fields: Vec<Field>,
    pub title: Option<LitStr>,
}

impl Struct {
    fn title(&self) -> String {
        match &self.title {
            None => self.ident.clone(),
            Some(title) => title.value(),
        }
    }

    fn from(ty: &Ident, value: DataStruct, attrs: &[Attribute]) -> Self {
        let Fields::Named(fields) = value.fields else {
            abort!(ty, "only structs with named fields are allowed for modal commands")
        };
        let fields = fields.named.into_iter()
            .map(Field::from)
            .collect();
        let name = {
            let mut raw = ty.to_string();
            let uppercase_indices = raw.rmatch_indices(|c: char| c.is_ascii_uppercase())
                .map(|(i, _)| i)
                .filter(|&i| i != 0)
                .collect::<Vec<_>>();
            for idx in uppercase_indices {
                raw.insert(idx, ' ');
            }
            raw
        };
        let mut this = Self {
            ident: name,
            fields,
            title: None,
        };
        for attr in attrs {
            if !attr.path.is_ident("modal") { continue; }
            this.handle_attribute(attr);
        }
        this
    }
}

pub struct Field {
    pub ident: Ident,
    pub ty: Type,
    pub optional: Option<Type>,
    pub long: bool,
}

impl From<syn::Field> for Field {
    fn from(field: syn::Field) -> Self {
        let mut this = Self {
            ident: field.ident.expect("only named fields"),
            ty: field.ty,
            optional: None,
            long: false,
        };
        for attr in &field.attrs {
            if !attr.path.is_ident("modal") { continue; }
            this.handle_attribute(attr);
        }
        if let Some(ty) = this.ty.generic_type_of("Option") {
            this.optional = Some(ty.clone());
        }
        this
    }
}