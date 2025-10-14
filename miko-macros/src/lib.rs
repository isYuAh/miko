use crate::route::core::route_handler;
use crate::route::RouteAttr;
use crate::toolkit::rout_arg::IntoFnArgs;
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemFn};

mod toolkit;
mod route;
mod extractor;

#[proc_macro_attribute]
pub fn route(attr: TokenStream, item: TokenStream) -> TokenStream {
  let args = parse_macro_input!(attr as RouteAttr);
  let fn_item = parse_macro_input!(item as ItemFn);
  route_handler(args, fn_item)
}

#[proc_macro_attribute]
pub fn miko(_attr: TokenStream, item: TokenStream) -> TokenStream {
  let input_fn = parse_macro_input!(item as ItemFn);
  let user_statements = &input_fn.block.stmts;
  quote! {
    #[::tokio::main]
    async fn main() {
      let mut _config = ::miko::config::config::ApplicationConfig::load_().unwrap_or_default();
      let mut router = Router::new();

      #( #user_statements )*

      router.merge(::miko::auto::collect_global_router());
      let app = ::miko::application::Application::new(_config, router.take());
      app.run().await.unwrap();
    }
  }.into()
}
macro_rules! derive_route_macro {
    ($macro_name: ident, $method_ident:ident) => {
        #[proc_macro_attribute]
        pub fn $macro_name(attr: TokenStream, item: TokenStream) -> TokenStream {
            let mut args = syn::parse_macro_input!(attr as RouteAttr);
            let fn_item = syn::parse_macro_input!(item as ItemFn);
            let method_to_add = ::hyper::Method::$method_ident;
            match &mut args.method {
                Some(existing_methods) => {
                    existing_methods.push(method_to_add);
                }
                None => {
                    args.method = Some(vec![method_to_add]);
                }
            }
            route_handler(args, fn_item)
        }
    };
}

derive_route_macro!(get, GET);
derive_route_macro!(post, POST);
derive_route_macro!(put, PUT);
derive_route_macro!(delete, DELETE);
derive_route_macro!(patch, PATCH);
derive_route_macro!(head, HEAD);
derive_route_macro!(options, OPTIONS);
derive_route_macro!(trace, TRACE);
derive_route_macro!(connect, CONNECT);