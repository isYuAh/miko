// use miko_core::IntoMethods;
// use proc_macro::TokenStream;
// use quote::quote;
// use syn::parse_macro_input;

// use crate::route::RouteAttr;

mod toolkit;
mod route;

// #[proc_macro_attribute]
// pub fn route(attr: TokenStream, item: TokenStream) -> TokenStream {
//   let args = parse_macro_input!(attr as RouteAttr);
//   let path = args.path;
//   let methods = args.method.unwrap_or("get".to_string()).into_methods();
//   quote! {}.into()
// }