use crate::toolkit::rout_arg::{FnArgResult, RouteFnArg};
use syn::parse_quote;

pub fn deal_with_path_attr(rfa: &RouteFnArg) -> FnArgResult {
    if rfa.mark.contains_key("path") {
        let ident = rfa.ident.clone();
        let ty = rfa.ty.clone();
        FnArgResult::Replace(parse_quote!(
            ::miko::handler::extractor::extractors::Path(#ident): ::miko::handler::extractor::extractors::Path<#ty>
        ))
    } else {
        FnArgResult::Remove
    }
}
