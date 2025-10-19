use crate::toolkit::attr::StrAttrMap;
use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use std::collections::HashMap;
use std::fmt;
use std::fmt::{Debug, Formatter};
use syn::{FnArg, Meta, Type, TypePath};
#[allow(dead_code)]
#[derive(Clone)]
pub struct RouteFnArg {
    pub ident: syn::Ident,
    pub ty: Type,
    pub attrs: Vec<syn::Attribute>,
    pub is_option: bool,
    pub mark: HashMap<String, StrAttrMap>,
    pub origin: FnArg,
}
impl Debug for RouteFnArg {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("RouteFnArg")
            .field("ident", &self.ident)
            .field("ty", &self.ty.to_token_stream().to_string())
            .field("mark", &self.mark)
            .finish()
    }
}

impl RouteFnArg {
    /// 从函数参数的 Punctuated 列表中解析出 RouteFnArg 向量。
    ///
    /// 该函数会处理 `FnArg::Typed` 参数，提取参数标识符、类型及自定义属性（如 `#[path]`、`#[body]`、`#[dep]`、`#[config]` 等），
    /// 并将解析结果打包为 `RouteFnArg`，以便后续宏展开使用。
    pub fn from_punctuated(
        inputs: &mut syn::punctuated::Punctuated<FnArg, syn::token::Comma>,
    ) -> Vec<RouteFnArg> {
        let mut out = Vec::new();
        for input in inputs {
            let input_clone = input.clone();
            match input {
                FnArg::Typed(pat) => {
                    let mut mark = HashMap::new();
                    let ident = match &*pat.pat {
                        syn::Pat::Ident(pat_ident) => Some(pat_ident.ident.clone()),
                        syn::Pat::TupleStruct(pat_ts) => {
                            let mut pat_ts = pat_ts.clone();
                            while let Some(syn::Pat::TupleStruct(pat_tsn)) = pat_ts.elems.first() {
                                pat_ts = pat_tsn.clone();
                            }
                            if let syn::Pat::Ident(pat_ident) = pat_ts.elems.first().unwrap() {
                                Some(pat_ident.ident.clone())
                            } else {
                                None
                            }
                        }
                        _ => None,
                    };
                    let (is_option, _option_ty) = is_option(&pat.ty);
                    if ident.is_none() {
                        panic!("RouteFnArg must have an ident");
                    }
                    for attr in &pat.attrs {
                        let mut sam = StrAttrMap::new();
                        if let Meta::List(list) = &attr.meta {
                            let tk = list.tokens.clone();
                            sam = syn::parse2(tk).unwrap();
                        }
                        let ident_str = attr.path().get_ident().unwrap().to_string();
                        mark.insert(ident_str, sam);
                    }
                    let rfa = RouteFnArg {
                        ident: ident.unwrap(),
                        ty: *pat.ty.clone(),
                        is_option,
                        attrs: vec![],
                        mark,
                        origin: input_clone,
                    };
                    out.push(rfa);
                }
                _ => {}
            }
        }
        out
    }
}

pub trait IntoFnArgs {
    fn gen_fn_args(&self, callback: impl Fn(&RouteFnArg) -> FnArgResult) -> Vec<FnArg>;
}

impl IntoFnArgs for Vec<RouteFnArg> {
    fn gen_fn_args(&self, callback: impl Fn(&RouteFnArg) -> FnArgResult) -> Vec<FnArg> {
        let mut out = Vec::new();
        for rfa in self {
            let mut clone = rfa.origin.clone();
            match callback(rfa) {
                FnArgResult::Remove => {}
                FnArgResult::Keep => out.push(rfa.origin.clone()),
                FnArgResult::Replace(new) => out.push(new),
                FnArgResult::RemoveAttr => {
                    if let FnArg::Typed(ref mut pat) = clone {
                        pat.attrs.clear();
                    }
                    out.push(clone);
                }
            }
        }
        out
    }
}

/// 判断给定类型是否为 `Option<T>`，
/// 返回 (is_option, Some(inner_type)) 或 (false, None)
pub fn is_option(ty: &Type) -> (bool, Option<Type>) {
    let Type::Path(TypePath { path, .. }) = ty else {
        return (false, None);
    };
    let last = path.segments.last().unwrap();
    if last.ident == "Option" {
        match &last.arguments {
            syn::PathArguments::AngleBracketed(args) => {
                let ty = args.args.first().unwrap();
                let ty = match ty {
                    syn::GenericArgument::Type(ty) => ty,
                    _ => panic!("Option must have a type"),
                };
                (true, Some(ty.clone()))
            }
            _ => (false, None),
        }
    } else {
        (false, None)
    }
}

/// 判断给定类型是否为 `Arc<T>`，
/// 返回 (is_arc, Some(inner_type)) 或 (false, None)
pub fn is_arc(ty: &Type) -> (bool, Option<Type>) {
    let Type::Path(TypePath { path, .. }) = ty else {
        return (false, None);
    };
    let last = path.segments.last().unwrap();
    if last.ident == "Arc" {
        match &last.arguments {
            syn::PathArguments::AngleBracketed(args) => {
                let ty = args.args.first().unwrap();
                let ty = match ty {
                    syn::GenericArgument::Type(ty) => ty,
                    _ => panic!("Arc must have a type"),
                };
                (true, Some(ty.clone()))
            }
            _ => (false, None),
        }
    } else {
        (false, None)
    }
}

#[allow(dead_code)]
pub enum FnArgResult {
    Remove,
    Keep,
    Replace(FnArg),
    RemoveAttr,
}

/// 为带有 `#[dep]` 标记的参数生成依赖注入的语句。
///
/// 该函数会为每个标记为 `dep` 的参数生成从全局依赖容器中异步获取该依赖的语句片段，并追加到 `dep_stmts`。
pub fn build_dep_injector(rfa: &Vec<RouteFnArg>, dep_stmts: &mut Vec<TokenStream>) {
    for rfa in rfa {
        if rfa.mark.contains_key("dep") {
            let dep_ty = rfa.ty.clone();
            let (is_arc, inner) = is_arc(&dep_ty);
            if !is_arc {
                panic!("dep param must be a Arc<T>");
            }
            let inner = inner.unwrap();
            let dep_ident = rfa.ident.clone();
            let stmt = quote! {
                let #dep_ident = __dep_container.get::<#inner>().await;
            };
            dep_stmts.push(stmt);
        }
    }
}

/// 为带有 `#[config(...)]` 的参数生成从配置读取并解析值的语句。
///
/// 支持基础类型 `String`, `u32`, `i32`, `bool`, `f64`，并根据参数是否为 `Option<T>` 决定是否解包或返回可选值。
pub fn build_config_value_injector(
    rfa: &Vec<RouteFnArg>,
    config_value_stmts: &mut Vec<TokenStream>,
) {
    for rfa in rfa {
        let mark_item = rfa.mark.get("config");
        if let Some(item) = mark_item {
            if let Some(path) = item.get_or_default("path") {
                let (is_option, inner) = is_option(&rfa.ty);
                let parse_expr;
                if is_option {
                    parse_expr =
                        prase_expr_by_type(&inner.unwrap(), path, rfa.ident.clone(), false);
                } else {
                    parse_expr = prase_expr_by_type(&rfa.ty, path, rfa.ident.clone(), true);
                }
                config_value_stmts.push(parse_expr);
            } else {
                panic!("config param must be like #[config(\"xx\")] or #[config(path=\"xx\")] ");
            }
        }
    }
}

fn prase_expr_by_type(ty: &Type, path: String, ident: syn::Ident, unwrap: bool) -> TokenStream {
    let expr = match ty {
        Type::Path(TypePath { path, .. }) => {
            let last = path.segments.last().unwrap();
            if last.ident == "String" {
                quote! {
                    v.as_str().map(|s| s.to_string())
                }
            } else if last.ident == "u32" {
                quote! {
                    v.as_integer().and_then(|i| i.try_into().ok())
                }
            } else if last.ident == "i32" {
                quote! {
                    v.as_integer().and_then(|i| i.try_into().ok())
                }
            } else if last.ident == "bool" {
                quote! {
                    v.as_bool()
                }
            } else if last.ident == "f64" {
                quote! {
                    v.as_float()
                }
            } else {
                panic!("unsupported config value type: {}", last.ident);
            }
        }
        _ => {
            panic!("unsupported config value type");
        }
    };
    if unwrap {
        quote! {
            let #ident = ::miko::app::config::get_config_value(#path).and_then(|v| {
                #expr
            }).unwrap();
        }
    } else {
        quote! {
            let #ident = ::miko::app::config::get_config_value(#path).and_then(|v| {
                #expr
            });
        }
    }
}
