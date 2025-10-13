use proc_macro2::Ident;
use quote::{quote};
use syn::{parse_quote, FnArg, ItemStruct};
use crate::toolkit::rout_arg::RouteFnArg;

pub fn build_struct_from_query(rfa: &Vec<RouteFnArg>, struct_name: Ident) -> (Option<ItemStruct>, Option<FnArg>) {
    let mut fields = Vec::new();
    let mut idents = Vec::new();
    for rfa in rfa {
        if rfa.mark.contains_key("query") {
            let name = &rfa.ident;
            let ty = rfa.ty.clone();
            idents.push(name.clone());
            fields.push(quote! {pub #name: #ty})
        }
    }
    if fields.len() > 0 {
        let q_struct: ItemStruct = parse_quote!{
            #[derive(::serde::Deserialize)]
            struct #struct_name {
                #(#fields),*
            }
        };
        let stmt: FnArg = parse_quote!{
            ::miko::handler::extractor::extractors::Query(#struct_name { #(#idents),* }): ::miko::handler::extractor::extractors::Query<#struct_name>
        };
        (Some(q_struct), Some(stmt))
    } else {
        (None, None)
    }
}