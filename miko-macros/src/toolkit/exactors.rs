use crate::toolkit::rout_arg::RouteFnArg;
use proc_macro2::Ident;
use quote::quote;
use syn::{FnArg, ItemStruct, parse_quote};

/// 根据带有 `#[query]` 标记的参数构建一个临时的查询结构体和对应的提取器参数。
///
/// - `rfa`：函数参数解析出的 RouteFnArg 列表；
/// - `struct_name`：为生成的结构体提供标识符。
///
/// 返回值为 (Option<ItemStruct>, Option<FnArg>)，当没有带 `#[query]` 的参数时返回 (None, None)。
pub fn build_struct_from_query(
    rfa: &Vec<RouteFnArg>,
    struct_name: Ident,
) -> (Option<ItemStruct>, Option<FnArg>) {
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
        let q_struct: ItemStruct = parse_quote! {
            #[derive(::miko::serde::Deserialize)]
            struct #struct_name {
                #(#fields),*
            }
        };
        let stmt: FnArg = parse_quote! {
            ::miko::extractor::Query(#struct_name { #(#idents),* }): ::miko::extractor::Query<#struct_name>
        };
        (Some(q_struct), Some(stmt))
    } else {
        (None, None)
    }
}
