use crate::toolkit::rout_arg::{FnArgResult, RouteFnArg};
use syn::{Type, parse_quote};

/// 处理带有 `#[body]` 标记或默认按类型推断的 body 参数。
///
/// - 如果显式标注 `str` 则保持原始类型（字符串）；
/// - 否则默认替换为 Json 提取器 `Json<T>`；
/// - 对于未标注但类型为 `String` 的参数也直接保持字符串类型。
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
