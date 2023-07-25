// use proc_macro2::{Ident, TokenStream as TokenStream2};
// use quote::quote;
// use syn::{AttributeArgs, FnArg, GenericArgument, ItemFn, Lit, LitStr, Meta, MetaNameValue, NestedMeta, PathArguments, PatType, Type};
//
// pub fn button_impl(attr: AttributeArgs, input: ItemFn) -> TokenStream2 {
//     let args = Args::new(attr);
//     println!("args = {:?}", args);
//     let command_ident = &input.sig.ident;
//     let command_label = args.label
//         .map(|l| l.value())
//         .unwrap_or_else(|| command_ident.to_string());
//     let bot_type = get_bot_type(input.sig.inputs.iter());
//     let braced_body = &input.block;
//
//     quote! {
//         #[derive(Clone, Debug)]
//         #[allow(non_camel_case_types)]
//         pub struct #command_ident;
//
//         #[::discorsd::async_trait]
//         impl ::discorsd::commands::ButtonCommand for #command_ident {
//             type Bot = #bot_type;
//
//             fn label(&self) -> ::std::string::String {
//                 #command_label.into()
//             }
//
//             async fn run(
//                 &self,
//                 state: ::std::sync::Arc<::discorsd::BotState<Self::Bot>>,
//                 interaction: ::discorsd::model::commands::InteractionUse<::discorsd::model::commands::ButtonPressData, ::discorsd::model::commands::Unused>
//             ) -> ::std::result::Result<::discorsd::model::commands::InteractionUse<::discorsd::model::commands::ButtonPressData, ::discorsd::model::commands::Used>, ::discorsd::errors::BotError>
//             #braced_body
//         }
//     }
// }
//
// #[derive(Default, Debug)]
// struct Args {
//     label: Option<LitStr>,
//     rename: Option<LitStr>,
// }
//
// impl Args {
//     fn new(args: AttributeArgs) -> Self {
//         let mut this = Self::default();
//         for attr in args {
//             match attr {
//                 NestedMeta::Meta(Meta::NameValue(MetaNameValue { path, eq_token: _eq_token, lit, })) => {
//                     let key = path.get_ident()
//                         .expect("paths are not supported")
//                         .to_string();
//                     let key = key.as_str();
//                     match key {
//                         "label" => {
//                             let Lit::Str(label) = lit else {
//                                 unreachable!("label only supports literal string arguments")
//                             };
//                             this.label = Some(label);
//                         },
//                         "rename" => todo!(),
//                         "disabled" => todo!(),
//                         _ => unreachable!(),
//                     }
//                 }
//                 _ => todo!(),
//             }
//         }
//         this
//     }
// }
//
// fn get_bot_type<'a>(punctuated: impl Iterator<Item=&'a FnArg>) -> Type {
//     // find 'state'
//     let arg = punctuated
//         .filter_map(|arg| match arg {
//             FnArg::Receiver(_) => None,
//             FnArg::Typed(pat_type) => Some(pat_type),
//         })
//         .find(|PatType { attrs, .. }|
//             attrs.iter()
//                 .position(|a| a.path.segments.first().unwrap().ident.to_string() == "state")
//                 .is_some()
//         ).expect("Needs a `#[state]` argument");
//     let Type::Path(path) = arg.ty.as_ref() else {
//         unreachable!("Arc path")
//     };
//     let arc_args = &path.path.segments.last()
//         .expect("Arc")
//         .arguments;
//     let PathArguments::AngleBracketed(args) = arc_args else {
//         unreachable!("Arc generics")
//     };
//     assert_eq!(args.args.len(), 1);
//     let bot_state = args.args.first()
//         .expect("BotState");
//     let GenericArgument::Type(Type::Path(path)) = bot_state else {
//         unreachable!("BotState is a type")
//     };
//     let bot_state_args = &path.path.segments.last()
//         .expect("BotState")
//         .arguments;
//     let PathArguments::AngleBracketed(args) = bot_state_args else {
//         unreachable!("Arc generics")
//     };
//     assert_eq!(args.args.len(), 1);
//     let bot_type = args.args.first()
//         .expect("Bot type");
//     let GenericArgument::Type(t) = bot_type else {
//         unreachable!("BotState generics")
//     };
//     t.clone()
// }