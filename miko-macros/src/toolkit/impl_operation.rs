use crate::toolkit::rout_arg::is_arc;
use proc_macro2::Ident;
use proc_macro2::TokenStream;
use quote::quote;
use syn::punctuated::Punctuated;
use syn::token::Comma;
use syn::{FnArg, ImplItem, ImplItemFn, Pat, PatIdent};

pub fn get_constructor(items: &Vec<ImplItem>) -> Option<&ImplItemFn> {
    for item in items {
        if let ImplItem::Fn(method) = item {
            if method.sig.ident == "new" {
                return Some(method);
            }
        }
    }
    None
}

pub fn inject_deps(
    args: &Punctuated<FnArg, Comma>,
    depend_get_stmts: &mut Vec<TokenStream>,
    arg_idents: &mut Vec<Ident>,
) {
    for arg in args {
        if let FnArg::Typed(pat) = arg {
            let arg_ident = match &*pat.pat {
                Pat::Ident(PatIdent { ident, .. }) => ident.clone(),
                _ => {
                    panic!("service method new argument must be Typed")
                }
            };
            let (is_arc, inner) = is_arc(&*pat.ty);
            if !is_arc {
                panic!("service method new argument must be Arc<T>")
            }
            let pat_ident = &inner.unwrap();
            if depend_get_stmts.is_empty() {
                depend_get_stmts.push(quote! {
                    let container = ::miko::dep::CONTAINER.get().unwrap().read().await;
                })
            }
            depend_get_stmts.push(quote! {
                let #arg_ident = container.get::<#pat_ident>().await.clone();
            });
            arg_idents.push(arg_ident);
        }
    }
}
