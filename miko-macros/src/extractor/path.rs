use crate::toolkit::rout_arg::{FnArgResult, RouteFnArg};
use syn::parse_quote;

/// 处理带有 `#[path]` 标记的参数，将其替换为 Path 提取器形式（`Path(ident): Path<T>`）。
///
/// 若参数未标记为 `path` 则返回 `FnArgResult::Remove`，表示宏应去掉该参数。
pub fn deal_with_path_attr(rfa: &RouteFnArg) -> FnArgResult {
    if rfa.mark.contains_key("path") {
        let ident = rfa.ident.clone();
        let ty = rfa.ty.clone();
        FnArgResult::Replace(parse_quote!(
            ::miko::extractor::Path(#ident): ::miko::extractor::Path<#ty>
        ))
    } else {
        FnArgResult::Remove
    }
}
