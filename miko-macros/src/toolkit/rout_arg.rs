use quote::{ToTokens, quote};
use std::collections::HashMap;
use std::fmt;
use std::fmt::{Debug, Formatter};
use syn::{Error, FnArg, LitStr, Meta, Type, TypePath};

#[derive(Clone)]
pub struct RouteFnArg {
    pub ident: syn::Ident,
    pub ty: Type,
    pub attrs: Vec<syn::Attribute>,
    pub is_option: bool,
    pub mark: HashMap<String, ArgAttr>,
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
#[derive(Debug, Clone)]
pub struct ArgAttr {
    pub map: HashMap<String, String>,
}

impl RouteFnArg {
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
                        let ident_str = attr.path().get_ident().unwrap().to_string();
                        mark.insert(ident_str, parse_attr(attr));
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

pub fn parse_attr(attr: &syn::Attribute) -> ArgAttr {
    let mut map = HashMap::new();
    let meta = &attr.meta;
    if let Meta::List(list) = meta {
        list.parse_nested_meta(|nmeta| {
            let key = nmeta.path.get_ident().unwrap().to_string();
            let val: Result<LitStr, Error> = nmeta.value().and_then(|v| v.parse());
            match val {
                Ok(val) => {
                    map.insert(key, val.value());
                }
                Err(_) => {
                    map.insert(key.clone(), key);
                }
            }
            Ok(())
        })
    } else if let Meta::Path(_path) = meta {
        Ok(())
    } else {
        panic!("not a list attr")
    }
    .expect("P IE");
    ArgAttr { map }
}

pub enum FnArgResult {
    Remove,
    Keep,
    Replace(FnArg),
    RemoveAttr,
}

pub fn build_dep_injector(rfa: &Vec<RouteFnArg>, dep_stmts: &mut Vec<proc_macro2::TokenStream>) {
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
