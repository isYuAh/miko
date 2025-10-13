use syn::{parse_quote, Type};
use crate::toolkit::rout_arg::{FnArgResult, RouteFnArg};

pub fn deal_with_body_attr(rfa: &RouteFnArg) -> FnArgResult {
    if rfa.mark.contains_key("body") {
        let ident = rfa.ident.clone();
        let ty = rfa.ty.clone();
        let map = rfa.mark.get("body");
        match map {
            Some(map) => {
                if map.map.contains_key("str") {
                    FnArgResult::Replace(parse_quote!(
                        #ident: #ty
                    ))
                } else {
                    FnArgResult::Replace(parse_quote!(
                        ::miko::handler::extractor::extractors::Json(#ident): ::miko::handler::extractor::extractors::Json<#ty>
                    ))
                }
            }
            // 自动判断
            None => {
                // String 直接转
                if is_string_type(&ty) {
                    FnArgResult::Replace(parse_quote!(
                        #ident: #ty
                    ))
                } else {
                    FnArgResult::Replace(parse_quote!(
                        ::miko::handler::extractor::extractors::Json(#ident): ::miko::handler::extractor::extractors::Json<#ty>
                    ))
                }
            }
        }
    } else {
        FnArgResult::Remove
    }
}

fn is_string_type(ty: &Type) -> bool {
    match ty {
        Type::Path(path) => {
            path.path.segments.last().unwrap().ident == "String" || path.path.is_ident("String")
        }
        _ => false,
    }
}